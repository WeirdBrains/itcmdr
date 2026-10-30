//! v0.7.0: master init, certified keys, db.key escrow/recovery, signed requests and roles over gRPC.
use ::itcmdr_server as srv;
use prost::Message;
use srv::auth::{NewKey, Role};
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::{Code, Request};

const TOKEN: &str = "legacy-token";

fn cfg(root: &std::path::Path, mode: &str) -> ServerConfig {
    ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        admin_token_hash: Some(srv::api::sha256_hex(TOKEN)),
        auth_mode: mode.into(),
        ..ServerConfig::default()
    }
}

fn signed<T: Message>(key: &ed25519_dalek::SigningKey, method: &str, msg: T) -> Request<T> {
    let md = srv::auth::sign_request(key, method, &msg.encode_to_vec());
    let mut r = Request::new(msg);
    for (k, v) in md {
        r.metadata_mut().insert(k, v.parse().unwrap());
    }
    r
}

#[tokio::test]
async fn init_escrow_and_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path(), "both");
    let pool = srv::db::init_db(&c).await.unwrap();
    sqlx::query("INSERT INTO runs(run_id,computer,ts_ms) VALUES ('keep','WS-01',1)")
        .execute(&pool)
        .await
        .unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    assert_eq!(out.words.split(' ').count(), 24);
    assert!(
        srv::auth::init(&pool, &c.layout()).await.is_err(),
        "init only once"
    );

    // Only public material is stored.
    let meta: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM server_meta")
        .fetch_all(&pool)
        .await
        .unwrap();
    let master = srv::keys::Master::from_words(&out.words).unwrap();
    for (k, v) in &meta {
        assert!(
            !v.contains(&hex::encode(master.signing.to_bytes())),
            "{k} leaks the master private key"
        );
    }
    assert_eq!(
        srv::keys::fingerprint(&master.public().ed25519),
        out.master_fingerprint
    );
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    assert_eq!(
        srv::keys::fingerprint(&root.verifying_key()),
        out.root_fingerprint
    );
    let role: String = sqlx::query_scalar(
        "SELECT r.name FROM clients c JOIN roles r ON r.id=c.role_id WHERE c.name='root'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(role, "owner");
    let pk: String = sqlx::query_scalar("SELECT public_key FROM clients WHERE name='root'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        pk,
        hex::encode(root.verifying_key().as_bytes()),
        "DB holds the public key"
    );
    pool.close().await;

    // Lose db.key (e.g. a replacement machine): the words bring it back from the escrow.
    let keyf = c.layout().keys().join("db.key");
    std::fs::remove_file(&keyf).unwrap();
    assert!(
        srv::auth::recover_db_key(&c.layout(), &srv::keys::generate_words(), false).is_err(),
        "other words fail"
    );
    srv::auth::recover_db_key(&c.layout(), &out.words, false).unwrap();
    assert!(
        srv::auth::recover_db_key(&c.layout(), &out.words, false).is_err(),
        "won't overwrite without force"
    );
    srv::auth::recover_db_key(&c.layout(), &out.words, true).unwrap();
    let pool = srv::db::init_db(&c).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM runs WHERE run_id='keep'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "data readable with the recovered key");
}

#[tokio::test]
async fn registration_requires_a_matching_master_certificate() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path(), "both");
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let master = srv::keys::Master::from_words(&out.words).unwrap();
    let k = srv::keys::new_keypair();
    let pk = hex::encode(k.verifying_key().as_bytes());
    let good = srv::keys::certify(
        &master.signing,
        "user",
        "Front Desk",
        "user",
        &k.verifying_key(),
    );
    let nk = |cert| NewKey {
        kind: "user",
        name: "Front Desk",
        role: Role::User,
        email: Some("desk@example.local"),
        phone: None,
        public_key_hex: &pk,
        certificate: cert,
    };

    let foreign = srv::keys::Master::from_words(&srv::keys::generate_words()).unwrap();
    let bad = srv::keys::certify(
        &foreign.signing,
        "user",
        "Front Desk",
        "user",
        &k.verifying_key(),
    );
    assert!(
        srv::auth::register(&pool, nk(&bad)).await.is_err(),
        "foreign master"
    );
    let wrong_role = srv::keys::certify(
        &master.signing,
        "user",
        "Front Desk",
        "admin",
        &k.verifying_key(),
    );
    assert!(
        srv::auth::register(&pool, nk(&wrong_role)).await.is_err(),
        "role mismatch"
    );
    let id = srv::auth::register(&pool, nk(&good)).await.unwrap();
    assert_eq!(uuid::Uuid::parse_str(&id).unwrap().get_version_num(), 4);
    assert!(
        srv::auth::register(&pool, nk(&good)).await.is_err(),
        "same key twice"
    );
    assert!(srv::auth::revoke(&pool, &good.fingerprint).await.unwrap());
    assert!(!srv::auth::revoke(&pool, &good.fingerprint).await.unwrap());
}

