use crate::config::ServerConfig;
use chrono::{Duration, Utc};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::str::FromStr;

pub type DbPool = Pool<Sqlite>;

pub async fn init_db(cfg: &ServerConfig) -> anyhow::Result<DbPool> {
    if let Some(parent) = cfg.database_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn_opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", cfg.database_path.display()))?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(conn_opts).await?;
    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await.ok();
    create_schema(&pool).await?;
    Ok(pool)
}

async fn create_schema(pool: &DbPool) -> anyhow::Result<()> {
    let schema = r#"
    CREATE TABLE IF NOT EXISTS runs (
        run_id TEXT PRIMARY KEY,
        computer TEXT NOT NULL,
        ts_ms INTEGER NOT NULL,
        role TEXT,
        report_only INTEGER DEFAULT 0,
        finished_ts_ms INTEGER,
        exit_code INTEGER
    );
    CREATE INDEX IF NOT EXISTS idx_runs_ts ON runs(ts_ms);
    CREATE INDEX IF NOT EXISTS idx_runs_computer_ts ON runs(computer, ts_ms);

    CREATE TABLE IF NOT EXISTS rollup_daily (
        day_utc TEXT NOT NULL,
        computer TEXT NOT NULL,
        runs INTEGER NOT NULL DEFAULT 0,
        incidents INTEGER NOT NULL DEFAULT 0,
        failed_repairs INTEGER NOT NULL DEFAULT 0,
        run_duration_ms_sum INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (day_utc, computer)
    );
    CREATE INDEX IF NOT EXISTS idx_rollup_daily_day ON rollup_daily(day_utc);

    CREATE TABLE IF NOT EXISTS checks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        run_id TEXT NOT NULL,
        computer TEXT NOT NULL,
        category TEXT NOT NULL,
        check_id TEXT NOT NULL,
        severity TEXT NOT NULL,
        status TEXT NOT NULL,
        attempted_repair INTEGER NOT NULL DEFAULT 0,
        action TEXT,
        result TEXT,
        ts_ms INTEGER NOT NULL,
        excepted INTEGER NOT NULL DEFAULT 0,
        exception_reason TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_checks_comp_check_ts ON checks(computer, check_id, ts_ms);

    CREATE TABLE IF NOT EXISTS incidents (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        computer TEXT NOT NULL,
        check_id TEXT NOT NULL,
        opened_ts_ms INTEGER NOT NULL,
        closed_ts_ms INTEGER,
        is_open INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX IF NOT EXISTS idx_incidents_open ON incidents(is_open);

    CREATE TABLE IF NOT EXISTS requests (
        request_id TEXT PRIMARY KEY,
        created_ts_ms INTEGER NOT NULL,
        options_json TEXT NOT NULL,
        status TEXT NOT NULL,
        message TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_requests_status ON requests(status);

    CREATE TABLE IF NOT EXISTS exceptions (
        id TEXT PRIMARY KEY,
        rule_json TEXT NOT NULL,
        active INTEGER NOT NULL DEFAULT 1
    );

    CREATE TABLE IF NOT EXISTS audit_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts_ms INTEGER NOT NULL,
        actor TEXT,
        action TEXT NOT NULL,
        detail TEXT
    );

    CREATE TABLE IF NOT EXISTS ingested_files (
        path TEXT PRIMARY KEY,
        size INTEGER NOT NULL,
        mtime_ms INTEGER NOT NULL
    )
    "#;
    for stmt in schema.split(';') {
        let s = stmt.trim();
        if !s.is_empty() {
            sqlx::query(s).execute(pool).await?;
        }
    }
    // Migrations for databases created by v0.1.x (an error means the column exists).
    for m in [
        "ALTER TABLE runs ADD COLUMN finished_ts_ms INTEGER",
        "ALTER TABLE runs ADD COLUMN exit_code INTEGER",
        "ALTER TABLE requests ADD COLUMN message TEXT",
    ] {
        let _ = sqlx::query(m).execute(pool).await;
    }
    // Drop legacy duplicates, then enforce one row per (run, check).
    sqlx::query("DELETE FROM checks WHERE id NOT IN (SELECT MIN(id) FROM checks GROUP BY run_id, check_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS ux_checks_run_check ON checks(run_id, check_id)")
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete data older than `retention_days` (runs, checks, closed incidents, rollups,
/// finished requests). The audit log and exceptions are kept.
pub async fn apply_retention(pool: &DbPool, retention_days: i64) -> anyhow::Result<()> {
    let cutoff = Utc::now() - Duration::days(retention_days);
    let cutoff_ms = cutoff.timestamp_millis();
    sqlx::query("DELETE FROM runs WHERE ts_ms < ?").bind(cutoff_ms).execute(pool).await?;
    sqlx::query("DELETE FROM checks WHERE ts_ms < ?").bind(cutoff_ms).execute(pool).await?;
    sqlx::query("DELETE FROM incidents WHERE is_open=0 AND closed_ts_ms < ?").bind(cutoff_ms).execute(pool).await?;
    sqlx::query("DELETE FROM requests WHERE created_ts_ms < ? AND status IN ('done','failed')")
        .bind(cutoff_ms)
        .execute(pool)
        .await?;
    let cutoff_day = cutoff.date_naive().to_string();
    sqlx::query("DELETE FROM rollup_daily WHERE day_utc < ?").bind(&cutoff_day).execute(pool).await?;
    Ok(())
}

fn day_of(ts_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts_ms).unwrap_or_default().date_naive().to_string()
}

async fn bump_rollup(pool: &DbPool, ts_ms: i64, computer: &str, runs: i64, incidents: i64, failed_repairs: i64) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO rollup_daily (day_utc, computer, runs, incidents, failed_repairs, run_duration_ms_sum) VALUES (?,?,?,?,?,0) \
         ON CONFLICT(day_utc,computer) DO UPDATE SET runs=runs+excluded.runs, incidents=incidents+excluded.incidents, failed_repairs=failed_repairs+excluded.failed_repairs",
    )
    .bind(day_of(ts_ms))
    .bind(computer)
    .bind(runs)
    .bind(incidents)
    .bind(failed_repairs)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns true when the run was new.
pub async fn record_run_start(pool: &DbPool, run_id: &str, computer: &str, role: Option<&str>, report_only: bool, ts_ms: i64) -> anyhow::Result<bool> {
    let res = sqlx::query("INSERT OR IGNORE INTO runs (run_id, computer, ts_ms, role, report_only) VALUES (?,?,?,?,?)")
        .bind(run_id)
        .bind(computer)
        .bind(ts_ms)
        .bind(role.unwrap_or(""))
        .bind(report_only as i32)
        .execute(pool)
        .await?;
    let new = res.rows_affected() == 1;
    if new {
        bump_rollup(pool, ts_ms, computer, 1, 0, 0).await?;
        crate::events::notify("run", computer, run_id);
    }
    Ok(new)
}

pub async fn record_run_complete(pool: &DbPool, run_id: &str, finished_ts_ms: i64, exit_code: Option<i32>) -> anyhow::Result<()> {
    sqlx::query("UPDATE runs SET finished_ts_ms=?, exit_code=? WHERE run_id=? AND finished_ts_ms IS NULL")
        .bind(finished_ts_ms)
        .bind(exit_code)
        .bind(run_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a check once (unique on run_id+check_id). Returns true when new.
pub async fn record_check(pool: &DbPool, rec: &crate::ingest::AgentRecord, ts_ms: i64) -> anyhow::Result<bool> {
    let res = sqlx::query("INSERT OR IGNORE INTO checks (run_id, computer, category, check_id, severity, status, attempted_repair, action, result, ts_ms, excepted, exception_reason) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&rec.run_id)
        .bind(&rec.computer)
        .bind(&rec.category)
        .bind(&rec.check_id)
        .bind(&rec.severity)
        .bind(&rec.status)
        .bind(rec.attempted_repair() as i32)
        .bind(rec.action.as_deref().unwrap_or_default())
        .bind(rec.result.as_deref().unwrap_or_default())
        .bind(ts_ms)
        .bind(rec.excepted.unwrap_or(false) as i32)
        .bind(rec.exception_reason.as_deref().unwrap_or_default())
        .execute(pool)
        .await?;
    if res.rows_affected() != 1 {
        return Ok(false);
    }
    let opened = update_incidents_for_check(pool, rec, ts_ms).await?;
    let failed_repair = rec.attempted_repair() && rec.result.as_deref().is_some_and(|r| r.eq_ignore_ascii_case("failed"));
    bump_rollup(pool, ts_ms, &rec.computer, 0, opened as i64, failed_repair as i64).await?;
    crate::events::notify("check", &rec.computer, &rec.run_id);
    Ok(true)
}

/// Incident state per (computer, check). A failure opens an incident unless one is
/// open; a later "ok" closes it. A check found broken and fixed in the same run is an
/// incident with zero time-to-repair. Records older than the newest already-seen record
/// for the pair (late arrivals from archives) don't change state.
/// Returns true when a new incident was opened.
async fn update_incidents_for_check(pool: &DbPool, rec: &crate::ingest::AgentRecord, ts_ms: i64) -> anyhow::Result<bool> {
    if rec.excepted.unwrap_or(false) {
        return Ok(false);
    }
    let newer: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM checks WHERE computer=? AND check_id=? AND ts_ms > ?")
        .bind(&rec.computer)
        .bind(&rec.check_id)
        .bind(ts_ms)
        .fetch_one(pool)
        .await?;
    if newer > 0 {
        return Ok(false);
    }
    let failed = rec.status.eq_ignore_ascii_case("failed");
    let repaired = !failed && rec.attempted_repair() && rec.result.as_deref().is_some_and(|r| r.eq_ignore_ascii_case("success"));
    let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM incidents WHERE computer=? AND check_id=? AND is_open=1")
        .bind(&rec.computer)
        .bind(&rec.check_id)
        .fetch_one(pool)
        .await?;
    let mut opened = false;
    if failed {
        if open == 0 {
            sqlx::query("INSERT INTO incidents (computer, check_id, opened_ts_ms, is_open) VALUES (?,?,?,1)")
                .bind(&rec.computer)
                .bind(&rec.check_id)
                .bind(ts_ms)
                .execute(pool)
                .await?;
            opened = true;
            crate::events::notify("incident-open", &rec.computer, &rec.check_id);
        }
    } else if rec.status.eq_ignore_ascii_case("ok") {
        if open > 0 {
            sqlx::query("UPDATE incidents SET closed_ts_ms=?, is_open=0 WHERE computer=? AND check_id=? AND is_open=1")
                .bind(ts_ms)
                .bind(&rec.computer)
                .bind(&rec.check_id)
                .execute(pool)
                .await?;
            crate::events::notify("incident-close", &rec.computer, &rec.check_id);
        } else if repaired {
            sqlx::query("INSERT INTO incidents (computer, check_id, opened_ts_ms, closed_ts_ms, is_open) VALUES (?,?,?,?,0)")
                .bind(&rec.computer)
                .bind(&rec.check_id)
                .bind(ts_ms)
                .bind(ts_ms)
                .execute(pool)
                .await?;
            opened = true;
        }
    }
    Ok(opened)
}

/// (size, mtime_ms) recorded the last time `path` was ingested.
pub async fn ingested_file(pool: &DbPool, path: &str) -> anyhow::Result<Option<(i64, i64)>> {
    let row = sqlx::query("SELECT size, mtime_ms FROM ingested_files WHERE path=?").bind(path).fetch_optional(pool).await?;
    Ok(row.map(|r| (r.get::<i64, _>("size"), r.get::<i64, _>("mtime_ms"))))
}

pub async fn mark_ingested(pool: &DbPool, path: &str, size: i64, mtime_ms: i64) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO ingested_files (path, size, mtime_ms) VALUES (?,?,?) ON CONFLICT(path) DO UPDATE SET size=excluded.size, mtime_ms=excluded.mtime_ms")
        .bind(path)
        .bind(size)
        .bind(mtime_ms)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_request(pool: &DbPool, request_id: &str, options_json: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO requests (request_id, created_ts_ms, options_json, status, message) VALUES (?,?,?,'queued','queued')")
        .bind(request_id)
        .bind(Utc::now().timestamp_millis())
        .bind(options_json)
        .execute(pool)
        .await?;
    crate::events::notify("request", "queued", request_id);
    Ok(())
}

/// (status, message); status is "unknown" when the id doesn't exist.
pub async fn get_request_status(pool: &DbPool, request_id: &str) -> anyhow::Result<(String, String)> {
    let row = sqlx::query("SELECT status, message FROM requests WHERE request_id=?").bind(request_id).fetch_optional(pool).await?;
    Ok(match row {
        Some(r) => {
            let st: String = r.get("status");
            let msg: Option<String> = r.get("message");
            (st.clone(), msg.unwrap_or(st))
        }
        None => ("unknown".into(), "unknown request".into()),
    })
}

/// Requests still in flight: (id, options_json).
pub async fn pending_requests(pool: &DbPool) -> anyhow::Result<Vec<(String, String)>> {
    let rows = sqlx::query("SELECT request_id, options_json FROM requests WHERE status IN ('queued','running')").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| (r.get("request_id"), r.get("options_json"))).collect())
}

pub async fn set_request_status(pool: &DbPool, request_id: &str, status: &str, message: &str) -> anyhow::Result<bool> {
    let res = sqlx::query("UPDATE requests SET status=?, message=? WHERE request_id=? AND (status<>? OR COALESCE(message,'')<>?)")
        .bind(status)
        .bind(message)
        .bind(request_id)
        .bind(status)
        .bind(message)
        .execute(pool)
        .await?;
    let changed = res.rows_affected() > 0;
    if changed {
        crate::events::notify("request", status, request_id);
    }
    Ok(changed)
}

pub async fn list_machines_overview(pool: &DbPool) -> anyhow::Result<Vec<(String, String, i64, i64)>> {
    let rows = sqlx::query(
        r#"
        SELECT r.computer as name,
               (SELECT COUNT(*) FROM incidents i WHERE i.computer=r.computer AND i.is_open=1) as open_incidents,
               MAX(r.ts_ms) as last_run
        FROM runs r
        GROUP BY r.computer
        ORDER BY r.computer
        "#,
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let name: String = r.try_get("name")?;
        let open_incidents: i64 = r.try_get::<Option<i64>, _>("open_incidents")?.unwrap_or(0);
        let last_run: i64 = r.try_get::<Option<i64>, _>("last_run")?.unwrap_or(0);
        let status = if open_incidents > 0 { "warning" } else { "ok" };
        out.push((name, status.to_string(), open_incidents, last_run));
    }
    Ok(out)
}

pub async fn save_exception(pool: &DbPool, id: &str, rule_json: &str, active: bool) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO exceptions (id, rule_json, active) VALUES (?,?,?)")
        .bind(id)
        .bind(rule_json)
        .bind(active as i32)
        .execute(pool)
        .await?;
    crate::events::notify("exception-save", "", id);
    Ok(())
}

/// Returns true when a rule with that id existed.
pub async fn expire_exception(pool: &DbPool, id: &str) -> anyhow::Result<bool> {
    let res = sqlx::query("UPDATE exceptions SET active=0 WHERE id=?").bind(id).execute(pool).await?;
    crate::events::notify("exception-expire", "", id);
    Ok(res.rows_affected() > 0)
}

/// (rule_json, active)
pub async fn list_exceptions(pool: &DbPool, include_inactive: bool) -> anyhow::Result<Vec<(String, bool)>> {
    let q = if include_inactive {
        "SELECT rule_json, active FROM exceptions ORDER BY id"
    } else {
        "SELECT rule_json, active FROM exceptions WHERE active=1 ORDER BY id"
    };
    let rows = sqlx::query(q).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| (r.get::<String, _>("rule_json"), r.get::<i64, _>("active") != 0)).collect())
}

pub async fn insert_audit(pool: &DbPool, actor: &str, action: &str, detail: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO audit_log (ts_ms, actor, action, detail) VALUES (?,?,?,?)")
        .bind(Utc::now().timestamp_millis())
        .bind(actor)
        .bind(action)
        .bind(detail)
        .execute(pool)
        .await?;
    crate::events::notify("audit", action, "");
    Ok(())
}

/// Newest first: (ts_ms, actor, action, detail)
pub async fn list_audit(pool: &DbPool, limit: i64) -> anyhow::Result<Vec<(i64, String, String, String)>> {
    let rows = sqlx::query("SELECT ts_ms, COALESCE(actor,'') AS actor, action, COALESCE(detail,'') AS detail FROM audit_log ORDER BY id DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| (r.get("ts_ms"), r.get("actor"), r.get("action"), r.get("detail"))).collect())
}
