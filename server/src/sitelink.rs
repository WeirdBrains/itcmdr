//! v0.19.0: the site side of the adminsvr link (docs/design/ADMINSVR.md).
//!
//! The site dials adminsvr (pinned TLS + the mTLS client certificate from site enrollment) and
//! sends its status and alerts. **Everything goes through `Redactor`** (decision 19): a
//! deny-by-default layer that builds each outbound message field by field from an allowlist.
//! Only opaque site-scoped ids (HMAC with a site-local secret that never leaves the site), alert
//! codes, severity, plugin package names, timestamps, counts and numeric rollups go up. Never:
//! hostnames, IPs, domains, usernames, paths, share names, message text, log lines, backup bytes
//! or key material. Display names of machines only with the site owner's opt-in.
//!
//! Commands from adminsvr carry the acting user's signature and the adminsvr root signature; the
//! site checks both against the root key pinned at enrollment, refuses never-remote kinds
//! (restore, keys, auth mode, ...) and anything off its local allowlist, and audits every one.
//! Backups never leave the site: `backup.start` runs a local backup; there is no restore.
use crate::config::ServerConfig;
use crate::db::DbPool;
use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::SigningKey;
use itcmdr_adminlink::pb;
use itcmdr_agentlink::tls::{self, Pem};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

/// Commands allowed by default (the site owner can narrow this locally).
/// v0.20.0 adds the schedule, setting and plugin kinds (ADMINSVR.md section 5 defaults).
pub const DEFAULT_ALLOWED: &[&str] = &[
    "backup.start",
    "alert.ack",
    "alert.resolve",
    "setting.set",
    "schedule.run",
    "schedule.pause",
    "schedule.resume",
    "plugin.enable",
    "plugin.disable",
    "plugin.install",
];
/// The v0.19 default: a site still on exactly this list gets the v0.20.0 default on load.
const DEFAULT_ALLOWED_V019: &[&str] = &["backup.start", "alert.ack", "alert.resolve"];

