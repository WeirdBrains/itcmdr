use crate::config::ServerConfig;
use chrono::{Duration, Utc};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::str::FromStr;

pub type DbPool = Pool<Sqlite>;

/// Bumped whenever the schema changes. Stored as `PRAGMA user_version`; backups record it and
/// restore refuses archives from a newer schema (older ones are migrated by `create_schema`).
pub const SCHEMA_VERSION: i64 = 2;

/// Open (creating if needed) the SQLCipher database. The data dir layout and DB key are created
/// on first use. A pre-0.6 plaintext database at the target path, or `itcmdr.sqlite` next to it,
/// is encrypted first (see `migrate_plaintext`).
static TRACKED: std::sync::OnceLock<std::sync::Mutex<Vec<DbPool>>> = std::sync::OnceLock::new();

/// Short-lived processes (itcmdr-cli): remember every pool so `close_tracked` can close them before
/// exit. Without it, SQLCipher connection threads can still be closing while the process tears down
/// OpenSSL, which crashes intermittently ("double free").
pub fn track_pools() {
    let _ = TRACKED.set(std::sync::Mutex::new(Vec::new()));
}

/// Close every tracked pool (waits for the connection threads).
pub async fn close_tracked() {
    let pools: Vec<DbPool> = match TRACKED.get() {
        Some(t) => std::mem::take(&mut *t.lock().unwrap_or_else(|e| e.into_inner())),
        None => return,
    };
    for p in pools {
        p.close().await;
    }
}

pub async fn init_db(cfg: &ServerConfig) -> anyhow::Result<DbPool> {
    let layout = cfg.layout();
    layout.ensure()?;
    let key = crate::storage::load_or_create_db_key(&layout)?;
    let path = cfg.database_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    migrate_plaintext(&path, &key).await?;
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(encrypted_options(&path, &key)?.create_if_missing(true))
        .await?;
    // Fails here (not later) when the key is wrong.
    sqlx::query("SELECT count(*) FROM sqlite_master")
        .execute(&pool)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "cannot open {} with keys/db.key (wrong key or not an itcmdr database): {e}",
                path.display()
            )
        })?;
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .ok();
    create_schema(&pool).await?;
    if let Some(t) = TRACKED.get() {
        t.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(pool.clone());
    }
    Ok(pool)
}

fn encrypted_options(
    path: &std::path::Path,
    key: &[u8; 32],
) -> anyhow::Result<SqliteConnectOptions> {
    // `key` must be the first pragma on every connection; sqlx applies it first.
    Ok(
        SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .pragma("key", crate::storage::key_pragma(key)),
    )
}

/// Encrypt a plaintext database into `target`. Sources: `target` itself (in place) or the legacy
/// `itcmdr.sqlite` beside it. The plaintext original is kept as `<name>.plaintext-backup`
/// (delete it once the server has run fine; it is NOT encrypted). Returns true when migrated.
pub async fn migrate_plaintext(target: &std::path::Path, key: &[u8; 32]) -> anyhow::Result<bool> {
    use crate::storage::{is_plaintext_sqlite, LEGACY_DB_FILE};
    let legacy = target.with_file_name(LEGACY_DB_FILE);
    let source = if is_plaintext_sqlite(target) {
        target.to_path_buf()
    } else if !target.exists() && legacy != target && is_plaintext_sqlite(&legacy) {
        legacy
    } else {
        return Ok(false);
    };
    tracing::warn!(source=%source.display(), target=%target.display(), "encrypting plaintext database");
    let tmp = target.with_extension("encrypting");
    let _ = std::fs::remove_file(&tmp);
    let counts_plain;
    {
        let plain = SqlitePoolOptions::new()
            .max_connections(1)
            // create flag so ATTACH can create the encrypted file (source must exist: checked above)
            .connect_with(
                SqliteConnectOptions::from_str(&format!("sqlite://{}", source.display()))?
                    .create_if_missing(true),
            )
            .await?;
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&plain)
            .await
            .ok();
        counts_plain = table_counts(&plain).await?;
        let tmp_s = tmp.display().to_string().replace('\'', "''");
        sqlx::query(&format!(
            "ATTACH DATABASE '{tmp_s}' AS enc KEY {}",
            crate::storage::key_pragma(key)
        ))
        .execute(&plain)
        .await?;
        sqlx::query("SELECT sqlcipher_export('enc')")
            .execute(&plain)
            .await?;
        sqlx::query("DETACH DATABASE enc").execute(&plain).await?;
        plain.close().await;
    }
    {
        let enc = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(encrypted_options(&tmp, key)?)
            .await?;
        let counts_enc = table_counts(&enc).await?;
        enc.close().await;
        anyhow::ensure!(
            counts_enc == counts_plain,
            "migration check failed: {counts_plain:?} vs {counts_enc:?}"
        );
    }
    let backup = source.with_file_name(format!(
        "{}.plaintext-backup",
        source.file_name().unwrap_or_default().to_string_lossy()
    ));
    std::fs::rename(&source, &backup)?;
    for ext in ["-wal", "-shm"] {
        let side = std::path::PathBuf::from(format!("{}{ext}", source.display()));
        let _ = std::fs::remove_file(side);
    }
    std::fs::rename(&tmp, target)?;
    finish_migration(target, &backup, key, &counts_plain).await?;
    Ok(true)
}

