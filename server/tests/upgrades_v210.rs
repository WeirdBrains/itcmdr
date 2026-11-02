//! v0.21.0 agent self-upgrade: owner-only releases (never pushed as plugins), the UpgradeOffer over
//! the real link (verified, staged, handed to the watchdog), a watchdog rollback reported after the
//! reconnect pausing the rollout with a Critical alert, and the ring/soak/auto-pause state machine.
use ::itcmdr_server as srv;
use itcmdr_agent::link::{self, End};
use itcmdr_watchdog as wd;
use srv::config::ServerConfig;
use std::path::{Path, PathBuf};
use std::time::Duration;

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    c: ServerConfig,
    pool: srv::db::DbPool,
    admin: ed25519_dalek::SigningKey,
    owner: ed25519_dalek::SigningKey,
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn pwsh() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut c: Vec<PathBuf> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|d| d.join(if cfg!(windows) { "pwsh.exe" } else { "pwsh" }))
        .collect();
    if cfg!(windows) {
        c.push(PathBuf::from(
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
        ));
    }
    c.push(home.join("Projects/tools/pwsh/pwsh"));
    c.into_iter().find(|p| p.is_file())
}

async fn env() -> Env {
    std::env::set_var(
        "ITCMDR_PLUGIN_PS_HOST",
        env!("CARGO_BIN_EXE_itcmdr-plugin-ps"),
    );
    std::env::set_var(
        "ITCMDR_PS_MODULE",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../agent/Itcmdr.Plugin.psm1"),
    );
    if let Some(p) = pwsh() {
        std::env::set_var("ITCMDR_POWERSHELL", p);
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let mut c = ServerConfig {
        bind_addr: format!("127.0.0.1:{}", free_port()),
        agent_bind_addr: format!("127.0.0.1:{}", free_port()),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        auth_mode: "signed".into(),
        ..ServerConfig::default()
    };
    c.backup.nightly = false;
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let m = srv::keys::Master::from_words(&out.words).unwrap();
    let admin = srv::keys::new_keypair();
    let cert = srv::keys::certify(
        &m.signing,
        "user",
        "Admin A",
        "admin",
        &admin.verifying_key(),
    );
    srv::auth::register(
        &pool,
        srv::auth::NewKey {
            kind: "user",
            name: "Admin A",
            role: srv::auth::Role::Admin,
            email: None,
            phone: None,
            public_key_hex: &hex::encode(admin.verifying_key().as_bytes()),
            certificate: &cert,
        },
    )
    .await
    .unwrap();
    let owner = srv::keys::new_keypair();
    let ocert = srv::keys::certify(
        &m.signing,
        "user",
        "Owner O",
        "owner",
        &owner.verifying_key(),
    );
    srv::auth::register(
        &pool,
        srv::auth::NewKey {
            kind: "user",
            name: "Owner O",
            role: srv::auth::Role::Owner,
            email: None,
            phone: None,
            public_key_hex: &hex::encode(owner.verifying_key().as_bytes()),
            certificate: &ocert,
        },
    )
    .await
    .unwrap();
    Env {
        owner,
        _tmp: tmp,
        root,
        c,
        pool,
        admin,
    }
}

async fn start(c: &ServerConfig) -> tokio::sync::oneshot::Sender<()> {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(srv::run(c.clone(), async move {
        let _ = rx.await;
    }));
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(&c.agent_bind_addr)
            .await
            .is_ok()
        {
            return tx;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("agent link did not start");
}

async fn token(e: &Env, machine: &str) -> String {
    srv::agents::create_token(
        &e.pool,
        &e.c.layout(),
        &e.c,
        machine,
        60,
        &e.c.agent_bind_addr,
        "test",
    )
    .await
    .unwrap()
    .token
}

async fn until<F, Fut>(what: &str, secs: u64, mut f: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..secs * 10 {
        if f().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("timed out waiting for {what}");
}

fn spawn_session(dir: &Path, st: &itcmdr_agent::state::State) -> tokio::task::JoinHandle<End> {
    let (d, s) = (dir.to_path_buf(), st.clone());
    tokio::spawn(async move { link::session(&d, &s, |_| {}).await })
}

const EXE: &str = itcmdr_agent::upgrade::EXE;

/// Build and install an itcmdr-agent-release package signed by `key`.
async fn install_release(
    e: &Env,
    version: &str,
    key: &ed25519_dalek::SigningKey,
) -> anyhow::Result<()> {
    let d = e.root.join(format!("rel-{version}"));
    std::fs::create_dir_all(&d)?;
    std::fs::write(d.join(EXE), format!("agent {version}"))?;
    std::fs::write(
        d.join("release.toml"),
        format!("version = \"{version}\"\nmin_from = \"0.20.0\"\n"),
    )?;
    std::fs::write(
        d.join("package.toml"),
        format!("name = \"itcmdr-agent-release\"\nversion = \"{version}\"\nfeatures = [\"verify\", \"execute\", \"log\"]\n\n[[files]]\nname = \"{EXE}\"\ntype = \"bin\"\nmain = true\nchecksum = \"\"\n\n[[files]]\nname = \"release.toml\"\ntype = \"config\"\nchecksum = \"\"\n"),
    )?;
    let out = e.root.join(format!("rel-{version}.tar.gz"));
    srv::packages::pack(&d, key, &out)?;
    srv::packages::install(
        &e.pool,
        &e.c.layout(),
        srv::packages::Source::Path(&out),
        Some(true),
        "test",
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn owner_release_is_offered_staged_and_a_rollback_pauses_the_rollout() {
    let e = env().await;
    // An admin-signed release installs (it is a valid package) but cannot be rolled out.
    install_release(&e, "98.0.0", &e.admin).await.unwrap();
    let err = srv::upgrades::start(&e.pool, &e.c.layout(), "98.0.0", "t")
        .await
        .unwrap_err();
    assert!(format!("{err:#}").contains("owner signature"), "{err:#}");
    install_release(&e, "99.0.0", &e.owner).await.unwrap();
    // Releases are never pushed as plugins.
    assert!(srv::agent_plugins::desired(&e.pool, &e.c.layout(), "WS-21")
        .await
        .unwrap()
        .is_empty());

    let prog = e.root.join("prog");
    std::fs::create_dir_all(prog.join("current")).unwrap();
    std::fs::write(prog.join("current").join(EXE), b"old").unwrap();
    std::env::set_var("ITCMDR_AGENT_PROGRAM", &prog);
    let stop = start(&e.c).await;
    let adir = e.root.join("agent");
    let st = link::enroll(
        &adir,
        &token(&e, "WS-21").await,
        Some("WS-21"),
        "workstation",
    )
    .await
    .unwrap();
    srv::settings::set(&e.pool, "agent.ring", "0", Some("WS-21"))
        .await
        .unwrap();

    let r = srv::upgrades::start(&e.pool, &e.c.layout(), "99.0.0", "t")
        .await
        .unwrap();
    assert_eq!((r.ring, r.state.as_str()), (0, "active"));
    assert!(
        srv::upgrades::start(&e.pool, &e.c.layout(), "99.0.0", "t")
            .await
            .is_err(),
        "one rollout at a time"
    );

    let s = spawn_session(&adir, &st);
    let rq = wd::request_path(&adir);
    until("the staged upgrade request", 40, || {
        let p = rq.clone();
        async move { p.exists() }
    })
    .await;
    let req: wd::Request = wd::read_json(&rq).unwrap();
    assert_eq!(req.version, "99.0.0");
    assert_eq!(
        std::fs::read(prog.join("staged/99.0.0").join(EXE)).unwrap(),
        b"agent 99.0.0"
    );
    let p = e.pool.clone();
    until("Applied upgrade ok", 20, || {
        let p = p.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT ok FROM agent_upgrades WHERE machine = 'WS-21'")
                .fetch_optional(&p)
                .await
                .unwrap()
                == Some(1)
        }
    })
    .await;
    s.abort();

    // The watchdog rolled back: the agent reports it after the reconnect; the rollout pauses.
    std::fs::remove_file(&rq).unwrap();
    wd::write_json(
        &wd::result_path(&adir),
        &wd::Outcome {
            version: "99.0.0".into(),
            from: "0.21.0".into(),
            ok: false,
            rolled_back: true,
            error: "99.0.0 was not healthy within 300 s; rolled back".into(),
            ts_ms: 0,
        },
    )
    .unwrap();
    let s = spawn_session(&adir, &st);
    let p = e.pool.clone();
    until("the rollout pauses", 20, || {
        let p = p.clone();
        async move {
            srv::upgrades::current(&p)
                .await
                .unwrap()
                .is_some_and(|r| r.state == "paused")
        }
    })
    .await;
    assert!(!wd::result_path(&adir).exists(), "result reported once");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM alerts WHERE key = 'agent-upgrade:WS-21' AND level = 'critical' AND state = 'open'")
        .fetch_one(&e.pool).await.unwrap();
    assert_eq!(n, 1);
    // Paused: no new offer for that version, even on another session.
    assert!(
        srv::upgrades::offer_for(&e.pool, &e.c.layout(), "WS-21", "0.21.0")
            .await
            .unwrap()
            .is_none()
    );
    // Resume clears the failure and the alert, and offers it again.
    srv::upgrades::resume(&e.pool, "t").await.unwrap();
    assert!(
        srv::upgrades::offer_for(&e.pool, &e.c.layout(), "WS-21", "0.21.0")
            .await
            .unwrap()
            .is_some()
    );
    // agent.upgrades = false holds the machine back.
    srv::settings::set(&e.pool, "agent.upgrades", "false", Some("WS-21"))
        .await
        .unwrap();
    assert!(
        srv::upgrades::offer_for(&e.pool, &e.c.layout(), "WS-21", "0.21.0")
            .await
            .unwrap()
            .is_none()
    );
    // What the Updates screen shows: newest release first, the admin-signed one flagged.
    let rel = srv::upgrades::releases(&e.pool, &e.c.layout())
        .await
        .unwrap();
    assert_eq!(rel.len(), 2);
    assert_eq!(
        (
            rel[0].version.as_str(),
            rel[0].owner_signed,
            rel[0].problem.as_str()
        ),
        ("99.0.0", true, "")
    );
    assert!(
        !rel[1].owner_signed && rel[1].problem.contains("owner signature"),
        "{:?}",
        rel[1]
    );
    let m = srv::upgrades::machines(&e.pool).await.unwrap();
    let w = m.iter().find(|m| m.machine == "WS-21").unwrap();
    assert_eq!(
        (w.ring, w.upgrades, w.last_version.as_str(), w.last_ok),
        (0, false, "99.0.0", true) // resumed: offered and staged again
    );
    s.abort();
    let _ = stop.send(());
    std::env::remove_var("ITCMDR_AGENT_PROGRAM");
}

async fn fake_agent(e: &Env, machine: &str, version: &str, seen: i64) {
    sqlx::query("INSERT INTO agents(id, machine, role, state, enrolled_ms, last_seen_ms, version, os, peer, cert_expires_ms, cert_fp, cert_serial, public_key) VALUES (?,?,'workstation','online',0,?,?,'','',0,'','','')")
        .bind(uuid::Uuid::new_v4().to_string()).bind(machine).bind(seen).bind(version)
        .execute(&e.pool).await.unwrap();
}

async fn set_version(e: &Env, machine: &str, v: &str, seen: i64) {
    sqlx::query("UPDATE agents SET version = ?, last_seen_ms = ? WHERE machine = ?")
        .bind(v)
        .bind(seen)
        .bind(machine)
        .execute(&e.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn rings_soak_advance_and_pause_on_critical() {
    let e = env().await;
    install_release(&e, "99.0.0", &e.owner).await.unwrap();
    let t0 = chrono::Utc::now().timestamp_millis() - 10 * 3_600_000;
    for (m, ring) in [
        ("WS-20", "0"),
        ("WS-31", "1"),
        ("WS-32", "2"),
        ("WS-33", "2"),
    ] {
        fake_agent(&e, m, "0.21.0", t0).await;
        srv::settings::set(&e.pool, "agent.ring", ring, Some(m))
            .await
            .unwrap();
    }
    srv::settings::set(&e.pool, "agent.upgradeSoakMins", "60", None)
        .await
        .unwrap();
    srv::upgrades::start(&e.pool, &e.c.layout(), "99.0.0", "t")
        .await
        .unwrap();
    let offered = |m: &'static str| {
        let (p, l) = (e.pool.clone(), e.c.layout());
        async move {
            srv::upgrades::offer_for(&p, &l, m, "0.21.0")
                .await
                .unwrap()
                .is_some()
        }
    };
    assert!(offered("WS-20").await && !offered("WS-31").await && !offered("WS-32").await);
    // Ring 0 not upgraded yet: it waits.
    let r = srv::upgrades::tick(&e.pool, t0).await.unwrap().unwrap();
    assert_eq!((r.ring, r.ring_ok_ms), (0, 0));
    assert_eq!(
        srv::upgrades::ring_problems(&e.pool, &r, t0)
            .await
            .unwrap()
            .len(),
        1
    );
    // Upgraded and healthy: the soak starts; 59 min later still ring 0, at 60 min ring 1.
    set_version(&e, "WS-20", "99.0.0", t0).await;
    let r = srv::upgrades::tick(&e.pool, t0).await.unwrap().unwrap();
    assert_eq!((r.ring, r.ring_ok_ms), (0, t0));
    set_version(&e, "WS-20", "99.0.0", t0 + 59 * 60_000).await;
    assert_eq!(
        srv::upgrades::tick(&e.pool, t0 + 59 * 60_000)
            .await
            .unwrap()
            .unwrap()
            .ring,
        0
    );
    set_version(&e, "WS-20", "99.0.0", t0 + 60 * 60_000).await;
    let r = srv::upgrades::tick(&e.pool, t0 + 60 * 60_000)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((r.ring, r.state.as_str()), (1, "active"));
    assert!(offered("WS-31").await && !offered("WS-32").await);
    // A machine that goes quiet during the soak resets it.
    let t1 = t0 + 61 * 60_000;
    set_version(&e, "WS-31", "99.0.0", t1).await;
    assert_eq!(
        srv::upgrades::tick(&e.pool, t1)
            .await
            .unwrap()
            .unwrap()
            .ring_ok_ms,
        t1
    );
    let late = t1 + 30 * 60_000;
    assert_eq!(
        srv::upgrades::tick(&e.pool, late)
            .await
            .unwrap()
            .unwrap()
            .ring_ok_ms,
        0,
        "not seen for 30 min"
    );
    // A new Critical alert on a ring machine pauses the rollout (auto-pause).
    srv::alerts::raise(
        &e.pool,
        &srv::alerts::Raise {
            key: "disk:WS-31".into(),
            level: "critical".into(),
            source: "agent".into(),
            machine: "WS-31".into(),
            title: "WS-31 disk full".into(),
            detail: String::new(),
            run_id: None,
        },
    )
    .await
    .unwrap();
    let r = srv::upgrades::tick(&e.pool, late).await.unwrap().unwrap();
    assert_eq!(r.state, "paused");
    assert!(r.reason.contains("WS-31 disk full"), "{}", r.reason);
    assert!(!offered("WS-31").await);
    // Resolve, resume; ring 1 then ring 2 complete; the rollout is done.
    srv::alerts::resolve_key(&e.pool, "disk:WS-31")
        .await
        .unwrap();
    srv::upgrades::resume(&e.pool, "t").await.unwrap();
    let t2 = chrono::Utc::now().timestamp_millis() + 1000;
    set_version(&e, "WS-31", "99.0.0", t2).await;
    srv::upgrades::tick(&e.pool, t2).await.unwrap();
    let t3 = t2 + 60 * 60_000;
    set_version(&e, "WS-31", "99.0.0", t3).await;
    assert_eq!(
        srv::upgrades::tick(&e.pool, t3)
            .await
            .unwrap()
            .unwrap()
            .ring,
        2
    );
    for m in ["WS-32", "WS-33"] {
        set_version(&e, m, "99.0.0", t3).await;
    }
    srv::upgrades::tick(&e.pool, t3).await.unwrap();
    let t4 = t3 + 60 * 60_000;
    for m in ["WS-32", "WS-33"] {
        set_version(&e, m, "99.0.0", t4).await;
    }
    let r = srv::upgrades::tick(&e.pool, t4).await.unwrap().unwrap();
    assert_eq!(r.state, "done");
    assert!(srv::upgrades::current(&e.pool).await.unwrap().is_none());
    // Nothing is offered once done.
    assert!(!offered("WS-33").await);
}
