//! v0.10.0 app-triggered restore (decision 13, server side): StageRestore (Owner only, words
//! checked before anything changes), the itcmdr-restore helper with its pre-restore safety copy,
//! GetRestoreStatus, and the automatic rollback when the health check fails.
#![allow(clippy::result_large_err)]
use ::itcmdr_server as srv;
use prost::Message;
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::time::Duration;
use tonic::{Code, Request};

fn signed<T: Message>(key: &ed25519_dalek::SigningKey, method: &str, msg: T) -> Request<T> {
    let md = srv::auth::sign_request(key, method, &msg.encode_to_vec());
    let mut r = Request::new(msg);
    for (k, v) in md {
        r.metadata_mut().insert(k, v.parse().unwrap());
    }
    r
}

const SR: &str = "/itcmdr.Itcmdr/StageRestore";
const RK: &str = "/itcmdr.Itcmdr/GetRestoreKey";

async fn sealed(
    cl: &mut ItcmdrClient<tonic::transport::Channel>,
    key: &ed25519_dalek::SigningKey,
    backup: &str,
    words: &str,
) -> Result<SealedWords, tonic::Status> {
    let k = cl
        .get_restore_key(signed(key, RK, GetRestoreKeyRequest {}))
        .await?
        .into_inner();
    let p: [u8; 32] = hex::decode(&k.x25519_pub).unwrap().try_into().unwrap();
    let s = srv::restore_seal::seal(&p, &k.key_id, backup, words).unwrap();
    Ok(SealedWords {
        key_id: s.key_id,
        eph: s.eph,
        nonce: s.nonce,
        ct: s.ct,
    })
}

/// The app's path: seal, dry run, seal again, restore.
async fn stage(
    cl: &mut ItcmdrClient<tonic::transport::Channel>,
    key: &ed25519_dalek::SigningKey,
    backup: &str,
    words: &str,
    dry_run: bool,
    dry_run_id: &str,
) -> Result<StageRestoreResponse, tonic::Status> {
    let sw = sealed(cl, key, backup, words).await?;
    Ok(cl
        .stage_restore(signed(
            key,
            SR,
            StageRestoreRequest {
                backup: backup.into(),
                sealed_words: Some(sw),
                dry_run,
                dry_run_id: dry_run_id.into(),
                ..Default::default()
            },
        ))
        .await?
        .into_inner())
}

async fn dry_then_stage(
    cl: &mut ItcmdrClient<tonic::transport::Channel>,
    key: &ed25519_dalek::SigningKey,
    backup: &str,
    words: &str,
) -> RestoreStatus {
    let d = stage(cl, key, backup, words, true, "").await.unwrap();
    assert!(!d.dry_run_id.is_empty() && d.status.is_none(), "{d:?}");
    stage(cl, key, backup, words, false, &d.dry_run_id)
        .await
        .unwrap()
        .status
        .unwrap()
}
const GS: &str = "/itcmdr.Itcmdr/GetRestoreStatus";

async fn limit(c: &ServerConfig) -> i64 {
    let pool = srv::db::init_db(c).await.unwrap();
    let v = srv::settings::get_i64(&pool, None, "search.default_limit").await;
    pool.close().await;
    v
}

async fn set_limit(c: &ServerConfig, v: &str) {
    let pool = srv::db::init_db(c).await.unwrap();
    srv::settings::set(&pool, "search.default_limit", v, None)
        .await
        .unwrap();
    pool.close().await;
}

