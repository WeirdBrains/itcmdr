//! Site alerts (v0.16.0, docs/design/SERVER-AGENT.md section 5).
//!
//! Levels: critical | cautious | informational. Sources: plugin (plugin errors), check (agent check results),
//! backup (scheduled/manual backup outcome), package (quarantine), server. There are no external notifiers:
//! alerts are shown in the on-site app and `itcmdr-cli` (and, from v0.19.0, go up to adminsvr redacted).
//!
//! Dedupe: one row per `key` while it is open, acked or silenced (count++, last_seen, newest title/detail; the
//! level only goes up). A resolved alert raised again within `alerts.dedupe_window_mins` reopens; later it starts
//! a new row. Auto-resolve: the same source reporting ok (`resolve_key` / `resolve_prefix`). Silences match on
//! machine and/or source until a time; a new alert that matches one is stored as `silenced`.

use crate::db::DbPool;
use anyhow::Result;
use serde::Serialize;
use sqlx::Row;

pub const LEVELS: [&str; 3] = ["critical", "cautious", "informational"];

fn rank(level: &str) -> i32 {
    match level {
        "critical" => 3,
        "cautious" => 2,
        _ => 1,
    }
}

/// Normalize a level name (also accepts the plugin error criticality names); unknown -> cautious.
pub fn level(s: &str) -> &'static str {
    match s.trim().to_ascii_lowercase().as_str() {
        "critical" | "error" => "critical",
        "informational" | "info" => "informational",
        _ => "cautious",
    }
}

pub const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS alerts (
        id TEXT PRIMARY KEY,
        key TEXT NOT NULL,
        level TEXT NOT NULL,
        source TEXT NOT NULL,
        machine TEXT NOT NULL,
        title TEXT NOT NULL,
        detail TEXT NOT NULL,
        first_seen_ms INTEGER NOT NULL,
        last_seen_ms INTEGER NOT NULL,
        count INTEGER NOT NULL,
        state TEXT NOT NULL,
        acked_by TEXT, acked_ms INTEGER,
        resolved_by TEXT, resolved_ms INTEGER,
        run_id TEXT
    );
    CREATE INDEX IF NOT EXISTS ix_alerts_key ON alerts(key, state);
    CREATE INDEX IF NOT EXISTS ix_alerts_state ON alerts(state, level);
    CREATE TABLE IF NOT EXISTS alert_silences (
        id TEXT PRIMARY KEY,
        machine TEXT NOT NULL,
        source TEXT NOT NULL,
        until_ms INTEGER NOT NULL,
        reason TEXT NOT NULL,
        created_by TEXT NOT NULL,
        created_ms INTEGER NOT NULL
    )
"#;

