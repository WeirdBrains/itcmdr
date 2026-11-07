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
    install_with(e, name, files, "").await
}

async fn install_with(e: &Env, name: &str, files: &[(&str, &str, &str, bool)], extra: &str) {
    let d = e.root.join(format!("src-{name}"));
    std::fs::create_dir_all(&d).unwrap();
    let mut toml = format!(
        "name = \"{name}\"\nversion = \"1.0.0\"\nfeatures = [\"verify\", \"execute\", \"log\"]\n{extra}"
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

// ---- v0.15.0: the plugin error contract end to end (PS module -> itcmdr-plugin-ps -> server) ----
const ERR_SCRIPT: &str = r#"
Import-Module $env:ITCMDR_PS_MODULE -Force
Invoke-ItcmdrPlugin -Info @{ name = 'err-demo'; version = '1.0.0'; features = @('execute') } `
  -Execute { param($Ctx)
    switch ($Ctx.args.mode) {
      'throw' { throw (New-ItcmdrPluginError -Message 'backup share is gone' -Criticality Critical -Kind 'ShareMissing' -Context @{ share = 'maintenanceRuns' }) }
      'uncaught' { [int]::Parse('nope') | Out-Null }
      'warn' { Write-ItcmdrPluginError -Message 'one disk is 91% full' -Criticality Cautious; @{ ok = $true; message = 'checked' } }
      default { @{ ok = $true; message = 'fine' } }
    }
  }
"#;

fn plugin_errors(p: &Path) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(p).unwrap();
    for l in text.lines() {
        itcmdr_plugin::log::validate(l).unwrap_or_else(|e| panic!("{e}: {l}"));
    }
    lines(p)
        .into_iter()
        .filter(|l| l["event"] == "plugin-error")
        .collect()
}

#[tokio::test]
async fn plugin_errors_reach_the_server_log_with_criticality() {
    if pwsh().is_none() {
        eprintln!("pwsh not found: skipped");
        return;
    }
    let e = env().await;
    install(
        &e,
        "err-demo",
        &[("main.ps1", "powershell", ERR_SCRIPT, true)],
    )
    .await;
    let run = |mode: &'static str| {
        let (pool, c) = (e.pool.clone(), e.c.clone());
        async move {
            srv::runtime::run_plugin(&pool, &c, spec("err-demo", &[("mode", mode)]), "tester")
                .await
                .unwrap()
        }
    };

    let r = run("throw").await;
    assert_eq!(r.status, "failed");
    assert_eq!(r.errors.len(), 1, "{r:?}");
    let pe = &r.errors[0];
    assert_eq!(pe.criticality, itcmdr_common::ErrorCriticality::Critical);
    assert_eq!(pe.kind, "ShareMissing");
    assert_eq!(pe.service_plugin, "err-demo");
    assert_eq!(
        pe.context.get("share").map(String::as_str),
        Some("maintenanceRuns")
    );
    assert!(!pe.backtrace.is_empty());
    let l = plugin_errors(&r.log_path);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0]["level"], "error");
    assert_eq!(l[0]["criticality"], "critical");
    assert_eq!(l[0]["message"], "backup share is gone");
    assert_eq!(l[0]["stage"], "execute");

    let r = run("uncaught").await;
    assert_eq!(r.status, "failed");
    assert_eq!(
        r.errors[0].criticality,
        itcmdr_common::ErrorCriticality::Cautious
    );
    assert!(r.errors[0].kind.contains("Exception"), "{:?}", r.errors[0]);
    let l = plugin_errors(&r.log_path);
    assert_eq!(l.last().unwrap()["level"], "warn");

    let r = run("warn").await;
    assert_eq!(r.status, "ok", "{r:?}");
    assert_eq!(r.errors.len(), 1);
    assert_eq!(r.errors[0].error, "one disk is 91% full");

    let r = run("none").await;
    assert_eq!(r.status, "ok");
    assert!(r.errors.is_empty());
}

#[tokio::test]
async fn default_error_criticality_from_the_package() {
    if pwsh().is_none() {
        eprintln!("pwsh not found: skipped");
        return;
    }
    let e = env().await;
    install_with(
        &e,
        "err-demo",
        &[("main.ps1", "powershell", ERR_SCRIPT, true)],
        "default_error_criticality = \"informational\"\n",
    )
    .await;
    let r = srv::runtime::run_plugin(
        &e.pool,
        &e.c,
        spec("err-demo", &[("mode", "uncaught")]),
        "t",
    )
    .await
    .unwrap();
    assert_eq!(
        r.errors[0].criticality,
        itcmdr_common::ErrorCriticality::Informational
    );
    assert_eq!(plugin_errors(&r.log_path).last().unwrap()["level"], "info");
    // A thrown error keeps its own criticality.
    let r = srv::runtime::run_plugin(&e.pool, &e.c, spec("err-demo", &[("mode", "throw")]), "t")
        .await
        .unwrap();
    assert_eq!(
        r.errors[0].criticality,
        itcmdr_common::ErrorCriticality::Critical
    );
}

#[tokio::test]
async fn unsupported_platform_never_spawns_the_plugin() {
    let e = env().await;
    // A plugin that would touch a marker file if it ever ran.
    let marker = e.root.join("started.txt");
    let script = format!(
        r#"
Import-Module $env:ITCMDR_PS_MODULE -Force
Set-Content -Path '{}' -Value 'started'
Invoke-ItcmdrPlugin -Info @{{ name = 'gate-demo'; version = '1.0.0'; features = @('execute') }} `
  -Execute {{ param($Ctx) @{{ ok = $true; message = 'ran' }} }}
"#,
        marker.display()
    );
    // Declare support for a platform we are not on (macos on Linux/Windows CI, linux on Windows).
    // The server must refuse to start the host at all.
    let want = if cfg!(target_os = "macos") {
        "linux"
    } else {
        "macos"
    };
    let extra = format!("supported_platforms = [\"{}\"]\n", want);
    install_with(
        &e,
        "gate-demo",
        &[("main.ps1", "powershell", &script, true)],
        &extra,
    )
    .await;
    let r = srv::runtime::run_plugin(&e.pool, &e.c, spec("gate-demo", &[]), "tester")
        .await
        .unwrap();
    assert_eq!(r.status, "error", "{r:?}");
    assert!(
        r.message.to_lowercase().contains("unsupported platform"),
        "{}",
        r.message
    );
    assert!(
        !marker.exists(),
        "plugin executable must not be started on an unsupported platform"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn unsupported_platform_blocks_bin_plugin_start() {
    let e = env().await;
    let marker = e.root.join("bin-started.txt");
    let body = format!(
        "#!/bin/sh\necho started > \"{}\"\nexec sleep 1\n",
        marker.display()
    );
    let mut d = e.root.join("bin-src");
    std::fs::create_dir_all(&d).unwrap();
    let toml = {
        let exclude = if cfg!(target_os = "linux") { "macos" } else { "linux" };
        format!(
            "name = \"gate-bin\"\nversion = \"1.0.0\"\nfeatures = [\"execute\"]\nsupported_platforms = [\"{}\"]\n\n[[files]]\nname = \"run.sh\"\ntype = \"bin\"\nmain = true\nchecksum = \"\"\n",
            exclude
        )
    };
    std::fs::write(d.join("run.sh"), &body).unwrap();
    std::fs::write(d.join("package.toml"), toml).unwrap();
    let out = e.root.join("gate-bin.tar.gz");
    packages::pack(&d, &e.admin, &out).unwrap();
    packages::install(&e.pool, &e.c.layout(), Source::Path(&out), Some(true), "test")
        .await
        .unwrap();
    // Make executable bit for bin.
    use std::os::unix::fs::PermissionsExt;
    let p = e
        .c
        .layout()
        .service_plugins()
        .join("gate-bin-1.0.0")
        .join("run.sh");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    let r = srv::runtime::run_plugin(&e.pool, &e.c, spec("gate-bin", &[]), "tester")
        .await
        .unwrap();
    assert_eq!(r.status, "error", "{r:?}");
    assert!(r.message.to_lowercase().contains("unsupported platform"));
    assert!(
        !marker.exists(),
        "bin plugin executable must not be started on an unsupported platform"
    );
}

#[test]
fn bad_default_error_criticality_is_refused() {
    let m = "name = \"x\"\nversion = \"1.0.0\"\ndefault_error_criticality = \"fatal\"\n[[files]]\nname = \"a\"\ntype = \"bin\"\nmain = true\nchecksum = \"\"\n";
    let e = packages::parse_manifest(m.as_bytes()).unwrap_err();
    assert!(e.0.contains("default_error_criticality"), "{}", e.0);
}
