//! v0.18.0: the Rust agent runs plugins and schedules (report-only).
//!
//! - **PluginUpdate**: every installed, enabled plugin selected by `agent.plugins` (per machine:
//!   `all`, `none` or a comma list of names) is pushed to the agent as its files, endorsed by the
//!   server root key over `plugin_bytes(name, version, sha256(package.toml))`. The server has
//!   already checked the admin/owner signature and checksums at install; the agent checks the
//!   endorsement and every checksum again. Plugins no longer selected get a PluginRemove.
//! - **ScheduleSet**: enabled schedules that target the machine and name plugins, plus the
//!   built-in nightly (`*` = every pushed plugin, for parity with the nightly PowerShell run).
//!   The agent keeps them and runs them locally, also while the server is unreachable.
//! - **RunNow**: `itcmdr-cli agent run <machine> <plugin>` queues a request; the link sends it.
//! - **RunReport**: delivered through the agent's durable queue (deduped by seq like alerts),
//!   stored in `agent_runs` (+ artifacts under `<data>/agent-artifacts/<run id>/`) and compared
//!   with the machine's latest nightly check results (`parity_runs`, run id `agent:<id>`).
use crate::agents::Agent;
use crate::db::DbPool;
use crate::packages::{self, InstalledPlugin};
use crate::storage::Layout;
use anyhow::{anyhow, ensure, Result};
use itcmdr_agentlink::pb;
use serde::Serialize;
use sqlx::Row;
use std::collections::BTreeMap;

pub const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS agent_runs (
        run_id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        machine TEXT NOT NULL,
        plugin TEXT NOT NULL,
        version TEXT NOT NULL,
        trigger TEXT NOT NULL,
        started_ms INTEGER NOT NULL,
        finished_ms INTEGER NOT NULL,
        status TEXT NOT NULL,
        message TEXT NOT NULL,
        report_only INTEGER NOT NULL,
        results TEXT NOT NULL,
        errors TEXT NOT NULL,
        artifacts TEXT NOT NULL,
        parity_matched INTEGER,
        parity_mismatched INTEGER,
        parity_detail TEXT,
        received_ms INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS ix_agent_runs ON agent_runs(machine, started_ms);
    CREATE TABLE IF NOT EXISTS agent_run_requests (
        id TEXT PRIMARY KEY,
        machine TEXT NOT NULL,
        plugin TEXT NOT NULL,
        args TEXT NOT NULL,
        actor TEXT NOT NULL,
        created_ms INTEGER NOT NULL,
        sent_ms INTEGER
    );
    CREATE TABLE IF NOT EXISTS agent_plugins (
        machine TEXT NOT NULL,
        name TEXT NOT NULL,
        version TEXT NOT NULL,
        ok INTEGER NOT NULL,
        error TEXT NOT NULL,
        ts_ms INTEGER NOT NULL,
        PRIMARY KEY (machine, name)
    );
    CREATE TABLE IF NOT EXISTS agent_config (
        machine TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        ok INTEGER NOT NULL,
        error TEXT NOT NULL,
        ts_ms INTEGER NOT NULL
    );
"#;

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// A plugin as the agent sees it: name, version, sha256 of its package.toml.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Want {
    pub name: String,
    pub version: String,
    pub sha: String,
}

impl Want {
    pub fn key(&self) -> String {
        format!("{}@{}:{}", self.name, self.version, self.sha)
    }
}

/// Parse Hello.plugins entries (`name@version:sha`).
pub fn parse_have(v: &[String]) -> BTreeMap<String, String> {
    v.iter()
        .filter_map(|s| {
            let (name, _) = s.split_once('@')?;
            Some((name.to_string(), s.clone()))
        })
        .collect()
}

/// `agent.plugins` for this machine: all | none | comma list.
async fn selection(pool: &DbPool, machine: &str) -> String {
    crate::agents::machine_text(pool, "agent.plugins", machine)
        .await
        .unwrap_or_else(|| "all".into())
}

