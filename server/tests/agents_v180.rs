//! v0.18.0 Rust agent runs plugins and schedules: endorsed PluginUpdate (refused when tampered),
//! RunNow -> report-only run -> RunReport through the durable queue with parity against the
//! nightly, ScheduleSet, an offline run delivered after the server comes back, PluginRemove,
//! Prepare (a refused root key leaves no enrollment) and Check (a revoked agent learns it).
use ::itcmdr_server as srv;
use itcmdr_agent::link::{self, End};
use srv::config::ServerConfig;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

const SCRIPT: &str = r#"
Import-Module $env:ITCMDR_PS_MODULE
Invoke-ItcmdrPlugin -Info @{ name = 'disk-free'; version = '1.0.0'; features = @('verify', 'execute', 'log') } `
  -Verify { param($Ctx) @{ ok = $true; message = 'ready' } } `
  -Execute { param($Ctx)
    Write-ItcmdrLog -EventName 'check' -Message 'C: has room' -CheckId 'DISK-C' -Status 'ok'
    if ($env:ITCMDR_ARTIFACT_DIR) { Set-Content -Path (Join-Path $env:ITCMDR_ARTIFACT_DIR 'df.txt') -Value 'C: 42.5 GiB' }
    @{ ok = $true; message = "checked (report_only=$($Ctx.report_only))"; results = @(@{ check_id = 'DISK-C'; status = 'ok'; message = '42.5 GiB free' }, @{ check_id = 'DISK-D'; status = 'fail'; message = 'D: full' }) } }
