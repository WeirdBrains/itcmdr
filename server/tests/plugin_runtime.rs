//! v0.10.0 plugin runtime: PowerShell plugins through itcmdr-plugin-ps, the stage order
//! (a failed Verify stops Execute, Log on failure), per-run itcmdr.log/v1 logs, metrics ingest
//! with malformed-line counting, on-demand plugin requests and plugin logs in backups.
use ::itcmdr_server as srv;
use srv::config::ServerConfig;
use srv::packages::{self, Source};
use std::path::{Path, PathBuf};

fn pwsh() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut c: Vec<PathBuf> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|d| d.join(if cfg!(windows) { "pwsh.exe" } else { "pwsh" }))
        .collect();
    c.push(home.join("Projects/tools/pwsh/pwsh"));
    c.into_iter().find(|p| p.is_file())
}

fn setup_env() {
    std::env::set_var(
        "ITCMDR_PLUGIN_PS_HOST",
        env!("CARGO_BIN_EXE_itcmdr-plugin-ps"),
    );
    std::env::set_var(
        "ITCMDR_PS_MODULE",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../agent/Itcmdr.Plugin.psm1"),
    );
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    c: ServerConfig,
    pool: srv::db::DbPool,
    words: String,
    admin: ed25519_dalek::SigningKey,
}

async fn env() -> Env {
    setup_env();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let mut c = ServerConfig {
        // These tests cover the shared token (migration mode "both").
        auth_mode: "both".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        ..ServerConfig::default()
    };
    c.backup.nightly = false;
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let m = srv::keys::Master::from_words(&out.words).unwrap();
    let admin = srv::keys::new_keypair();
    let cert = srv::keys::certify(&m.signing, "user", "Dr A", "admin", &admin.verifying_key());
    let pk = hex::encode(admin.verifying_key().as_bytes());
    srv::auth::register(
        &pool,
        srv::auth::NewKey {
            kind: "user",
            name: "Dr A",
            role: srv::auth::Role::Admin,
            email: None,
            phone: None,
            public_key_hex: &pk,
            certificate: &cert,
        },
    )
    .await
    .unwrap();
    if let Some(p) = pwsh() {
        srv::settings::set(&pool, "plugins.powershell", &p.display().to_string(), None)
            .await
            .unwrap();
    }
    Env {
        _tmp: tmp,
        root,
        c,
        pool,
        words: out.words.to_string(),
        admin,
    }
}

