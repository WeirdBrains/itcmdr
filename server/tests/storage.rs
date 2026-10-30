//! v0.6.0 storage: data dir layout, SQLCipher at rest, auth tables, plaintext migration.
use itcmdr_server::config::ServerConfig;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

fn cfg(root: &Path) -> ServerConfig {
    ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        ..ServerConfig::default()
    }
}

async fn plain_pool(p: &Path) -> sqlx::SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::from_str(&format!("sqlite://{}", p.display()))
                .unwrap()
                .create_if_missing(true),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn fresh_database_is_encrypted_with_auth_tables() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    let db = tmp.path().join("data").join("itcmdr.database");
    assert_eq!(c.database_path(), db);
    for d in ["service_plugins", "quarantine", "keys", "plugin-logs"] {
        assert!(tmp.path().join("data").join(d).is_dir());
    }
    let roles: Vec<String> = sqlx::query_scalar("SELECT name FROM roles ORDER BY name")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(roles, vec!["admin", "client", "owner", "user"]);
    let rid = itcmdr_server::db::role_id(&pool, "user")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(uuid::Uuid::parse_str(&rid).unwrap().get_version_num(), 4);
    sqlx::query("INSERT INTO users(id,name,role_id,public_key,fingerprint) VALUES (?,?,?,?,?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind("Front Desk")
        .bind(&rid)
        .bind("pk")
        .bind("fp1")
        .execute(&pool)
        .await
        .unwrap();
    let dup = sqlx::query("INSERT INTO clients(id,name,role_id,public_key,fingerprint) VALUES ('x','c',?, 'pk','fp2'), ('y','d',?,'pk','fp2')")
        .bind(&rid)
        .bind(&rid)
        .execute(&pool)
        .await;
    assert!(dup.is_err(), "fingerprints are unique");
    let created: String = sqlx::query_scalar("SELECT created_at FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(created.ends_with('Z'));
    pool.close().await;

    assert!(
        !itcmdr_server::storage::is_plaintext_sqlite(&db),
        "file must not be plaintext"
    );
    let raw = std::fs::read(&db).unwrap();
    assert!(
        !raw.windows(5).any(|w| w == b"roles"),
        "table names must not be readable"
    );
    // Without the key the file is unreadable.
    let p = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::from_str(&format!("sqlite://{}", db.display())).unwrap(),
        )
        .await;
    if let Ok(p) = p {
        assert!(sqlx::query("SELECT count(*) FROM sqlite_master")
            .execute(&p)
            .await
            .is_err());
    }
    // Re-open with the stored key works and keeps the roles (no duplicates).
    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM roles")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 4);
}

#[tokio::test]
async fn wrong_key_is_a_clear_error() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    itcmdr_server::db::init_db(&c).await.unwrap().close().await;
    let keyf = tmp.path().join("data/keys/db.key");
    std::fs::remove_file(&keyf).unwrap();
    itcmdr_server::storage::write_secret(&keyf, &[7u8; 32]).unwrap();
    let err = itcmdr_server::db::init_db(&c)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("wrong key"), "{err}");
}

async fn make_legacy(p: &Path) {
    let pool = plain_pool(p).await;
    sqlx::query("CREATE TABLE runs (run_id TEXT PRIMARY KEY, computer TEXT NOT NULL, ts_ms INTEGER NOT NULL, role TEXT, report_only INTEGER DEFAULT 0)")
        .execute(&pool)
        .await
        .unwrap();
    for i in 0..3 {
        sqlx::query("INSERT INTO runs(run_id,computer,ts_ms) VALUES (?, 'WS-01', ?)")
            .bind(format!("r{i}"))
            .bind(i)
            .execute(&pool)
            .await
            .unwrap();
    }
    pool.close().await;
}

#[tokio::test]
async fn migrates_legacy_plaintext_database() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    std::fs::create_dir_all(tmp.path().join("data")).unwrap();
    let legacy = tmp.path().join("data/itcmdr.sqlite");
    make_legacy(&legacy).await;
    assert!(itcmdr_server::storage::is_plaintext_sqlite(&legacy));

    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM runs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 3, "rows survive the migration");
    // Old schema got the newer columns too.
    sqlx::query("SELECT finished_ts_ms, exit_code FROM runs")
        .fetch_all(&pool)
        .await
        .unwrap();
    pool.close().await;
    assert!(!legacy.exists());
    assert!(tmp
        .path()
        .join("data/itcmdr.sqlite.plaintext-backup")
        .exists());
    assert!(!itcmdr_server::storage::is_plaintext_sqlite(
        &c.database_path()
    ));
    // Second start: nothing to migrate.
    assert!(!itcmdr_server::db::migrate_plaintext(
        &c.database_path(),
        &itcmdr_server::storage::load_or_create_db_key(&c.layout()).unwrap()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn migrates_plaintext_in_place_for_an_explicit_database_path() {
    let tmp = tempfile::tempdir().unwrap();
    let mut c = cfg(tmp.path());
    let p = tmp.path().join("custom.db");
    make_legacy(&p).await;
    c.database_path = Some(p.clone());
    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM runs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 3);
    assert!(!itcmdr_server::storage::is_plaintext_sqlite(&p));
    assert!(tmp.path().join("custom.db.plaintext-backup").exists());
}

#[test]
fn data_dir_flag_beats_config() {
    let mut c = ServerConfig {
        data_dir: Some("/from-config".into()),
        ..ServerConfig::default()
    };
    if std::env::var_os("ITCMDR_DATA_DIR").is_none() {
        assert_eq!(c.layout().root, Path::new("/from-config"));
    }
    c.data_dir_flag = Some("/from-flag".into());
    assert_eq!(c.layout().root, Path::new("/from-flag"));
    assert_eq!(c.database_path(), Path::new("/from-flag/itcmdr.database"));
}
