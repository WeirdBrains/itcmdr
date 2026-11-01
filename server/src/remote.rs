//! v0.20.0: what adminsvr may see of the site's configuration and plugins, and the commands that
//! change them (docs/design/ADMINSVR.md sections 3 and 5).
//!
//! - Config document: every setting with its kind and whether it can be set remotely; values only
//!   for numbers, switches and times (text and path values never leave the site). Schedules by
//!   opaque id, names only with the display-name opt-in, cron and timezone only when they are
//!   plain, lists through the same redaction as alert codes.
//! - Plugins document: installed package names (redacted like alert codes), versions, enabled,
//!   sha256; quarantine entries as sha256 + a reason code (never the file name or the text).
//! - Commands: setting.set (remote-settable keys only), schedule.run|pause|resume,
//!   plugin.enable|disable|install. Every one can be a dry run: checked and previewed, nothing
//!   changed. A pushed plugin goes through the site's own validation and quarantine.
use crate::config::ServerConfig;
use crate::db::DbPool;
use crate::settings::{Def, Kind};
use crate::sitelink::{plain_token, version_token, Redactor, State};
use anyhow::{anyhow, Result};
use ed25519_dalek::{Signer, SigningKey};
use itcmdr_adminlink::pb;
use prost::Message;
use sha2::Digest;

/// Settings that never change from adminsvr, whatever their kind (the link, auth, keys).
const NEVER_PREFIX: &[&str] = &["adminsvr.", "auth.", "server.auth_mode", "server.bind_addr"];

fn value_visible(d: &Def) -> bool {
    matches!(d.kind, Kind::Int { .. } | Kind::Bool | Kind::Time)
}

/// May adminsvr set this key? Numbers, switches and times that are not read-only or link/auth.
pub fn remote_settable(d: &Def) -> bool {
    !d.read_only && value_visible(d) && !NEVER_PREFIX.iter().any(|p| d.key.starts_with(p))
}

fn plain(s: &str, extra: &[char]) -> bool {
    s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(&c))
}

fn doc_version<M: Message + Clone>(m: &M, clear: impl Fn(&mut M)) -> String {
    let mut c = m.clone();
    clear(&mut c);
    hex::encode(sha2::Sha256::digest(c.encode_to_vec()))
}

async fn schedule_item(r: &Redactor, s: &crate::schedule::Schedule) -> pb::ScheduleItem {
    let sp = &s.spec;
    pb::ScheduleItem {
        id: r.pid("schedule", &s.id),
        name: if r.share_names() && plain(&s.name, &[' ', '-', '_', '.']) {
            s.name.clone()
        } else {
            String::new()
        },
        enabled: s.enabled,
        when: if plain(&sp.when, &[' ', '*', '/', ',', '-', '?', '#', ':']) { sp.when.clone() } else { String::new() },
        timezone: if plain(&sp.timezone, &['/', '_', '-', '+']) { sp.timezone.clone() } else { String::new() },
        action: if sp.action == "backup" { "backup".into() } else { "run".into() },
        guard: pick(&sp.guard, &crate::schedule::GUARDS),
        missed: pick(&sp.missed, &crate::schedule::MISSED),
        repair: sp.repair,
        builtin: pick(&sp.builtin_key, &["nightly-agent-run", "nightly-backup"]),
        all_machines: sp.all_machines,
        machines: sp.machines.len() as i32,
        groups: sp.groups.iter().map(|g| r.code(g)).collect(),
        categories: sp.categories.iter().map(|g| r.code(g)).collect(),
        plugins: sp.plugins.iter().map(|g| r.plugin(g)).collect(),
        last_fire_ms: s.last_fire_ms,
    }
}

fn pick(v: &str, allowed: &[&str]) -> String {
    if allowed.contains(&v) { v.to_string() } else { String::new() }
}