/// Link state, in the keys folder (never sent anywhere except the public key and the CSR).
#[derive(Serialize, Deserialize, Clone)]
pub struct State {
    pub site_id: String,
    pub addr: String,
    pub pin: String,
    pub root_public_key: String,
    pub site_key: String,
    pub client: ClientCert,
    pub ca_cert_pem: String,
    /// HMAC secret for opaque ids (site-local)
    pub id_secret: String,
    #[serde(default)]
    pub share_display_names: bool,
    #[serde(default = "default_allowed")]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub seq: u64,
    pub heartbeat_secs: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientCert {
    pub cert: String,
    pub key: String,
}

fn default_allowed() -> Vec<String> {
    DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect()
}

pub fn state_path(cfg: &ServerConfig) -> PathBuf {
    cfg.layout().keys().join("adminsvr-link.json")
}

pub fn load(cfg: &ServerConfig) -> Option<State> {
    load_raw(cfg).map(|mut s| {
        if s.allowed
            .iter()
            .map(String::as_str)
            .eq(DEFAULT_ALLOWED_V019.iter().copied())
        {
            s.allowed = default_allowed();
        }
        s
    })
}

fn load_raw(cfg: &ServerConfig) -> Option<State> {
    serde_json::from_slice(&std::fs::read(state_path(cfg)).ok()?).ok()
}

fn save(cfg: &ServerConfig, s: &State) -> Result<()> {
    let p = state_path(cfg);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(&tmp, &p)?;
    Ok(())
}

static SAVE_LOCK: Mutex<()> = Mutex::new(());

fn update(cfg: &ServerConfig, f: impl FnOnce(&mut State)) -> Result<State> {
    let _g = SAVE_LOCK.lock();
    let mut s = load(cfg).ok_or_else(|| anyhow!("this site is not linked to an adminsvr"))?;
    f(&mut s);
    save(cfg, &s)?;
    Ok(s)
}

/// Enroll this site with a one-time site token (in person, `itcmdr-cli adminsvr enroll`).
pub async fn enroll(pool: &DbPool, cfg: &ServerConfig, token: &str, actor: &str) -> Result<State> {
    if load(cfg).is_some() {
        bail!("already linked to an adminsvr; run `itcmdr-cli adminsvr unlink` first");
    }
    let t = itcmdr_adminlink::SiteToken::decode(token)?;
    let ch = tls::connect(&t.addr, &t.pin, None)
        .await
        .context("connect to adminsvr")?;
    let mut c = pb::site_link_client::SiteLinkClient::new(ch);
    // Check adminsvr's root key before the token is used (nothing is recorded if it differs).
    let p = c
        .prepare(pb::SitePrepareRequest {
            token_secret: t.secret.clone(),
        })
        .await
        .map_err(|s| anyhow!("adminsvr refused: {}", s.message()))?
        .into_inner();
    if p.root_public_key != t.root {
        bail!("adminsvr's root key is not the one in the site token; refusing to link");
    }
    let key = itcmdr_keys::new_keypair();
    let (csr, tls_key) = tls::new_csr("itcmdr-site")?;
    let r = c
        .enroll(pb::SiteEnrollRequest {
            token_secret: t.secret.clone(),
            site_public_key: hex::encode(key.verifying_key().as_bytes()),
            csr_pem: csr,
            version: env!("CARGO_PKG_VERSION").into(),
        })
        .await
        .map_err(|s| anyhow!("adminsvr refused: {}", s.message()))?
        .into_inner();
    if r.root_public_key != t.root || r.ca_cert_pem != p.ca_cert_pem {
        bail!("adminsvr changed its keys during enrollment; refusing");
    }
    let s = State {
        site_id: r.site_id,
        addr: t.addr,
        pin: t.pin,
        root_public_key: t.root,
        site_key: hex::encode(key.to_bytes()),
        client: ClientCert {
            cert: r.client_cert_pem,
            key: tls_key,
        },
        ca_cert_pem: r.ca_cert_pem,
        id_secret: hex::encode(itcmdr_keys::random_seed().as_ref()),
        share_display_names: false,
        allowed: default_allowed(),
        seq: 0,
        heartbeat_secs: (r.heartbeat_secs as i64).max(5),
    };
    {
        let _g = SAVE_LOCK.lock();
        save(cfg, &s)?;
    }
    crate::db::insert_audit(
        pool,
        actor,
        "adminsvr.enroll",
        &format!("site {}", s.site_id),
    )
    .await?;
    Ok(s)
}

pub async fn unlink(pool: &DbPool, cfg: &ServerConfig, actor: &str) -> Result<()> {
    let s = load(cfg).ok_or_else(|| anyhow!("not linked"))?;
    std::fs::remove_file(state_path(cfg))?;
    crate::db::insert_audit(
        pool,
        actor,
        "adminsvr.unlink",
        &format!("site {}", s.site_id),
    )
    .await?;
    Ok(())
}

/// Site owner, in person: share machine display names with adminsvr (off by default).
pub async fn set_share_names(
    pool: &DbPool,
    cfg: &ServerConfig,
    on: bool,
    actor: &str,
) -> Result<State> {
    let s = update(cfg, |s| s.share_display_names = on)?;
    crate::db::insert_audit(
        pool,
        actor,
        "adminsvr.share-names",
        if on { "on" } else { "off" },
    )
    .await?;
    Ok(s)
}

/// Site owner, in person: the commands adminsvr may send (never-remote kinds are refused anyway).
pub async fn set_allowed(
    pool: &DbPool,
    cfg: &ServerConfig,
    kinds: &[String],
    actor: &str,
) -> Result<State> {
    for k in kinds {
        if !itcmdr_adminlink::COMMANDS.contains(&k.as_str()) {
            bail!(
                "unknown command {k} (known: {})",
                itcmdr_adminlink::COMMANDS.join(", ")
            );
        }
    }
    let s = update(cfg, |s| s.allowed = kinds.to_vec())?;
    crate::db::insert_audit(pool, actor, "adminsvr.allow", &kinds.join(",")).await?;
    Ok(s)
}

// ---------------------------------------------------------------- redaction

/// The one redaction layer (deny by default). Built fresh for each pass from site data.
pub struct Redactor {
    secret: Vec<u8>,
    share_names: bool,
    /// lowercase strings that must never leave: machine names, host name, usernames, paths...
    sensitive: Vec<String>,
    machines: Vec<String>,
    plugins: Vec<String>,
    /// code words allowed through as-is (lowercase): the fixed list, check ids, plugin names
    vocab: std::collections::HashSet<String>,
}

/// Alert code words that may go up unchanged. Anything else in a key is replaced by an opaque id.
pub const VOCAB: &[&str] = &[
    "agent",
    "agent-online",
    "agent-nonresponsive",
    "agent-late",
    "agent-cert",
    "agent-clock",
    "agent-run",
    "agent-plugin",
    "agent-queue",
    "plugin",
    "repair",
    "quarantine",
    "backup",
    "nightly",
    "manual",
    "remote",
    "check",
    "fail",
    "failed",
    "error",
    "errors",
    "exception",
    "timeout",
    "verify",
    "execute",
    "log",
    "disk-low",
    "disk-full",
    "service-down",
    "cert-expiry",
    "restore",
    "schedule",
    "missed",
    "site-unreachable",
];

const SOURCES: &[&str] = &[
    "agent",
    "plugin",
    "check",
    "repair",
    "backup",
    "quarantine",
    "server",
    "schedule",
    "adminsvr",
];
const LEVELS: &[&str] = &["critical", "cautious", "informational"];
const STATES: &[&str] = &["open", "acked", "silenced", "resolved"];
const AGENT_STATES: &[&str] = &[
    "online",
    "late",
    "non_responsive",
    "enrolled",
    "maintenance",
    "revoked",
    "offline",
];

fn pick(v: &str, allowed: &[&str], other: &str) -> String {
    if allowed.contains(&v) {
        v.to_string()
    } else {
        other.to_string()
    }
}

pub(crate) fn plain_token(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 40
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-' || *c == b'_')
}

pub(crate) fn version_token(s: &str) -> String {
    if s.len() <= 32
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c.is_ascii_lowercase())
    {
        s.to_string()
    } else {
        String::new()
    }
}