#[derive(Debug, Clone, Default)]
pub struct Raise {
    pub key: String,
    pub level: String,
    pub source: String,
    pub machine: String,
    pub title: String,
    pub detail: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Alert {
    pub id: String,
    pub key: String,
    pub level: String,
    pub source: String,
    pub machine: String,
    pub title: String,
    pub detail: String,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
    pub count: i64,
    pub state: String,
    pub acked_by: String,
    pub acked_ms: i64,
    pub resolved_by: String,
    pub resolved_ms: i64,
    pub run_id: String,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn row_to_alert(r: &sqlx::sqlite::SqliteRow) -> Alert {
    Alert {
        id: r.get("id"),
        key: r.get("key"),
        level: r.get("level"),
        source: r.get("source"),
        machine: r.get("machine"),
        title: r.get("title"),
        detail: r.get("detail"),
        first_seen_ms: r.get("first_seen_ms"),
        last_seen_ms: r.get("last_seen_ms"),
        count: r.get("count"),
        state: r.get("state"),
        acked_by: r.get::<Option<String>, _>("acked_by").unwrap_or_default(),
        acked_ms: r.get::<Option<i64>, _>("acked_ms").unwrap_or_default(),
        resolved_by: r
            .get::<Option<String>, _>("resolved_by")
            .unwrap_or_default(),
        resolved_ms: r.get::<Option<i64>, _>("resolved_ms").unwrap_or_default(),
        run_id: r.get::<Option<String>, _>("run_id").unwrap_or_default(),
    }
}

async fn silenced(pool: &DbPool, machine: &str, source: &str, at: i64) -> Result<bool> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM alert_silences WHERE until_ms > ? AND (machine = '' OR machine = ?) AND (source = '' OR source = ?)",
    )
    .bind(at)
    .bind(machine)
    .bind(source)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

/// Raise (or dedupe into) an alert. Returns the alert id. Never fails the caller's work: callers log errors.
pub async fn raise(pool: &DbPool, a: &Raise) -> Result<String> {
    let t = now();
    let lvl = level(&a.level);
    let title: String = a.title.chars().take(300).collect();
    let detail: String = a.detail.chars().take(4000).collect();
    if let Some(r) = sqlx::query(
        "SELECT id, level FROM alerts WHERE key = ? AND state IN ('open','acked','silenced') ORDER BY last_seen_ms DESC LIMIT 1",
    )
    .bind(&a.key)
    .fetch_optional(pool)
    .await?
    {
        let id: String = r.get("id");
        let old: String = r.get("level");
        let new_lvl = if rank(lvl) > rank(&old) { lvl } else { old.as_str() };
        sqlx::query("UPDATE alerts SET count = count + 1, last_seen_ms = ?, title = ?, detail = ?, level = ?, run_id = COALESCE(?, run_id) WHERE id = ?")
            .bind(t)
            .bind(&title)
            .bind(&detail)
            .bind(new_lvl)
            .bind(&a.run_id)
            .bind(&id)
            .execute(pool)
            .await?;
        return Ok(id);
    }
    let window = crate::settings::get_i64(pool, None, "alerts.dedupe_window_mins")
        .await
        .max(0)
        * 60_000;
    let state = if silenced(pool, &a.machine, &a.source, t).await? {
        "silenced"
    } else {
        "open"
    };
    if let Some(id) = sqlx::query_scalar::<_, String>(
        "SELECT id FROM alerts WHERE key = ? AND state = 'resolved' AND resolved_ms >= ? ORDER BY resolved_ms DESC LIMIT 1",
    )
    .bind(&a.key)
    .bind(t - window)
    .fetch_optional(pool)
    .await?
    {
        sqlx::query("UPDATE alerts SET state = ?, count = count + 1, last_seen_ms = ?, title = ?, detail = ?, level = ?, resolved_by = NULL, resolved_ms = NULL, acked_by = NULL, acked_ms = NULL, run_id = COALESCE(?, run_id) WHERE id = ?")
            .bind(state)
            .bind(t)
            .bind(&title)
            .bind(&detail)
            .bind(lvl)
            .bind(&a.run_id)
            .bind(&id)
            .execute(pool)
            .await?;
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO alerts(id, key, level, source, machine, title, detail, first_seen_ms, last_seen_ms, count, state, run_id) VALUES (?,?,?,?,?,?,?,?,?,1,?,?)")
        .bind(&id)
        .bind(&a.key)
        .bind(lvl)
        .bind(&a.source)
        .bind(&a.machine)
        .bind(&title)
        .bind(&detail)
        .bind(t)
        .bind(t)
        .bind(state)
        .bind(&a.run_id)
        .execute(pool)
        .await?;
    Ok(id)
}

/// Auto-resolve the unresolved alert with this key (the source reported ok again). Returns rows resolved.
pub async fn resolve_key(pool: &DbPool, key: &str) -> Result<u64> {
    Ok(sqlx::query("UPDATE alerts SET state = 'resolved', resolved_by = 'auto', resolved_ms = ? WHERE key = ? AND state IN ('open','acked','silenced')")
        .bind(now())
        .bind(key)
        .execute(pool)
        .await?
        .rows_affected())
}

/// Auto-resolve every unresolved alert whose key starts with `prefix`.
pub async fn resolve_prefix(pool: &DbPool, prefix: &str) -> Result<u64> {
    let like = format!(
        "{}%",
        prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    Ok(sqlx::query("UPDATE alerts SET state = 'resolved', resolved_by = 'auto', resolved_ms = ? WHERE key LIKE ? ESCAPE '\\' AND state IN ('open','acked','silenced')")
        .bind(now())
        .bind(like)
        .execute(pool)
        .await?
        .rows_affected())
}

/// Auto-resolve a plugin's alerts on one machine.
pub async fn resolve_plugin(pool: &DbPool, plugin: &str, machine: &str) -> Result<u64> {
    let like = format!(
        "plugin:{}:%",
        plugin
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    Ok(sqlx::query("UPDATE alerts SET state = 'resolved', resolved_by = 'auto', resolved_ms = ? WHERE key LIKE ? ESCAPE '\\' AND machine = ? AND state IN ('open','acked','silenced')")
        .bind(now())
        .bind(like)
        .bind(machine)
        .execute(pool)
        .await?
        .rows_affected())
}

pub async fn get(pool: &DbPool, id: &str) -> Result<Option<Alert>> {
    Ok(sqlx::query("SELECT * FROM alerts WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .map(|r| row_to_alert(&r)))
}

/// Ack an open/silenced alert (person has seen it). None when no such alert; Err if resolved.
pub async fn ack(pool: &DbPool, id: &str, actor: &str) -> Result<Option<Alert>> {
    sqlx::query("UPDATE alerts SET state = 'acked', acked_by = ?, acked_ms = ? WHERE id = ? AND state IN ('open','silenced')")
        .bind(actor)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    get(pool, id).await
}

pub async fn resolve(pool: &DbPool, id: &str, actor: &str) -> Result<Option<Alert>> {
    sqlx::query("UPDATE alerts SET state = 'resolved', resolved_by = ?, resolved_ms = ? WHERE id = ? AND state <> 'resolved'")
        .bind(actor)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    get(pool, id).await
}

/// Silence machine and/or source (empty = any) until `until_ms`; open matching alerts become silenced.
/// Returns (silence id, alerts silenced now).
pub async fn silence(
    pool: &DbPool,
    machine: &str,
    source: &str,
    until_ms: i64,
    reason: &str,
    actor: &str,
) -> Result<(String, u64)> {
    anyhow::ensure!(
        !(machine.is_empty() && source.is_empty()),
        "give a machine or a source to silence"
    );
    anyhow::ensure!(until_ms > now(), "until must be in the future");
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO alert_silences(id, machine, source, until_ms, reason, created_by, created_ms) VALUES (?,?,?,?,?,?,?)")
        .bind(&id)
        .bind(machine)
        .bind(source)
        .bind(until_ms)
        .bind(reason)
        .bind(actor)
        .bind(now())
        .execute(pool)
        .await?;
    let n = sqlx::query("UPDATE alerts SET state = 'silenced' WHERE state = 'open' AND (? = '' OR machine = ?) AND (? = '' OR source = ?)")
        .bind(machine)
        .bind(machine)
        .bind(source)
        .bind(source)
        .execute(pool)
        .await?
        .rows_affected();
    Ok((id, n))
}

/// Silences past their time: their alerts go back to open (still unresolved). Run by the maintenance loop.
pub async fn expire_silences(pool: &DbPool) -> Result<u64> {
    let t = now();
    sqlx::query("DELETE FROM alert_silences WHERE until_ms <= ?")
        .bind(t)
        .execute(pool)
        .await?;
    let rows = sqlx::query("SELECT id, machine, source FROM alerts WHERE state = 'silenced'")
        .fetch_all(pool)
        .await?;
    let mut n = 0;
    for r in rows {
        let (id, m, s): (String, String, String) = (r.get("id"), r.get("machine"), r.get("source"));
        if !silenced(pool, &m, &s, t).await? {
            sqlx::query("UPDATE alerts SET state = 'open' WHERE id = ? AND state = 'silenced'")
                .bind(&id)
                .execute(pool)
                .await?;
            n += 1;
        }
    }
    Ok(n)
}

#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub level: Option<String>,
    /// open (open + acked + silenced) | resolved | all | a single state name
    pub state: Option<String>,
    pub machine: Option<String>,
    pub source: Option<String>,
    pub limit: i64,
}

pub async fn list(pool: &DbPool, f: &Filter) -> Result<Vec<Alert>> {
    let mut sql = String::from("SELECT * FROM alerts WHERE 1=1");
    let mut binds: Vec<String> = vec![];
    match f.state.as_deref().unwrap_or("open") {
        "all" => {}
        "open" => sql.push_str(" AND state IN ('open','acked','silenced')"),
        s => {
            sql.push_str(" AND state = ?");
            binds.push(s.into());
        }
    }
    if let Some(l) = &f.level {
        sql.push_str(" AND level = ?");
        binds.push(level(l).into());
    }
    if let Some(m) = &f.machine {
        sql.push_str(" AND machine = ?");
        binds.push(m.clone());
    }
    if let Some(s) = &f.source {
        sql.push_str(" AND source = ?");
        binds.push(s.clone());
    }
    let limit = if f.limit <= 0 { 200 } else { f.limit.min(5000) };
    sql.push_str(&format!(" ORDER BY CASE level WHEN 'critical' THEN 0 WHEN 'cautious' THEN 1 ELSE 2 END, last_seen_ms DESC LIMIT {limit}"));
    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = q.bind(b);
    }
    Ok(q.fetch_all(pool).await?.iter().map(row_to_alert).collect())
}

/// Unresolved, unsilenced alerts per level (critical, cautious, informational): the app badge.
pub async fn open_counts(pool: &DbPool) -> Result<[i64; 3]> {
    let mut out = [0i64; 3];
    for (i, l) in LEVELS.iter().enumerate() {
        out[i] = sqlx::query_scalar(
            "SELECT count(*) FROM alerts WHERE level = ? AND state IN ('open','acked')",
        )
        .bind(l)
        .fetch_one(pool)
        .await?;
    }
    Ok(out)
}

// ---- sources ----

/// A plugin run finished: each plugin error raises an alert at its criticality (key plugin:<name>:<kind>:<machine>);
/// a run that ended ok with no errors resolves that plugin's alerts on that machine.
pub async fn from_plugin_run(pool: &DbPool, rep: &crate::runtime::RunReport, machine: &str) {
    for e in &rep.errors {
        let r = Raise {
            key: format!("plugin:{}:{}:{}", rep.plugin, e.kind, machine),
            level: e.criticality.as_str().into(),
            source: "plugin".into(),
            machine: machine.into(),
            title: format!(
                "{} {}: {}",
                rep.plugin,
                e.kind,
                e.error.lines().next().unwrap_or_default()
            ),
            detail: e.error.clone(),
            run_id: Some(rep.run_id.clone()),
        };
        if let Err(err) = raise(pool, &r).await {
            tracing::warn!(error=%err, "alert raise failed");
        }
    }
    if rep.status == "ok" && rep.errors.is_empty() {
        if let Err(e) = resolve_plugin(pool, &rep.plugin, machine).await {
            tracing::warn!(error=%e, "alert resolve failed");
        }
    }
}

/// An agent check record was stored: failed (not excepted) -> alert (critical when the check's severity is
/// critical, else cautious); a repair that worked -> informational; ok -> resolve.
pub async fn from_check(pool: &DbPool, rec: &crate::ingest::AgentRecord) {
    let key = format!("check:{}:{}", rec.check_id, rec.computer);
    let status = rec.status.to_ascii_lowercase();
    let repaired = rec.attempted_repair()
        && rec
            .result
            .as_deref()
            .is_some_and(|r| r.eq_ignore_ascii_case("success"));
    let res = if status == "failed" && !rec.excepted.unwrap_or(false) {
        let crit = rec.severity.eq_ignore_ascii_case("critical");
        raise(
            pool,
            &Raise {
                key,
                level: if crit { "critical" } else { "cautious" }.into(),
                source: "check".into(),
                machine: rec.computer.clone(),
                title: format!("{} failed on {}", rec.check_id, rec.computer),
                detail: format!(
                    "category {}; repair: {}",
                    rec.category,
                    rec.result.as_deref().unwrap_or("none")
                ),
                run_id: Some(rec.run_id.clone()),
            },
        )
        .await
        .map(|_| ())
    } else if status == "ok" || status == "skipped" {
        let r = resolve_key(pool, &key).await.map(|_| ());
        if repaired {
            let _ = raise(
                pool,
                &Raise {
                    key: format!("repair:{}:{}", rec.check_id, rec.computer),
                    level: "informational".into(),
                    source: "check".into(),
                    machine: rec.computer.clone(),
                    title: format!("{} repaired on {}", rec.check_id, rec.computer),
                    detail: format!("category {}", rec.category),
                    run_id: Some(rec.run_id.clone()),
                },
            )
            .await;
        }
        r
    } else {
        Ok(())
    };
    if let Err(e) = res {
        tracing::warn!(error=%e, "alert from check failed");
    }
}

/// A backup finished (key backup:<kind>): failure -> critical, success -> resolve.
pub async fn from_backup(pool: &DbPool, kind: &str, outcome: Result<(), String>) {
    let key = format!("backup:{kind}");
    let r = match outcome {
        Ok(()) => resolve_key(pool, &key).await.map(|_| ()),
        Err(msg) => raise(
            pool,
            &Raise {
                key,
                level: "critical".into(),
                source: "backup".into(),
                machine: crate::runtime::hostname(),
                title: format!("{kind} backup failed"),
                detail: msg,
                run_id: None,
            },
        )
        .await
        .map(|_| ()),
    };
    if let Err(e) = r {
        tracing::warn!(error=%e, "alert from backup failed");
    }
}

/// A plugin package was quarantined (cautious; resolved by hand).
pub async fn from_quarantine(pool: &DbPool, label: &str, reason: &str) {
    let r = raise(
        pool,
        &Raise {
            key: format!("quarantine:{label}"),
            level: "cautious".into(),
            source: "package".into(),
            machine: crate::runtime::hostname(),
            title: format!("plugin package quarantined: {label}"),
            detail: reason.into(),
            run_id: None,
        },
    )
    .await;
    if let Err(e) = r {
        tracing::warn!(error=%e, "alert from quarantine failed");
    }
}
