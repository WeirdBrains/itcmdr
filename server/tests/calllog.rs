//! v0.15.0 call log: every RPC gets a call_log row (method, kind, peer, principal, status, sizes,
//! duration), audited calls link to it, ListCalls filters, and the daily export/prune.
use ::itcmdr_server as srv;
use prost::Message;
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::{Code, Request};

const LC: &str = "/itcmdr.Itcmdr/ListCalls";
const LA: &str = "/itcmdr.Itcmdr/ListAudit";
const TR: &str = "/itcmdr.Itcmdr/TriggerRuns";

fn cfg(root: &std::path::Path) -> ServerConfig {
    ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        auth_mode: "signed".into(),
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

struct Server {
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Server {
    /// Shut the server down and wait for it: its SQLite worker threads must be joined before the
    /// test binary exits, or they race libc/OpenSSL exit handlers (SIGSEGV at exit under load).
    async fn stop(self) {
        let _ = self.stop.send(());
        self.task.await.unwrap().unwrap();
    }
}

async fn start(c: ServerConfig) -> (ItcmdrClient<Channel>, Server) {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut c = c;
    c.bind_addr = format!("127.0.0.1:{port}");
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(srv::run(c, async move {
        let _ = rx.await;
    }));
    for _ in 0..100 {
        if let Ok(cl) = ItcmdrClient::connect(format!("http://127.0.0.1:{port}")).await {
            return (cl, Server { stop: tx, task });
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not start");
}

async fn calls(
    cl: &mut ItcmdrClient<Channel>,
    key: &ed25519_dalek::SigningKey,
    q: ListCallsRequest,
) -> Vec<CallEntry> {
    // The writer is asynchronous: give it a moment.
    tokio::time::sleep(Duration::from_millis(300)).await;
    cl.list_calls(signed(key, LC, q))
        .await
        .unwrap()
        .into_inner()
        .calls
}

#[tokio::test]
async fn every_call_is_logged_and_audit_links_to_it() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::auth::init(&pool, &c.layout()).await.unwrap();
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let fp = srv::keys::fingerprint(&root.verifying_key());
    pool.close().await;
    let (mut cl, server) = start(c.clone()).await;

    // Unsigned in signed mode: refused, logged as anonymous UNAUTHENTICATED.
    let e = cl
        .list_audit(ListAuditRequest { limit: 5 })
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::Unauthenticated);
    // Wrongly signed (signature for another method): the claimed key is shown as unverified.
    let e = cl
        .list_audit(signed(
            &root,
            "/itcmdr.Itcmdr/Other",
            ListAuditRequest { limit: 5 },
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::Unauthenticated);
    // A signed, audited call.
    cl.trigger_runs(signed(
        &root,
        TR,
        TriggerRunsRequest {
            machines: vec!["WS-01".into()],
            report_only: true,
            ..Default::default()
        },
    ))
    .await
    .unwrap();

    let all = calls(&mut cl, &root, ListCallsRequest::default()).await;
    let la: Vec<_> = all.iter().filter(|c| c.method == "ListAudit").collect();
    assert_eq!(la.len(), 2, "{all:#?}");
    assert!(la
        .iter()
        .all(|c| c.status == 16 && c.status_name == "UNAUTHENTICATED"));
    assert!(la.iter().any(|c| c.principal == "anonymous"));
    assert!(la
        .iter()
        .any(|c| c.principal == format!("unverified key {}", &fp[..16])));
    assert!(la.iter().all(|c| c.peer.starts_with("127.0.0.1:")));
    assert!(la
        .iter()
        .all(|c| c.service == "itcmdr.Itcmdr" && c.kind == "unary"));
    assert!(la.iter().any(|c| !c.error.is_empty()));

    let tr = all.iter().find(|c| c.method == "TriggerRuns").unwrap();
    assert_eq!(tr.status, 0);
    assert!(tr.principal.starts_with("root ("), "{}", tr.principal);
    assert!(tr.req_bytes > 0 && tr.resp_bytes > 0, "{tr:?}");
    assert!(tr.duration_ms >= 0);

    // The audit row of the trigger links to its call.
    let audit = cl
        .list_audit(signed(&root, LA, ListAuditRequest { limit: 20 }))
        .await
        .unwrap()
        .into_inner()
        .entries;
    let a = audit.iter().find(|a| a.action == "trigger-runs").unwrap();
    assert_eq!(a.call_id, tr.id);
    let one = calls(
        &mut cl,
        &root,
        ListCallsRequest {
            id: a.call_id.clone(),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].method, "TriggerRuns");

    // Filters.
    let errs = calls(
        &mut cl,
        &root,
        ListCallsRequest {
            errors_only: true,
            ..Default::default()
        },
    )
    .await;
    assert!(!errs.is_empty() && errs.iter().all(|c| c.status != 0));
    let by_status = calls(
        &mut cl,
        &root,
        ListCallsRequest {
            status: Some(0),
            method: "Trigger".into(),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(by_status.len(), 1);
    let by_who = calls(
        &mut cl,
        &root,
        ListCallsRequest {
            principal: "anonymous".into(),
            ..Default::default()
        },
    )
    .await;
    assert!(by_who.iter().all(|c| c.principal == "anonymous"));

    // A server stream is logged with its kind once the client drops it.
    let mut s = cl
        .watch_updates(signed(
            &root,
            "/itcmdr.Itcmdr/WatchUpdates",
            WatchUpdatesRequest::default(),
        ))
        .await
        .unwrap()
        .into_inner();
    let _ = tokio::time::timeout(Duration::from_millis(200), s.message()).await;
    drop(s);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let w = calls(
        &mut cl,
        &root,
        ListCallsRequest {
            method: "WatchUpdates".into(),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].kind, "server_stream");

    // Bodies are never stored.
    let pool = srv::db::init_db(&c).await.unwrap();
    let dump: Vec<(String, String)> =
        sqlx::query_as("SELECT COALESCE(error,''), COALESCE(principal,'') FROM call_log")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(dump
        .iter()
        .all(|(e, p)| !e.contains("WS-01") && !p.contains("WS-01")));
    pool.close().await;
    drop(cl);
    server.stop().await;
}

#[tokio::test]
async fn export_then_prune() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    for (id, age_days, status) in [("old", 40, 0), ("mid", 3, 16), ("today", 0, 0)] {
        sqlx::query("INSERT INTO call_log(id, ts_ms, service, method, kind, peer, principal, status, duration_ms, req_bytes, resp_bytes, error) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(id).bind(now - age_days * 86_400_000).bind("itcmdr.Itcmdr").bind("ListMachines").bind("unary")
            .bind("127.0.0.1:1").bind("Dr A").bind(status).bind(2).bind(5).bind(9).bind("")
            .execute(&pool).await.unwrap();
    }
    let dir = srv::calllog::export_dir(&c.layout());
    let days = srv::calllog::export_finished_days(&pool, &dir, "SERVER01", 30)
        .await
        .unwrap();
    // The 40-day-old row is outside the export window; the 3-day-old one is exported; today waits.
    assert_eq!(days.len(), 1, "{days:?}");
    let f = dir.join(format!("calls-{}.jsonl", days[0]));
    let text = std::fs::read_to_string(&f).unwrap();
    for l in text.lines() {
        let m = itcmdr_plugin::log::validate(l).unwrap();
        assert_eq!(m["event"], "call");
        assert_eq!(m["level"], "warn");
        assert_eq!(m["run_id"], "mid");
    }
    // Running again exports nothing new.
    assert!(
        srv::calllog::export_finished_days(&pool, &dir, "SERVER01", 30)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(srv::calllog::prune(&pool, 30).await.unwrap(), 1);
    let left: Vec<String> = sqlx::query_scalar("SELECT id FROM call_log ORDER BY ts_ms")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(left, ["mid", "today"]);
    pool.close().await;
}