impl Redactor {
    pub fn new(
        secret: &str,
        share_names: bool,
        sensitive: Vec<String>,
        machines: Vec<String>,
        plugins: Vec<String>,
        check_ids: Vec<String>,
    ) -> Self {
        let vocab = VOCAB
            .iter()
            .map(|s| s.to_string())
            .chain(check_ids.iter().map(|c| c.to_lowercase()))
            .chain(plugins.iter().map(|c| c.to_lowercase()))
            .collect();
        let mut s: Vec<String> = sensitive
            .into_iter()
            .chain(machines.iter().cloned())
            .map(|x| x.trim().to_lowercase())
            .filter(|x| x.len() >= 3)
            .collect();
        s.sort();
        s.dedup();
        Redactor {
            secret: secret.as_bytes().to_vec(),
            share_names,
            sensitive: s,
            machines: machines.iter().map(|m| m.to_lowercase()).collect(),
            plugins,
            vocab,
        }
    }

    /// Everything the site knows that must not leave: names, paths, host, users.
    pub async fn for_site(pool: &DbPool, cfg: &ServerConfig, st: &State) -> Self {
        let mut machines: Vec<String> = sqlx::query_scalar("SELECT DISTINCT machine FROM agents")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        machines.extend(
            sqlx::query_scalar::<_, String>(
                "SELECT DISTINCT machine FROM alerts WHERE machine <> ''",
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default(),
        );
        machines.extend(
            sqlx::query_scalar::<_, String>("SELECT DISTINCT computer FROM runs")
                .fetch_all(pool)
                .await
                .unwrap_or_default(),
        );
        machines.push(crate::runtime::hostname());
        let mut sens: Vec<String> = vec![];
        for q in ["SELECT name FROM users", "SELECT name FROM clients"] {
            sens.extend(
                sqlx::query_scalar::<_, String>(q)
                    .fetch_all(pool)
                    .await
                    .unwrap_or_default(),
            );
        }
        for p in [
            Some(cfg.layout().root.clone()),
            Some(cfg.logs_dir.clone()),
            cfg.backup.dir.clone(),
        ]
        .into_iter()
        .flatten()
        {
            let p = p.display().to_string();
            sens.extend(
                p.split(['/', '\\'])
                    .filter(|x| x.len() >= 4)
                    .map(String::from),
            );
            sens.push(p);
        }
        if let Ok(u) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
            sens.push(u);
        }
        let plugins: Vec<String> = crate::packages::list(pool, &cfg.layout())
            .await
            .map(|v| v.into_iter().map(|p| p.name).collect())
            .unwrap_or_default();
        // The opt-in is read from disk on every pass, so turning it off takes effect at once.
        let share = load(cfg).is_some_and(|x| x.share_display_names);
        let checks: Vec<String> = sqlx::query_scalar("SELECT DISTINCT check_id FROM checks")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        Redactor::new(&st.id_secret, share, sens, machines, plugins, checks)
    }

