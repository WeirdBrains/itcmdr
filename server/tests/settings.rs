//! v0.9.1 settings everywhere (decision 15): registry defaults, RPC roles + audit, overrides
//! that change runtime behaviour, and the agent-settings.json export.
use ::itcmdr_server as srv;
use prost::Message;
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::path::Path;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::{Code, Request};

fn cfg(root: &Path) -> ServerConfig {
    let mut c = ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        ..ServerConfig::default()
    };
    c.backup.nightly = false;
    c
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

fn signed<T: Message>(key: &ed25519_dalek::SigningKey, method: &str, msg: T) -> Request<T> {
    let md = srv::auth::sign_request(key, method, &msg.encode_to_vec());
    let mut r = Request::new(msg);
    for (k, v) in md {
        r.metadata_mut().insert(k, v.parse().unwrap());
    }
    r
}

const LS: &str = "/itcmdr.Itcmdr/ListSettings";
const UPD: &str = "/itcmdr.Itcmdr/UpdateSetting";
const SL: &str = "/itcmdr.Itcmdr/SearchLogs";

fn upd(key: &str, value: &str, machine: &str) -> UpdateSettingRequest {
    UpdateSettingRequest {
        key: key.into(),
        value: value.into(),
        machine: machine.into(),
        unset: false,
    }
}

