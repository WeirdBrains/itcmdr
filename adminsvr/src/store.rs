//! Postgres store. Schema is created/updated at start (`meta.schema`).
use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};

pub type Pool = PgPool;
pub const SCHEMA_VERSION: i64 = 1;

pub async fn connect(url: &str) -> Result<Pool> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(url)
        .await
        .context("connect to Postgres (db_url)")
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('owner','admin','sre')),
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','disabled')),
  master_pub TEXT,
  master_fp TEXT UNIQUE,
  key_backup_opt_out BOOLEAN NOT NULL DEFAULT false,
  created_by TEXT NOT NULL,
  created_ms BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS enroll_codes (
  secret_hash TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  expires_ms BIGINT NOT NULL,
  used_ms BIGINT
);
CREATE TABLE IF NOT EXISTS devices (
  fingerprint TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  public_key TEXT NOT NULL,
  certificate TEXT NOT NULL,
  platform TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL CHECK (status IN ('pending','active','revoked')),
  created_ms BIGINT NOT NULL,
  last_used_ms BIGINT NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS key_backups (
  user_id TEXT PRIMARY KEY REFERENCES users(id),
  blob TEXT NOT NULL,
  updated_ms BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS sites (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  site_pub TEXT,
  cert_serial TEXT,
  version TEXT NOT NULL DEFAULT '',
  last_seen_ms BIGINT NOT NULL DEFAULT 0,
  share_names BOOLEAN NOT NULL DEFAULT false,
  last_status BYTEA,
  last_seq BIGINT NOT NULL DEFAULT 0,
  created_ms BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS site_tokens (
  secret_hash TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites(id),
  expires_ms BIGINT NOT NULL,
  used_ms BIGINT
);
CREATE TABLE IF NOT EXISTS site_scopes (
  user_id TEXT NOT NULL REFERENCES users(id),
  site_id TEXT NOT NULL REFERENCES sites(id),
  PRIMARY KEY (user_id, site_id)
);
CREATE TABLE IF NOT EXISTS alerts (
  site_id TEXT NOT NULL,
  alert_id TEXT NOT NULL,
  origin TEXT NOT NULL,
  state TEXT NOT NULL,
  severity TEXT NOT NULL,
  last_seen_ms BIGINT NOT NULL,
  body BYTEA NOT NULL,
  PRIMARY KEY (site_id, alert_id)
);
CREATE TABLE IF NOT EXISTS commands (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  requested_by TEXT NOT NULL,
  state TEXT NOT NULL,
  result_code TEXT NOT NULL DEFAULT '',
  created_ms BIGINT NOT NULL,
  body BYTEA NOT NULL
);
CREATE TABLE IF NOT EXISTS audit_chain (
  seq BIGINT PRIMARY KEY,
  ts_ms BIGINT NOT NULL,
  actor TEXT NOT NULL,
  actor_key TEXT NOT NULL,
  action TEXT NOT NULL,
  target TEXT NOT NULL,
  payload_sha256 TEXT NOT NULL,
  request_sig TEXT NOT NULL,
  prev_hash TEXT NOT NULL,
  hash TEXT NOT NULL
);
"#;

pub async fn migrate(pool: &Pool) -> Result<()> {
    sqlx::raw_sql(SCHEMA)
        .execute(pool)
        .await
        .context("schema")?;
    sqlx::query("INSERT INTO meta (k, v) VALUES ('schema', $1) ON CONFLICT (k) DO UPDATE SET v = EXCLUDED.v")
        .bind(SCHEMA_VERSION.to_string())
        .execute(pool)
        .await?;
    Ok(())
}
