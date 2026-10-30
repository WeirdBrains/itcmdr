//! Server-side scheduler (decision 14). Schedules live in the encrypted DB; due runs are queued
//! through the on-demand path (`jobs::queue_run`), so pollers, locks and request status are reused.
//!
//! - `when`: a preset (`hourly`, `every 6h`, `daily 01:30`, `weekdays 07:00`, `weekly mon 02:00`)
//!   or `cron <min> <hour> <dom> <month> <dow>` (5 fields), evaluated in the schedule's timezone
//!   (IANA name, or `local` = the server's zone). DST: times that don't exist are skipped by the
//!   cron engine; repeated times fire once.
//! - Office-hours guard (`schedule.office_hours`, default `Mon-Fri 07:00-18:00`, schedule tz):
//!   `report-only-in-hours` (default) runs repairs as report-only inside office hours,
//!   `skip-in-hours` skips the run, `none` ignores hours. The agent's own clinical guard still applies.
//! - Missed runs (server down past `schedule.grace_secs`): `skip` records them as missed,
//!   `catch-up` runs once. Never more than one queued/running request per schedule.
use crate::config::ServerConfig;
use crate::db::DbPool;
use crate::jobs::RunRequest;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Datelike, NaiveTime, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Spec {
    pub all_machines: bool,
    pub machines: Vec<String>,
    /// Machine groups = agent roles (Server, Workstation, RCU) as last reported by each machine.
    pub groups: Vec<String>,
    pub categories: Vec<String>,
    pub plugins: Vec<String>,
    pub repair: bool,
    pub when: String,
    pub timezone: String,
    pub guard: String,
    pub missed: String,
    /// Built-in schedules (v0.11.0) can't be deleted by accident.
    pub builtin: bool,
    /// Which built-in default this is (`nightly-agent-run`, `nightly-backup`); empty otherwise.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub builtin_key: String,
    /// "" = queue a run (categories/plugins); "backup" = take the nightly server backup.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub action: String,
}

pub const ACTIONS: [&str; 2] = ["", "backup"];

pub const GUARDS: [&str; 3] = ["report-only-in-hours", "skip-in-hours", "none"];
pub const MISSED: [&str; 2] = ["skip", "catch-up"];

#[derive(Debug, Clone, Serialize)]
pub struct Schedule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub spec: Spec,
    pub last_fire_ms: i64,
    pub created_ms: i64,
    pub updated_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunRow {
    pub schedule_id: String,
    pub ts_ms: i64,
    pub due_ms: i64,
    pub request_id: Option<String>,
    pub outcome: String,
    pub note: String,
}

/// Turn a preset or `cron ...` into a 6-field (seconds first) expression for the cron crate.
pub fn to_cron(when: &str) -> Result<String> {
    let w = when.trim().to_ascii_lowercase();
    let parts: Vec<&str> = w.split_whitespace().collect();
    let hm = |s: &str| -> Result<(u32, u32)> {
        let t =
            NaiveTime::parse_from_str(s, "%H:%M").map_err(|_| anyhow!("bad time {s:?} (HH:mm)"))?;
        Ok((
            t.format("%H").to_string().parse()?,
            t.format("%M").to_string().parse()?,
        ))
    };
    let e = match parts.as_slice() {
        ["hourly"] => "0 0 * * * *".to_string(),
        ["every", n] if n.ends_with('h') => {
            let n: u32 = n.trim_end_matches('h').parse().map_err(|_| anyhow!("bad interval {n}"))?;
            if !(1..=23).contains(&n) {
                bail!("every Nh needs 1..23");
            }
            format!("0 0 */{n} * * *")
        }
        ["daily", t] => {
            let (h, m) = hm(t)?;
            format!("0 {m} {h} * * *")
        }
        ["weekdays", t] => {
            let (h, m) = hm(t)?;
            format!("0 {m} {h} * * Mon-Fri")
        }
        ["weekly", d, t] => {
            let (h, m) = hm(t)?;
            let d = Weekday::from_str(d).map_err(|_| anyhow!("bad weekday {d}"))?;
            format!("0 {m} {h} * * {d:?}")
        }
        ["cron", rest @ ..] if rest.len() == 5 => format!("0 {}", rest.join(" ")),
        _ => bail!(
            "unknown schedule {when:?}: use hourly, every Nh, daily HH:mm, weekdays HH:mm, weekly <day> HH:mm or cron <5 fields>"
        ),
    };
    cron::Schedule::from_str(&e).map_err(|x| anyhow!("invalid cron {when:?}: {x}"))?;
    Ok(e)
}