#[tokio::test]
async fn settings_rpcs_are_admin_only_validated_audited_and_applied() {
    let a = tempfile::tempdir().unwrap();
    let c = cfg(a.path());
    let lines: String = (0..5)
        .map(|i| format!("{{\"computer\":\"WS-01\",\"status\":\"ok\",\"n\":{i}}}\n"))
        .collect();
    std::fs::create_dir_all(&c.logs_dir).unwrap();
    std::fs::write(c.logs_dir.join("WS-01.jsonl"), lines).unwrap();
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let master = srv::keys::Master::from_words(&out.words).unwrap();
    let user = srv::keys::new_keypair();
    let cert = srv::keys::certify(
        &master.signing,
        "user",
        "Front Desk",
        "user",
        &user.verifying_key(),
    );
    let upk = hex::encode(user.verifying_key().as_bytes());
    srv::auth::register(
        &pool,
        srv::auth::NewKey {
            kind: "user",
            name: "Front Desk",
            role: srv::auth::Role::User,
            email: None,
            phone: None,
            public_key_hex: &upk,
            certificate: &cert,
        },
    )
    .await
    .unwrap();
    pool.close().await;
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let (mut cl, stop) = start(c.clone()).await;

    // defaults = today's behaviour, every key listed
    let l = cl
        .list_settings(signed(&root, LS, ListSettingsRequest::default()))
        .await
        .unwrap()
        .into_inner();
    let get = |l: &ListSettingsResponse, k: &str| {
        l.settings.iter().find(|s| s.key == k).cloned().unwrap()
    };
    assert_eq!(l.settings.len(), srv::settings::registry().len());
    for (k, v) in [
        ("search.default_limit", "100"),
        ("agent.quietMinutes", "10"),
        ("plugins.enable_on_install", "true"),
        ("server.ingest_interval_secs", "30"),
        ("paths.localLogs", "C:\\maintenanceRuns"),
    ] {
        let s = get(&l, k);
        assert_eq!((s.value.as_str(), s.source.as_str()), (v, "default"), "{k}");
    }
    assert_eq!(get(&l, "agent.quietMinutes").scope, "agent");
    assert!(get(&l, "server.bind_addr").read_only);

    // roles
    for code in [
        cl.list_settings(signed(&user, LS, ListSettingsRequest::default()))
            .await
            .unwrap_err()
            .code(),
        cl.update_setting(signed(&user, UPD, upd("search.default_limit", "2", "")))
            .await
            .unwrap_err()
            .code(),
        cl.list_settings(ListSettingsRequest::default())
            .await
            .unwrap_err()
            .code(),
    ] {
        assert!(
            matches!(
                code,
                Code::PermissionDenied | Code::Unauthenticated | Code::FailedPrecondition
            ),
            "{code:?}"
        );
    }
    // validation
    for (k, v, m) in [
        ("search.default_limit", "0", ""),
        ("search.default_limit", "abc", ""),
        ("search.default_limit", "5000", ""), // above search.max_limit
        ("agent.nightlyTime", "25:00", ""),
        ("plugins.enable_on_install", "maybe", ""),
        ("server.bind_addr", "198.51.100.1:1", ""),
        ("search.default_limit", "5", "WS-01"), // machine values are agent-only
        ("no.such.key", "1", ""),
    ] {
        let e = cl
            .update_setting(signed(&root, UPD, upd(k, v, m)))
            .await
            .unwrap_err();
        assert_eq!(e.code(), Code::InvalidArgument, "{k}={v}");
    }

    // override applies at runtime: search page size
    let page = |cl: &mut ItcmdrClient<Channel>| {
        let mut cl = cl.clone();
        let root = root.clone();
        async move {
            cl.search_logs(signed(&root, SL, SearchLogsRequest::default()))
                .await
                .unwrap()
                .into_inner()
        }
    };
    assert_eq!(page(&mut cl).await.lines.len(), 5);
    let r = cl
        .update_setting(signed(&root, UPD, upd("search.default_limit", "2", "")))
        .await
        .unwrap()
        .into_inner()
        .setting
        .unwrap();
    assert_eq!((r.value.as_str(), r.source.as_str()), ("2", "db"));
    let p = page(&mut cl).await;
    assert_eq!((p.lines.len(), p.total), (2, 5));

    // agent settings: global + per machine, exported for the PCs
    cl.update_setting(signed(&root, UPD, upd("agent.quietMinutes", "15", "")))
        .await
        .unwrap();
    cl.update_setting(signed(&root, UPD, upd("agent.quietMinutes", "30", "WS-01")))
        .await
        .unwrap();
    let m = cl
        .list_settings(signed(
            &root,
            LS,
            ListSettingsRequest {
                machine: "WS-01".into(),
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(get(&m, "agent.quietMinutes").value, "30");
    assert_eq!(get(&m, "agent.quietMinutes").source, "db:WS-01");
    assert_eq!(m.machines_with_overrides, vec!["WS-01".to_string()]);
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(c.logs_dir.join("agent-settings.json")).unwrap())
            .unwrap();
    assert_eq!(doc["schema"], "itcmdr-agent-settings/v1");
    assert_eq!(doc["global"]["agent.quietMinutes"], 15);
    assert_eq!(doc["machines"]["WS-01"]["agent.quietMinutes"], 30);
    assert!(
        doc["global"].get("search.default_limit").is_none(),
        "server keys stay on the server"
    );

    // unset
    cl.update_setting(signed(
        &root,
        UPD,
        UpdateSettingRequest {
            unset: true,
            ..upd("search.default_limit", "", "")
        },
    ))
    .await
    .unwrap();
    assert_eq!(page(&mut cl).await.lines.len(), 5);

    let _ = stop.send(());
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pool = srv::db::init_db(&c).await.unwrap();
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='settings-update'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n, 4, "only successful updates are logged");
}

#[tokio::test]
async fn metric_thresholds_follow_settings() {
    let a = tempfile::tempdir().unwrap();
    let c = cfg(a.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    for i in 0..2 {
        sqlx::query(
            "INSERT INTO incidents(computer, check_id, opened_ts_ms) VALUES ('WS-01','NET-DNS',?)",
        )
        .bind(now - 1000 - i)
        .execute(&pool)
        .await
        .unwrap();
    }
    let has = |pts: &[srv::generated::itcmdr::MetricPoint]| {
        pts.iter()
            .any(|p| p.key.starts_with("repeat_offender:WS-01"))
    };
    let pts = srv::metrics::get_metrics(&pool, 7, Rollup::Daily)
        .await
        .unwrap();
    assert!(!has(&pts), "2 incidents < default 3");
    srv::settings::set(&pool, "metrics.repeat_min", "2", None)
        .await
        .unwrap();
    let pts = srv::metrics::get_metrics(&pool, 7, Rollup::Daily)
        .await
        .unwrap();
    assert!(has(&pts), "threshold lowered to 2");
}
