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

/// A fresh database URL, or None (tests skip) when no Postgres is available.
pub async fn db() -> Option<String> {
    let pg = PG.get_or_init(start_pg).as_ref()?;
    let name = format!("t{}", uuid::Uuid::new_v4().simple());
    let pool = sqlx::PgPool::connect(&format!("{}/postgres", pg.admin_url))
        .await
        .ok()?;
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&pool)
        .await
        .ok()?;
    pool.close().await;
    Some(format!("{}/{name}", pg.admin_url))
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
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = Config::for_test(tmp.path().join("data"), db, free_port(), free_port());
    cfg.heartbeat_secs = 1;
    cfg.site_missed = 3;
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