async fn wait_terminal(
    cl: &mut ItcmdrClient<tonic::transport::Channel>,
    root: &mut ed25519_dalek::SigningKey,
) -> RestoreStatus {
    for _ in 0..240 {
        // The helper restarts the server, so calls fail for a moment.
        let r = cl
            .get_restore_status(signed(root, GS, GetRestoreStatusRequest {}))
            .await;
        let Ok(r) = r else {
            // The restore issues a new root key: pick it up once the old one is refused.
            *root =
                srv::auth::load_root_key(&server_cfg().layout()).unwrap_or_else(|_| root.clone());
            tokio::time::sleep(Duration::from_millis(250)).await;
            if let Ok(x) = ItcmdrClient::connect(server_url()).await {
                *cl = x;
            }
            continue;
        };
        let Some(s) = r.into_inner().status else {
            tokio::time::sleep(Duration::from_millis(250)).await;
            continue;
        };
        if ["done", "rolled-back", "failed"].contains(&s.state.as_str()) {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let c = server_cfg();
    let st = std::fs::read_to_string(c.layout().root.join("restore").join("status.json"))
        .unwrap_or_default();
    panic!("restore did not finish; helper status: {st}");
}

#[tokio::test]
async fn stage_restore_done_then_rolled_back_on_failed_health() {
    let tmp = tempfile::tempdir().unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut c = ServerConfig {
        bind_addr: format!("127.0.0.1:{port}"),
        logs_dir: tmp.path().join("maintenanceRuns"),
        data_dir: Some(tmp.path().join("data")),
        ..ServerConfig::default()
    };
    c.backup.nightly = false;
    // A real server process, stopped and started by the helper like the service: Windows cannot
    // replace the database while an in-process server still holds it open.
    let pidf = tmp.path().join("server.pid");
    let (start, stop) = server_commands(env!("CARGO_BIN_EXE_itcmdr-server"), &pidf);
    c.restore.start_command = start.clone();
    c.restore.stop_command = stop.clone();
    let force = tmp.path().join("force-health-fail");
    // The helper is a separate process: it finds this config like the real one does.
    let cfgp = tmp.path().join("server.json");
    std::fs::write(&cfgp, serde_json::to_vec(&c).unwrap()).unwrap();
    std::env::set_var("ITCMDR_SERVER_CONFIG", &cfgp);
    std::env::set_var("ITCMDR_RESTORE_BIN", env!("CARGO_BIN_EXE_itcmdr-restore"));
    std::env::set_var(srv::restore::ENV_FORCE_HEALTH_FAIL, &force);

    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let words = out.words.to_string();
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
    srv::settings::set(&pool, "search.default_limit", "5", None)
        .await
        .unwrap();
    let b = srv::backup::create(&pool, &c, "manual", None)
        .await
        .unwrap();
    srv::settings::set(&pool, "search.default_limit", "7", None)
        .await
        .unwrap();
    pool.close().await;
    let mut root = srv::auth::load_root_key(&c.layout()).unwrap();

    run(&start);
    let _stop = StopOnDrop(stop.clone());
    let mut cl = None;
    for _ in 0..100 {
        if let Ok(x) = ItcmdrClient::connect(format!("http://127.0.0.1:{port}")).await {
            cl = Some(x);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let mut cl = cl.expect("server up");
    let bn = b.name.clone();

    // Owner only, signed only; nothing ran yet.
    assert_eq!(
        sealed(&mut cl, &admin, &bn, &words)
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );
    let e = cl
        .stage_restore(Request::new(StageRestoreRequest {
            backup: bn.clone(),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::Unauthenticated, "token/no signature: {e:?}");
    assert!(cl
        .get_restore_status(signed(&root, GS, GetRestoreStatusRequest {}))
        .await
        .unwrap()
        .into_inner()
        .status
        .is_none());
    // Plaintext words are refused outright.
    let e = cl
        .stage_restore(signed(
            &root,
            SR,
            StageRestoreRequest {
                backup: bn.clone(),
                words: words.clone(),
                dry_run: true,
                ..Default::default()
            },
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::InvalidArgument, "{e:?}");
    // Wrong words or a bad name: the dry run fails.
    let wrong = srv::keys::generate_words();
    let e = stage(&mut cl, &root, &bn, &wrong, true, "")
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::FailedPrecondition, "{e:?}");
    let e = stage(&mut cl, &root, "../server.json", &words, true, "")
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::FailedPrecondition);
    // No dry run, or a dry run for another backup: refused.
    let e = stage(&mut cl, &root, &bn, &words, false, "")
        .await
        .unwrap_err();
    assert!(e.message().contains("dry run"), "{e:?}");
    // A sealed key is single use and bound to the backup name.
    let sw = sealed(&mut cl, &root, &bn, &words).await.unwrap();
    let mk = |sw: SealedWords| StageRestoreRequest {
        backup: bn.clone(),
        sealed_words: Some(sw),
        dry_run: true,
        ..Default::default()
    };
    cl.stage_restore(signed(&root, SR, mk(sw.clone())))
        .await
        .unwrap();
    let e = cl
        .stage_restore(signed(&root, SR, mk(sw)))
        .await
        .unwrap_err();
    assert!(e.message().contains("already used"), "{e:?}");
    assert_eq!(limit(&c).await, 7);
    {
        let pool = srv::db::init_db(&c).await.unwrap();
        let names: Vec<String> =
            sqlx::query_scalar("SELECT action FROM audit_log WHERE action LIKE 'restore-%'")
                .fetch_all(&pool)
                .await
                .unwrap();
        for a in ["restore-dry-run", "restore-refused"] {
            assert!(names.contains(&a.to_string()), "{a} audited: {names:?}");
        }
        pool.close().await;
    }

    // Restore: dry run, then the helper restores and the health check passes.
    let st = dry_then_stage(&mut cl, &root, &bn, &words).await;
    assert_eq!(st.state, "staged");
    let st = wait_terminal(&mut cl, &mut root).await;
    assert_eq!(st.state, "done", "{st:?}");
    assert!(!st.safety_copy.is_empty());
    assert_eq!(limit(&c).await, 5, "restored");

    // Forced health failure: back to the pre-restore copy.
    set_limit(&c, "9").await;
    std::fs::write(&force, b"1").unwrap();
    dry_then_stage(&mut cl, &root, &bn, &words).await;
    let st = wait_terminal(&mut cl, &mut root).await;
    std::fs::remove_file(&force).unwrap();
    assert_eq!(st.state, "rolled-back", "{st:?}");
    assert!(st.message.contains("health check"), "{st:?}");
    assert_eq!(limit(&c).await, 9, "rolled back to the safety copy");

    let pool = srv::db::init_db(&c).await.unwrap();
    let acts: Vec<String> = sqlx::query_scalar(
        "SELECT detail FROM audit_log WHERE action LIKE 'restore-%' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        acts.iter().all(|d| !d.contains(&words)),
        "words never in the audit log"
    );
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_log WHERE action LIKE 'restore-%' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        names.contains(&"restore-rolled-back".to_string()),
        "{names:?}"
    );

    pool.close().await;
}

fn run(argv: &[String]) {
    let st = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .status()
        .unwrap();
    assert!(st.success(), "{argv:?}");
}

/// Start/stop commands for a background itcmdr-server that records its pid (it finds its config
/// through ITCMDR_SERVER_CONFIG, inherited from this test).
fn server_commands(exe: &str, pidf: &std::path::Path) -> (Vec<String>, Vec<String>) {
    let pid = pidf.display().to_string();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    if cfg!(windows) {
        let ps = |c: String| {
            s(&[
                "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &c,
            ])
        };
        (
            ps(format!(
                "(Start-Process -FilePath '{exe}' -WindowStyle Hidden -PassThru).Id | Set-Content -LiteralPath '{pid}'"
            )),
            ps(format!(
                "Stop-Process -Id (Get-Content -LiteralPath '{pid}') -Force; Start-Sleep -Milliseconds 500"
            )),
        )
    } else {
        (
            s(&[
                "sh",
                "-c",
                &format!("'{exe}' >/dev/null 2>&1 & echo $! > '{pid}'"),
            ]),
            s(&["sh", "-c", &format!("kill $(cat '{pid}'); sleep 0.5")]),
        )
    }
}

fn server_cfg() -> ServerConfig {
    let cfgp = std::env::var("ITCMDR_SERVER_CONFIG").unwrap();
    serde_json::from_slice(&std::fs::read(cfgp).unwrap()).unwrap()
}

fn server_url() -> String {
    format!("http://{}", server_cfg().bind_addr)
}

/// Stops the background server even when an assertion fails.
struct StopOnDrop(Vec<String>);
impl Drop for StopOnDrop {
    fn drop(&mut self) {
        let _ = std::process::Command::new(&self.0[0])
            .args(&self.0[1..])
            .status();
    }
}