/// Next fire times strictly after `after`, in UTC.
pub fn next_times(spec: &Spec, after: DateTime<Utc>, n: usize) -> Result<Vec<DateTime<Utc>>> {
    let s = cron::Schedule::from_str(&to_cron(&spec.when)?).map_err(|e| anyhow!("{e}"))?;
    let tz = spec.timezone.trim();
    // "local" = the host's IANA zone when it can be named: chrono's Local on Windows put a
    // 02:00 run at 01:00 on the fall-back day (seen on the Windows VM, v0.11.0).
    let local_zone = || {
        iana_time_zone::get_timezone()
            .ok()
            .and_then(|n| n.parse::<chrono_tz::Tz>().ok())
    };
    Ok(if tz.is_empty() || tz.eq_ignore_ascii_case("local") {
        match local_zone() {
            Some(z) => first_of_repeats(&s, &after.with_timezone(&z), n),
            None => first_of_repeats(&s, &after.with_timezone(&chrono::Local), n),
        }
    } else {
        let z: chrono_tz::Tz = tz.parse().map_err(|_| anyhow!("unknown timezone {tz:?}"))?;
        first_of_repeats(&s, &after.with_timezone(&z), n)
    })
}

/// Upcoming times, skipping the second occurrence of a wall time that repeats when the clocks go
/// back (01:30 on the last day of DST fires once, at the first 01:30).
fn first_of_repeats<Z: TimeZone>(
    s: &cron::Schedule,
    after: &DateTime<Z>,
    n: usize,
) -> Vec<DateTime<Utc>> {
    s.after(after)
        .filter(
            |t| match t.timezone().from_local_datetime(&t.naive_local()) {
                chrono::LocalResult::Ambiguous(first, _) => first == *t,
                _ => true,
            },
        )
        .take(n)
        .map(|t| t.with_timezone(&Utc))
        .collect()
}

/// `Mon-Fri 07:00-18:00` (days optional, ranges or lists like `Mon,Wed`).
#[derive(Debug, Clone, PartialEq)]
pub struct Hours {
    pub days: Vec<Weekday>,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

pub fn parse_hours(s: &str) -> Result<Hours> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    let (days_s, times) = match parts.as_slice() {
        [t] => ("Mon-Sun", *t),
        [d, t] => (*d, *t),
        _ => bail!("office hours must look like \"Mon-Fri 07:00-18:00\""),
    };
    let (a, b) = times
        .split_once('-')
        .ok_or_else(|| anyhow!("bad hours {times}"))?;
    let t = |x: &str| NaiveTime::parse_from_str(x, "%H:%M").map_err(|_| anyhow!("bad time {x}"));
    let (start, end) = (t(a)?, t(b)?);
    if start >= end {
        bail!("office hours must start before they end");
    }
    let mut days = Vec::new();
    for item in days_s.split(',') {
        let wd = |x: &str| Weekday::from_str(x).map_err(|_| anyhow!("bad day {x}"));
        match item.split_once('-') {
            Some((x, y)) => {
                let (mut d, end_d) = (wd(x)?, wd(y)?);
                loop {
                    days.push(d);
                    if d == end_d {
                        break;
                    }
                    d = d.succ();
                }
            }
            None => days.push(wd(item)?),
        }
    }
    Ok(Hours { days, start, end })
}

/// Is `at` inside office hours, in the schedule's zone?
pub fn in_hours(h: &Hours, spec: &Spec, at: DateTime<Utc>) -> bool {
    let tz = spec.timezone.trim();
    let (wd, t) = if tz.is_empty() || tz.eq_ignore_ascii_case("local") {
        let l = at.with_timezone(&chrono::Local);
        (l.weekday(), l.time())
    } else {
        match tz.parse::<chrono_tz::Tz>() {
            Ok(z) => {
                let l = at.with_timezone(&z);
                (l.weekday(), l.time())
            }
            Err(_) => return false,
        }
    };
    h.days.contains(&wd) && t >= h.start && t < h.end
}