pub async fn config_doc(pool: &DbPool, cfg: &ServerConfig, st: &State, r: &Redactor) -> pb::SiteConfig {
    let mut settings = Vec::new();
    for x in crate::settings::resolve_all(pool, Some(cfg), None).await.unwrap_or_default() {
        let d = &x.def;
        let visible = value_visible(d);
        let (min, max) = match d.kind {
            Kind::Int { min, max } => (min, max),
            _ => (0, 0),
        };
        settings.push(pb::SettingItem {
            key: d.key.to_string(),
            kind: d.kind.name().to_string(),
            value: if visible { x.value.to_string() } else { String::new() },
            hidden: !visible,
            remote: remote_settable(d),
            source: pick(&x.source, &["default", "server.json", "db"]),
            scope: format!("{:?}", d.scope).to_lowercase(),
            min,
            max,
        });
    }
    let mut schedules = Vec::new();
    for s in crate::schedule::list(pool).await.unwrap_or_default() {
        schedules.push(schedule_item(r, &s).await);
    }
    let mut doc = pb::SiteConfig {
        version: String::new(),
        settings,
        schedules,
        require_two_person: crate::settings::get_bool(pool, None, "adminsvr.requireTwoPerson").await,
        allowed: st
            .allowed
            .iter()
            .filter(|k| itcmdr_adminlink::COMMANDS.contains(&k.as_str()))
            .cloned()
            .collect(),
    };
    doc.version = doc_version(&doc, |d| d.version.clear());
    doc
}

/// Quarantine reasons as codes (the text can carry file names).
pub fn reason_code(reason: &str) -> String {
    let l = reason.to_lowercase();
    let code = if l.contains("signature") || l.contains("signed") || l.contains("signer") {
        "bad-signature"
    } else if l.contains("sha256") || l.contains("checksum") || l.contains("hash") {
        "checksum"
    } else if l.contains("too large") || l.contains("size") || l.contains("mib") {
        "too-large"
    } else if l.contains("path") {
        "bad-path"
    } else if l.contains("missing") {
        "missing-file"
    } else if l.contains("manifest") || l.contains("package.toml") || l.contains("version") || l.contains("name") {
        "bad-manifest"
    } else if l.contains("tar") || l.contains("gzip") || l.contains("archive") {
        "bad-archive"
    } else {
        "rejected"
    };
    code.into()
}

pub async fn plugins_doc(pool: &DbPool, cfg: &ServerConfig, r: &Redactor) -> pb::SitePlugins {
    let layout = cfg.layout();
    let installed = crate::packages::list(pool, &layout)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| pb::PluginItem {
            name: r.plugin(&p.name),
            version: version_token(&p.version),
            enabled: p.enabled,
            sha256: if p.package_sha256.len() == 64 && p.package_sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                p.package_sha256.to_lowercase()
            } else {
                String::new()
            },
        })
        .collect();
    let quarantine = crate::packages::list_quarantine(&layout)
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(50)
        .map(|q| pb::QuarantineItem {
            sha256: if q.sha256.len() == 64 && q.sha256.chars().all(|c| c.is_ascii_hexdigit()) { q.sha256.to_lowercase() } else { String::new() },
            reason: reason_code(&q.reason),
            ts_ms: chrono::DateTime::parse_from_rfc3339(&q.ts).map(|t| t.timestamp_millis()).unwrap_or(0),
        })
        .collect();
    let mut doc = pb::SitePlugins { version: String::new(), installed, quarantine };
    doc.version = doc_version(&doc, |d| d.version.clear());
    doc
}

/// Fetches catalog packages from adminsvr over the link's channel (site-signed request).
#[derive(Clone)]
pub struct Fetcher {
    pub client: pb::site_link_client::SiteLinkClient<tonic::transport::Channel>,
    pub key: SigningKey,
    pub site_id: String,
}