/// Read the encrypted database back at its final path. When the per-table row counts match the
/// plaintext source, the plaintext backup is overwritten and deleted (decision 12). Otherwise the
/// backup is kept and a warning is logged. Returns true when the backup was removed.
pub async fn finish_migration(
    target: &std::path::Path,
    backup: &std::path::Path,
    key: &[u8; 32],
    expected: &[(String, i64)],
) -> anyhow::Result<bool> {
    let readback = async {
        let enc = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(encrypted_options(target, key)?)
            .await?;
        let c = table_counts(&enc).await;
        enc.close().await;
        c
    }
    .await;
    match readback {
        Ok(c) if c.as_slice() == expected => {
            crate::storage::shred_file(backup)?;
            tracing::warn!(backup=%backup.display(), "plaintext database migrated and verified; plaintext backup overwritten and deleted");
            Ok(true)
        }
        Ok(c) => {
            tracing::error!(backup=%backup.display(), ?expected, actual=?c, "encrypted database read-back mismatch; plaintext backup KEPT");
            Ok(false)
        }
        Err(e) => {
            tracing::error!(backup=%backup.display(), error=%e, "encrypted database read-back failed; plaintext backup KEPT");
            Ok(false)
        }
    }
}

/// Per-table row counts (used to verify a migration).
pub async fn table_counts_of(pool: &DbPool) -> anyhow::Result<Vec<(String, i64)>> {
    table_counts(pool).await
}

