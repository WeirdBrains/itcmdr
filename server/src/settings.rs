//! Settings (decision 15): every tunable value is a typed, validated key.
//!
//! - **Server keys** (snake_case, e.g. `backup.hour`): DB override > server.json > built-in default.
//! - **Agent keys** (site.json paths, e.g. `agent.quietMinutes`): served centrally in
//!   `<logs_dir>/agent-settings.json` (global values + per-machine overrides). The agent merges
//!   built-in default < central global < central machine < its local site.json.
//!
//! Overrides live in the encrypted DB (`server_meta` key `settings.v2`). Changes go through
//! `set` / `unset` (gRPC `UpdateSetting`, `itcmdr-cli config set|unset`), are validated first
//! and audited by the caller.
use crate::config::{BackupConfig, ServerConfig};
use crate::db::DbPool;
use anyhow::{anyhow, bail, ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

const STORE: &str = "settings.v2";
const LEGACY_STORE: &str = "settings.backup";
pub const AGENT_EXPORT: &str = "agent-settings.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Server,
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Kind {
    Int {
        min: i64,
        max: i64,
    },
    Bool,
    /// HH:MM, 24 h, local time
    Time,
    /// Absolute folder. `writable`: must exist and be writable on the server when set.
    /// `optional`: "" means "use the default location".
    Path {
        writable: bool,
        optional: bool,
    },
    Text,
}

impl Kind {
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Int { .. } => "int",
            Kind::Bool => "bool",
            Kind::Time => "time",
            Kind::Path { .. } => "path",
            Kind::Text => "text",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Def {
    pub key: &'static str,
    pub scope: Scope,
    pub kind: Kind,
    pub default: Value,
    pub description: &'static str,
    /// Only changeable in server.json (security- or startup-critical).
    pub read_only: bool,
    /// Takes effect after a restart / reinstall rather than on the next pass.
    pub applies: &'static str,
    #[serde(skip)]
    pub from_cfg: Option<fn(&ServerConfig) -> Value>,
}

const NEXT: &str = "next pass";

fn int(min: i64, max: i64) -> Kind {
    Kind::Int { min, max }
}

/// Every setting, with today's behaviour as the default.
pub fn registry() -> Vec<Def> {
    use Scope::*;
    let d = |key, scope, kind, default: Value, description| Def {
        key,
        scope,
        kind,
        default,
        description,
        read_only: false,
        applies: NEXT,
        from_cfg: None,
    };
    let path = |writable, optional| Kind::Path { writable, optional };
    let mut v = vec![
        // ---- server: backup (decision 13)
        Def { from_cfg: Some(|c| json!(c.backup.dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default())),
            ..d("backup.dir", Server, path(true, true), json!(""), "Backup archive folder (e.g. the share). Empty = <data_dir>/backups.") },
        Def { from_cfg: Some(|c| json!(c.backup.nightly)), ..d("backup.nightly", Server, Kind::Bool, json!(true), "Take a nightly backup.") },
        Def { from_cfg: Some(|c| json!(c.backup.hour)), ..d("backup.hour", Server, int(0, 23), json!(2), "Local hour from which the nightly backup is due.") },
        Def { from_cfg: Some(|c| json!(c.backup.keep_daily)), ..d("backup.keep_daily", Server, int(0, 3650), json!(7), "Nightly backups kept: newest per day, for this many days.") },
        Def { from_cfg: Some(|c| json!(c.backup.keep_weekly)), ..d("backup.keep_weekly", Server, int(0, 520), json!(4), "Nightly backups kept: newest per ISO week, for this many weeks.") },
        Def { from_cfg: Some(|c| json!(c.backup.keep_blocks)), ..d("backup.keep_blocks", Server, int(0, 1300), json!(13), "Nightly backups kept: newest per 4-week block (ISO weeks 4N-3..4N).") },
        // ---- server: core
        Def { from_cfg: Some(|c| json!(c.ingest_interval_secs)), ..d("server.ingest_interval_secs", Server, int(5, 3600), json!(30), "Seconds between log ingest / request sync passes.") },
        Def { from_cfg: Some(|c| json!(c.request_timeout_secs)), ..d("server.request_timeout_secs", Server, int(60, 7 * 86400), json!(3 * 3600), "On-demand requests still queued/running after this many seconds become timed_out.") },
        Def { from_cfg: Some(|c| json!(c.retention_days)), ..d("server.retention_days", Server, int(30, 3650), json!(400), "Days of run/check data kept in the database.") },
        Def { from_cfg: Some(|c| json!(c.bind_addr)), read_only: true, applies: "restart",
            ..d("server.bind_addr", Server, Kind::Text, json!("127.0.0.1:50051"), "gRPC listen address (server.json only).") },
        Def { from_cfg: Some(|c| json!(c.auth_mode)), read_only: true, applies: "restart",
            ..d("server.auth_mode", Server, Kind::Text, json!("both"), "token | both | signed (server.json only: security-critical).") },
        d("metrics.flap_min", Server, int(2, 1000), json!(3), "ok<->failed changes in the window before a check counts as flapping."),
        d("metrics.repeat_min", Server, int(2, 1000), json!(3), "Incidents in the window before a machine/check is a repeat offender."),
        d("auth.max_skew_secs", Server, int(30, 3600), json!(300), "Allowed clock difference for signed requests (seconds)."),
        d("search.default_limit", Server, int(1, 1000), json!(100), "Log search page size when the client asks for none."),
        d("search.max_limit", Server, int(100, 10000), json!(1000), "Largest log search page."),
        d("plugins.max_package_mib", Server, int(1, 2048), json!(256), "Largest plugin package accepted (MiB)."),
        d("plugins.enable_on_install", Server, Kind::Bool, json!(true), "Enable a package that passed validation when it is installed (other versions are disabled)."),
        d("plugins.timeout_secs", Server, int(10, 86400), json!(600), "Per-stage timeout of a plugin run (seconds)."),
        d("plugins.powershell", Server, Kind::Text, json!(if cfg!(windows) { "powershell.exe" } else { "pwsh" }), "PowerShell used by itcmdr-plugin-ps (Windows PowerShell 5.1 by default)."),
        d("schedule.office_hours", Server, Kind::Text, json!("Mon-Fri 07:00-18:00"), "Office hours for the schedule guard, in each schedule's timezone."),
        d("schedule.grace_secs", Server, int(30, 86400), json!(300), "A run due longer ago than this counts as missed (server was down)."),
        d("restore.health_timeout_secs", Server, int(5, 3600), json!(120), "How long the restore helper waits for the restarted server before rolling back."),
        // ---- agent (served centrally, site.json names)
        d("paths.localLogs", Agent, path(false, false), json!("C:\\maintenanceRuns"), "Local run logs and requests\\ folder on each PC."),
        d("paths.agentData", Agent, path(false, false), json!("C:\\ProgramData\\itcmdr"), "Agent cache (exceptions, settings) and spool folder."),
        d("retention.agentLocalDays", Agent, int(1, 3650), json!(400), "Days of local run logs kept on each PC."),
        d("agent.quietMinutes", Agent, int(0, 240), json!(10), "Clinical guard: no user input for this many minutes before disruptive repairs."),
        d("agent.printerStuckMinutes", Agent, int(5, 10080), json!(60), "A print job older than this counts as stuck."),
        d("agent.backupMaxAgeHours", Agent, int(1, 720), json!(26), "Default max age of a backup task run / newest backup file (per-item maxAgeHours wins)."),
        d("agent.backupMinBytes", Agent, int(0, 1_000_000_000_000), json!(1), "Default minimum size of the newest backup file (per-item minBytes wins)."),
        d("agent.tcpTimeoutMs", Agent, int(100, 60000), json!(3000), "TCP probe timeout (ms)."),
        d("agent.staleLockMinutes", Agent, int(10, 1440), json!(150), "Poller: a request lock older than this was left by a crashed run."),
        Def { applies: "agent reinstall", ..d("agent.nightlyTime", Agent, Kind::Time, json!("01:30"), "Nightly maintenance task time (before stagger).") },
        Def { applies: "server reinstall", ..d("rotation.timeLocal", Agent, Kind::Time, json!("02:10"), "Log rotation task time on the server.") },
    ];
    v.sort_by_key(|d| d.key);
    v
}

pub fn def(key: &str) -> Result<Def> {
    registry()
        .into_iter()
        .find(|d| d.key == key)
        .ok_or_else(|| anyhow!("unknown setting {key:?} (itcmdr-cli config list shows them all)"))
}

/// Parse the text form (CLI / app) into a typed value and validate it.
pub fn parse(d: &Def, text: &str) -> Result<Value> {
    let t = text.trim();
    let v = match d.kind {
        Kind::Int { .. } => json!(t
            .parse::<i64>()
            .map_err(|_| anyhow!("{} must be a whole number", d.key))?),
        Kind::Bool => json!(match t.to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => true,
            "false" | "off" | "no" | "0" => false,
            _ => bail!("{} must be true or false", d.key),
        }),
        _ => json!(t),
    };
    validate(d, &v)?;
    Ok(v)
}

fn looks_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    Path::new(p).is_absolute()
        || (b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && (b[2] == b'\\' || b[2] == b'/'))
        || p.starts_with("\\\\")
}