const SCRIPT: &str = r#"
Import-Module $env:ITCMDR_PS_MODULE
Invoke-ItcmdrPlugin -Info @{ name = 'disk-free'; version = '1.0.0'; features = @('verify', 'execute', 'log') } `
  -Verify { param($Ctx)
    if ($Ctx.args.PSObject.Properties['fail']) { throw 'share not reachable' }
    @{ ok = $true; message = 'ready' } } `
  -Execute { param($Ctx)
    Write-ItcmdrLog -EventName 'check' -Message 'C: has room' -CheckId 'DISK-C' -Status 'ok'
    Write-ItcmdrMetric -Name 'disk_free' -Value 42.5 -Unit 'GiB' -Labels @{ drive = 'C:' }
    Write-ItcmdrMetric -Name 'cleanups' -Value 2 -Kind 'counter'
    @{ ok = $true; message = "checked (report_only=$($Ctx.report_only))"; results = @(@{ check_id = 'DISK-C'; status = 'ok'; message = '42.5 GiB free' }) } } `
  -Log { param($Ctx, $Stage, $Err)
    Write-ItcmdrLog -Level 'error' -EventName 'stage-failed' -Message "${Stage}: $Err" }
"#;

async fn install(e: &Env, name: &str, files: &[(&str, &str, &str, bool)]) {
    let d = e.root.join(format!("src-{name}"));
    std::fs::create_dir_all(&d).unwrap();
    let mut toml = format!(
        "name = \"{name}\"\nversion = \"1.0.0\"\nfeatures = [\"verify\", \"execute\", \"log\"]\n"
    );
    for (f, kind, body, main) in files {
        std::fs::write(d.join(f), body).unwrap();
        toml += &format!(
            "\n[[files]]\nname = \"{f}\"\ntype = \"{kind}\"\nmain = {main}\nchecksum = \"\"\n"
        );
    }
    std::fs::write(d.join("package.toml"), toml).unwrap();
    let out = e.root.join(format!("{name}.tar.gz"));
    packages::pack(&d, &e.admin, &out).unwrap();
    packages::install(
        &e.pool,
        &e.c.layout(),
        Source::Path(&out),
        Some(true),
        "test",
    )
    .await
    .unwrap();
}

fn lines(p: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn spec(plugin: &str, args: &[(&str, &str)]) -> srv::runtime::RunSpec {
    srv::runtime::RunSpec {
        plugin: plugin.into(),
        report_only: true,
        args: args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

#[tokio::test]
async fn powershell_plugin_runs_verify_then_execute_and_logs_v1() {
    if pwsh().is_none() {
        eprintln!("pwsh not found: skipped");
        return;
    }
    let e = env().await;
    install(&e, "disk-free", &[("main.ps1", "powershell", SCRIPT, true)]).await;

    let r = srv::runtime::run_plugin(&e.pool, &e.c, spec("disk-free", &[]), "tester")
        .await
        .unwrap();
    assert_eq!(r.status, "ok", "{r:?}");
    let st: Vec<_> = r.stages.iter().map(|s| (s.stage.as_str(), s.ok)).collect();
    assert_eq!(st, [("verify", true), ("execute", true)]);
    assert_eq!(
        r.results,
        [("DISK-C".into(), "ok".into(), "42.5 GiB free".into())]
    );
    assert!(r.message.contains("report_only=True"), "{}", r.message);
    assert!(r
        .log_path
        .starts_with(e.c.layout().plugin_logs().join("disk-free")));

    // Every line (the server's and the plugin's) is valid itcmdr.log/v1.
    let text = std::fs::read_to_string(&r.log_path).unwrap();
    for l in text.lines() {
        itcmdr_plugin::log::validate(l).unwrap_or_else(|e| panic!("{e}: {l}"));
    }
    let ls = lines(&r.log_path);
    let events: Vec<_> = ls
        .iter()
        .map(|l| l["event"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(events.first().unwrap(), "run-start");
    assert_eq!(events.last().unwrap(), "run-end");
    assert!(events.contains(&"check".into()) && events.contains(&"metrics".into()));
    assert!(ls.iter().all(|l| l["run_id"] == r.run_id.as_str()));
    // Mirrored to the share for the weekly rotation.
    let day = r.log_path.file_name().unwrap();
    assert!(e
        .c
        .logs_dir
        .join("plugin-logs/disk-free")
        .join(day)
        .is_file());

    let runs = srv::runtime::list_runs(&e.pool, Some("disk-free"), 10)
        .await
        .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (runs[0].status.as_str(), runs[0].version.as_str()),
        ("ok", "1.0.0")
    );
    let audit: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action = 'plugin-run'")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!(audit, 1);

    // A failed Verify stops Execute and Log gets the failure.
    let r = srv::runtime::run_plugin(&e.pool, &e.c, spec("disk-free", &[("fail", "1")]), "tester")
        .await
        .unwrap();
    assert_eq!(r.status, "verify-failed", "{r:?}");
    assert!(r.stages.iter().all(|s| s.stage != "execute"), "{r:?}");
    assert!(r.message.contains("share not reachable"), "{}", r.message);
    let ls = lines(&r.log_path);
    let mine: Vec<_> = ls
        .iter()
        .filter(|l| l["run_id"] == r.run_id.as_str())
        .collect();
    assert!(mine.iter().any(|l| l["event"] == "stage-failed"
        && l["level"] == "error"
        && l["message"].as_str().unwrap().contains("verify")));
    assert!(!mine.iter().any(|l| l["event"] == "check"));

    // Metrics come out of the logs; a malformed line is counted, not fatal.
    let root = e.c.layout().plugin_logs();
    let s = srv::plugin_logs::ingest(&e.pool, &root).await.unwrap();
    assert_eq!(s.rejects, 0);
    assert_eq!(s.metrics, 2);
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&r.log_path)
        .unwrap();
    writeln!(f, "{{not json").unwrap();
    writeln!(f, r#"{{"schema":"itcmdr.log/v1","ts":"x"}}"#).unwrap();
    drop(f);
    let s = srv::plugin_logs::ingest(&e.pool, &root).await.unwrap();
    assert_eq!((s.lines, s.rejects, s.metrics), (2, 2, 0));
    assert_eq!(srv::plugin_logs::rejects(&e.pool).await.unwrap()[0].1, 2);
    let pts = srv::plugin_logs::query(
        &e.pool,
        "disk_free",
        &[("drive".into(), "C:".into())],
        7,
        false,
    )
    .await
    .unwrap();
    assert_eq!(pts.len(), 1);
    assert_eq!(pts[0].value, 42.5);
    assert!(srv::plugin_logs::query(
        &e.pool,
        "disk_free",
        &[("drive".into(), "D:".into())],
        7,
        false
    )
    .await
    .unwrap()
    .is_empty());
    // Re-ingest does not double count.
    let s = srv::plugin_logs::ingest(&e.pool, &root).await.unwrap();
    assert_eq!((s.lines, s.metrics), (0, 0));

    // Plugin logs are in backups and come back on restore.
    let b = srv::backup::create(&e.pool, &e.c, "manual", None)
        .await
        .unwrap();
    let want = format!("plugin-logs/disk-free/{}", day.to_string_lossy());
    assert!(b.manifest.files.iter().any(|f| f.path == want), "{want}");
    let fresh = tempfile::tempdir().unwrap();
    let mut fc = e.c.clone();
    fc.data_dir = Some(fresh.path().join("data"));
    fc.logs_dir = fresh.path().join("maintenanceRuns");
    srv::backup::restore(&fc, &b.path, &e.words, false)
        .await
        .unwrap();
    assert!(fc
        .layout()
        .plugin_logs()
        .join("disk-free")
        .join(day)
        .is_file());
}

#[tokio::test]
async fn on_demand_requests_run_plugins_on_the_server() {
    if pwsh().is_none() {
        eprintln!("pwsh not found: skipped");
        return;
    }
    let e = env().await;
    install(&e, "disk-free", &[("main.ps1", "powershell", SCRIPT, true)]).await;
    let id = srv::jobs::queue_run(
        &e.pool,
        &e.c,
        &srv::jobs::RunRequest {
            plugins: vec!["disk-free".into()],
            report_only: true,
            ..Default::default()
        },
        "tester",
    )
    .await
    .unwrap();
    // Plugin-only: no request file for the agents.
    let rd = e.c.logs_dir.join("requests");
    assert!(!rd.is_dir() || std::fs::read_dir(&rd).unwrap().next().is_none());
    let mut status = String::new();
    for _ in 0..240 {
        status = sqlx::query_scalar("SELECT status FROM requests WHERE request_id = ?")
            .bind(&id)
            .fetch_one(&e.pool)
            .await
            .unwrap();
        if status != "queued" && status != "running" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    assert_eq!(status, "done");
    let runs = srv::runtime::list_runs(&e.pool, None, 10).await.unwrap();
    assert_eq!(runs[0].request_id.as_deref(), Some(id.as_str()));
}

#[cfg(unix)]
#[tokio::test]
async fn bin_plugins_and_bad_plugins() {
    let e = env().await;
    // Not installed / not enabled.
    let err = srv::runtime::run_plugin(&e.pool, &e.c, spec("nope", &[]), "t")
        .await
        .unwrap_err();
    assert!(format!("{err:#}").contains("nope"), "{err:#}");

    // A bin plugin that never prints the handshake fails cleanly.
    let junk = "#!/bin/sh\necho hello\nsleep 1\n";
    install(&e, "junk", &[("run.sh", "bin", junk, true)]).await;
    use std::os::unix::fs::PermissionsExt;
    let make_exec = |n: &str, f: &str| {
        let p =
            e.c.layout()
                .service_plugins()
                .join(format!("{n}-1.0.0"))
                .join(f);
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    };
    make_exec("junk", "run.sh");
    let r = srv::runtime::run_plugin(&e.pool, &e.c, spec("junk", &[]), "t").await;
    let msg = match r {
        Ok(r) => {
            assert!(r.status == "failed" || r.status == "error", "{r:?}");
            r.message
        }
        Err(e) => format!("{e:#}"),
    };
    assert!(msg.to_lowercase().contains("handshake"), "{msg}");

    // A bin plugin (a shell wrapper around the PS host) runs like any other.
    if pwsh().is_some() {
        let wrap = format!(
            "#!/bin/sh\nexec \"{}\" --script \"$(dirname \"$0\")/main.ps1\"\n",
            env!("CARGO_BIN_EXE_itcmdr-plugin-ps")
        );
        std::env::set_var("ITCMDR_POWERSHELL", pwsh().unwrap());
        install(
            &e,
            "disk-free",
            &[
                ("run.sh", "bin", &wrap, true),
                ("main.ps1", "powershell", SCRIPT, false),
            ],
        )
        .await;
        make_exec("disk-free", "run.sh");
        let r = srv::runtime::run_plugin(&e.pool, &e.c, spec("disk-free", &[]), "t")
            .await
            .unwrap();
        assert_eq!(r.status, "ok", "{r:?}");
    }
}
