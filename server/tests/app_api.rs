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

fn plugin_source(root: &std::path::Path, name: &str, version: &str) -> std::path::PathBuf {
    let d = root.join(format!("src-{name}-{version}"));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("run.ps1"), "Write-Output 'hi'\n").unwrap();
    std::fs::write(
        d.join("package.toml"),
        format!(
            "name = \"{name}\"\nversion = \"{version}\"\nfeatures = [\"verify\", \"execute\", \"log\"]\n\n\
             [[files]]\nname = \"run.ps1\"\ntype = \"powershell\"\nmain = true\nchecksum = \"\"\n"
        ),
    )
    .unwrap();
    d
}

/// Serve `body` once over plain HTTP on loopback (stands in for an HTTPS package host).
fn serve_once(body: Vec<u8>) -> String {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut s, _)) = l.accept() {
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf);
            let _ = write!(
                s,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = s.write_all(&body);
        }
    });
    format!("http://127.0.0.1:{port}/pkg.tar.gz")
}

#[test]
fn package_urls_are_https_only_except_loopback() {
    use srv::packages::url_allowed;
    assert!(url_allowed("https://plugins.example.local/a.tar.gz"));
    assert!(url_allowed("http://127.0.0.1:8080/a.tar.gz"));
    assert!(url_allowed("http://localhost/a"));
    assert!(url_allowed("http://[::1]:9/a"));
    assert!(!url_allowed("http://plugins.example.local/a.tar.gz"));
    assert!(!url_allowed("http://127.0.0.1.example.local/a"));
    assert!(!url_allowed("ftp://x/a"));
    assert!(!url_allowed("file:///etc/passwd"));
}