pub fn validate(name: &str, s: &mut Spec) -> Result<()> {
    if name.trim().is_empty() || name.len() > 100 {
        bail!("name must be 1-100 characters");
    }
    if s.guard.is_empty() {
        s.guard = GUARDS[0].into();
    }
    if s.missed.is_empty() {
        s.missed = "skip".into();
    }
    if s.timezone.is_empty() {
        s.timezone = "local".into();
    }
    if !GUARDS.contains(&s.guard.as_str()) {
        bail!("guard must be one of {GUARDS:?}");
    }
    if !MISSED.contains(&s.missed.as_str()) {
        bail!("missed must be one of {MISSED:?}");
    }
    to_cron(&s.when)?;
    next_times(s, Utc::now(), 1)?;
    if !ACTIONS.contains(&s.action.as_str()) {
        bail!("action must be one of {ACTIONS:?}");
    }
    if s.action == "backup" {
        // Runs on the server: no machines, categories or plugins.
        s.all_machines = false;
        s.machines.clear();
        s.groups.clear();
        s.categories.clear();
        s.plugins.clear();
        s.repair = false;
        return Ok(());
    }
    let targets =
        s.all_machines as u8 + (!s.machines.is_empty()) as u8 + (!s.groups.is_empty()) as u8;
    if s.plugins.is_empty() && targets != 1 {
        bail!("target exactly one of: all machines, machines, groups");
    }
    if s.categories.is_empty() && s.plugins.is_empty() && targets == 0 {
        bail!("nothing to run");
    }
    for p in &s.plugins {
        if p.trim().is_empty() {
            bail!("empty plugin name");
        }
    }
    Ok(())
}

fn row(r: sqlx::sqlite::SqliteRow) -> Result<Schedule> {
    use sqlx::Row;
    Ok(Schedule {
        id: r.try_get("id")?,
        name: r.try_get("name")?,
        enabled: r.try_get::<i64, _>("enabled")? != 0,
        spec: serde_json::from_str(&r.try_get::<String, _>("spec")?)?,
        last_fire_ms: r.try_get("last_fire_ms")?,
        created_ms: r.try_get("created_ms")?,
        updated_ms: r.try_get("updated_ms")?,
    })
}

pub async fn list(pool: &DbPool) -> Result<Vec<Schedule>> {
    sqlx::query("SELECT * FROM schedules ORDER BY name")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(row)
        .collect()
}

/// By id or name.
pub async fn get(pool: &DbPool, id_or_name: &str) -> Result<Schedule> {
    let r = sqlx::query("SELECT * FROM schedules WHERE id = ? OR name = ?")
        .bind(id_or_name)
        .bind(id_or_name)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow!("no schedule {id_or_name:?}"))?;
    row(r)
}

pub async fn create(pool: &DbPool, name: &str, mut spec: Spec, enabled: bool) -> Result<Schedule> {
    validate(name, &mut spec)?;
    let now = Utc::now().timestamp_millis();
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO schedules(id, name, spec, enabled, last_fire_ms, created_ms, updated_ms) VALUES (?,?,?,?,?,?,?)")
        .bind(&id)
        .bind(name.trim())
        .bind(serde_json::to_string(&spec)?)
        .bind(enabled as i64)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                anyhow!("a schedule named {name:?} exists")
            } else {
                e.into()
            }
        })?;
    get(pool, &id).await
}