"#;

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    c: ServerConfig,
    pool: srv::db::DbPool,
    admin: ed25519_dalek::SigningKey,
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
    Env {
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

async fn install_plugin(e: &Env) {
    let d = e.root.join("src-disk-free");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("main.ps1"), SCRIPT).unwrap();
    std::fs::write(
        d.join("package.toml"),
        "name = \"disk-free\"\nversion = \"1.0.0\"\nfeatures = [\"verify\", \"execute\", \"log\"]\n\n[[files]]\nname = \"main.ps1\"\ntype = \"powershell\"\nmain = true\nchecksum = \"\"\n",
    )
    .unwrap();
    let out = e.root.join("disk-free.tar.gz");
    srv::packages::pack(&d, &e.admin, &out).unwrap();
    srv::packages::install(
        &e.pool,
        &e.c.layout(),
        srv::packages::Source::Path(&out),
        Some(true),
        "test",
    )
    .await
    .unwrap();
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

#[tokio::test]
async fn prepare_refuses_before_enrollment_and_check_reports_revoked() {
    let e = env().await;
    let _stop = start(&e.c).await;
    let adir = e.root.join("agent");
    let good = token(&e, "WS-11").await;
    let mut bad = itcmdr_agentlink::Token::decode(&good).unwrap();
    bad.root = "22".repeat(32);
    let err = link::enroll(&adir, &bad.encode(), Some("WS-11"), "workstation")
        .await
        .unwrap_err();
    assert!(format!("{err:#}").contains("root key"), "{err:#}");
    // Nothing recorded, and the same token still works.
    assert!(srv::agents::by_machine(&e.pool, "WS-11")
        .await
        .unwrap()
        .is_none());
    let st = link::enroll(&adir, &good, Some("WS-11"), "workstation")
        .await
        .unwrap();
    assert!(!st.root_public_key.is_empty());
    assert!(!link::is_revoked(&st).await);
    srv::agents::revoke(&e.pool, "WS-11").await.unwrap();
    assert!(link::is_revoked(&st).await);
}

#[tokio::test]
async fn agent_runs_pushed_plugins_and_schedules() {
    if pwsh().is_none() {
        eprintln!("pwsh not found: skipped");
        return;
    }
    let e = env().await;
    install_plugin(&e).await;
    let stop = start(&e.c).await;
    let adir = e.root.join("agent");
    let st = link::enroll(
        &adir,
        &token(&e, "WS-12").await,
        Some("WS-12"),
        "workstation",
    )
    .await
    .unwrap();

    // A tampered endorsement is refused by the agent.
    let want = srv::agent_plugins::desired(&e.pool, &e.c.layout(), "WS-12")
        .await
        .unwrap();
    assert_eq!(want.len(), 1);
    let mut u = srv::agent_plugins::update_for(&e.c.layout(), &want[0].0, &want[0].1).unwrap();
    let good_sig = u.endorsement.clone();
    u.endorsement = "00".repeat(64);
    let other = tempfile::tempdir().unwrap();
    let err = itcmdr_agent::plugins::apply(other.path(), &st.root_public_key, &u).unwrap_err();
    assert!(format!("{err:#}").contains("endorsed"), "{err:#}");
    u.endorsement = good_sig;
    u.files
        .iter_mut()
        .find(|f| f.name == "main.ps1")
        .unwrap()
        .data
        .extend_from_slice(b"\n# x");
    let err = itcmdr_agent::plugins::apply(other.path(), &st.root_public_key, &u).unwrap_err();
    assert!(format!("{err:#}").contains("checksum"), "{err:#}");

    // A schedule naming the plugin, for this machine.
    let spec = srv::schedule::Spec {
        machines: vec!["WS-12".into()],
        plugins: vec!["disk-free".into()],
        when: "daily 03:00".into(),
        guard: "none".into(),
        missed: "catch-up".into(),
        ..Default::default()
    };
    let sch = srv::schedule::create(&e.pool, "agent disk", spec, true)
        .await
        .unwrap();

    // Link: the plugin and the schedules arrive and are applied.
    let h = spawn_session(&adir, &st);
    let pool = e.pool.clone();
    until("plugin applied", 20, || {
        let pool = pool.clone();
        async move {
            srv::agent_plugins::list_plugins(&pool, Some("WS-12"))
                .await
                .unwrap()
                .iter()
                .any(|p| p["name"] == "disk-free" && p["ok"] == true)
        }
    })
    .await;
    assert!(itcmdr_agent::plugins::installed(&adir)[0].starts_with("disk-free@1.0.0:"));
    let s = itcmdr_agent::plugins::load_schedules(&adir);
    assert_eq!(s.schedules.len(), 1);
    assert_eq!(s.schedules[0].id, sch.id);
    assert!(s.schedules[0].catch_up);

    // A nightly result for parity: DISK-C ok (matches), DISK-D absent (mismatch).
    let t = chrono::Utc::now().timestamp_millis();
    sqlx::query("INSERT INTO runs(run_id, computer, ts_ms) VALUES ('n1','WS-12',?)")
        .bind(t - 3_600_000)
        .execute(&e.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO checks(run_id, computer, category, check_id, severity, status, ts_ms) VALUES ('n1','WS-12','storage','DISK-C','low','ok',?)").bind(t - 3_600_000).execute(&e.pool).await.unwrap();

    // RunNow -> report-only run -> report with results, artifact and parity.
    let rid =
        srv::agent_plugins::request_run(&e.pool, "WS-12", "disk-free", &Default::default(), "test")
            .await
            .unwrap();
    let pool = e.pool.clone();
    until("run report", 60, || {
        let pool = pool.clone();
        async move {
            !srv::agent_plugins::list_runs(&pool, Some("WS-12"), 5)
                .await
                .unwrap()
                .is_empty()
        }
    })
    .await;
    let r = &srv::agent_plugins::list_runs(&e.pool, Some("WS-12"), 5)
        .await
        .unwrap()[0];
    assert_eq!(r.status, "ok", "{r:?}");
    assert!(r.report_only);
    assert_eq!(r.trigger, format!("run-now:{rid}"));
    assert!(r.message.contains("report_only=True"), "{}", r.message);
    assert_eq!(r.results.as_array().unwrap().len(), 2);
    assert_eq!((r.parity_matched, r.parity_mismatched), (Some(1), Some(1)));
    assert_eq!(r.artifacts[0]["name"], "df.txt");
    assert!(e
        .c
        .layout()
        .root
        .join("agent-artifacts")
        .join(&r.run_id)
        .join("df.txt")
        .is_file());
    let par: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM parity_runs WHERE run_id = ?")
        .bind(format!("agent:{}", r.run_id))
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(par, 1);
    h.abort();

    // Offline: the server is down, a scheduled run happens and is queued...
    let _ = stop.send(());
    let addr = e.c.agent_bind_addr.clone();
    until("server down", 20, || {
        let a = addr.clone();
        async move { tokio::net::TcpStream::connect(&a).await.is_err() }
    })
    .await;
    link::run_and_queue(
        &adir,
        "WS-12",
        "disk-free",
        &Default::default(),
        &format!("schedule:{}", sch.id),
    )
    .await;
    assert!(!itcmdr_agent::queue::ingest(&adir, 256).unwrap().is_empty());
    // ...and delivered once the server is back.
    let _stop2 = start(&e.c).await;
    let h = spawn_session(&adir, &st);
    let pool = e.pool.clone();
    until("queued report", 30, || {
        let pool = pool.clone();
        async move {
            srv::agent_plugins::list_runs(&pool, Some("WS-12"), 5)
                .await
                .unwrap()
                .iter()
                .any(|r| r.trigger.starts_with("schedule:"))
        }
    })
    .await;
    assert_eq!(
        srv::agent_plugins::list_runs(&e.pool, Some("WS-12"), 10)
            .await
            .unwrap()
            .len(),
        2
    );
    h.abort();

    // Disable the plugin: the next link removes it from the agent.
    srv::packages::disable(&e.pool, "disk-free", "test")
        .await
        .unwrap();
    let h = spawn_session(&adir, &st);
    let d = adir.clone();
    until("plugin removed", 20, || {
        let d = d.clone();
        async move { itcmdr_agent::plugins::installed(&d).is_empty() }
    })
    .await;
    h.abort();
}