/// Validate an `agent.plugins` value.
pub fn valid_selection(s: &str) -> Result<()> {
    let t = s.trim();
    if t.eq_ignore_ascii_case("all") || t.eq_ignore_ascii_case("none") {
        return Ok(());
    }
    ensure!(
        !t.is_empty()
            && t.split(',').all(|n| {
                let n = n.trim();
                !n.is_empty()
                    && n.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
            }),
        "agent.plugins must be all, none or a comma-separated list of plugin names"
    );
    Ok(())
}

/// Plugins this agent should hold.
pub async fn desired(
    pool: &DbPool,
    layout: &Layout,
    machine: &str,
) -> Result<Vec<(Want, InstalledPlugin)>> {
    let sel = selection(pool, machine).await;
    let names: Option<Vec<String>> = if sel.trim().eq_ignore_ascii_case("all") {
        None
    } else if sel.trim().eq_ignore_ascii_case("none") {
        Some(vec![])
    } else {
        Some(sel.split(',').map(|s| s.trim().to_string()).collect())
    };
    let mut out = vec![];
    for p in packages::list(pool, layout).await? {
        if !p.enabled
            || crate::upgrades::is_release(&p.name)
            || names.as_ref().is_some_and(|n| !n.contains(&p.name))
        {
            continue;
        }
        let Ok(m) = std::fs::read(p.dir.join(packages::MANIFEST)) else {
            continue;
        };
        out.push((
            Want {
                name: p.name.clone(),
                version: p.version.clone(),
                sha: itcmdr_agentlink::sha256_hex(&m),
            },
            p,
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// The PluginUpdate for one installed plugin (files + root endorsement).
pub fn update_for(layout: &Layout, w: &Want, p: &InstalledPlugin) -> Result<pb::PluginUpdate> {
    let mbytes = std::fs::read(p.dir.join(packages::MANIFEST))?;
    let m = packages::parse_manifest(&mbytes).map_err(|r| anyhow!("{}: {}", p.name, r.0))?;
    let mut files = vec![pb::PluginFile {
        name: packages::MANIFEST.into(),
        data: mbytes,
    }];
    if let Ok(sig) = std::fs::read(p.dir.join(packages::SIGNATURE)) {
        files.push(pb::PluginFile {
            name: packages::SIGNATURE.into(),
            data: sig,
        });
    }
    for f in &m.files {
        files.push(pb::PluginFile {
            name: f.name.clone(),
            data: std::fs::read(p.dir.join(&f.name))?,
        });
    }
    let root = crate::auth::load_root_key(layout)?;
    use ed25519_dalek::Signer;
    let sig = root.sign(&itcmdr_agentlink::plugin_bytes(&w.name, &w.version, &w.sha));
    Ok(pb::PluginUpdate {
        name: w.name.clone(),
        version: w.version.clone(),
        files,
        endorsement: hex::encode(sig.to_bytes()),
    })
}

/// v0.20.1: the agent's config document: the cutover state and the agent-scoped settings
/// (global values with this machine's overrides), endorsed by the server root key.
pub async fn config_update(
    pool: &DbPool,
    layout: &Layout,
    machine: &str,
) -> Result<pb::ConfigUpdate> {
    let (version, json) = config_doc(pool, machine).await?;
    let root = crate::auth::load_root_key(layout)?;
    use ed25519_dalek::Signer;
    let sig = root.sign(&itcmdr_agentlink::config_bytes(&version, &json));
    Ok(pb::ConfigUpdate {
        version,
        json,
        endorsement: hex::encode(sig.to_bytes()),
    })
}

/// The unsigned config document and its version.
pub async fn config_doc(pool: &DbPool, machine: &str) -> Result<(String, String)> {
    let mut settings = serde_json::Map::new();
    for r in crate::settings::resolve_all(pool, None, Some(machine)).await? {
        if r.def.scope == crate::settings::Scope::Agent {
            settings.insert(r.def.key.to_string(), r.value);
        }
    }
    let doc = serde_json::json!({
        "v": 1,
        "cutover": crate::cutover::state(pool).await?,
        "settings": settings,
    });
    let json = serde_json::to_string(&doc)?;
    Ok((itcmdr_agentlink::config_version(&json), json))
}

/// v0.20.1: the config version each machine's agent acknowledged (ok only).
pub async fn config_acked(pool: &DbPool, machine: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT version FROM agent_config WHERE machine = ? COLLATE NOCASE AND ok = 1",
    )
    .bind(machine)
    .fetch_optional(pool)
    .await?)
}

/// Schedules for this machine, with a version (hash of the set).
pub async fn schedule_set(pool: &DbPool, machine: &str) -> Result<pb::ScheduleSet> {
    let mut out = vec![];
    for s in crate::schedule::list(pool).await? {
        if !s.enabled || !s.spec.action.is_empty() {
            continue;
        }
        let nightly = s.spec.builtin_key == crate::cutover::AGENT_KEY;
        if s.spec.plugins.is_empty() && !nightly {
            continue; // category schedules are the PowerShell agent's
        }
        let mut targeted = s.spec.all_machines
            || s.spec
                .machines
                .iter()
                .any(|m| m.eq_ignore_ascii_case(machine));
        if !targeted && !s.spec.groups.is_empty() {
            targeted = crate::schedule::machines_in_groups(pool, &s.spec.groups)
                .await?
                .iter()
                .any(|m| m.eq_ignore_ascii_case(machine));
        }
        if !targeted {
            continue;
        }
        let Ok(cron) = crate::schedule::to_cron(&s.spec.when) else {
            continue;
        };
        out.push(pb::AgentSchedule {
            id: s.id.clone(),
            name: s.name.clone(),
            cron,
            timezone: s.spec.timezone.clone(),
            plugins: if s.spec.plugins.is_empty() {
                vec!["*".into()]
            } else {
                s.spec.plugins.clone()
            },
            catch_up: s.spec.missed == "catch-up",
            args: Default::default(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    let mut set = pb::ScheduleSet {
        version: String::new(),
        schedules: out,
    };
    use prost::Message;
    set.version = itcmdr_agentlink::sha256_hex(&set.encode_to_vec())[..16].to_string();
    Ok(set)
}

/// Queue an on-demand run on a machine's agent (`itcmdr-cli agent run`).
pub async fn request_run(
    pool: &DbPool,
    machine: &str,
    plugin: &str,
    args: &BTreeMap<String, String>,
    actor: &str,
) -> Result<String> {
    ensure!(
        crate::agents::by_machine(pool, machine).await?.is_some(),
        "no enrolled agent on {machine}"
    );
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO agent_run_requests(id, machine, plugin, args, actor, created_ms) VALUES (?,?,?,?,?,?)")
        .bind(&id)
        .bind(machine)
        .bind(plugin)
        .bind(serde_json::to_string(args)?)
        .bind(actor)
        .bind(now())
        .execute(pool)
        .await?;
    Ok(id)
}

/// Requests not yet sent to this machine's agent; marks them sent.
pub async fn take_requests(pool: &DbPool, machine: &str) -> Result<Vec<pb::RunNow>> {
    let rows = sqlx::query("SELECT id, plugin, args FROM agent_run_requests WHERE machine = ? COLLATE NOCASE AND sent_ms IS NULL ORDER BY created_ms")
        .bind(machine)
        .fetch_all(pool)
        .await?;
    let mut out = vec![];
    for r in rows {
        let id: String = r.get("id");
        let args: BTreeMap<String, String> =
            serde_json::from_str(&r.get::<String, _>("args")).unwrap_or_default();
        sqlx::query("UPDATE agent_run_requests SET sent_ms = ? WHERE id = ?")
            .bind(now())
            .bind(&id)
            .execute(pool)
            .await?;
        out.push(pb::RunNow {
            request_id: id,
            plugin: r.get("plugin"),
            args: args.into_iter().collect(),
        });
    }
    Ok(out)
}

/// Record an Applied from the agent; a failed plugin install raises a Cautious alert.
pub async fn applied(pool: &DbPool, a: &Agent, ap: &pb::Applied) -> Result<()> {
    if ap.kind == "upgrade" {
        tracing::info!(machine=%a.machine, version=%ap.version, ok=ap.ok, error=%ap.error, "agent upgrade applied");
        return crate::upgrades::applied(pool, &a.machine, ap).await;
    }
    if ap.kind == "config" {
        sqlx::query("INSERT INTO agent_config(machine, version, ok, error, ts_ms) VALUES (?,?,?,?,?) ON CONFLICT(machine) DO UPDATE SET version = excluded.version, ok = excluded.ok, error = excluded.error, ts_ms = excluded.ts_ms")
            .bind(&a.machine)
            .bind(&ap.version)
            .bind(ap.ok)
            .bind(ap.error.chars().take(2000).collect::<String>())
            .bind(now())
            .execute(pool)
            .await?;
    }
    if ap.kind == "plugin" || ap.kind == "plugin-remove" {
        if ap.kind == "plugin-remove" && ap.ok {
            sqlx::query("DELETE FROM agent_plugins WHERE machine = ? AND name = ?")
                .bind(&a.machine)
                .bind(&ap.name)
                .execute(pool)
                .await?;
        } else {
            sqlx::query("INSERT INTO agent_plugins(machine, name, version, ok, error, ts_ms) VALUES (?,?,?,?,?,?) ON CONFLICT(machine, name) DO UPDATE SET version = excluded.version, ok = excluded.ok, error = excluded.error, ts_ms = excluded.ts_ms")
                .bind(&a.machine)
                .bind(&ap.name)
                .bind(&ap.version)
                .bind(ap.ok)
                .bind(ap.error.chars().take(2000).collect::<String>())
                .bind(now())
                .execute(pool)
                .await?;
        }
    }
    let key = format!("agent-plugin:{}:{}", a.machine, ap.name);
    if ap.ok {
        crate::alerts::resolve_key(pool, &key).await?;
    } else {
        crate::alerts::raise(
            pool,
            &crate::alerts::Raise {
                key,
                level: "cautious".into(),
                source: "agent".into(),
                machine: a.machine.clone(),
                title: format!(
                    "{} could not apply {} {} {}",
                    a.machine, ap.kind, ap.name, ap.version
                ),
                detail: ap.error.chars().take(2000).collect(),
                run_id: None,
            },
        )
        .await?;
    }
    tracing::info!(machine=%a.machine, kind=%ap.kind, name=%ap.name, ok=ap.ok, "agent applied");
    Ok(())
}

fn builtin_parity_status(status: &str, attempted: bool, result: &str) -> &'static str {
    match status {
        "skipped" => "skipped",
        "failed" => "fail",
        _ if attempted && result == "success" => "fail",
        _ => "ok",
    }
}

fn plugin_parity_status(s: &str) -> String {
    match s {
        "repaired" | "failed" | "fail" => "fail".into(),
        "excepted" => "ok".into(),
        other => other.to_string(),
    }
}

/// Compare an agent run's check results with the machine's latest nightly (within 36 h).
async fn parity(
    pool: &DbPool,
    machine: &str,
    rep: &pb::RunReport,
) -> Result<Option<(i64, Vec<serde_json::Value>)>> {
    if rep.results.is_empty() {
        return Ok(None);
    }
    let since = rep.started_ms - 36 * 3_600_000;
    let run: Option<String> = sqlx::query_scalar(
        "SELECT run_id FROM runs WHERE computer = ? COLLATE NOCASE AND ts_ms >= ? ORDER BY ts_ms DESC LIMIT 1",
    )
    .bind(machine)
    .bind(since)
    .fetch_optional(pool)
    .await?;
    let Some(run) = run else {
        return Ok(None);
    };
    let rows = sqlx::query("SELECT check_id, status, attempted_repair, COALESCE(result,'') AS result FROM checks WHERE run_id = ?")
        .bind(&run)
        .fetch_all(pool)
        .await?;
    let b: BTreeMap<String, &'static str> = rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("check_id"),
                builtin_parity_status(
                    &r.get::<String, _>("status"),
                    r.get::<i64, _>("attempted_repair") != 0,
                    &r.get::<String, _>("result"),
                ),
            )
        })
        .collect();
    let mut matched = 0;
    let mut mis = vec![];
    for r in &rep.results {
        let p = plugin_parity_status(&r.status);
        let bs = b.get(&r.check_id).copied().unwrap_or("missing");
        if bs == p {
            matched += 1;
        } else {
            mis.push(serde_json::json!({"checkId": r.check_id, "builtin": bs, "plugin": p, "nightly_run": run}));
        }
    }
    Ok(Some((matched, mis)))
}

/// Store a RunReport (once per durable seq) and its parity line. Returns false for a duplicate.
pub async fn store_report(
    pool: &DbPool,
    layout: &Layout,
    a: &Agent,
    seq: u64,
    rep: &pb::RunReport,
) -> Result<bool> {
    if !crate::agents::claim_seq(pool, &a.id, seq).await? {
        return Ok(false);
    }
    ensure!(
        uuid::Uuid::parse_str(&rep.run_id).is_ok(),
        "run id is not a uuid"
    );
    let mut arts = vec![];
    if !rep.artifacts.is_empty() {
        let d = layout.root.join("agent-artifacts").join(&rep.run_id);
        std::fs::create_dir_all(&d)?;
        for x in &rep.artifacts {
            let name: String = x
                .name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || "-_.".contains(c) {
                        c
                    } else {
                        '_'
                    }
                })
                .take(100)
                .collect();
            let name = name.trim_start_matches('.').to_string();
            let ok = itcmdr_agentlink::sha256_hex(&x.data) == x.sha256;
            if !name.is_empty() && ok {
                std::fs::write(d.join(&name), &x.data)?;
            }
            arts.push(serde_json::json!({"name": name, "sha256": x.sha256, "bytes": x.data.len(), "stored": ok}));
        }
    }
    if !rep.log_lines.is_empty() {
        let d = layout.plugin_logs().join(&rep.plugin).join("agents");
        std::fs::create_dir_all(&d)?;
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(d.join(format!("{}.jsonl", a.machine)))?;
        f.write_all(rep.log_lines.as_bytes())?;
        if !rep.log_lines.ends_with('\n') {
            f.write_all(b"\n")?;
        }
    }
    let par = parity(pool, &a.machine, rep).await?;
    let results: Vec<_> = rep
        .results
        .iter()
        .map(|r| serde_json::json!({"check_id": r.check_id, "status": r.status, "message": r.message}))
        .collect();
    sqlx::query("INSERT OR IGNORE INTO agent_runs(run_id, agent_id, machine, plugin, version, trigger, started_ms, finished_ms, status, message, report_only, results, errors, artifacts, parity_matched, parity_mismatched, parity_detail, received_ms) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&rep.run_id)
        .bind(&a.id)
        .bind(&a.machine)
        .bind(&rep.plugin)
        .bind(&rep.version)
        .bind(&rep.trigger)
        .bind(rep.started_ms)
        .bind(rep.finished_ms)
        .bind(&rep.status)
        .bind(rep.message.chars().take(4000).collect::<String>())
        .bind(rep.report_only)
        .bind(serde_json::to_string(&results)?)
        .bind(if rep.errors_json.is_empty() { "[]".to_string() } else { rep.errors_json.clone() })
        .bind(serde_json::to_string(&arts)?)
        .bind(par.as_ref().map(|p| p.0))
        .bind(par.as_ref().map(|p| p.1.len() as i64))
        .bind(par.as_ref().map(|p| serde_json::Value::from(p.1.clone()).to_string()))
        .bind(now())
        .execute(pool)
        .await?;
    if let Some((m, mis)) = &par {
        crate::cutover::record_parity(
            pool,
            &format!("agent:{}", rep.run_id),
            &a.machine,
            rep.started_ms,
            *m,
            mis,
            &[],
        )
        .await?;
    }
    let key = format!("agent-run:{}:{}", a.machine, rep.plugin);
    if rep.status == "ok" {
        crate::alerts::resolve_key(pool, &key).await?;
    } else {
        crate::alerts::raise(
            pool,
            &crate::alerts::Raise {
                key,
                level: "cautious".into(),
                source: "agent".into(),
                machine: a.machine.clone(),
                title: format!("{} on {}: {}", rep.plugin, a.machine, rep.status),
                detail: rep.message.chars().take(2000).collect(),
                run_id: Some(rep.run_id.clone()),
            },
        )
        .await?;
    }
    Ok(true)
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRun {
    pub run_id: String,
    pub machine: String,
    pub plugin: String,
    pub version: String,
    pub trigger: String,
    pub started_ms: i64,
    pub finished_ms: i64,
    pub status: String,
    pub message: String,
    pub report_only: bool,
    pub results: serde_json::Value,
    pub artifacts: serde_json::Value,
    pub parity_matched: Option<i64>,
    pub parity_mismatched: Option<i64>,
}

pub async fn list_runs(pool: &DbPool, machine: Option<&str>, limit: i64) -> Result<Vec<AgentRun>> {
    let rows = sqlx::query("SELECT * FROM agent_runs WHERE (? IS NULL OR machine = ? COLLATE NOCASE) ORDER BY started_ms DESC LIMIT ?")
        .bind(machine)
        .bind(machine)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .map(|r| AgentRun {
            run_id: r.get("run_id"),
            machine: r.get("machine"),
            plugin: r.get("plugin"),
            version: r.get("version"),
            trigger: r.get("trigger"),
            started_ms: r.get("started_ms"),
            finished_ms: r.get("finished_ms"),
            status: r.get("status"),
            message: r.get("message"),
            report_only: r.get::<i64, _>("report_only") != 0,
            results: serde_json::from_str(&r.get::<String, _>("results")).unwrap_or_default(),
            artifacts: serde_json::from_str(&r.get::<String, _>("artifacts")).unwrap_or_default(),
            parity_matched: r.get("parity_matched"),
            parity_mismatched: r.get("parity_mismatched"),
        })
        .collect())
}

/// What each agent holds (from Applied).
pub async fn list_plugins(pool: &DbPool, machine: Option<&str>) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT * FROM agent_plugins WHERE (? IS NULL OR machine = ? COLLATE NOCASE) ORDER BY machine, name")
        .bind(machine)
        .bind(machine)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .map(|r| serde_json::json!({"machine": r.get::<String,_>("machine"), "name": r.get::<String,_>("name"), "version": r.get::<String,_>("version"), "ok": r.get::<i64,_>("ok") != 0, "error": r.get::<String,_>("error"), "ts_ms": r.get::<i64,_>("ts_ms")}))
        .collect())
}

