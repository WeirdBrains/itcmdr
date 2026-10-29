use crate::config::ServerConfig;
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
use chrono::{Utc, Duration};

pub type DbPool = Pool<Sqlite>;

pub async fn init_db(cfg: &ServerConfig) -> anyhow::Result<DbPool> {
    if let Some(parent) = cfg.database_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn_opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", cfg.database_path.display()))?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(conn_opts)
        .await?;
    // WAL mode for concurrent readers
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
        report_only INTEGER DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_runs_ts ON runs(ts_ms);
    CREATE INDEX IF NOT EXISTS idx_runs_computer_ts ON runs(computer, ts_ms);

    CREATE TABLE IF NOT EXISTS rollup_daily (
        day_utc TEXT NOT NULL,          -- YYYY-MM-DD
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
        status TEXT NOT NULL
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
    "#;
    for stmt in schema.split(';') {
        let s = stmt.trim();
        if s.is_empty() { continue; }
        sqlx::query(s).execute(pool).await?;
    }
    Ok(())
}

pub async fn apply_retention(pool: &DbPool, retention_days: i64) -> anyhow::Result<()> {
    let cutoff = Utc::now() - Duration::days(retention_days);
    let cutoff_ms = cutoff.timestamp_millis();
    sqlx::query("DELETE FROM runs WHERE ts_ms < ?").bind(cutoff_ms).execute(pool).await?;
    // rollup_daily is keyed by day; compute cutoff date
    let cutoff_day = cutoff.date_naive().to_string();
    sqlx::query("DELETE FROM rollup_daily WHERE day_utc < ?").bind(&cutoff_day).execute(pool).await?;
    Ok(())
}

pub async fn record_run_start(pool: &DbPool, run_id: &str, computer: &str, role: Option<&str>, report_only: bool, ts_ms: i64) -> anyhow::Result<()> {
    sqlx::query("INSERT OR IGNORE INTO runs (run_id, computer, ts_ms, role, report_only) VALUES (?,?,?,?,?)")
        .bind(run_id).bind(computer).bind(ts_ms).bind(role.unwrap_or("")).bind(if report_only {1} else {0})
        .execute(pool).await?;
    Ok(())
}

pub async fn record_check(pool: &DbPool, rec: &crate::ingest::AgentRecord, ts_ms: i64) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO checks (run_id, computer, category, check_id, severity, status, attempted_repair, action, result, ts_ms, excepted, exception_reason) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&rec.run_id)
        .bind(&rec.computer)
        .bind(&rec.category)
        .bind(&rec.check_id)
        .bind(&rec.severity)
        .bind(&rec.status)
        .bind(if rec.action.as_deref() == Some("repair") { 1 } else { 0 })
        .bind(rec.action.as_deref().unwrap_or_default())
        .bind("") // result not included in AgentRecord stub; extend if needed
        .bind(ts_ms)
        .bind(rec.excepted.unwrap_or(false) as i32)
        .bind(rec.exception_reason.as_deref().unwrap_or_default())
        .execute(pool).await?;
    update_incidents_for_check(pool, rec, ts_ms).await?;
    update_rollup_daily_for_check(pool, rec, ts_ms).await?;
    Ok(())
}

async fn update_incidents_for_check(pool: &DbPool, rec: &crate::ingest::AgentRecord, ts_ms: i64) -> anyhow::Result<()> {
    if rec.excepted.unwrap_or(false) { return Ok(()); }
    if rec.status.eq_ignore_ascii_case("failed") {
        // open if not open
        let open = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM incidents WHERE computer=? AND check_id=? AND is_open=1")
            .bind(&rec.computer).bind(&rec.check_id).fetch_one(pool).await?;
        if open == 0 {
            sqlx::query("INSERT INTO incidents (computer, check_id, opened_ts_ms, is_open) VALUES (?,?,?,1)")
                .bind(&rec.computer).bind(&rec.check_id).bind(ts_ms).execute(pool).await?;
        }
    } else if rec.status.eq_ignore_ascii_case("ok") {
        // close any open
        sqlx::query("UPDATE incidents SET closed_ts_ms=?, is_open=0 WHERE computer=? AND check_id=? AND is_open=1")
            .bind(ts_ms).bind(&rec.computer).bind(&rec.check_id).execute(pool).await?;
    }
    Ok(())
}