pub async fn update(
    pool: &DbPool,
    id: &str,
    name: Option<&str>,
    mut spec: Spec,
) -> Result<Schedule> {
    let cur = get(pool, id).await?;
    let name = name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or(&cur.name)
        .to_string();
    spec.builtin = cur.spec.builtin;
    spec.builtin_key = cur.spec.builtin_key.clone();
    if cur.spec.builtin {
        // A built-in keeps what it does; timing, targets, categories, repair and guard stay editable.
        spec.action = cur.spec.action.clone();
    }
    validate(&name, &mut spec)?;
    // A changed timing starts counting from now (no burst of "missed" runs).
    let last = if spec.when != cur.spec.when || spec.timezone != cur.spec.timezone {
        Utc::now().timestamp_millis()
    } else {
        cur.last_fire_ms
    };
    sqlx::query(
        "UPDATE schedules SET name = ?, spec = ?, last_fire_ms = ?, updated_ms = ? WHERE id = ?",
    )
    .bind(name.trim())
    .bind(serde_json::to_string(&spec)?)
    .bind(last)
    .bind(Utc::now().timestamp_millis())
    .bind(&cur.id)
    .execute(pool)
    .await?;
    get(pool, &cur.id).await
}

/// Delete. A built-in needs `force` plus `confirm` = its exact name (callers check owner/admin) and is
/// then remembered as deleted, so upgrades don't re-seed it (`reset-to-default` brings it back).
pub async fn delete(
    pool: &DbPool,
    id: &str,
    force: bool,
    confirm: Option<&str>,
) -> Result<Schedule> {
    let s = get(pool, id).await?;
    if s.spec.builtin && !force {
        bail!(
            "{} is a built-in schedule: pause it instead (or delete with force, owner/admin only)",
            s.name
        );
    }
    if s.spec.builtin && confirm != Some(s.name.as_str()) {
        bail!(
            "{} is a built-in schedule: deleting it needs the confirm set to its exact name (pausing is usually what you want)",
            s.name
        );
    }
    sqlx::query("DELETE FROM schedules WHERE id = ?")
        .bind(&s.id)
        .execute(pool)
        .await?;
    if s.spec.builtin && !s.spec.builtin_key.is_empty() {
        crate::cutover::mark_deleted(pool, &s.spec.builtin_key, true).await?;
    }
    Ok(s)
}

pub async fn set_enabled(pool: &DbPool, id: &str, enabled: bool) -> Result<Schedule> {
    let s = get(pool, id).await?;
    // Resuming doesn't replay the paused period.
    let last = if enabled && !s.enabled {
        Utc::now().timestamp_millis()
    } else {
        s.last_fire_ms
    };
    sqlx::query("UPDATE schedules SET enabled = ?, last_fire_ms = ?, updated_ms = ? WHERE id = ?")
        .bind(enabled as i64)
        .bind(last)
        .bind(Utc::now().timestamp_millis())
        .bind(&s.id)
        .execute(pool)
        .await?;
    get(pool, &s.id).await
}

pub async fn history(pool: &DbPool, id: Option<&str>, limit: i64) -> Result<Vec<RunRow>> {
    let sid = match id {
        Some(i) => Some(get(pool, i).await?.id),
        None => None,
    };
    let rows: Vec<(String, i64, i64, Option<String>, String, String)> = sqlx::query_as(
        "SELECT schedule_id, ts_ms, due_ms, request_id, outcome, note FROM schedule_runs \
         WHERE (? IS NULL OR schedule_id = ?) ORDER BY id DESC LIMIT ?",
    )
    .bind(&sid)
    .bind(&sid)
    .bind(limit.clamp(1, 1000))
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| RunRow {
            schedule_id: r.0,
            ts_ms: r.1,
            due_ms: r.2,
            request_id: r.3,
            outcome: r.4,
            note: r.5,
        })
        .collect())
}

async fn record(
    pool: &DbPool,
    sid: &str,
    due: i64,
    req: Option<&str>,
    outcome: &str,
    note: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO schedule_runs(schedule_id, ts_ms, due_ms, request_id, outcome, note) VALUES (?,?,?,?,?,?)")
        .bind(sid)
        .bind(Utc::now().timestamp_millis())
        .bind(due)
        .bind(req)
        .bind(outcome)
        .bind(note)
        .execute(pool)
        .await?;
    Ok(())
}

/// Machines currently in the given groups (roles from each machine's latest run).
pub async fn machines_in_groups(pool: &DbPool, groups: &[String]) -> Result<Vec<String>> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT computer, role FROM runs r WHERE ts_ms = (SELECT MAX(ts_ms) FROM runs x WHERE x.computer = r.computer) GROUP BY computer",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, r)| {
            r.as_deref()
                .is_some_and(|r| groups.iter().any(|g| g.eq_ignore_ascii_case(r)))
        })
        .map(|(c, _)| c)
        .collect())
}