impl Fetcher {
    pub async fn fetch(&self, sha256: &str, max: u64) -> Result<Vec<u8>> {
        let ts = chrono::Utc::now().timestamp_millis();
        let sig = hex::encode(
            self.key
                .sign(&itcmdr_adminlink::fetch_bytes(&self.site_id, sha256, ts))
                .to_bytes(),
        );
        let mut s = self
            .client
            .clone()
            .fetch_package(pb::PackageRequest { site_id: self.site_id.clone(), sha256: sha256.into(), ts_ms: ts, sig })
            .await?
            .into_inner();
        let mut out = Vec::new();
        while let Some(ch) = s.message().await? {
            out.extend_from_slice(&ch.data);
            anyhow::ensure!(out.len() as u64 <= max, "package larger than plugins.max_package_mib");
        }
        anyhow::ensure!(
            hex::encode(sha2::Sha256::digest(&out)) == sha256,
            "package checksum differs from the command"
        );
        Ok(out)
    }
}

pub struct Outcome {
    pub code: &'static str,
    pub preview: Vec<pb::ConfigChange>,
}

fn change(target: impl Into<String>, before: impl Into<String>, after: impl Into<String>) -> pb::ConfigChange {
    pb::ConfigChange { target: target.into(), before: before.into(), after: after.into() }
}

fn ok(preview: Vec<pb::ConfigChange>) -> Outcome {
    Outcome { code: "ok", preview }
}

fn arg<'a>(c: &'a pb::Command, k: &str) -> &'a str {
    c.args.get(k).map(String::as_str).unwrap_or("")
}

async fn find_schedule(pool: &DbPool, r: &Redactor, pid: &str) -> Option<crate::schedule::Schedule> {
    crate::schedule::list(pool)
        .await
        .ok()?
        .into_iter()
        .find(|s| r.pid("schedule", &s.id) == pid)
}

async fn find_plugin(pool: &DbPool, cfg: &ServerConfig, r: &Redactor, name: &str) -> Vec<crate::packages::InstalledPlugin> {
    crate::packages::list(pool, &cfg.layout())
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|p| r.plugin(&p.name) == name)
        .collect()
}

fn enabled_version(v: &[crate::packages::InstalledPlugin]) -> String {
    v.iter().find(|p| p.enabled).map(|p| version_token(&p.version)).unwrap_or_else(|| "disabled".into())
}