async fn table_counts(pool: &DbPool) -> anyhow::Result<Vec<(String, i64)>> {
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .fetch_all(pool)
        .await?;
    let mut out = Vec::new();
    for n in names {
        let c: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM \"{}\"", n.replace('"', "")))
                .fetch_one(pool)
                .await?;
        out.push((n, c));
    }
    Ok(out)
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
    );

    -- Auth (v0.6.0): keys are PUBLIC keys only, ids are random UUIDs, fingerprint = sha256(public key) hex (no semicolons in comments: the schema is split on them)
    CREATE TABLE IF NOT EXISTS roles (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE
    );
    CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT,
        phone TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
        role_id TEXT NOT NULL REFERENCES roles(id),
        public_key TEXT NOT NULL,
        fingerprint TEXT NOT NULL UNIQUE,
        certificate TEXT,
        revoked_at TEXT
    );
    CREATE TABLE IF NOT EXISTS clients (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
        role_id TEXT NOT NULL REFERENCES roles(id),
        public_key TEXT NOT NULL,
        fingerprint TEXT NOT NULL UNIQUE,
        certificate TEXT,
        revoked_at TEXT
    );
    -- Service plugins (v0.9.0, schema 2). Files live in service_plugins/{name}-{version}
    CREATE TABLE IF NOT EXISTS plugins (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        version TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 0,
        installed_at TEXT NOT NULL,
        signer TEXT NOT NULL,
        package_sha256 TEXT NOT NULL,
        UNIQUE(name, version)
    );
    CREATE TABLE IF NOT EXISTS server_meta (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
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
    sqlx::query(
        "DELETE FROM checks WHERE id NOT IN (SELECT MIN(id) FROM checks GROUP BY run_id, check_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_checks_run_check ON checks(run_id, check_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))
        .execute(pool)
        .await?;
    for role in ROLES {
        sqlx::query("INSERT INTO roles(id, name) SELECT ?, ? WHERE NOT EXISTS (SELECT 1 FROM roles WHERE name = ?)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(role)
            .bind(role)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Roles, most to least privileged.
pub const ROLES: [&str; 4] = ["owner", "admin", "user", "client"];

pub async fn role_id(pool: &DbPool, name: &str) -> anyhow::Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?)
}

/// Delete data older than `retention_days` (runs, checks, closed incidents, rollups,
/// finished requests). The audit log and exceptions are kept.
pub async fn apply_retention(pool: &DbPool, retention_days: i64) -> anyhow::Result<()> {
    let cutoff = Utc::now() - Duration::days(retention_days);
    let cutoff_ms = cutoff.timestamp_millis();
    sqlx::query("DELETE FROM runs WHERE ts_ms < ?")
        .bind(cutoff_ms)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM checks WHERE ts_ms < ?")
        .bind(cutoff_ms)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM incidents WHERE is_open=0 AND closed_ts_ms < ?")
        .bind(cutoff_ms)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM requests WHERE created_ts_ms < ? AND status IN ('done','failed')")
        .bind(cutoff_ms)
        .execute(pool)
        .await?;
    let cutoff_day = cutoff.date_naive().to_string();
    sqlx::query("DELETE FROM rollup_daily WHERE day_utc < ?")
        .bind(&cutoff_day)
        .execute(pool)
        .await?;
    Ok(())
}

fn day_of(ts_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts_ms)
        .unwrap_or_default()
        .date_naive()
        .to_string()
}

async fn bump_rollup(
    pool: &DbPool,
    ts_ms: i64,
    computer: &str,
    runs: i64,
    incidents: i64,
    failed_repairs: i64,
) -> anyhow::Result<()> {
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
pub async fn record_run_start(
    pool: &DbPool,
    run_id: &str,
    computer: &str,
    role: Option<&str>,
    report_only: bool,
    ts_ms: i64,
) -> anyhow::Result<bool> {
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

pub async fn record_run_complete(
    pool: &DbPool,
    run_id: &str,
    finished_ts_ms: i64,
    exit_code: Option<i32>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE runs SET finished_ts_ms=?, exit_code=? WHERE run_id=? AND finished_ts_ms IS NULL",
    )
    .bind(finished_ts_ms)
    .bind(exit_code)
    .bind(run_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert a check once (unique on run_id+check_id). Returns true when new.
pub async fn record_check(
    pool: &DbPool,
    rec: &crate::ingest::AgentRecord,
    ts_ms: i64,
) -> anyhow::Result<bool> {
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
    let failed_repair = rec.attempted_repair()
        && rec
            .result
            .as_deref()
            .is_some_and(|r| r.eq_ignore_ascii_case("failed"));
    bump_rollup(
        pool,
        ts_ms,
        &rec.computer,
        0,
        opened as i64,
        failed_repair as i64,
    )
    .await?;
    crate::events::notify("check", &rec.computer, &rec.run_id);
    Ok(true)
}

/// Incident state per (computer, check). A failure opens an incident unless one is
/// open; a later "ok" closes it. A check found broken and fixed in the same run is an
/// incident with zero time-to-repair. Records older than the newest already-seen record
/// for the pair (late arrivals from archives) don't change state.
/// Returns true when a new incident was opened.
async fn update_incidents_for_check(
    pool: &DbPool,
    rec: &crate::ingest::AgentRecord,
    ts_ms: i64,
) -> anyhow::Result<bool> {
    if rec.excepted.unwrap_or(false) {
        return Ok(false);
    }
    let newer: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM checks WHERE computer=? AND check_id=? AND ts_ms > ?",
    )
    .bind(&rec.computer)
    .bind(&rec.check_id)
    .bind(ts_ms)
    .fetch_one(pool)
    .await?;
    if newer > 0 {
        return Ok(false);
    }
    let failed = rec.status.eq_ignore_ascii_case("failed");
    let repaired = !failed
        && rec.attempted_repair()
        && rec
            .result
            .as_deref()
            .is_some_and(|r| r.eq_ignore_ascii_case("success"));
    let open: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM incidents WHERE computer=? AND check_id=? AND is_open=1",
    )
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
    let row = sqlx::query("SELECT size, mtime_ms FROM ingested_files WHERE path=?")
        .bind(path)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| (r.get::<i64, _>("size"), r.get::<i64, _>("mtime_ms"))))
}