    /// Stable opaque id: keyed hash (secret, kind \0 value), 16 hex.
    pub fn pid(&self, kind: &str, v: &str) -> String {
        let k = itcmdr_keys::hkdf32(
            &self.secret,
            format!("{kind}\0{}", v.to_lowercase()).as_bytes(),
            "itcmdr/site-pid/v1",
        );
        format!(
            "{}-{}",
            &kind[..1],
            &hex::encode(sha2::Sha256::digest(k.as_ref()))[..16]
        )
    }

    /// An installed plugin's package name as it may go up (names of other things: opaque).
    pub fn plugin(&self, name: &str) -> String {
        if self.plugins.iter().any(|x| x == name) && plain_token(name) && !self.is_sensitive(name) {
            name.to_string()
        } else {
            self.pid("plugin", name)
        }
    }

    pub fn share_names(&self) -> bool {
        self.share_names
    }

    fn is_sensitive(&self, seg: &str) -> bool {
        let l = seg.to_lowercase();
        self.sensitive.iter().any(|s| {
            l.contains(s.as_str()) || (l.len() >= 3 && s.contains(&l) && self.machines.contains(&l))
        })
    }

    /// Alert key -> code: machine names become opaque ids; plain tokens stay; anything else is hashed.
    pub fn code(&self, key: &str) -> String {
        key.split(':')
            .take(8)
            .map(|seg| {
                if self.machines.contains(&seg.to_lowercase()) {
                    self.pid("machine", seg)
                } else if seg.len() <= 40
                    && seg
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                    && self.vocab.contains(&seg.to_lowercase())
                    && !self.is_sensitive(seg)
                {
                    seg.to_string()
                } else {
                    self.pid("x", seg)
                }
            })
            .collect::<Vec<_>>()
            .join(":")
    }

    fn display(&self, machine: &str) -> String {
        if self.share_names
            && !machine.is_empty()
            && machine.len() <= 64
            && machine
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            machine.to_string()
        } else {
            String::new()
        }
    }

    pub fn alert(&self, a: &crate::alerts::Alert) -> pb::SiteAlert {
        let plugin = if a.source == "plugin" || a.key.starts_with("plugin:") {
            a.key
                .split(':')
                .nth(1)
                .filter(|p| self.plugins.iter().any(|x| x == p) && plain_token(p))
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };
        pb::SiteAlert {
            id: self.pid("alert", &a.id),
            code: self.code(&a.key),
            severity: pick(&a.level, LEVELS, "informational"),
            state: pick(&a.state, STATES, "open"),
            source: pick(&a.source, SOURCES, "other"),
            plugin,
            machine: if a.machine.is_empty() {
                String::new()
            } else {
                self.pid("machine", &a.machine)
            },
            first_seen_ms: a.first_seen_ms,
            last_seen_ms: a.last_seen_ms,
            count: a.count,
            machine_display_name: self.display(&a.machine),
        }
    }

    pub fn machine(&self, a: &crate::agents::Agent) -> pb::SiteMachine {
        pb::SiteMachine {
            id: self.pid("machine", &a.machine),
            state: pick(&a.state, AGENT_STATES, "other"),
            last_seen_ms: a.last_seen_ms,
            version: version_token(&a.version),
            display_name: self.display(&a.machine),
        }
    }
}