async fn update_rollup_daily_for_check(pool: &DbPool, rec: &crate::ingest::AgentRecord, ts_ms: i64) -> anyhow::Result<()> {
    let day = chrono::NaiveDateTime::from_timestamp_millis(ts_ms).unwrap_or_else(|| chrono::NaiveDateTime::from_timestamp_opt(ts_ms/1000,0).unwrap()).date().to_string();
    // Upsert daily row
    sqlx::query("INSERT INTO rollup_daily (day_utc, computer, runs, incidents, failed_repairs, run_duration_ms_sum) VALUES (?,?,?,?,?,0) ON CONFLICT(day_utc,computer) DO UPDATE SET runs=rollup_daily.runs+1, incidents=rollup_daily.incidents+excluded.incidents")
        .bind(&day)
        .bind(&rec.computer)
        .bind(1)
        .bind(if rec.status.eq_ignore_ascii_case("failed") && !rec.excepted.unwrap_or(false) {1} else {0})
        .bind(0)
        .execute(pool).await?;
    Ok(())
}

pub async fn insert_request(pool: &DbPool, request_id: &str, options_json: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO requests (request_id, created_ts_ms, options_json, status) VALUES (?,?,?,?)")
        .bind(request_id).bind(Utc::now().timestamp_millis()).bind(options_json).bind("queued")
        .execute(pool).await?;
    Ok(())
}

pub async fn get_request_status(pool: &DbPool, request_id: &str) -> anyhow::Result<String> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM requests WHERE request_id=?").bind(request_id).fetch_optional(pool).await?;
    Ok(status.unwrap_or_else(|| "unknown".into()))
}

pub async fn list_machines_overview(pool: &DbPool) -> anyhow::Result<Vec<(String, String, i64, i64)>> {
    // name, status, open_incidents, last_run
    let rows = sqlx::query(
        r#"
        SELECT r.computer as name,
               COALESCE(CASE WHEN (SELECT COUNT(*) FROM incidents i WHERE i.computer=r.computer AND i.is_open=1)>0 THEN 'warning' ELSE 'ok' END, 'ok') as status,
               (SELECT COUNT(*) FROM incidents i WHERE i.computer=r.computer AND i.is_open=1) as open_incidents,
               MAX(r.ts_ms) as last_run
        FROM runs r
        GROUP BY r.computer
        ORDER BY r.computer
        "#
    ).fetch_all(pool).await?;
    use sqlx::Row;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let name: String = r.try_get("name")?;
        let status: String = r.try_get::<Option<String>, _>("status")?.unwrap_or_else(|| "ok".into());
        let open_incidents: i64 = r.try_get::<Option<i64>, _>("open_incidents")?.unwrap_or(0);
        let last_run: i64 = r.try_get::<Option<i64>, _>("last_run")?.unwrap_or(0);
        out.push((name, status, open_incidents, last_run));
    }
    Ok(out)
}

pub async fn save_exception(pool: &DbPool, id: &str, rule_json: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO exceptions (id, rule_json, active) VALUES (?,?,1)")
        .bind(id).bind(rule_json).execute(pool).await?;
    Ok(())
}

pub async fn expire_exception(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE exceptions SET active=0 WHERE id=?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn list_exceptions(pool: &DbPool) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query("SELECT rule_json FROM exceptions WHERE active=1").fetch_all(pool).await?;
    use sqlx::Row;
    Ok(rows.into_iter().filter_map(|r| r.try_get::<String, _>("rule_json").ok()).collect())
}