/// Is a request from this schedule still queued/running?
async fn active_request(pool: &DbPool, sid: &str) -> Result<Option<String>> {
    let r: Option<String> = sqlx::query_scalar(
        "SELECT q.request_id FROM schedule_runs s JOIN requests q ON q.request_id = s.request_id \
         WHERE s.schedule_id = ? AND q.status IN ('queued','running') ORDER BY s.id DESC LIMIT 1",
    )
    .bind(sid)
    .fetch_optional(pool)
    .await?;
    Ok(r)
}

/// Queue one run of `s` now (guard, concurrency and targets applied). Returns (outcome, request, note).
pub async fn fire(
    pool: &DbPool,
    cfg: &ServerConfig,
    s: &Schedule,
    due_ms: i64,
    reason: &str,
    actor: &str,
    apply_guard: bool,
) -> Result<(String, Option<String>, String)> {
    let mut notes = vec![reason.to_string()];
    if s.spec.action == "backup" {
        if !matches!(crate::auth::master_public(pool).await, Ok(Some(_))) {
            // Like the old loop: nothing to encrypt for before `init`; no audit noise.
            let note = format!("{reason}; skipped: not initialized yet (itcmdr-cli init)");
            record(pool, &s.id, due_ms, None, "skipped", &note).await?;
            return Ok(("skipped".into(), None, note));
        }
        let (o, note) = match crate::backup::nightly_now(pool, cfg).await {
            Ok(n) => ("done", format!("{reason}; {n}")),
            Err(e) => ("failed", format!("{reason}; {e:#}")),
        };
        record(pool, &s.id, due_ms, None, o, &note).await?;
        let _ = crate::db::insert_audit(
            pool,
            actor,
            "schedule-fire",
            &serde_json::json!({"schedule": s.name, "id": s.id, "outcome": o, "note": note})
                .to_string(),
        )
        .await;
        return Ok((o.into(), None, note));
    }
    if let Some(r) = active_request(pool, &s.id).await? {
        let note = format!("previous run {r} still active");
        record(pool, &s.id, due_ms, None, "skipped", &note).await?;
        return Ok(("skipped".into(), None, note));
    }
    let mut repair = s.spec.repair;
    if apply_guard && s.spec.guard != "none" {
        let hours_s = crate::settings::get(pool, None, "schedule.office_hours")
            .await
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        if let Ok(h) = parse_hours(&hours_s) {
            if in_hours(&h, &s.spec, Utc::now()) {
                if s.spec.guard == "skip-in-hours" {
                    let note = format!("{reason}; skipped: inside office hours ({hours_s})");
                    record(pool, &s.id, due_ms, None, "skipped", &note).await?;
                    return Ok(("skipped".into(), None, note));
                }
                if repair {
                    repair = false;
                    notes.push(format!("report-only: inside office hours ({hours_s})"));
                }
            }
        }
    }
    let machines = if !s.spec.groups.is_empty() {
        let m = machines_in_groups(pool, &s.spec.groups).await?;
        if m.is_empty() && s.spec.plugins.is_empty() {
            let note = format!(
                "{reason}; skipped: no machines in groups {:?}",
                s.spec.groups
            );
            record(pool, &s.id, due_ms, None, "skipped", &note).await?;
            return Ok(("skipped".into(), None, note));
        }
        m
    } else {
        s.spec.machines.clone()
    };
    let req = RunRequest {
        machines,
        categories: s.spec.categories.clone(),
        plugins: s.spec.plugins.clone(),
        report_only: !repair,
        repair,
        force: false,
        schedule_id: Some(s.id.clone()),
    };
    let id = crate::jobs::queue_run(pool, cfg, &req, actor)
        .await
        .context("queueing the run")?;
    let note = notes.join("; ");
    record(pool, &s.id, due_ms, Some(&id), "queued", &note).await?;
    let _ = crate::db::insert_audit(
        pool,
        actor,
        "schedule-fire",
        &serde_json::json!({"schedule": s.name, "id": s.id, "request_id": id, "note": note})
            .to_string(),
    )
    .await;
    Ok(("queued".into(), Some(id), note))
}