pub fn validate(d: &Def, v: &Value) -> Result<()> {
    match d.kind {
        Kind::Int { min, max } => {
            let n = v
                .as_i64()
                .ok_or_else(|| anyhow!("{} must be a whole number", d.key))?;
            ensure!(
                (min..=max).contains(&n),
                "{} must be between {min} and {max}",
                d.key
            );
        }
        Kind::Bool => ensure!(v.is_boolean(), "{} must be true or false", d.key),
        Kind::Time => {
            let s = v.as_str().unwrap_or("");
            ensure!(
                s.len() == 5 && chrono::NaiveTime::parse_from_str(s, "%H:%M").is_ok(),
                "{} must be a time like 02:30",
                d.key
            );
        }
        Kind::Path { writable, optional } => {
            let s = v
                .as_str()
                .ok_or_else(|| anyhow!("{} must be a folder path", d.key))?;
            if s.is_empty() {
                ensure!(optional, "{} cannot be empty", d.key);
                return Ok(());
            }
            ensure!(
                !s.contains('\0') && s.len() < 1024,
                "{} is not a valid path",
                d.key
            );
            ensure!(looks_absolute(s), "{} must be an absolute path", d.key);
            if writable {
                check_writable_dir(Path::new(s))?;
            }
        }
        Kind::Text => ensure!(
            v.as_str().is_some_and(|s| !s.trim().is_empty()),
            "{} cannot be empty",
            d.key
        ),
    }
    Ok(())
}

