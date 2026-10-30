//! End-to-end: start the real server (lib::run) on a free port and drive it over gRPC.
use ::itcmdr_server as srv;
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::io::Write;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::{Code, Request};

const TOKEN: &str = "integration-test-token";

fn authed<T>(msg: T) -> Request<T> {
    let mut r = Request::new(msg);
    r.metadata_mut()
        .insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    r.metadata_mut()
        .insert("x-itcmdr-actor", "tester".parse().unwrap());
    r
}

async fn connect(addr: &str) -> ItcmdrClient<Channel> {
    for _ in 0..100 {
        if let Ok(c) = ItcmdrClient::connect(format!("http://{addr}")).await {
            return c;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not come up on {addr}");
}

async fn eventually<F, Fut>(mut f: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..100 {
        if f().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("condition not met in time");
}

#[tokio::test]
async fn grpc_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let addr = format!("127.0.0.1:{port}");
    let cfg = ServerConfig {
        bind_addr: addr.clone(),
        logs_dir: tmp.path().join("maintenanceRuns"),
        data_dir: Some(tmp.path().join("data")),
        admin_token_hash: Some(srv::api::sha256_hex(TOKEN)),
        ..ServerConfig::default()
    };
    std::fs::create_dir_all(&cfg.logs_dir).unwrap();
    let day = (chrono::Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let mut f = std::fs::File::create(cfg.logs_dir.join("WS-01_run-a.log")).unwrap();
    writeln!(f, r#"{{"type":"run-start","runId":"run-a","computer":"WS-01","role":"Workstation","reportOnly":true,"started":"{day}T01:00:00-04:00"}}"#).unwrap();
    writeln!(f, r#"{{"type":"check","runId":"run-a","computer":"WS-01","category":"network","checkId":"NET-DNS","severity":"warning","status":"failed","ts":"{day}T01:00:05-04:00"}}"#).unwrap();
    writeln!(f, r#"{{"type":"run-complete","runId":"run-a","computer":"WS-01","finished":"{day}T01:01:00-04:00","exitCode":1}}"#).unwrap();
    drop(f);

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(srv::run(cfg.clone(), async move {
        let _ = stop_rx.await;
    }));
    tokio::time::sleep(Duration::from_millis(300)).await;
    if server.is_finished() {
        panic!("server exited early: {:?}", server.await);
    }
    let mut c = connect(&addr).await;

    // The background tick ingests the log on startup.
    eventually(|| {
        let mut c = c.clone();
        async move {
            let m = c
                .list_machines(ListMachinesRequest {})
                .await
                .unwrap()
                .into_inner()
                .machines;
            m.iter()
                .any(|m| m.name == "WS-01" && m.open_incidents == 1 && m.status == "warning")
        }
    })
    .await;

    let logs = c
        .search_logs(SearchLogsRequest {
            status: "failed".into(),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(logs.total, 1);
    assert!(logs.lines[0].contains("NET-DNS"));

    let metrics = c
        .get_metrics(GetMetricsRequest {
            window_days: 7,
            rollup: Rollup::Daily as i32,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(metrics
        .points
        .iter()
        .any(|p| p.key == "incidents_total:WS-01" && p.value == 1.0));

    // Mutations need the token.
    let err = c
        .trigger_runs(TriggerRunsRequest {
            machines: vec!["WS-01".into()],
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unauthenticated);
    let mut bad = Request::new(TriggerRunsRequest::default());
    bad.metadata_mut()
        .insert("authorization", "Bearer wrong".parse().unwrap());
    assert_eq!(
        c.trigger_runs(bad).await.unwrap_err().code(),
        Code::Unauthenticated
    );

    let rid = c
        .trigger_runs(authed(TriggerRunsRequest {
            machines: vec!["WS-01".into()],
            report_only: true,
            ..Default::default()
        }))
        .await
        .unwrap()
        .into_inner()
        .request_id;
    assert!(cfg.requests_dir().join(format!("{rid}.json")).exists());
    let st = c
        .get_request_status(GetRequestStatusRequest {
            request_id: rid.clone(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(st.status, request_status::Status::Queued as i32);

    // A poller answers -> done.
    let resp = cfg.responses_dir();
    std::fs::create_dir_all(&resp).unwrap();
    std::fs::write(
        resp.join(format!("{rid}.WS-01.json")),
        format!(r#"{{"requestId":"{rid}","machine":"WS-01","status":"done"}}"#),
    )
    .unwrap();
    let st = c
        .get_request_status(GetRequestStatusRequest { request_id: rid })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(st.status, request_status::Status::Done as i32);

    // Exceptions round-trip and are exported for agents.
    let rule = ExceptionRule {
        scope: Some(ExceptionScope {
            machines: vec!["WS-01".into()],
            role: String::new(),
        }),
        check_id: "NET-DNS".into(),
        action: "ignore".into(),
        reason: "test".into(),
        ..Default::default()
    };
    let saved = c
        .save_exception(authed(SaveExceptionRequest { rule: Some(rule) }))
        .await
        .unwrap()
        .into_inner()
        .rule
        .unwrap();
    assert!(!saved.id.is_empty());
    let list = c
        .list_exceptions(ListExceptionsRequest {
            include_inactive: false,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list.rules.len(), 1);
    let export = std::fs::read_to_string(cfg.exceptions_export_path()).unwrap();
    assert!(export.contains("NET-DNS"));
    c.expire_exception(authed(ExpireExceptionRequest { id: saved.id }))
        .await
        .unwrap();
    assert!(c
        .list_exceptions(ListExceptionsRequest {
            include_inactive: false
        })
        .await
        .unwrap()
        .into_inner()
        .rules
        .is_empty());

    let audit = c
        .list_audit(ListAuditRequest { limit: 50 })
        .await
        .unwrap()
        .into_inner()
        .entries;
    let actions: Vec<&str> = audit.iter().map(|e| e.action.as_str()).collect();
    for a in ["trigger-runs", "exception-add", "exception-expire"] {
        assert!(actions.contains(&a), "missing audit {a}: {actions:?}");
    }
    // Seeding the built-in schedules on start is the server's own entry.
    assert!(audit.iter().all(
        |e| e.actor == "tester" || (e.action == "schedule-seed" && e.actor == "itcmdr-server")
    ));

    stop_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("server stops")
        .unwrap()
        .unwrap();
}