/// One scheduler pass at `now`: fire due schedules, handle missed ones.
pub async fn tick(pool: &DbPool, cfg: &ServerConfig, now: DateTime<Utc>) -> Result<usize> {
    let grace = crate::settings::get_i64(pool, None, "schedule.grace_secs").await * 1000;
    let mut fired = 0;
    for s in list(pool).await?.into_iter().filter(|s| s.enabled) {
        let last = DateTime::from_timestamp_millis(s.last_fire_ms).unwrap_or(now);
        let due: Vec<DateTime<Utc>> = match next_times(&s.spec, last, 10_000) {
            Ok(v) => v.into_iter().take_while(|t| *t <= now).collect(),
            Err(e) => {
                tracing::warn!(schedule=%s.name, error=%e, "bad schedule");
                continue;
            }
        };
        let Some(latest) = due.last().copied() else {
            continue;
        };
        let late = (now - latest).num_milliseconds() > grace;
        let missed = due.len() - 1 + late as usize;
        let (run, reason) = if !late {
            (
                true,
                if missed > 0 {
                    format!("scheduled; {missed} earlier run(s) missed")
                } else {
                    "scheduled".to_string()
                },
            )
        } else if s.spec.missed == "catch-up" {
            (true, format!("catch-up after {missed} missed run(s)"))
        } else {
            (false, String::new())
        };
        if missed > 0 {
            let first = due.first().copied().unwrap_or(latest);
            record(
                pool,
                &s.id,
                first.timestamp_millis(),
                None,
                "missed",
                &format!(
                    "{missed} run(s) missed between {} and {} (server down?), policy {}",
                    first.to_rfc3339(),
                    latest.to_rfc3339(),
                    s.spec.missed
                ),
            )
            .await?;
        }
        if run {
            match fire(
                pool,
                cfg,
                &s,
                latest.timestamp_millis(),
                &reason,
                "scheduler",
                true,
            )
            .await
            {
                Ok((o, ..)) => fired += (o == "queued") as usize,
                Err(e) => {
                    record(
                        pool,
                        &s.id,
                        latest.timestamp_millis(),
                        None,
                        "failed",
                        &format!("{e:#}"),
                    )
                    .await?;
                }
            }
        }
        sqlx::query("UPDATE schedules SET last_fire_ms = ? WHERE id = ?")
            .bind(now.timestamp_millis().max(latest.timestamp_millis()))
            .bind(&s.id)
            .execute(pool)
            .await?;
    }
    Ok(fired)
}