async fn start(c: ServerConfig) -> (ItcmdrClient<Channel>, tokio::sync::oneshot::Sender<()>) {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut c = c;
    c.bind_addr = format!("127.0.0.1:{port}");
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(srv::run(c, async move {
        let _ = rx.await;
    }));
    for _ in 0..100 {
        if let Ok(cl) = ItcmdrClient::connect(format!("http://127.0.0.1:{port}")).await {
            return (cl, tx);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not start");
}

#[tokio::test]
async fn signed_requests_and_roles_over_grpc() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path(), "signed");
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let master = srv::keys::Master::from_words(&out.words).unwrap();
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let user = srv::keys::new_keypair();
    let ucert = srv::keys::certify(
        &master.signing,
        "user",
        "Front Desk",
        "user",
        &user.verifying_key(),
    );
    let upk = hex::encode(user.verifying_key().as_bytes());
    srv::auth::register(
        &pool,
        NewKey {
            kind: "user",
            name: "Front Desk",
            role: Role::User,
            email: None,
            phone: None,
            public_key_hex: &upk,
            certificate: &ucert,
        },
    )
    .await
    .unwrap();
    pool.close().await;
    let (mut cl, stop) = start(c.clone()).await;
    const LM: &str = "/itcmdr.Itcmdr/ListMachines";
    const TR: &str = "/itcmdr.Itcmdr/TriggerRuns";
    const SE: &str = "/itcmdr.Itcmdr/SaveException";

    // Signed mode: unsigned reads and the legacy token are refused.
    assert_eq!(
        cl.list_machines(ListMachinesRequest {})
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );
    let mut tok = Request::new(TriggerRunsRequest::default());
    tok.metadata_mut()
        .insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    assert_eq!(
        cl.trigger_runs(tok).await.unwrap_err().code(),
        Code::Unauthenticated,
        "token retired in signed mode"
    );

    cl.list_machines(signed(&root, LM, ListMachinesRequest {}))
        .await
        .unwrap();
    cl.list_machines(signed(&user, LM, ListMachinesRequest {}))
        .await
        .unwrap();

    // Roles: a user may trigger runs but not change exceptions; the owner may.
    let trig = TriggerRunsRequest {
        machines: vec!["WS-01".into()],
        report_only: true,
        ..Default::default()
    };
    cl.trigger_runs(signed(&user, TR, trig.clone()))
        .await
        .unwrap();
    let rule = ExceptionRule {
        check_id: "NET-DNS".into(),
        action: "ignore".into(),
        reason: "t".into(),
        scope: Some(ExceptionScope::default()),
        ..Default::default()
    };
    let e = cl
        .save_exception(signed(
            &user,
            SE,
            SaveExceptionRequest {
                rule: Some(rule.clone()),
            },
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::PermissionDenied);
    cl.save_exception(signed(&root, SE, SaveExceptionRequest { rule: Some(rule) }))
        .await
        .unwrap();
    let audit = cl
        .list_audit(signed(
            &root,
            "/itcmdr.Itcmdr/ListAudit",
            ListAuditRequest { limit: 10 },
        ))
        .await
        .unwrap()
        .into_inner()
        .entries;
    assert!(audit
        .iter()
        .any(|a| a.action == "trigger-runs" && a.actor.starts_with("Front Desk (")));
    assert_eq!(
        cl.list_audit(signed(
            &user,
            "/itcmdr.Itcmdr/ListAudit",
            ListAuditRequest { limit: 10 }
        ))
        .await
        .unwrap_err()
        .code(),
        Code::PermissionDenied
    );

    // Replay: the exact same signed request twice.
    let once = signed(&user, TR, trig.clone());
    let md = once.metadata().clone();
    cl.trigger_runs(once).await.unwrap();
    let mut again = Request::new(trig.clone());
    *again.metadata_mut() = md;
    assert!(cl
        .trigger_runs(again)
        .await
        .unwrap_err()
        .message()
        .contains("replayed"));

    // Tampered body: signature made for a different message.
    let mut t = signed(&user, TR, trig.clone());
    t.get_mut().report_only = false;
    assert!(cl
        .trigger_runs(t)
        .await
        .unwrap_err()
        .message()
        .contains("bad signature"));
    // Signed for another method.
    let wrong_method = signed(&user, LM, trig.clone());
    assert_eq!(
        cl.trigger_runs(wrong_method).await.unwrap_err().code(),
        Code::Unauthenticated
    );
    // Stale timestamp.
    let mut old = signed(&user, TR, trig.clone());
    let ts = (chrono::Utc::now().timestamp_millis() - 10 * 60 * 1000).to_string();
    old.metadata_mut()
        .insert("x-itcmdr-ts", ts.parse().unwrap());
    assert!(cl
        .trigger_runs(old)
        .await
        .unwrap_err()
        .message()
        .contains("window"));
    // Unknown key.
    let stranger = srv::keys::new_keypair();
    assert!(cl
        .trigger_runs(signed(&stranger, TR, trig.clone()))
        .await
        .unwrap_err()
        .message()
        .contains("unknown key"));

    // Revocation takes effect immediately.
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::auth::revoke(&pool, &ucert.fingerprint).await.unwrap();
    assert!(cl
        .trigger_runs(signed(&user, TR, trig))
        .await
        .unwrap_err()
        .message()
        .contains("revoked"));
    stop.send(()).unwrap();
}

#[tokio::test]
async fn both_mode_keeps_the_token_working_during_migration() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path(), "both");
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::auth::init(&pool, &c.layout()).await.unwrap();
    pool.close().await;
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let (mut cl, stop) = start(c).await;
    cl.list_machines(ListMachinesRequest {}).await.unwrap();
    let mut tok = Request::new(TriggerRunsRequest::default());
    tok.metadata_mut()
        .insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    cl.trigger_runs(tok).await.unwrap();
    cl.trigger_runs(signed(
        &root,
        "/itcmdr.Itcmdr/TriggerRuns",
        TriggerRunsRequest::default(),
    ))
    .await
    .unwrap();
    // A bad signature is not silently downgraded to "no auth".
    let mut bad = signed(&root, "/itcmdr.Itcmdr/ListMachines", ListMachinesRequest {});
    bad.metadata_mut()
        .insert("x-itcmdr-sig", "00".repeat(64).parse().unwrap());
    assert_eq!(
        cl.list_machines(bad).await.unwrap_err().code(),
        Code::Unauthenticated
    );
    stop.send(()).unwrap();
}