// ---------------------------------------------------------------- what goes up

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

static STARTED: std::sync::OnceLock<i64> = std::sync::OnceLock::new();

/// Backup metadata only (decision 18): age, size and last result. Never the archive.
pub async fn backup_status(pool: &DbPool, cfg: &ServerConfig) -> (i64, i64, String) {
    let dir = crate::settings::effective_backup(pool, cfg)
        .await
        .ok()
        .and_then(|b| b.dir)
        .unwrap_or_else(|| cfg.layout().backups());
    let newest = crate::backup::list(&dir).ok().and_then(|v| {
        v.into_iter()
            .filter_map(|b| crate::backup::parse_name_ms(&b.name).map(|ms| (ms, b.size)))
            .max_by_key(|x| x.0)
    });
    let failing: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM alerts WHERE key LIKE 'backup:%' AND state IN ('open','acked')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    match newest {
        Some((ms, size)) => (
            (now_ms() - ms) / 1000,
            size as i64,
            if failing > 0 { "failed" } else { "ok" }.into(),
        ),
        None => (-1, 0, if failing > 0 { "failed" } else { "none" }.into()),
    }
}

pub async fn status(pool: &DbPool, cfg: &ServerConfig, r: &Redactor) -> pb::SiteStatus {
    let agents = crate::agents::list(pool, false).await.unwrap_or_default();
    let n = |s: &str| agents.iter().filter(|a| a.state == s).count() as i32;
    let (age, size, last) = backup_status(pool, cfg).await;
    let counts = crate::alerts::open_counts(pool).await.unwrap_or_default();
    let mode = crate::api::AuthMode::from_config(&cfg.auth_mode);
    pb::SiteStatus {
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: (now_ms() - *STARTED.get_or_init(now_ms)) / 1000,
        auth_mode: pick(
            &format!("{mode:?}").to_lowercase(),
            &["signed", "both", "token"],
            "other",
        ),
        backup_age_secs: age,
        backup_size_bytes: size,
        backup_last_result: last,
        agents_online: n("online"),
        agents_late: n("late"),
        agents_non_responsive: n("non_responsive"),
        agents_other: agents.len() as i32 - n("online") - n("late") - n("non_responsive"),
        open_alerts: counts.to_vec(),
        machines: agents.iter().map(|a| r.machine(a)).collect(),
        cutover_state: String::new(),
        ..summaries(pool, cfg).await
    }
}

/// v0.20.0 metrics summaries: counts and durations of the last 24 h, the data volume. Numbers only.
/// (runs, failed, avg secs, max secs) rows from agent_runs/plugin_runs.
type RunSummary = (i64, Option<i64>, Option<i64>, f64, i64);

pub async fn summaries(pool: &DbPool, cfg: &ServerConfig) -> pb::SiteStatus {
    let since = now_ms() - 86_400_000;
    // agent_runs (Rust agent) and plugin_runs (on the server): finished, failed, durations
    let q = "SELECT count(*), sum(CASE WHEN status <> 'ok' THEN 1 ELSE 0 END), \
             sum(CASE WHEN status = 'error' THEN 1 ELSE 0 END), \
             coalesce(avg(finished_ms - started_ms), 0), coalesce(max(finished_ms - started_ms), 0) \
             FROM (SELECT status, started_ms, finished_ms FROM agent_runs WHERE finished_ms >= ? \
                   UNION ALL SELECT status, started_ms, finished_ms FROM plugin_runs WHERE finished_ms >= ?)";
    let row: Option<RunSummary> = sqlx::query_as(q)
        .bind(since)
        .bind(since)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
    let (n, failed, errors, avg, max) = row.unwrap_or((0, None, None, 0.0, 0));
    let opened: i64 = sqlx::query_scalar("SELECT count(*) FROM alerts WHERE first_seen_ms >= ?")
        .bind(since)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let dir = cfg.layout().root;
    let (free, total) = (
        fs4::available_space(&dir).map(|v| v as i64).unwrap_or(-1),
        fs4::total_space(&dir).map(|v| v as i64).unwrap_or(-1),
    );
    pb::SiteStatus {
        disk_free_bytes: free,
        disk_total_bytes: total,
        runs_24h: n as i32,
        runs_failed_24h: failed.unwrap_or(0) as i32,
        run_secs_avg_24h: (avg / 1000.0).round() as i64,
        run_secs_max_24h: max / 1000,
        plugin_errors_24h: errors.unwrap_or(0) as i32,
        alerts_opened_24h: opened as i32,
        ..Default::default()
    }
}