/// For tests: pretend the schedule last fired at `ms`.
pub async fn set_last_fire(pool: &DbPool, id: &str, ms: i64) -> Result<()> {
    sqlx::query("UPDATE schedules SET last_fire_ms = ? WHERE id = ? OR name = ?")
        .bind(ms)
        .bind(id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[allow(dead_code)]
fn _assert_tz<T: TimeZone>() {}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(when: &str, tz: &str) -> Spec {
        Spec {
            all_machines: true,
            categories: vec!["network".into()],
            when: when.into(),
            timezone: tz.into(),
            ..Default::default()
        }
    }

    #[test]
    fn presets_and_cron() {
        assert_eq!(to_cron("daily 01:30").unwrap(), "0 30 1 * * *");
        assert_eq!(to_cron("every 6h").unwrap(), "0 0 */6 * * *");
        assert_eq!(to_cron("weekly mon 02:00").unwrap(), "0 0 2 * * Mon");
        assert_eq!(to_cron("weekdays 07:15").unwrap(), "0 15 7 * * Mon-Fri");
        assert_eq!(to_cron("cron 30 1 * * *").unwrap(), "0 30 1 * * *");
        for bad in [
            "daily 25:00",
            "every 0h",
            "every 30h",
            "weekly funday 01:00",
            "cron 1 2 3",
            "sometimes",
            "cron 99 * * * *",
        ] {
            assert!(to_cron(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn dst_keeps_local_wall_time() {
        let s = spec("daily 09:00", "America/New_York");
        // 2026 DST: starts Mar 8, ends Nov 1 (US).
        let before = Utc.with_ymd_and_hms(2026, 3, 6, 12, 0, 0).unwrap();
        let t = next_times(&s, before, 3).unwrap();
        assert_eq!(
            t[0],
            Utc.with_ymd_and_hms(2026, 3, 6, 14, 0, 0).unwrap(),
            "EST is 5h behind UTC"
        );
        assert_eq!(t[1], Utc.with_ymd_and_hms(2026, 3, 7, 14, 0, 0).unwrap());
        assert_eq!(
            t[2],
            Utc.with_ymd_and_hms(2026, 3, 8, 13, 0, 0).unwrap(),
            "EDT is 4h behind UTC"
        );
        let fall =
            next_times(&s, Utc.with_ymd_and_hms(2026, 10, 31, 12, 0, 0).unwrap(), 2).unwrap();
        assert_eq!(
            fall[0],
            Utc.with_ymd_and_hms(2026, 10, 31, 13, 0, 0).unwrap()
        );
        assert_eq!(
            fall[1],
            Utc.with_ymd_and_hms(2026, 11, 1, 14, 0, 0).unwrap()
        );
        // 02:30 doesn't exist on Mar 8 in New York: that day is skipped, not doubled.
        let gap = spec("daily 02:30", "America/New_York");
        let g = next_times(&gap, Utc.with_ymd_and_hms(2026, 3, 7, 12, 0, 0).unwrap(), 2).unwrap();
        assert_eq!(g[0], Utc.with_ymd_and_hms(2026, 3, 9, 6, 30, 0).unwrap());
        // 01:30 happens twice on Nov 1: it fires once.
        let rep = spec("daily 01:30", "America/New_York");
        let r = next_times(
            &rep,
            Utc.with_ymd_and_hms(2026, 10, 31, 12, 0, 0).unwrap(),
            2,
        )
        .unwrap();
        assert_eq!(
            r[1].date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );
        assert!(next_times(&spec("daily 01:00", "Mars/Olympus"), before, 1).is_err());
    }

    #[test]
    fn office_hours() {
        let h = parse_hours("Mon-Fri 07:00-18:00").unwrap();
        assert_eq!(h.days.len(), 5);
        let s = spec("daily 01:00", "UTC");
        let fri_noon = Utc.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap();
        assert!(in_hours(&h, &s, fri_noon));
        assert!(!in_hours(&h, &s, fri_noon + chrono::Duration::hours(7)));
        assert!(
            !in_hours(&h, &s, fri_noon + chrono::Duration::days(1)),
            "Saturday"
        );
        assert_eq!(
            parse_hours("Sat,Sun 09:00-12:00").unwrap().days,
            vec![Weekday::Sat, Weekday::Sun]
        );
        assert_eq!(parse_hours("Fri-Mon 09:00-12:00").unwrap().days.len(), 4);
        for bad in [
            "",
            "Mon-Fri",
            "Mon-Fri 18:00-07:00",
            "Xyz 07:00-08:00",
            "07:00",
        ] {
            assert!(parse_hours(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn validation() {
        let mut ok = spec("daily 01:30", "");
        validate("nightly", &mut ok).unwrap();
        assert_eq!(
            (ok.guard.as_str(), ok.missed.as_str(), ok.timezone.as_str()),
            ("report-only-in-hours", "skip", "local")
        );
        let mut two = Spec {
            machines: vec!["WS-01".into()],
            ..spec("daily 01:30", "")
        };
        assert!(validate("x", &mut two).is_err(), "two targets");
        let mut none = Spec {
            all_machines: false,
            ..spec("daily 01:30", "")
        };
        assert!(validate("x", &mut none).is_err(), "no target");
        let mut g = Spec {
            guard: "maybe".into(),
            ..spec("daily 01:30", "")
        };
        assert!(validate("x", &mut g).is_err());
        let mut p = Spec {
            all_machines: false,
            categories: vec![],
            plugins: vec!["disk-free".into()],
            ..spec("hourly", "")
        };
        validate("plugin only", &mut p).unwrap();
        assert!(validate("", &mut spec("hourly", "")).is_err());
    }
}
