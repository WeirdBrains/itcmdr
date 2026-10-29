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
        ts_ms INTEGER NOT NULL
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

