//! v0.16.0 site alerts: dedupe, level escalation, reopen within the window, auto-resolve, silence and its expiry,
//! the check/plugin/backup sources, and the signed RPCs (list with badge counts, ack, resolve, silence, audit).
use ::itcmdr_server as srv;
use prost::Message;
use srv::alerts::{self, Filter, Raise};
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::Request;

fn cfg(root: &std::path::Path) -> ServerConfig {
    ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        auth_mode: "signed".into(),
        ..ServerConfig::default()
    }
}

fn r(key: &str, level: &str) -> Raise {
    Raise {
        key: key.into(),
        level: level.into(),
        source: "check".into(),
        machine: "WS-01".into(),
        title: format!("t {key}"),
        detail: "d".into(),
        run_id: None,
    }
}

fn rec(check: &str, status: &str, attempted: bool, result: &str) -> srv::ingest::AgentRecord {
    srv::ingest::AgentRecord {
        run_id: uuid::Uuid::new_v4().to_string(),
        computer: "WS-02".into(),
        category: "services".into(),
        check_id: check.into(),
        severity: "warning".into(),
        status: status.into(),
        attempted: Some(attempted),
        action: None,
        result: Some(result.into()),
        excepted: Some(false),
        exception_reason: None,
    }
}

#[tokio::test]
async fn dedupe_escalate_reopen_resolve_silence() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();

    let a = alerts::raise(&pool, &r("k1", "informational"))
        .await
        .unwrap();
    let b = alerts::raise(&pool, &r("k1", "critical")).await.unwrap();
    let c2 = alerts::raise(&pool, &r("k1", "cautious")).await.unwrap();
    assert_eq!(a, b);
    assert_eq!(a, c2);
    let x = alerts::get(&pool, &a).await.unwrap().unwrap();
    assert_eq!(
        (x.count, x.level.as_str(), x.state.as_str()),
        (3, "critical", "open")
    );
    assert_eq!(alerts::open_counts(&pool).await.unwrap(), [1, 0, 0]);

    // ack keeps it unresolved and still deduped
    alerts::ack(&pool, &a, "tester").await.unwrap();
    assert_eq!(alerts::raise(&pool, &r("k1", "cautious")).await.unwrap(), a);
    assert_eq!(
        alerts::get(&pool, &a).await.unwrap().unwrap().state,
        "acked"
    );

    // auto-resolve, then raised again within the window: same row reopens
    assert_eq!(alerts::resolve_key(&pool, "k1").await.unwrap(), 1);
    assert_eq!(alerts::open_counts(&pool).await.unwrap(), [0, 0, 0]);
    let again = alerts::raise(&pool, &r("k1", "cautious")).await.unwrap();
    assert_eq!(again, a);
    let x = alerts::get(&pool, &a).await.unwrap().unwrap();
    assert_eq!(
        (x.state.as_str(), x.level.as_str(), x.resolved_by.as_str()),
        ("open", "cautious", "")
    );

    // outside the window (0 minutes): a new row
    srv::settings::set(&pool, "alerts.dedupe_window_mins", "0", None)
        .await
        .unwrap();
    alerts::resolve(&pool, &a, "tester").await.unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    let fresh = alerts::raise(&pool, &r("k1", "cautious")).await.unwrap();
    assert_ne!(fresh, a);

    // silence a machine: open ones become silenced, new ones arrive silenced, badge ignores them
    let until = chrono::Utc::now().timestamp_millis() + 400;
    let (_, n) = alerts::silence(&pool, "WS-01", "", until, "maintenance", "tester")
        .await
        .unwrap();
    assert_eq!(n, 1);
    let s2 = alerts::raise(&pool, &r("k2", "critical")).await.unwrap();
    assert_eq!(
        alerts::get(&pool, &s2).await.unwrap().unwrap().state,
        "silenced"
    );
    assert_eq!(alerts::open_counts(&pool).await.unwrap(), [0, 0, 0]);
    assert!(alerts::silence(&pool, "", "", until, "", "t")
        .await
        .is_err());
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(alerts::expire_silences(&pool).await.unwrap(), 2);
    assert_eq!(alerts::open_counts(&pool).await.unwrap(), [1, 1, 0]);
    let open = alerts::list(&pool, &Filter::default()).await.unwrap();
    assert_eq!(open[0].level, "critical"); // worst first
}