/// Test hook: every outbound SiteMessage, as sent (after redaction and signing).
static TAP: Mutex<Option<mpsc::UnboundedSender<pb::SiteMessage>>> = Mutex::new(None);
pub fn set_tap(t: Option<mpsc::UnboundedSender<pb::SiteMessage>>) {
    *TAP.lock() = t;
}

struct Out {
    key: SigningKey,
    seq: u64,
    tx: mpsc::Sender<pb::SiteMessage>,
}

impl Out {
    async fn send(&mut self, body: pb::site_message::Body) -> Result<()> {
        self.seq += 1;
        let m = itcmdr_adminlink::sign_site(
            &self.key,
            pb::SiteMessage {
                seq: self.seq,
                ts_ms: now_ms(),
                sig: String::new(),
                body: Some(body),
            },
        );
        if let Some(t) = TAP.lock().as_ref() {
            let _ = t.send(m.clone());
        }
        self.tx.send(m).await.map_err(|_| anyhow!("link closed"))
    }
}

// ---------------------------------------------------------------- commands

/// Check and run one command. Returns the result code; audits on the site with the user's identity.
pub async fn apply(
    pool: &DbPool,
    cfg: &ServerConfig,
    st: &State,
    r: &Redactor,
    c: &pb::Command,
    fetch: Option<&crate::remote::Fetcher>,
) -> (bool, String, Vec<pb::ConfigChange>) {
    let root = match itcmdr_keys::parse_public_key(&st.root_public_key) {
        Ok(k) => k,
        Err(_) => return (false, "refused".into(), vec![]),
    };
    let mut code = itcmdr_adminlink::check_command(&root, c, now_ms()).to_string();
    if code == "ok" && c.site_id != st.site_id {
        code = "refused".into();
    }
    if code == "ok" && !st.allowed.iter().any(|k| k == &c.kind) {
        code = "not-allowed".into();
    }
    // v0.20.0 two-person rule, checked here whatever adminsvr did (a dry run changes nothing).
    if code == "ok"
        && !c.dry_run
        && itcmdr_adminlink::TWO_PERSON.contains(&c.kind.as_str())
        && crate::settings::get_bool(pool, None, "adminsvr.requireTwoPerson").await
        && !matches!(itcmdr_adminlink::check_approver(&root, c), Ok(true))
    {
        code = "needs-approval".into();
    }
    let actor = format!("adminsvr:{}:{}", st.site_id, c.user_id);
    let kind: String = c
        .kind
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '.' || *ch == '-')
        .take(40)
        .collect();
    if code != "ok" {
        let _ = crate::db::insert_audit(
            pool,
            &actor,
            "adminsvr.command.refused",
            &format!("{kind} id={} code={code}", c.id),
        )
        .await;
        return (false, code, vec![]);
    }
    let mut preview = Vec::new();
    let mut code_out: Option<&'static str> = None;
    let res: Result<()> = async {
        if c.dry_run
            && matches!(
                c.kind.as_str(),
                "backup.start" | "alert.ack" | "alert.resolve"
            )
        {
            preview.push(pb::ConfigChange {
                target: c.kind.clone(),
                before: String::new(),
                after: "would run".into(),
            });
            return Ok(());
        }
        match c.kind.as_str() {
            "backup.start" => {
                let out = crate::backup::create(pool, cfg, "manual", None).await;
                crate::alerts::from_backup(
                    pool,
                    "manual",
                    out.as_ref().map(|_| ()).map_err(|e| e.to_string()),
                )
                .await;
                out.map(|_| ())
            }
            "alert.ack" | "alert.resolve" => {
                let want = c.args.get("alert_id").cloned().unwrap_or_default();
                let all = crate::alerts::list(
                    pool,
                    &crate::alerts::Filter {
                        state: Some("all".into()),
                        limit: 5000,
                        ..Default::default()
                    },
                )
                .await?;
                let a = all
                    .iter()
                    .find(|a| r.pid("alert", &a.id) == want)
                    .ok_or_else(|| anyhow!("no such alert"))?;
                if c.kind == "alert.ack" {
                    crate::alerts::ack(pool, &a.id, &actor).await?;
                } else {
                    crate::alerts::resolve(pool, &a.id, &actor).await?;
                }
                Ok(())
            }
            k if k.contains('.') => {
                let o = crate::remote::run(pool, cfg, r, c, fetch, &actor).await?;
                preview = o.preview;
                if o.code != "ok" {
                    code_out = Some(o.code);
                }
                Ok(())
            }
            _ => bail!("unknown"),
        }
    }
    .await;
    let (ok, code) = match (&res, code_out) {
        (Ok(()), Some(c)) => (false, c.to_string()),
        (Ok(()), None) => (true, "ok".to_string()),
        (Err(e), _) => {
            tracing::warn!(error=%e, kind=%kind, "adminsvr command failed");
            (false, "failed".to_string())
        }
    };
    let _ = crate::db::insert_audit(
        pool,
        &actor,
        &format!(
            "adminsvr.command.{kind}{}",
            if c.dry_run { ".dry-run" } else { "" }
        ),
        &format!(
            "id={} code={code} user_key={} approver={}",
            c.id,
            &c.user_key[..c.user_key.len().min(16)],
            &c.approver_id[..c.approver_id.len().min(40)]
        ),
    )
    .await;
    (ok, code, preview)
}

