//! Test Postgres: a throwaway cluster per test binary (initdb + pg_ctl from ITCMDR_PG_BIN, or
//! ~/Projects/tools/pgsql/bin), one database per test. Or ITCMDR_TEST_PG_URL (an admin URL).
#![allow(dead_code)]
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

pub struct Pg {
    pub admin_url: String,
    _dir: Option<tempfile::TempDir>,
    bin: Option<PathBuf>,
}

impl Drop for Pg {
    fn drop(&mut self) {
        if let (Some(b), Some(d)) = (&self.bin, &self._dir) {
            let _ = Command::new(b.join("pg_ctl"))
                .args(["-D"])
                .arg(d.path().join("data"))
                .args(["-m", "immediate", "stop"])
                .output();
        }
    }
}

static PG: OnceLock<Option<Pg>> = OnceLock::new();

fn start_pg() -> Option<Pg> {
    if let Ok(u) = std::env::var("ITCMDR_TEST_PG_URL") {
        return Some(Pg {
            admin_url: u,
            _dir: None,
            bin: None,
        });
    }
    let bin = std::env::var_os("ITCMDR_PG_BIN")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Projects/tools/pgsql/bin"))
        })?;
    let exe = if cfg!(windows) {
        "initdb.exe"
    } else {
        "initdb"
    };
    if !bin.join(exe).is_file() {
        return None;
    }
    let dir = tempfile::tempdir().ok()?;
    let data = dir.path().join("data");
    let out = Command::new(bin.join("initdb"))
        .arg("-D")
        .arg(&data)
        .args(["-U", "postgres", "-A", "trust", "-E", "UTF8", "--no-sync"])
        .output()
        .ok()?;
    if !out.status.success() {
        eprintln!("initdb failed: {}", String::from_utf8_lossy(&out.stderr));
        return None;
    }
    let port = free_port();
    let sock = if cfg!(windows) {
        String::new()
    } else {
        format!(" -k {}", dir.path().display())
    };
    let st = Command::new(bin.join("pg_ctl"))
        .arg("-D")
        .arg(&data)
        .arg("-l")
        .arg(dir.path().join("pg.log"))
        .arg("-o")
        .arg(format!("-p {port} -h 127.0.0.1 -F{sock}"))
        .args(["-w", "start"])
        .output()
        .ok()?;
    if !st.status.success() {
        eprintln!("pg_ctl failed: {}", String::from_utf8_lossy(&st.stderr));
        return None;
    }
    Some(Pg {
        admin_url: format!("postgres://postgres@127.0.0.1:{port}"),
        _dir: Some(dir),
        bin: Some(bin),
    })
}

/// A test database: a URL plus whatever keeps it alive.
pub struct TestDb {
    pub url: String,
    _dir: Option<tempfile::TempDir>,
}

/// A fresh Postgres database, or None (skipped, unless ITCMDR_REQUIRE_PG=1) without Postgres.
pub async fn pg_db() -> Option<TestDb> {
    let Some(pg) = PG.get_or_init(start_pg).as_ref() else {
        assert!(
            std::env::var("ITCMDR_REQUIRE_PG").is_err(),
            "ITCMDR_REQUIRE_PG set but no Postgres (ITCMDR_PG_BIN / ITCMDR_TEST_PG_URL)"
        );
        eprintln!("no Postgres (ITCMDR_PG_BIN / ITCMDR_TEST_PG_URL): Postgres variant skipped");
        return None;
    };
    let name = format!("t{}", uuid::Uuid::new_v4().simple());
    let admin = format!("{}/postgres", pg.admin_url);
    tokio::task::spawn_blocking(move || {
        use diesel::{Connection, RunQueryDsl};
        let mut c = diesel::PgConnection::establish(&admin).ok()?;
        diesel::sql_query(format!("CREATE DATABASE {name}"))
            .execute(&mut c)
            .ok()?;
        Some(TestDb {
            url: format!("{}/{name}", admin.trim_end_matches("/postgres")),
            _dir: None,
        })
    })
    .await
    .ok()?
}

/// A fresh SQLite database file (always available).
pub fn sqlite_db() -> TestDb {
    let d = tempfile::tempdir().unwrap();
    TestDb {
        url: format!("sqlite://{}", d.path().join("adminsvr.db").display()),
        _dir: Some(d),
    }
}

/// `both!(body, sqlite_name, postgres_name)`: run `async fn body(db: String)` on each backend.
#[macro_export]
macro_rules! both {
    ($body:ident, $lite:ident, $pg:ident) => {
        #[tokio::test]
        async fn $lite() {
            let d = common::sqlite_db();
            $body(d.url.clone()).await;
        }
        #[tokio::test]
        async fn $pg() {
            if let Some(d) = common::pg_db().await {
                $body(d.url.clone()).await;
            }
        }
    };
}

use itcmdr_admin::Client;
use itcmdr_adminsvr::config::Config;
use std::time::Duration;

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub struct Srv {
    pub cfg: Config,
    pub _tmp: tempfile::TempDir,
    pub stop: Option<tokio::sync::oneshot::Sender<()>>,
}

pub async fn start(db: &str) -> Srv {
    start_with(db, |_| {}).await
}

pub async fn start_with(db: &str, f: impl FnOnce(&mut Config)) -> Srv {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = Config::for_test(tmp.path().join("data"), db, free_port(), free_port());
    cfg.heartbeat_secs = 1;
    cfg.site_missed = 3;
    f(&mut cfg);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let c = cfg.clone();
    tokio::spawn(async move {
        if let Err(e) = itcmdr_adminsvr::run(c, async move {
            let _ = rx.await;
        })
        .await
        {
            eprintln!("adminsvr: {e:#}");
        }
    });
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(&cfg.api_addr).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Srv {
        cfg,
        _tmp: tmp,
        stop: Some(tx),
    }
}

pub async fn owner(s: &Srv) -> (Client, itcmdr_admin::Enrolled) {
    let code = itcmdr_adminsvr::api::init(&s.cfg, "owner@example.com", "Owner O")
        .await
        .unwrap();
    let e = itcmdr_admin::enroll(&code, "test", false, None)
        .await
        .unwrap();
    assert_eq!(e.user.role, "owner");
    assert_eq!(e.user.devices[0].status, "active");
    (Client::connect(&e.identity).await.unwrap(), e)
}

pub async fn pool(url: &str) -> itcmdr_adminsvr::store::Pool {
    itcmdr_adminsvr::store::connect(url).await.unwrap()
}
