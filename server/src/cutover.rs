//! Built-in default schedules and the safe cutover from the SYSTEM task (decisions 10 and 14, v0.11.0).
//!
//! - **Default schedules**, seeded idempotently on every start (fresh install or upgrade):
//!   `nightly-agent-run` (the nightly task's time, all machines, all categories, repairs on,
//!   clinical guard on) and `nightly-backup` (replaces the hard-wired backup check). Editable and
//!   pausable; deleting one needs owner/admin plus an explicit confirm and is remembered, so an
//!   upgrade doesn't bring it back; `reset-to-default` restores it.
//! - **Cutover state** (`cutover.state` = `task` | `schedule`). Until the switch the task is the
//!   source of truth, the agent schedule stays paused, and every nightly run also runs the plugin
//!   versions report-only and writes a `parity` line. `switch` needs a clean week of parity, the
//!   Windows VM verification flag, and every recently active machine reading `schedule-state.json`.
//!   `rollback` is the one-command way back. Agents follow the state through `schedule-state.json`
//!   (written every scheduler pass) and run locally when it goes stale (server unreachable).
use crate::config::ServerConfig;
use crate::db::DbPool;
use crate::schedule::{self, Schedule, Spec};
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use serde::Serialize;
use std::path::PathBuf;

pub const AGENT_KEY: &str = "nightly-agent-run";
pub const BACKUP_KEY: &str = "nightly-backup";
pub const KEYS: [&str; 2] = [AGENT_KEY, BACKUP_KEY];
pub const CATEGORIES: [&str; 5] = [
    "network",
    "connectivity",
    "configuration",
    "backup",
    "services",
];
/// Full days of matching results needed before the switch (decision 10).
pub const CLEAN_DAYS: i64 = 7;
/// A machine "sees schedules" when it read schedule-state.json within this window.
pub const SEEN_WINDOW_H: i64 = 48;
pub const STATE_FILE: &str = "schedule-state.json";

async fn meta(pool: &DbPool, key: &str) -> Result<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM server_meta WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?,
    )
}

async fn set_meta(pool: &DbPool, key: &str, value: Option<&str>) -> Result<()> {
    match value {
        Some(v) => sqlx::query("INSERT INTO server_meta(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
            .bind(key).bind(v).execute(pool).await?,
        None => sqlx::query("DELETE FROM server_meta WHERE key = ?").bind(key).execute(pool).await?,
    };
    Ok(())
}

/// Remember (or forget) that a built-in default was deleted on purpose.
pub async fn mark_deleted(pool: &DbPool, key: &str, deleted: bool) -> Result<()> {
    set_meta(
        pool,
        &format!("schedule.default.{key}.deleted"),
        deleted.then_some("1"),
    )
    .await
}

pub async fn state(pool: &DbPool) -> Result<String> {
    Ok(meta(pool, "cutover.state")
        .await?
        .unwrap_or_else(|| "task".into()))
}

pub async fn vm_verified(pool: &DbPool) -> Result<bool> {
    Ok(meta(pool, "cutover.vm_verified").await?.is_some())
}

/// Record that the plugin runtime + poller were verified on the Windows VM (gate b).
pub async fn set_vm_verified(pool: &DbPool, actor: &str, on: bool) -> Result<()> {
    let v = on.then(|| format!("{} {}", Utc::now().to_rfc3339(), actor));
    set_meta(pool, "cutover.vm_verified", v.as_deref()).await?;
    let _ = crate::db::insert_audit(
        pool,
        actor,
        "cutover-vm-verified",
        &serde_json::json!({ "verified": on }).to_string(),
    )
    .await;
    Ok(())
}

/// The default (name, spec, enabled) of a built-in, from today's settings: the agent run uses
/// `agent.nightlyTime` (the task's trigger time), the backup uses `backup.hour` / `backup.nightly`.
pub async fn default_spec(
    pool: &DbPool,
    cfg: &ServerConfig,
    key: &str,
) -> Result<(String, Spec, bool)> {
    match key {
        AGENT_KEY => {
            let t = crate::settings::get(pool, Some(cfg), "agent.nightlyTime")
                .await
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "01:30".into());
            let spec = Spec {
                all_machines: true,
                categories: CATEGORIES.iter().map(|s| s.to_string()).collect(),
                // Same as the SYSTEM task: repairs on; the agent's clinical guard still applies.
                repair: true,
                when: format!("daily {t}"),
                timezone: "local".into(),
                guard: "report-only-in-hours".into(),
                missed: "skip".into(),
                builtin: true,
                builtin_key: AGENT_KEY.into(),
                ..Default::default()
            };
            Ok((
                "Nightly agent run".into(),
                spec,
                state(pool).await? == "schedule",
            ))
        }
        BACKUP_KEY => {
            let bc = crate::settings::effective_backup(pool, cfg).await?;
            let spec = Spec {
                when: format!("daily {:02}:00", bc.hour),
                timezone: "local".into(),
                guard: "none".into(),
                // The old loop took the backup any time after the hour: catch up after downtime.
                missed: "catch-up".into(),
                builtin: true,
                builtin_key: BACKUP_KEY.into(),
                action: "backup".into(),
                ..Default::default()
            };
            Ok(("Nightly backup".into(), spec, bc.nightly))
        }
        other => bail!("unknown built-in schedule {other:?} (one of {KEYS:?})"),
    }
}

