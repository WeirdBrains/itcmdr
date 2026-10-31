//! v0.12.0 app support: the auth-mode migration (secure default, existing installs keep "both"),
//! WhoAmI, key registration/revocation over gRPC (signed only), ListPlugins and PreviewSchedule.
use ::itcmdr_server as srv;
use prost::Message;
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::time::Duration;
use tonic::{Code, Request};

const TOKEN: &str = "legacy-token";

fn signed<T: Message>(key: &ed25519_dalek::SigningKey, method: &str, msg: T) -> Request<T> {
    let md = srv::auth::sign_request(key, method, &msg.encode_to_vec());
    let mut r = Request::new(msg);
    for (k, v) in md {
        r.metadata_mut().insert(k, v.parse().unwrap());
    }
    r
}

fn token<T>(msg: T) -> Request<T> {
    let mut r = Request::new(msg);
    r.metadata_mut()
        .insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    r
}

fn write_cfg(root: &std::path::Path, mode: Option<&str>) -> std::path::PathBuf {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut v = serde_json::json!({
        "bind_addr": format!("127.0.0.1:{port}"),
        "logs_dir": root.join("maintenanceRuns"),
        "data_dir": root.join("data"),
        "admin_token_hash": srv::api::sha256_hex(TOKEN),
        "backup": {"nightly": false},
    });
    if let Some(m) = mode {
        v["auth_mode"] = serde_json::json!(m);
    }
    let p = root.join("server.json");
    std::fs::write(&p, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    p
}

async fn settled(path: &std::path::Path) -> ServerConfig {
    let mut c = ServerConfig::from_path(path.to_path_buf()).unwrap();
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::auth::settle_auth_mode(&pool, &mut c).await.unwrap();
    pool.close().await;
    c
}

#[tokio::test]
async fn auth_mode_default_is_signed_but_existing_installs_keep_both() {
    // New install, no auth_mode in server.json: signed, and it stays signed on later starts.
    let t = tempfile::tempdir().unwrap();
    let p = write_cfg(t.path(), None);
    let c = settled(&p).await;
    assert_eq!(
        (c.auth_mode.as_str(), c.auth_mode_source.as_str()),
        ("signed", "new-install")
    );
    let c = settled(&p).await;
    assert_eq!(c.auth_mode, "signed", "second start of a new install");

    // An install from before v0.12.0 (database with tables, no marker): keeps both.
    let t = tempfile::tempdir().unwrap();
    let p = write_cfg(t.path(), None);
    let c = ServerConfig::from_path(p.clone()).unwrap();
    let pool = srv::db::init_db(&c).await.unwrap();
    sqlx::query("DELETE FROM server_meta WHERE key = 'auth_mode_default'")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let c = settled(&p).await;
    assert_eq!(
        (c.auth_mode.as_str(), c.auth_mode_source.as_str()),
        ("both", "kept-both")
    );

    // server.json always wins.
    let p = write_cfg(t.path(), Some("signed"));
    assert_eq!(settled(&p).await.auth_mode, "signed");
    let p = write_cfg(t.path(), Some("token"));
    assert_eq!(settled(&p).await.auth_mode, "token");

    // Unknown or unsettled values fail secure.
    assert_eq!(
        srv::api::AuthMode::from_config(""),
        srv::api::AuthMode::Signed
    );
    assert_eq!(
        srv::api::AuthMode::from_config("bogus"),
        srv::api::AuthMode::Signed
    );
    assert_eq!(
        srv::api::AuthMode::from_config("both"),
        srv::api::AuthMode::Both
    );
}

#[test]
fn cli_auth_set_mode_refuses_lockout_and_writes_server_json() {
    let t = tempfile::tempdir().unwrap();
    let p = write_cfg(t.path(), None);
    let cli = env!("CARGO_BIN_EXE_itcmdr-cli");
    let run = |args: &[&str]| {
        std::process::Command::new(cli)
            .args(args)
            .env("ITCMDR_SERVER_CONFIG", &p)
            .output()
            .unwrap()
    };
    let o = run(&["auth", "status"]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{o:?}");
    assert!(out.contains("auth_mode: signed"), "{out}");
    let o = run(&["auth", "set-mode", "signed"]);
    assert!(!o.status.success(), "no app key yet: refused");
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("locked out"),
        "{o:?}"
    );
    let o = run(&["auth", "set-mode", "both"]);
    assert!(o.status.success(), "{o:?}");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    assert_eq!(v["auth_mode"], "both");
    assert_eq!(
        v["admin_token_hash"],
        srv::api::sha256_hex(TOKEN),
        "other keys kept"
    );
    let o = run(&["auth", "status"]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("set in server.json"));
}

#[tokio::test]
async fn app_rpcs_whoami_keys_plugins_preview() {
    let t = tempfile::tempdir().unwrap();
    let p = write_cfg(t.path(), Some("both"));
    let c = ServerConfig::from_path(p).unwrap();
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    pool.close().await;
    let m = srv::keys::Master::from_words(&out.words).unwrap();
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let port = c.bind_addr.rsplit(':').next().unwrap().to_string();
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

    // WhoAmI: signed and token.
    let w = cl
        .who_am_i(signed(&root, "/itcmdr.Itcmdr/WhoAmI", WhoAmIRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        (w.name.as_str(), w.role.as_str(), w.signed),
        ("root", "owner", true)
    );
    assert_eq!(w.auth_mode, "both");
    let w = cl
        .who_am_i(token(WhoAmIRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!((w.role.as_str(), w.signed), ("admin", false));

    // The app made its own key; the owner certified it offline (keys certify) and registers it.
    let app = srv::keys::new_keypair();
    let cert = srv::keys::certify(
        &m.signing,
        "user",
        "Front desk",
        "admin",
        &app.verifying_key(),
    );
    let bundle = serde_json::json!({"kind": "user", "name": "Front desk", "role": "admin",
        "public_key": hex::encode(app.verifying_key().as_bytes()), "certificate": cert});
    let req = || RegisterKeyRequest {
        bundle_json: bundle.to_string(),
        email: "desk@example.local".into(),
        ..Default::default()
    };
    assert_eq!(
        cl.register_key(token(req())).await.unwrap_err().code(),
        Code::Unauthenticated,
        "the shared token cannot register keys"
    );
    let k = cl
        .register_key(signed(&root, "/itcmdr.Itcmdr/RegisterKey", req()))
        .await
        .unwrap()
        .into_inner()
        .key
        .unwrap();
    assert_eq!((k.name.as_str(), k.role.as_str()), ("Front desk", "admin"));
    // A bundle with a forged certificate is refused.
    let other = srv::keys::new_keypair();
    let mut forged = bundle.clone();
    forged["public_key"] = serde_json::json!(hex::encode(other.verifying_key().as_bytes()));
    let e = cl
        .register_key(signed(
            &root,
            "/itcmdr.Itcmdr/RegisterKey",
            RegisterKeyRequest {
                bundle_json: forged.to_string(),
                ..Default::default()
            },
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::InvalidArgument);
    // Admin: can list, cannot register or revoke.
    let keys = cl
        .list_keys(signed(&app, "/itcmdr.Itcmdr/ListKeys", ListKeysRequest {}))
        .await
        .unwrap()
        .into_inner()
        .keys;
    assert!(keys.iter().any(|k| k.name == "Front desk"));
    assert_eq!(
        cl.revoke_key(signed(
            &app,
            "/itcmdr.Itcmdr/RevokeKey",
            RevokeKeyRequest {
                fingerprint: k.fingerprint.clone()
            }
        ))
        .await
        .unwrap_err()
        .code(),
        Code::PermissionDenied
    );
    assert_eq!(
        cl.list_keys(token(ListKeysRequest {}))
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );

    // ListPlugins (empty here) and PreviewSchedule.
    let pl = cl
        .list_plugins(signed(
            &app,
            "/itcmdr.Itcmdr/ListPlugins",
            ListPluginsRequest {},
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(pl.plugins.is_empty());
    let prev = |when: &str| SaveScheduleRequest {
        schedule: Some(Schedule {
            name: "x".into(),
            all_machines: true,
            when: when.into(),
            timezone: "UTC".into(),
            guard: "none".into(),
            missed: "skip".into(),
            ..Default::default()
        }),
    };
    let r = cl
        .preview_schedule(signed(
            &app,
            "/itcmdr.Itcmdr/PreviewSchedule",
            prev("daily 03:15"),
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(r.error.is_empty(), "{r:?}");
    assert_eq!(r.next_runs.len(), 5);
    assert!(r.next_runs.windows(2).all(|w| w[1] - w[0] == 86_400_000));
    let r = cl
        .preview_schedule(signed(
            &app,
            "/itcmdr.Itcmdr/PreviewSchedule",
            prev("someday"),
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(!r.error.is_empty() && r.next_runs.is_empty());

    // Owner revokes the app key; it stops working at once. Self-revoke is refused.
    assert_eq!(
        cl.revoke_key(signed(
            &root,
            "/itcmdr.Itcmdr/RevokeKey",
            RevokeKeyRequest {
                fingerprint: srv::keys::fingerprint(&root.verifying_key())
            }
        ))
        .await
        .unwrap_err()
        .code(),
        Code::FailedPrecondition
    );
    assert!(
        cl.revoke_key(signed(
            &root,
            "/itcmdr.Itcmdr/RevokeKey",
            RevokeKeyRequest {
                fingerprint: k.fingerprint.clone()
            }
        ))
        .await
        .unwrap()
        .into_inner()
        .revoked
    );
    assert_eq!(
        cl.who_am_i(signed(&app, "/itcmdr.Itcmdr/WhoAmI", WhoAmIRequest {}))
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );
    let pool = srv::db::init_db(&c).await.unwrap();
    let acts: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_log WHERE action LIKE 'key-%'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(acts.contains(&"key-register".into()) && acts.contains(&"key-revoke".into()));
    let _ = tx.send(());
}