/// Run (or preview, when `c.dry_run`) one of the v0.20.0 kinds. Signatures, the allowlist and the
/// two-person rule are already checked by the caller.
pub async fn run(
    pool: &DbPool,
    cfg: &ServerConfig,
    r: &Redactor,
    c: &pb::Command,
    fetch: Option<&Fetcher>,
    actor: &str,
) -> Result<Outcome> {
    let dry = c.dry_run;
    match c.kind.as_str() {
        "setting.set" => {
            let key = arg(c, "key");
            let Ok(d) = crate::settings::def(key) else { return Ok(Outcome { code: "invalid", preview: vec![] }) };
            if !remote_settable(&d) {
                return Ok(Outcome { code: "not-allowed", preview: vec![] });
            }
            let Ok(v) = crate::settings::parse(&d, arg(c, "value")) else { return Ok(Outcome { code: "invalid", preview: vec![] }) };
            if crate::settings::validate(&d, &v).is_err() {
                return Ok(Outcome { code: "invalid", preview: vec![] });
            }
            let before = crate::settings::get(pool, Some(cfg), key).await?.to_string();
            let p = vec![change(key, before, v.to_string())];
            if !dry {
                crate::settings::set(pool, key, arg(c, "value"), None).await?;
                crate::db::insert_audit(pool, actor, "settings-set", &serde_json::json!({"key": key, "value": v}).to_string()).await?;
                if d.scope == crate::settings::Scope::Agent {
                    let _ = crate::settings::export_agent(pool, cfg).await;
                }
            }
            Ok(ok(p))
        }
        "schedule.run" | "schedule.pause" | "schedule.resume" => {
            let Some(s) = find_schedule(pool, r, arg(c, "schedule")).await else {
                return Ok(Outcome { code: "invalid", preview: vec![] });
            };
            let t = format!("schedule:{}", r.pid("schedule", &s.id));
            let on = |b: bool| if b { "enabled" } else { "paused" };
            match c.kind.as_str() {
                "schedule.run" => {
                    if !dry {
                        let (outcome, _, _) = crate::schedule::fire(pool, cfg, &s, chrono::Utc::now().timestamp_millis(), &format!("run now by {actor}"), actor, false).await?;
                        crate::db::insert_audit(pool, actor, "schedule-run-now", &serde_json::json!({"id": s.id, "outcome": outcome}).to_string()).await?;
                    }
                    Ok(ok(vec![change(t, on(s.enabled), "run now")]))
                }
                k => {
                    let en = k == "schedule.resume";
                    if !dry {
                        crate::schedule::set_enabled(pool, &s.id, en).await?;
                        crate::db::insert_audit(pool, actor, if en { "schedule-resume" } else { "schedule-pause" }, &serde_json::json!({"id": s.id}).to_string()).await?;
                    }
                    Ok(ok(vec![change(t, on(s.enabled), on(en))]))
                }
            }
        }
        "plugin.enable" | "plugin.disable" => {
            let name = arg(c, "name");
            let found = find_plugin(pool, cfg, r, name).await;
            let Some(real) = found.first().map(|p| p.name.clone()) else {
                return Ok(Outcome { code: "invalid", preview: vec![] });
            };
            let before = enabled_version(&found);
            let t = format!("plugin:{name}");
            if c.kind == "plugin.disable" {
                if !dry {
                    crate::packages::disable(pool, &real, actor).await?;
                }
                return Ok(ok(vec![change(t, before, "disabled")]));
            }
            let want = arg(c, "version");
            let target = if want.is_empty() {
                if found.len() != 1 {
                    return Ok(Outcome { code: "invalid", preview: vec![] });
                }
                found[0].version.clone()
            } else if let Some(p) = found.iter().find(|p| p.version == want) {
                p.version.clone()
            } else {
                return Ok(Outcome { code: "invalid", preview: vec![] });
            };
            if !dry {
                crate::packages::enable(pool, &format!("{real}@{target}"), actor).await?;
            }
            Ok(ok(vec![change(t, before, version_token(&target))]))
        }
        "plugin.install" => {
            let sha = arg(c, "sha256").to_lowercase();
            if sha.len() != 64 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Ok(Outcome { code: "invalid", preview: vec![] });
            }
            let Some(f) = fetch else { return Ok(Outcome { code: "refused", preview: vec![] }) };
            let max = crate::settings::get_i64(pool, None, "plugins.max_package_mib").await as u64 * 1024 * 1024;
            let bytes = f.fetch(&sha, max).await.map_err(|e| anyhow!("fetch: {e:#}"))?;
            let v = crate::packages::validate(pool, &bytes, max).await?;
            let (name, version) = match &v {
                Ok(v) => (v.manifest.name.clone(), v.manifest.version.clone()),
                Err(_) => (String::new(), String::new()),
            };
            let shown = if plain_token(&name) { name.clone() } else { r.pid("plugin", &name) };
            let before = enabled_version(&find_plugin(pool, cfg, r, &shown).await);
            let p = vec![change(format!("plugin:{shown}"), before, version_token(&version))];
            if dry {
                return Ok(Outcome { code: if v.is_ok() { "ok" } else { "invalid" }, preview: if v.is_ok() { p } else { vec![] } });
            }
            let label = format!("adminsvr:{}", &sha[..16]);
            match crate::packages::install(pool, &cfg.layout(), crate::packages::Source::Bytes(&label, &bytes), None, actor).await {
                Ok(_) => Ok(ok(p)),
                Err(crate::packages::InstallError::Quarantined { .. }) => Ok(Outcome { code: "quarantined", preview: vec![] }),
                Err(crate::packages::InstallError::Other(e)) => Err(e),
            }
        }
        _ => Ok(Outcome { code: "not-allowed", preview: vec![] }),
    }
}