pub async fn find_builtin(pool: &DbPool, key: &str) -> Result<Option<Schedule>> {
    Ok(schedule::list(pool)
        .await?
        .into_iter()
        .find(|s| s.spec.builtin && s.spec.builtin_key == key))
}

/// Seed missing built-ins (not ones deleted on purpose). Idempotent. Returns the names created.
pub async fn seed_defaults(pool: &DbPool, cfg: &ServerConfig) -> Result<Vec<String>> {
    let mut made = Vec::new();
    for key in KEYS {
        if find_builtin(pool, key).await?.is_some()
            || meta(pool, &format!("schedule.default.{key}.deleted"))
                .await?
                .is_some()
        {
            continue;
        }
        made.push(create_default(pool, cfg, key, "itcmdr-server").await?.name);
    }
    Ok(made)
}

async fn create_default(
    pool: &DbPool,
    cfg: &ServerConfig,
    key: &str,
    actor: &str,
) -> Result<Schedule> {
    let (mut name, spec, enabled) = default_spec(pool, cfg, key).await?;
    if schedule::get(pool, &name).await.is_ok() {
        name.push_str(" (built-in)");
    }
    let s = schedule::create(pool, &name, spec, enabled).await?;
    if key == BACKUP_KEY {
        // Like the old loop: today's backup is still due if the hour has passed.
        let midnight = Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|t| Local.from_local_datetime(&t).single())
            .map(|t| t.timestamp_millis())
            .unwrap_or(s.last_fire_ms);
        schedule::set_last_fire(pool, &s.id, midnight).await?;
    }
    let _ = crate::db::insert_audit(
        pool,
        actor,
        "schedule-seed",
        &serde_json::json!({"key": key, "id": s.id, "name": s.name, "enabled": enabled})
            .to_string(),
    )
    .await;
    schedule::get(pool, &s.id).await
}

/// `key`, or the id/name of a built-in schedule, to its key.
async fn key_of(pool: &DbPool, which: &str) -> Result<String> {
    if KEYS.contains(&which) {
        return Ok(which.into());
    }
    match schedule::get(pool, which).await {
        Ok(s) if s.spec.builtin => Ok(s.spec.builtin_key),
        Ok(s) => bail!("{} is not a built-in schedule", s.name),
        Err(_) => Err(anyhow!(
            "unknown built-in schedule {which:?} (one of {KEYS:?})"
        )),
    }
}

/// Restore a built-in to its default settings (re-creating it if it was deleted).
pub async fn reset_to_default(
    pool: &DbPool,
    cfg: &ServerConfig,
    which: &str,
    actor: &str,
) -> Result<Schedule> {
    let key = key_of(pool, which).await?;
    mark_deleted(pool, &key, false).await?;
    let s = match find_builtin(pool, &key).await? {
        None => create_default(pool, cfg, &key, actor).await?,
        Some(cur) => {
            let (_, spec, enabled) = default_spec(pool, cfg, &key).await?;
            let s = schedule::update(pool, &cur.id, None, spec).await?;
            if s.enabled != enabled {
                schedule::set_enabled(pool, &s.id, enabled).await?
            } else {
                s
            }
        }
    };
    let _ = crate::db::insert_audit(
        pool,
        actor,
        "schedule-reset",
        &serde_json::json!({"key": key, "id": s.id, "name": s.name}).to_string(),
    )
    .await;
    Ok(s)
}