#[tokio::test]
async fn plugin_rpcs_install_upload_and_url_enable_disable_remove() {
    let t = tempfile::tempdir().unwrap();
    let p = write_cfg(t.path(), Some("both"));
    let c = ServerConfig::from_path(p).unwrap();
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let m = srv::keys::Master::from_words(&out.words).unwrap();
    let mk = |name: &str, role: &str| {
        let k = srv::keys::new_keypair();
        let cert = srv::keys::certify(&m.signing, "user", name, role, &k.verifying_key());
        (k, cert)
    };
    let (admin, acert) = mk("Dr A", "admin");
    let (user, ucert) = mk("Desk", "user");
    for (k, cert, name, role) in [
        (&admin, &acert, "Dr A", srv::auth::Role::Admin),
        (&user, &ucert, "Desk", srv::auth::Role::User),
    ] {
        let pk = hex::encode(k.verifying_key().as_bytes());
        srv::auth::register(
            &pool,
            srv::auth::NewKey {
                kind: "user",
                name,
                role,
                email: None,
                phone: None,
                public_key_hex: &pk,
                certificate: cert,
            },
        )
        .await
        .unwrap();
    }
    pool.close().await;
    let pack = |name: &str, v: &str| {
        let o = t.path().join(format!("{name}-{v}.tar.gz"));
        srv::packages::pack(&plugin_source(t.path(), name, v), &admin, &o).unwrap();
        std::fs::read(o).unwrap()
    };
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
    const IP: &str = "/itcmdr.Itcmdr/InstallPlugin";
    let up = |b: Vec<u8>, disabled: bool| InstallPluginRequest {
        package: b,
        file_name: "disk-free.tar.gz".into(),
        disabled,
        ..Default::default()
    };

    // Admin + signed only.
    assert_eq!(
        cl.install_plugin(token(up(pack("disk-free", "1.0.0"), false)))
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );
    assert_eq!(
        cl.install_plugin(signed(&user, IP, up(pack("disk-free", "1.0.0"), false)))
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );
    // Upload: enabled on install by default.
    let r = cl
        .install_plugin(signed(&admin, IP, up(pack("disk-free", "1.0.0"), false)))
        .await
        .unwrap()
        .into_inner();
    assert!(
        !r.quarantined && r.plugin.as_ref().unwrap().enabled,
        "{r:?}"
    );
    // URL (loopback http stands in for https), installed disabled.
    let url = serve_once(pack("disk-free", "1.1.0"));
    let r = cl
        .install_plugin(signed(
            &admin,
            IP,
            InstallPluginRequest {
                url,
                disabled: true,
                ..Default::default()
            },
        ))
        .await
        .unwrap()
        .into_inner();
    let pl = r.plugin.unwrap();
    assert_eq!((pl.version.as_str(), pl.enabled), ("1.1.0", false));
    // Plain http to a non-loopback host, or both/neither source: refused before any fetch.
    for bad in [
        InstallPluginRequest {
            url: "http://plugins.example.local/x.tar.gz".into(),
            ..Default::default()
        },
        InstallPluginRequest::default(),
        InstallPluginRequest {
            url: "https://plugins.example.local/x.tar.gz".into(),
            package: vec![1],
            ..Default::default()
        },
    ] {
        assert_eq!(
            cl.install_plugin(signed(&admin, IP, bad))
                .await
                .unwrap_err()
                .code(),
            Code::InvalidArgument
        );
    }
    // A broken package is quarantined, and listed.
    let r = cl
        .install_plugin(signed(&admin, IP, up(b"not a package".to_vec(), false)))
        .await
        .unwrap()
        .into_inner();
    assert!(r.quarantined && r.plugin.is_none(), "{r:?}");
    let q = cl
        .list_quarantine(signed(
            &admin,
            "/itcmdr.Itcmdr/ListQuarantine",
            ListQuarantineRequest {},
        ))
        .await
        .unwrap()
        .into_inner()
        .entries;
    assert_eq!(q.len(), 1);
    assert!(q[0].source.starts_with("upload:"), "{q:?}");

    // Enable 1.1.0 (1.0.0 is disabled then), disable, remove.
    let pr = |spec: &str, force: bool| PluginRef {
        spec: spec.into(),
        force,
    };
    cl.enable_plugin(signed(
        &admin,
        "/itcmdr.Itcmdr/EnablePlugin",
        pr("disk-free@1.1.0", false),
    ))
    .await
    .unwrap();
    let list = cl
        .list_plugins(signed(
            &admin,
            "/itcmdr.Itcmdr/ListPlugins",
            ListPluginsRequest {},
        ))
        .await
        .unwrap()
        .into_inner()
        .plugins;
    let en: Vec<_> = list
        .iter()
        .filter(|p| p.enabled)
        .map(|p| p.version.as_str())
        .collect();
    assert_eq!(en, ["1.1.0"]);
    assert!(
        cl.remove_plugin(signed(
            &admin,
            "/itcmdr.Itcmdr/RemovePlugin",
            pr("disk-free@1.1.0", false)
        ))
        .await
        .is_err(),
        "an enabled plugin needs force"
    );
    cl.disable_plugin(signed(
        &admin,
        "/itcmdr.Itcmdr/DisablePlugin",
        pr("disk-free", false),
    ))
    .await
    .unwrap();
    cl.remove_plugin(signed(
        &admin,
        "/itcmdr.Itcmdr/RemovePlugin",
        pr("disk-free@1.1.0", false),
    ))
    .await
    .unwrap();
    assert_eq!(
        cl.enable_plugin(signed(
            &user,
            "/itcmdr.Itcmdr/EnablePlugin",
            pr("disk-free", false)
        ))
        .await
        .unwrap_err()
        .code(),
        Code::PermissionDenied
    );
    let pool = srv::db::init_db(&c).await.unwrap();
    let acts: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_log WHERE action LIKE 'plugin-%'")
            .fetch_all(&pool)
            .await
            .unwrap();
    for a in [
        "plugin-install",
        "plugin-quarantine",
        "plugin-enable",
        "plugin-disable",
        "plugin-remove",
    ] {
        assert!(acts.contains(&a.to_string()), "{a}: {acts:?}");
    }
    let _ = tx.send(());
}