// ---------------------------------------------------------------- the link loop

#[derive(PartialEq, Clone)]
struct Seen(String, i64, i64);

async fn session(pool: &DbPool, cfg: &ServerConfig, st: &State) -> Result<()> {
    let client = Pem {
        cert: st.client.cert.clone(),
        key: st.client.key.clone(),
    };
    let ch = tls::connect(&st.addr, &st.pin, Some(&client)).await?;
    let mut c = pb::site_link_client::SiteLinkClient::new(ch);
    let (tx, rx) = mpsc::channel::<pb::SiteMessage>(256);
    let key = itcmdr_keys::key_from_seed(&hex::decode(&st.site_key)?)?;
    let mut out = Out {
        key,
        seq: st.seq.max(now_ms() as u64),
        tx,
    };
    out.send(pb::site_message::Body::Hello(pb::SiteHello {
        site_id: st.site_id.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
        share_display_names: st.share_display_names,
    }))
    .await?;
    let mut inbound = c
        .link(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await?
        .into_inner();
    tracing::info!(site=%st.site_id, "adminsvr link up");
    let mut seen: HashMap<String, Seen> = HashMap::new();
    let hb = crate::settings::get_i64(pool, None, "adminsvr.heartbeatSecs").await;
    let mut status_t = tokio::time::interval(Duration::from_secs(
        if hb > 0 { hb } else { st.heartbeat_secs }.max(5) as u64,
    ));
    let mut alert_t = tokio::time::interval(Duration::from_secs(3));
    let (done_tx, mut done_rx) = mpsc::channel::<(String, bool, String, Vec<pb::ConfigChange>)>(16);
    let fetcher = crate::remote::Fetcher {
        client: c.clone(),
        key: out.key.clone(),
        site_id: st.site_id.clone(),
    };
    // v0.20.0 config and plugin documents: on link-up, then whenever they change.
    let (mut cfg_v, mut plug_v) = (String::new(), String::new());
    let mut docs_t = tokio::time::interval(Duration::from_secs(15));
    loop {
        tokio::select! {
            m = inbound.message() => {
                let m = m?.ok_or_else(|| anyhow!("adminsvr closed the link"))?;
                match m.body {
                    Some(pb::admin_message::Body::Ping(_)) => out.send(pb::site_message::Body::Pong(pb::Pong { ts_ms: now_ms() })).await?,
                    Some(pb::admin_message::Body::Command(cmd)) => {
                        let (pool, cfg, st, done) = (pool.clone(), cfg.clone(), st.clone(), done_tx.clone());
                        // allowlist and opt-in as they are now on disk (the site owner may have changed them)
                        let st = load(&cfg).unwrap_or(st);
                        let red = Redactor::for_site(&pool, &cfg, &st).await;
                        let f = fetcher.clone();
                        tokio::spawn(async move {
                            let (ok, code, preview) = apply(&pool, &cfg, &st, &red, &cmd, Some(&f)).await;
                            let _ = done.send((cmd.id, ok, code, preview)).await;
                        });
                    }
                    None => {}
                }
            }
            Some((id, ok, code, preview)) = done_rx.recv() => {
                out.send(pb::site_message::Body::Result(pb::CommandResult { command_id: id, ok, code, preview })).await?;
                docs_t.reset_immediately();
            }
            _ = docs_t.tick() => {
                let st = load(cfg).unwrap_or_else(|| st.clone());
                let r = Redactor::for_site(pool, cfg, &st).await;
                let d = crate::remote::config_doc(pool, cfg, &st, &r).await;
                if d.version != cfg_v {
                    cfg_v = d.version.clone();
                    out.send(pb::site_message::Body::Config(d)).await?;
                }
                let d = crate::remote::plugins_doc(pool, cfg, &r).await;
                if d.version != plug_v {
                    plug_v = d.version.clone();
                    out.send(pb::site_message::Body::Plugins(d)).await?;
                }
            }
            _ = status_t.tick() => {
                if !crate::settings::get_bool(pool, None, "adminsvr.enabled").await || load(cfg).is_none() {
                    tracing::info!("adminsvr link closed (adminsvr.enabled=false or unlinked)");
                    return Ok(());
                }
                let r = Redactor::for_site(pool, cfg, st).await;
                out.send(pb::site_message::Body::Status(status(pool, cfg, &r).await)).await?;
            }
            _ = alert_t.tick() => {
                let since = now_ms() - 7 * 86_400_000;
                let all = crate::alerts::list(pool, &crate::alerts::Filter { state: Some("all".into()), limit: 2000, ..Default::default() }).await.unwrap_or_default();
                let r = Redactor::for_site(pool, cfg, st).await;
                for a in all.iter().filter(|a| a.last_seen_ms >= since || a.state != "resolved") {
                    let s = Seen(a.state.clone(), a.last_seen_ms, a.count);
                    if seen.get(&a.id) != Some(&s) {
                        out.send(pb::site_message::Body::Alert(r.alert(a))).await?;
                        seen.insert(a.id.clone(), s);
                    }
                }
            }
        }
        if out.seq.is_multiple_of(50) {
            let s = out.seq;
            let _ = update(cfg, |x| x.seq = s);
        }
    }
}

/// Run the link while the site is enrolled and `adminsvr.enabled`. Backoff 1 s -> 5 min.
pub async fn run(pool: DbPool, cfg: ServerConfig) {
    STARTED.get_or_init(now_ms);
    let mut backoff = 1u64;
    loop {
        let enabled = crate::settings::get_bool(&pool, None, "adminsvr.enabled").await;
        match load(&cfg) {
            Some(st) if enabled => {
                let t0 = std::time::Instant::now();
                if let Err(e) = session(&pool, &cfg, &st).await {
                    tracing::warn!(error=%format!("{e:#}"), "adminsvr link down; the site keeps running standalone");
                }
                if t0.elapsed() > Duration::from_secs(60) {
                    backoff = 1;
                }
                let jitter = (now_ms() as u64 % 1000) as f64 / 1000.0;
                tokio::time::sleep(Duration::from_secs_f64(
                    backoff as f64 * (1.0 + jitter * 0.3),
                ))
                .await;
                backoff = (backoff * 2).min(300);
            }
            _ => tokio::time::sleep(Duration::from_secs(10)).await,
        }
    }
}