fn local_day(ts_ms: i64) -> String {
    Local
        .timestamp_millis_opt(ts_ms)
        .single()
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// Store one `parity` line from a nightly log (idempotent per run).
pub async fn record_parity(
    pool: &DbPool,
    run_id: &str,
    machine: &str,
    ts_ms: i64,
    matched: i64,
    mismatched: &[serde_json::Value],
    errors: &[serde_json::Value],
) -> Result<()> {
    let detail = serde_json::json!({"mismatched": mismatched, "errors": errors}).to_string();
    sqlx::query("INSERT INTO parity_runs(run_id, machine, ts_ms, day, matched, mismatched, errors, detail) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(run_id) DO UPDATE SET matched = excluded.matched, mismatched = excluded.mismatched, errors = excluded.errors, detail = excluded.detail")
        .bind(run_id)
        .bind(machine)
        .bind(ts_ms)
        .bind(local_day(ts_ms))
        .bind(matched)
        .bind(mismatched.len() as i64)
        .bind(errors.len() as i64)
        .bind(detail)
        .execute(pool)
        .await?;
    Ok(())
}

/// Latest cutover view of a machine (from its run-start lines).
pub async fn record_agent_state(
    pool: &DbPool,
    machine: &str,
    ts_ms: i64,
    cutover: &str,
    trigger: &str,
    schedule_state_ms: i64,
    plugin_mode: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO agent_state(machine, ts_ms, cutover, trigger, schedule_state_ms, plugin_mode) VALUES (?,?,?,?,?,?) ON CONFLICT(machine) DO UPDATE SET ts_ms = excluded.ts_ms, cutover = excluded.cutover, trigger = excluded.trigger, schedule_state_ms = excluded.schedule_state_ms, plugin_mode = excluded.plugin_mode WHERE excluded.ts_ms >= agent_state.ts_ms")
        .bind(machine)
        .bind(ts_ms)
        .bind(cutover)
        .bind(trigger)
        .bind(schedule_state_ms)
        .bind(plugin_mode)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct DayParity {
    pub day: String,
    pub runs: i64,
    pub matched: i64,
    pub mismatched: i64,
    pub errors: i64,
    pub machines: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineView {
    pub machine: String,
    pub last_run_ms: i64,
    pub cutover: String,
    pub trigger: String,
    pub plugin_mode: String,
    pub schedule_state_ms: i64,
    pub sees_schedules: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Gate {
    pub state: String,
    pub ok: bool,
    pub vm_verified: bool,
    pub days: Vec<DayParity>,
    pub machines: Vec<MachineView>,
    pub problems: Vec<String>,
}

/// Can we switch? (a) the last CLEAN_DAYS full local days each have parity results, all
/// matching, for every machine that ran; (b) the VM flag; (c) every machine active in the last
/// two days reads schedule-state.json.
pub async fn gate(pool: &DbPool, now: DateTime<Utc>) -> Result<Gate> {
    let mut problems = Vec::new();
    let today = now.with_timezone(&Local).date_naive();
    let mut days = Vec::new();
    for d in (1..=CLEAN_DAYS).rev() {
        let day = (today - Duration::days(d)).format("%Y-%m-%d").to_string();
        let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
            "SELECT machine, matched, mismatched, errors FROM parity_runs WHERE day = ? ORDER BY machine",
        )
        .bind(&day)
        .fetch_all(pool)
        .await?;
        let start = Local
            .from_local_datetime(&(today - Duration::days(d)).and_hms_opt(0, 0, 0).unwrap())
            .single()
            .map(|t| t.timestamp_millis())
            .unwrap_or(0);
        let ran: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT computer FROM runs WHERE ts_ms >= ? AND ts_ms < ? ORDER BY computer",
        )
        .bind(start)
        .bind(start + 86_400_000)
        .fetch_all(pool)
        .await?;
        let mut machines: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
        machines.dedup();
        let dp = DayParity {
            day: day.clone(),
            runs: rows.len() as i64,
            matched: rows.iter().map(|r| r.1).sum(),
            mismatched: rows.iter().map(|r| r.2).sum(),
            errors: rows.iter().map(|r| r.3).sum(),
            machines: machines.clone(),
        };
        if dp.runs == 0 {
            problems.push(format!("{day}: no parity results"));
        }
        if dp.mismatched > 0 {
            problems.push(format!("{day}: {} mismatched check(s)", dp.mismatched));
        }
        if dp.errors > 0 {
            problems.push(format!("{day}: {} plugin error(s)", dp.errors));
        }
        for m in ran.iter().filter(|m| !machines.contains(m)) {
            problems.push(format!("{day}: {m} ran without a parity result"));
        }
        days.push(dp);
    }
    let rows: Vec<(String, i64, String, String, i64, String)> = sqlx::query_as(
        "SELECT machine, ts_ms, cutover, trigger, schedule_state_ms, plugin_mode FROM agent_state ORDER BY machine",
    )
    .fetch_all(pool)
    .await?;
    let window = SEEN_WINDOW_H * 3_600_000;
    let now_ms = now.timestamp_millis();
    let machines: Vec<MachineView> = rows
        .into_iter()
        .map(|(machine, ts, cutover, trigger, ss, pm)| MachineView {
            machine,
            last_run_ms: ts,
            cutover,
            trigger,
            plugin_mode: pm,
            schedule_state_ms: ss,
            sees_schedules: ss > 0 && now_ms - ss < window,
        })
        .collect();
    let active: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT computer FROM runs WHERE ts_ms >= ? ORDER BY computer")
            .bind(now_ms - window)
            .fetch_all(pool)
            .await?;
    for m in &active {
        if !machines.iter().any(|v| &v.machine == m && v.sees_schedules) {
            problems.push(format!(
                "{m}: has not read {STATE_FILE} in the last {SEEN_WINDOW_H} h"
            ));
        }
    }
    let vm = vm_verified(pool).await?;
    if !vm {
        problems
            .push("Windows VM verification not recorded (itcmdr-cli cutover vm-verified)".into());
    }
    Ok(Gate {
        state: state(pool).await?,
        ok: problems.is_empty(),
        vm_verified: vm,
        days,
        machines,
        problems,
    })
}

/// Switch the nightly run from the SYSTEM task to the default schedule (gated).
pub async fn switch(
    pool: &DbPool,
    cfg: &ServerConfig,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<Gate> {
    let g = gate(pool, now).await?;
    if !g.ok {
        bail!("not safe to switch yet:\n  - {}", g.problems.join("\n  - "));
    }
    let s = find_builtin(pool, AGENT_KEY)
        .await?
        .ok_or_else(|| anyhow!("the nightly agent run schedule is missing: itcmdr-cli schedule reset-to-default {AGENT_KEY}"))?;
    set_meta(pool, "cutover.state", Some("schedule")).await?;
    set_meta(pool, "cutover.switched", Some(&now.to_rfc3339())).await?;
    schedule::set_enabled(pool, &s.id, true).await?;
    let _ = crate::db::insert_audit(
        pool,
        actor,
        "cutover-switch",
        &serde_json::json!({"schedule": s.id}).to_string(),
    )
    .await;
    export_state(pool, cfg).await?;
    gate(pool, now).await
}

/// One-command rollback: the task is the source of truth again; the agent schedule is paused.
pub async fn rollback(pool: &DbPool, cfg: &ServerConfig, actor: &str) -> Result<()> {
    set_meta(pool, "cutover.state", Some("task")).await?;
    if let Some(s) = find_builtin(pool, AGENT_KEY).await? {
        schedule::set_enabled(pool, &s.id, false).await?;
    }
    let _ = crate::db::insert_audit(pool, actor, "cutover-rollback", "{}").await;
    export_state(pool, cfg).await?;
    Ok(())
}

fn sched_json(s: &Schedule) -> serde_json::Value {
    let next = schedule::next_times(&s.spec, Utc::now(), 1)
        .ok()
        .and_then(|v| v.first().map(|t| t.with_timezone(&Local).to_rfc3339()));
    serde_json::json!({"id": s.id, "name": s.name, "enabled": s.enabled, "when": s.spec.when, "next": next})
}

pub fn state_path(cfg: &ServerConfig) -> PathBuf {
    cfg.logs_dir.join(STATE_FILE)
}

/// Write `<logs_dir>/schedule-state.json` for the agents (every scheduler pass).
pub async fn export_state(pool: &DbPool, cfg: &ServerConfig) -> Result<PathBuf> {
    let nightly = find_builtin(pool, AGENT_KEY).await?;
    let backup = find_builtin(pool, BACKUP_KEY).await?;
    let doc = serde_json::json!({
        "v": 1,
        "ts": Local::now().to_rfc3339(),
        "cutover": state(pool).await?,
        "nightly": nightly.as_ref().map(sched_json),
        "backup": backup.as_ref().map(sched_json),
    });
    let p = state_path(cfg);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&doc)?)?;
    std::fs::rename(&tmp, &p)?;
    Ok(p)
}