pub async fn mark_ingested(
    pool: &DbPool,
    path: &str,
    size: i64,
    mtime_ms: i64,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO ingested_files (path, size, mtime_ms) VALUES (?,?,?) ON CONFLICT(path) DO UPDATE SET size=excluded.size, mtime_ms=excluded.mtime_ms")
        .bind(path)
        .bind(size)
        .bind(mtime_ms)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_request(
    pool: &DbPool,
    request_id: &str,
    options_json: &str,
) -> anyhow::Result<()> {
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
pub async fn get_request_status(
    pool: &DbPool,
    request_id: &str,
) -> anyhow::Result<(String, String)> {
    let row = sqlx::query("SELECT status, message FROM requests WHERE request_id=?")
        .bind(request_id)
        .fetch_optional(pool)
        .await?;
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
/// (request_id, options_json, created_ts_ms) of queued/running requests.
pub async fn pending_requests(pool: &DbPool) -> anyhow::Result<Vec<(String, String, i64)>> {
    let rows = sqlx::query("SELECT request_id, options_json, created_ts_ms FROM requests WHERE status IN ('queued','running')").fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.get("request_id"),
                r.get("options_json"),
                r.get("created_ts_ms"),
            )
        })
        .collect())
}

pub async fn set_request_status(
    pool: &DbPool,
    request_id: &str,
    status: &str,
    message: &str,
) -> anyhow::Result<bool> {
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

pub async fn list_machines_overview(
    pool: &DbPool,
) -> anyhow::Result<Vec<(String, String, i64, i64)>> {
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

pub async fn save_exception(
    pool: &DbPool,
    id: &str,
    rule_json: &str,
    active: bool,
) -> anyhow::Result<()> {
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
    let res = sqlx::query("UPDATE exceptions SET active=0 WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    crate::events::notify("exception-expire", "", id);
    Ok(res.rows_affected() > 0)
}

/// (rule_json, active)
pub async fn list_exceptions(
    pool: &DbPool,
    include_inactive: bool,
) -> anyhow::Result<Vec<(String, bool)>> {
    let q = if include_inactive {
        "SELECT rule_json, active FROM exceptions ORDER BY id"
    } else {
        "SELECT rule_json, active FROM exceptions WHERE active=1 ORDER BY id"
    };
    let rows = sqlx::query(q).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.get::<String, _>("rule_json"),
                r.get::<i64, _>("active") != 0,
            )
        })
        .collect())
}

pub async fn insert_audit(
    pool: &DbPool,
    actor: &str,
    action: &str,
    detail: &str,
) -> anyhow::Result<()> {
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
pub async fn list_audit(
    pool: &DbPool,
    limit: i64,
) -> anyhow::Result<Vec<(i64, String, String, String)>> {
    let rows = sqlx::query("SELECT ts_ms, COALESCE(actor,'') AS actor, action, COALESCE(detail,'') AS detail FROM audit_log ORDER BY id DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.get("ts_ms"),
                r.get("actor"),
                r.get("action"),
                r.get("detail"),
            )
        })
        .collect())
}