/// What to send now: updates for missing/changed plugins, removals, and the schedule set when its
/// version differs. `have` is updated as if the agent applied everything.
#[allow(clippy::too_many_arguments)]
pub async fn sync_messages(
    pool: &DbPool,
    layout: &Layout,
    machine: &str,
    have: &mut BTreeMap<String, String>,
    schedule_version: &mut String,
    config_version: &mut String,
    running: &str,
    upgrade_sent: &mut String,
) -> Result<Vec<pb::server_message::Body>> {
    let mut out = vec![];
    // v0.21.0: one offer per session and version (the agent answers with Applied "upgrade").
    match crate::upgrades::offer_for(pool, layout, machine, running).await {
        Ok(Some(o)) if *upgrade_sent != o.version => {
            *upgrade_sent = o.version.clone();
            out.push(pb::server_message::Body::Upgrade(o));
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(%machine, error=%format!("{e:#}"), "agent upgrade offer"),
    }
    let cu = config_update(pool, layout, machine).await?;
    if *config_version != cu.version {
        *config_version = cu.version.clone();
        out.push(pb::server_message::Body::Config(cu));
    }
    let want = desired(pool, layout, machine).await?;
    for (w, p) in &want {
        if have.get(&w.name) != Some(&w.key()) {
            match update_for(layout, w, p) {
                Ok(u) => out.push(pb::server_message::Body::Plugin(u)),
                Err(e) => {
                    tracing::warn!(plugin=%w.name, error=%format!("{e:#}"), "plugin update for agent")
                }
            }
            have.insert(w.name.clone(), w.key());
        }
    }
    let names: Vec<String> = have.keys().cloned().collect();
    for n in names {
        if !want.iter().any(|(w, _)| w.name == n) {
            out.push(pb::server_message::Body::PluginRemove(pb::PluginRemove {
                name: n.clone(),
            }));
            have.remove(&n);
        }
    }
    let set = schedule_set(pool, machine).await?;
    if *schedule_version != set.version {
        *schedule_version = set.version.clone();
        out.push(pb::server_message::Body::Schedules(set));
    }
    Ok(out)
}