/// The folder must exist and be writable (checked with a temp file).
pub fn check_writable_dir(p: &Path) -> Result<()> {
    ensure!(p.is_absolute(), "{} is not an absolute path", p.display());
    ensure!(
        p.is_dir(),
        "{} does not exist or is not a folder",
        p.display()
    );
    let probe = p.join(format!(".itcmdr-write-test-{}", uuid::Uuid::new_v4()));
    std::fs::write(&probe, b"ok").map_err(|e| anyhow!("{} is not writable: {e}", p.display()))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Store {
    #[serde(default)]
    pub global: BTreeMap<String, Value>,
    #[serde(default)]
    pub machines: BTreeMap<String, BTreeMap<String, Value>>,
}

async fn meta(pool: &DbPool, key: &str) -> Result<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM server_meta WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn load(pool: &DbPool) -> Result<Store> {
    let mut s: Store = match meta(pool, STORE).await? {
        Some(j) => serde_json::from_str(&j)?,
        None => Store::default(),
    };
    // v0.8.1 stored backup overrides separately; fold them in (they lose to v2 values).
    if let Some(j) = meta(pool, LEGACY_STORE).await? {
        let old: serde_json::Map<String, Value> = serde_json::from_str(&j).unwrap_or_default();
        for (k, v) in old {
            let v = if k == "dir" && v.is_null() {
                json!("")
            } else {
                v
            };
            s.global.entry(format!("backup.{k}")).or_insert(v);
        }
    }
    Ok(s)
}

async fn save(pool: &DbPool, s: &Store) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO server_meta(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(STORE)
        .bind(serde_json::to_string(s)?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM server_meta WHERE key = ?")
        .bind(LEGACY_STORE)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

fn valid_machine(m: &str) -> bool {
    !m.is_empty()
        && m.len() <= 63
        && m.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// One resolved setting.
#[derive(Debug, Clone, Serialize)]
pub struct Resolved {
    #[serde(flatten)]
    pub def: Def,
    pub value: Value,
    /// default | server.json | db | db:<machine>
    pub source: String,
}

/// Resolve every key. `machine`: include that machine's agent overrides.
pub async fn resolve_all(
    pool: &DbPool,
    cfg: Option<&ServerConfig>,
    machine: Option<&str>,
) -> Result<Vec<Resolved>> {
    let s = load(pool).await?;
    let mut out = Vec::new();
    for d in registry() {
        let mut value = d.default.clone();
        let mut source = "default".to_string();
        if let (Some(f), Some(c)) = (d.from_cfg, cfg) {
            let v = f(c);
            if v != d.default {
                value = v;
                source = "server.json".into();
            }
        }
        if !d.read_only {
            if let Some(v) = s.global.get(d.key) {
                value = v.clone();
                source = "db".into();
            }
            if d.scope == Scope::Agent {
                if let Some(v) = machine
                    .and_then(|m| s.machines.get(m))
                    .and_then(|o| o.get(d.key))
                {
                    value = v.clone();
                    source = format!("db:{}", machine.unwrap());
                }
            }
        }
        out.push(Resolved {
            def: d,
            value,
            source,
        });
    }
    Ok(out)
}

pub async fn get(pool: &DbPool, cfg: Option<&ServerConfig>, key: &str) -> Result<Value> {
    def(key)?;
    Ok(resolve_all(pool, cfg, None)
        .await?
        .into_iter()
        .find(|r| r.def.key == key)
        .map(|r| r.value)
        .unwrap())
}

/// Integer setting (server keys without a server.json field can pass `cfg = None`).
pub async fn get_i64(pool: &DbPool, cfg: Option<&ServerConfig>, key: &str) -> i64 {
    match get(pool, cfg, key).await {
        Ok(v) => v
            .as_i64()
            .unwrap_or_else(|| def(key).unwrap().default.as_i64().unwrap()),
        Err(e) => {
            tracing::warn!(error=%e, key, "setting unreadable, using the default");
            def(key)
                .ok()
                .and_then(|d| d.default.as_i64())
                .unwrap_or_default()
        }
    }
}

pub async fn get_bool(pool: &DbPool, cfg: Option<&ServerConfig>, key: &str) -> bool {
    get(pool, cfg, key)
        .await
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| {
            def(key)
                .ok()
                .and_then(|d| d.default.as_bool())
                .unwrap_or(false)
        })
}

/// Set a key (validated first). `machine` only for agent keys. Returns the stored value.
pub async fn set(pool: &DbPool, key: &str, text: &str, machine: Option<&str>) -> Result<Value> {
    let d = def(key)?;
    ensure!(!d.read_only, "{key} can only be changed in server.json");
    let v = parse(&d, text)?;
    let mut s = load(pool).await?;
    match machine {
        Some(m) => {
            ensure!(
                d.scope == Scope::Agent,
                "{key} is a server setting; per-machine values are for agent settings"
            );
            ensure!(valid_machine(m), "invalid machine name {m:?}");
            s.machines
                .entry(m.to_string())
                .or_default()
                .insert(key.into(), v.clone());
        }
        None => {
            s.global.insert(key.into(), v.clone());
        }
    }
    if key == "schedule.office_hours" {
        if let Some(t) = v.as_str() {
            crate::schedule::parse_hours(t)?;
        }
    }
    if key == "plugins.powershell" {
        ensure!(
            v.as_str().is_some_and(|t| !t.trim().is_empty()),
            "plugins.powershell cannot be empty"
        );
    }
    if key.starts_with("search.") {
        let get = |k: &str, dflt: i64| s.global.get(k).and_then(|v| v.as_i64()).unwrap_or(dflt);
        ensure!(
            get("search.default_limit", 100) <= get("search.max_limit", 1000),
            "search.default_limit cannot exceed search.max_limit"
        );
    }
    if key.starts_with("backup.keep_") {
        let get = |k: &str, dflt: i64| s.global.get(k).and_then(|v| v.as_i64()).unwrap_or(dflt);
        ensure!(
            get("backup.keep_daily", 7)
                + get("backup.keep_weekly", 4)
                + get("backup.keep_blocks", 13)
                > 0,
            "retention would delete every nightly backup"
        );
    }
    save(pool, &s).await?;
    Ok(v)
}

/// Remove an override (global or one machine's).
pub async fn unset(pool: &DbPool, key: &str, machine: Option<&str>) -> Result<()> {
    def(key)?;
    let mut s = load(pool).await?;
    match machine {
        Some(m) => {
            if let Some(o) = s.machines.get_mut(m) {
                o.remove(key);
                if o.is_empty() {
                    s.machines.remove(m);
                }
            }
        }
        None => {
            s.global.remove(key);
        }
    }
    save(pool, &s).await
}

/// Write `<logs_dir>/agent-settings.json` for the agents (atomic).
pub async fn export_agent(pool: &DbPool, cfg: &ServerConfig) -> Result<std::path::PathBuf> {
    let s = load(pool).await?;
    let mut global = BTreeMap::new();
    for r in resolve_all(pool, Some(cfg), None).await? {
        if r.def.scope == Scope::Agent {
            global.insert(r.def.key.to_string(), r.value);
        }
    }
    let machines: BTreeMap<_, _> = s
        .machines
        .iter()
        .map(|(m, o)| (m.clone(), o.clone()))
        .collect();
    let doc = json!({
        "schema": "itcmdr-agent-settings/v1",
        "generated": chrono::Utc::now().to_rfc3339(),
        "global": global,
        "machines": machines,
    });
    std::fs::create_dir_all(&cfg.logs_dir)?;
    let p = cfg.logs_dir.join(AGENT_EXPORT);
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&doc)?)?;
    std::fs::rename(&tmp, &p)?;
    Ok(p)
}

// ---- backup block (v0.8.1 API, now on the registry)

/// Settable backup keys (kept for `GetSettings` / `UpdateSettings`).
pub const BACKUP_KEYS: [&str; 6] = [
    "backup.dir",
    "backup.nightly",
    "backup.hour",
    "backup.keep_daily",
    "backup.keep_weekly",
    "backup.keep_blocks",
];

/// server.json `backup` with the stored overrides applied.
pub async fn effective_backup(pool: &DbPool, cfg: &ServerConfig) -> Result<BackupConfig> {
    let all = resolve_all(pool, Some(cfg), None).await?;
    let v = |k: &str| all.iter().find(|r| r.def.key == k).unwrap().value.clone();
    let dir = v("backup.dir").as_str().unwrap_or("").to_string();
    Ok(BackupConfig {
        dir: (!dir.is_empty()).then(|| dir.into()),
        nightly: v("backup.nightly").as_bool().unwrap_or(true),
        hour: v("backup.hour").as_u64().unwrap_or(2) as u32,
        keep_daily: v("backup.keep_daily").as_u64().unwrap_or(7) as usize,
        keep_weekly: v("backup.keep_weekly").as_u64().unwrap_or(4) as usize,
        keep_blocks: v("backup.keep_blocks").as_u64().unwrap_or(13) as usize,
    })
}

/// Effective backup folder (overrides, then server.json, then `<data_dir>/backups`).
pub async fn backup_dir(pool: &DbPool, cfg: &ServerConfig) -> Result<std::path::PathBuf> {
    Ok(effective_backup(pool, cfg)
        .await?
        .dir
        .unwrap_or_else(|| cfg.layout().backups()))
}

/// Replace all backup settings (what the app's backup form sends). All validated before saving.
pub async fn set_backup(pool: &DbPool, b: &BackupConfig) -> Result<()> {
    let vals = [
        (
            "backup.dir",
            b.dir
                .as_ref()
                .map(|d| d.display().to_string())
                .unwrap_or_default(),
        ),
        ("backup.nightly", b.nightly.to_string()),
        ("backup.hour", b.hour.to_string()),
        ("backup.keep_daily", b.keep_daily.to_string()),
        ("backup.keep_weekly", b.keep_weekly.to_string()),
        ("backup.keep_blocks", b.keep_blocks.to_string()),
    ];
    ensure!(
        b.keep_daily + b.keep_weekly + b.keep_blocks > 0,
        "retention would delete every nightly backup"
    );
    let mut s = load(pool).await?;
    for (k, t) in vals {
        s.global.insert(k.into(), parse(&def(k)?, &t)?);
    }
    save(pool, &s).await
}

/// v0.8.1 helpers used by `itcmdr-cli config set|unset`.
pub async fn set_key(pool: &DbPool, _cfg: &ServerConfig, key: &str, value: &str) -> Result<Value> {
    set(pool, key, value, None).await
}
pub async fn unset_key(pool: &DbPool, key: &str) -> Result<()> {
    unset(pool, key, None).await
}

/// Keys overridden in the DB (globally).
pub async fn overridden(pool: &DbPool) -> Result<Vec<String>> {
    Ok(load(pool).await?.global.keys().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_consistent() {
        let r = registry();
        let mut keys: Vec<_> = r.iter().map(|d| d.key).collect();
        keys.dedup();
        assert_eq!(keys.len(), r.len(), "duplicate keys");
        for d in &r {
            // Paths with writable=true touch the disk; skip validating their defaults.
            if !matches!(d.kind, Kind::Path { writable: true, .. }) {
                validate(d, &d.default)
                    .unwrap_or_else(|e| panic!("{}: default invalid: {e}", d.key));
            }
        }
    }

    #[test]
    fn defaults_equal_current_behaviour() {
        let c = ServerConfig::default();
        let get = |k: &str| def(k).unwrap().default;
        assert_eq!(
            get("server.ingest_interval_secs"),
            json!(c.ingest_interval_secs)
        );
        assert_eq!(
            get("server.request_timeout_secs"),
            json!(c.request_timeout_secs)
        );
        assert_eq!(get("server.retention_days"), json!(c.retention_days));
        assert_eq!(get("server.bind_addr"), json!(c.bind_addr));
        assert_eq!(get("server.auth_mode"), json!(c.auth_mode));
        assert_eq!(get("backup.hour"), json!(c.backup.hour));
        assert_eq!(get("backup.keep_blocks"), json!(c.backup.keep_blocks));
        assert_eq!(get("metrics.flap_min"), json!(3));
        assert_eq!(get("auth.max_skew_secs"), json!(300));
    }

    #[test]
    fn parsing_and_validation() {
        let p = |k: &str, t: &str| parse(&def(k).unwrap(), t);
        assert_eq!(p("backup.hour", "3").unwrap(), json!(3));
        assert!(p("backup.hour", "24").is_err() && p("backup.hour", "x").is_err());
        assert_eq!(p("backup.nightly", "off").unwrap(), json!(false));
        assert!(p("backup.nightly", "maybe").is_err());
        assert!(
            p("agent.nightlyTime", "25:00").is_err() && p("agent.nightlyTime", "1:30").is_err()
        );
        assert_eq!(p("agent.nightlyTime", "23:59").unwrap(), json!("23:59"));
        assert!(
            p("paths.localLogs", "relative\\dir").is_err() && p("paths.localLogs", "").is_err()
        );
        assert!(
            p("paths.localLogs", "D:\\itcmdr\\logs").is_ok()
                && p("paths.localLogs", "\\\\SERVER01\\share").is_ok()
        );
        assert!(p("backup.dir", "").is_ok(), "empty = default folder");
        assert!(p("backup.dir", "/definitely/not/here").is_err());
    }
}