#[tokio::test]
async fn sources_checks_backup_plugin() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();

    // a failed check raises (cautious), ok resolves it, a repair that worked is informational
    srv::db::record_check(&pool, &rec("SVC-Fault", "failed", true, "failed"), 1)
        .await
        .unwrap();
    let open = alerts::list(
        &pool,
        &Filter {
            source: Some("check".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(
        (open[0].level.as_str(), open[0].key.as_str()),
        ("cautious", "check:SVC-Fault:WS-02")
    );
    srv::db::record_check(&pool, &rec("SVC-Fault", "ok", true, "success"), 2)
        .await
        .unwrap();
    let all = alerts::list(
        &pool,
        &Filter {
            state: Some("all".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let k = |key: &str| all.iter().find(|a| a.key == key).cloned().unwrap();
    assert_eq!(k("check:SVC-Fault:WS-02").state, "resolved");
    assert_eq!(k("check:SVC-Fault:WS-02").resolved_by, "auto");
    assert_eq!(k("repair:SVC-Fault:WS-02").level, "informational");
    // an excepted failure raises nothing
    let mut ex = rec("SVC-Other", "failed", false, "");
    ex.excepted = Some(true);
    srv::db::record_check(&pool, &ex, 3).await.unwrap();
    assert!(alerts::list(
        &pool,
        &Filter {
            state: Some("all".into()),
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .iter()
    .all(|a| !a.key.contains("SVC-Other")));

    // backups: failure is critical, success resolves
    alerts::from_backup(&pool, "nightly", Err("disk full".into())).await;
    assert_eq!(alerts::open_counts(&pool).await.unwrap()[0], 1);
    alerts::from_backup(&pool, "nightly", Ok(())).await;
    assert_eq!(alerts::open_counts(&pool).await.unwrap()[0], 0);

    // plugin errors at their criticality; an ok run resolves that plugin's alerts on that machine only
    let mut rep = srv::runtime::RunReport {
        run_id: "r1".into(),
        plugin: "itcmdr-services".into(),
        version: "1".into(),
        status: "ok".into(),
        message: String::new(),
        stages: vec![],
        results: vec![],
        log_path: tmp.path().join("x.log"),
        errors: vec![itcmdr_common::PluginErrorRecord::new(
            "itcmdr-services",
            itcmdr_common::ErrorCriticality::Critical,
            "SVC-Fault: repair failed",
        )],
    };
    alerts::from_plugin_run(&pool, &rep, "WS-03").await;
    alerts::from_plugin_run(&pool, &rep, "WS-04").await;
    assert_eq!(alerts::open_counts(&pool).await.unwrap()[0], 2);
    rep.errors.clear();
    alerts::from_plugin_run(&pool, &rep, "WS-03").await;
    let open = alerts::list(
        &pool,
        &Filter {
            source: Some("plugin".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].machine, "WS-04");
}

fn signed<T: Message>(key: &ed25519_dalek::SigningKey, method: &str, msg: T) -> Request<T> {
    let md = srv::auth::sign_request(key, method, &msg.encode_to_vec());
    let mut r = Request::new(msg);
    for (k, v) in md {
        r.metadata_mut().insert(k, v.parse().unwrap());
    }
    r
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
async fn rpcs_list_ack_resolve_silence_audited() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::auth::init(&pool, &c.layout()).await.unwrap();
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let id = alerts::raise(&pool, &r("rpc-1", "critical")).await.unwrap();
    alerts::raise(&pool, &r("rpc-2", "informational"))
        .await
        .unwrap();
    pool.close().await;
    let (mut cl, _stop) = start(c.clone()).await;

    // unsigned in signed mode is refused
    assert!(cl.list_alerts(ListAlertsRequest::default()).await.is_err());
    let l = cl
        .list_alerts(signed(
            &root,
            "/itcmdr.Itcmdr/ListAlerts",
            ListAlertsRequest::default(),
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(l.alerts.len(), 2);
    assert_eq!(
        (l.open_critical, l.open_cautious, l.open_informational),
        (1, 0, 1)
    );
    assert_eq!(l.alerts[0].id, id);

    let a = cl
        .ack_alert(signed(
            &root,
            "/itcmdr.Itcmdr/AckAlert",
            AlertRef { id: id.clone() },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(a.state, "acked");
    assert!(!a.acked_by.is_empty());
    let a = cl
        .resolve_alert(signed(
            &root,
            "/itcmdr.Itcmdr/ResolveAlert",
            AlertRef { id: id.clone() },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(a.state, "resolved");
    let e = cl
        .ack_alert(signed(
            &root,
            "/itcmdr.Itcmdr/AckAlert",
            AlertRef { id: "nope".into() },
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), tonic::Code::NotFound);
    let s = cl
        .silence_alerts(signed(
            &root,
            "/itcmdr.Itcmdr/SilenceAlerts",
            SilenceAlertsRequest {
                machine: "WS-01".into(),
                source: String::new(),
                until_ms: chrono::Utc::now().timestamp_millis() + 3_600_000,
                reason: "holidays".into(),
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(s.silenced, 1);
    let e = cl
        .silence_alerts(signed(
            &root,
            "/itcmdr.Itcmdr/SilenceAlerts",
            SilenceAlertsRequest::default(),
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), tonic::Code::InvalidArgument);

    let audit = cl
        .list_audit(signed(
            &root,
            "/itcmdr.Itcmdr/ListAudit",
            ListAuditRequest { limit: 50 },
        ))
        .await
        .unwrap()
        .into_inner();
    let acts: Vec<String> = audit.entries.iter().map(|e| e.action.clone()).collect();
    for want in ["alert-ack", "alert-resolve", "alert-silence"] {
        assert!(
            acts.iter().any(|a| a == want),
            "missing audit {want}: {acts:?}"
        );
    }
}
