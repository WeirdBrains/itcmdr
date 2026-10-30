//! v0.10.0 app-triggered restore (decision 13, server side): StageRestore (Owner only, words
//! checked before anything changes), the itcmdr-restore helper with its pre-restore safety copy,
//! GetRestoreStatus, and the automatic rollback when the health check fails.
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
    root: &ed25519_dalek::SigningKey,
) -> RestoreStatus {
    for _ in 0..240 {
        let s = cl
            .get_restore_status(signed(root, GS, GetRestoreStatusRequest {}))
            .await
            .unwrap()
            .into_inner()
            .status
            .unwrap();
        if ["done", "rolled-back", "failed"].contains(&s.state.as_str()) {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("restore did not finish");
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
    // The helper is a separate process: it finds this config like the real one does.
    let cfgp = tmp.path().join("server.json");
    std::fs::write(&cfgp, serde_json::to_vec(&c).unwrap()).unwrap();
    std::env::set_var("ITCMDR_SERVER_CONFIG", &cfgp);
    std::env::set_var("ITCMDR_RESTORE_BIN", env!("CARGO_BIN_EXE_itcmdr-restore"));
    std::env::remove_var(srv::restore::ENV_FORCE_HEALTH_FAIL);

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
    let root = srv::auth::load_root_key(&c.layout()).unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(srv::run(c.clone(), async move {
        let _ = rx.await;
    }));
    let mut cl = None;
    for _ in 0..100 {
        if let Ok(x) = ItcmdrClient::connect(format!("http://127.0.0.1:{port}")).await {
            cl = Some(x);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let mut cl = cl.expect("server up");
    let req = |w: &str| StageRestoreRequest {
        backup: b.name.clone(),
        words: w.into(),
    };

    // Owner only; nothing ran yet.
    assert_eq!(
        cl.stage_restore(signed(&admin, SR, req(&words)))
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );
    assert!(cl
        .get_restore_status(signed(&root, GS, GetRestoreStatusRequest {}))
        .await
        .unwrap()
        .into_inner()
        .status
        .is_none());
    // Wrong words or a bad name: refused before anything changes.
    let wrong = srv::keys::generate_words();
    let e = cl
        .stage_restore(signed(&root, SR, req(&wrong)))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::FailedPrecondition, "{e:?}");
    let e = cl
        .stage_restore(signed(
            &root,
            SR,
            StageRestoreRequest {
                backup: "../server.json".into(),
                words: words.clone(),
            },
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::FailedPrecondition);
    assert_eq!(limit(&c).await, 7);

    // Restore: the helper restores and the health check passes.
    let st = cl
        .stage_restore(signed(&root, SR, req(&words)))
        .await
        .unwrap()
        .into_inner()
        .status
        .unwrap();
    assert_eq!(st.state, "staged");
    let st = wait_terminal(&mut cl, &root).await;
    assert_eq!(st.state, "done", "{st:?}");
    assert!(!st.safety_copy.is_empty());
    assert_eq!(limit(&c).await, 5, "restored");

    // Forced health failure: back to the pre-restore copy.
    set_limit(&c, "9").await;
    std::env::set_var(srv::restore::ENV_FORCE_HEALTH_FAIL, "1");
    cl.stage_restore(signed(&root, SR, req(&words)))
        .await
        .unwrap();
    let st = wait_terminal(&mut cl, &root).await;
    std::env::remove_var(srv::restore::ENV_FORCE_HEALTH_FAIL);
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
    let _ = tx.send(());
}
