//! v0.17.1 Rust agents: site CA certified by the root key, renewal over the link with expiry
//! alerts, revoked serials refused in the TLS handshake, the office-hours profile and the
//! offline alert queue.
use ::itcmdr_server as srv;
use itcmdr_agent::link::{self, End};
use itcmdr_agentlink::{rustls, tokio_rustls};
use srv::config::ServerConfig;
use std::time::Duration;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn cfg(root: &std::path::Path) -> ServerConfig {
    ServerConfig {
        bind_addr: format!("127.0.0.1:{}", free_port()),
        agent_bind_addr: format!("127.0.0.1:{}", free_port()),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        auth_mode: "signed".into(),
        ..ServerConfig::default()
    }
}

async fn start(c: &ServerConfig) -> tokio::sync::oneshot::Sender<()> {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(srv::run(c.clone(), async move {
        let _ = rx.await;
    }));
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(&c.agent_bind_addr)
            .await
            .is_ok()
        {
            return tx;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("agent link did not start");
}

async fn token(pool: &srv::db::DbPool, c: &ServerConfig, machine: &str) -> String {
    srv::agents::create_token(
        pool,
        &c.layout(),
        c,
        machine,
        60,
        &c.agent_bind_addr,
        "test",
    )
    .await
    .unwrap()
    .token
}

async fn agent(pool: &srv::db::DbPool, m: &str) -> srv::agents::Agent {
    srv::agents::by_machine(pool, m).await.unwrap().unwrap()
}

/// Run a session in the background; returns the Welcome channel and the join handle.
fn spawn_session(
    dir: &std::path::Path,
    st: &itcmdr_agent::state::State,
) -> (
    tokio::sync::mpsc::UnboundedReceiver<i32>,
    tokio::task::JoinHandle<End>,
) {
    let (wtx, wrx) = tokio::sync::mpsc::unbounded_channel();
    let (d, s) = (dir.to_path_buf(), st.clone());
    let h = tokio::spawn(async move {
        link::session(&d, &s, |w| {
            let _ = wtx.send(w.queue_max_mib);
        })
        .await
    });
    (wrx, h)
}

async fn welcome(rx: &mut tokio::sync::mpsc::UnboundedReceiver<i32>) -> i32 {
    tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .unwrap()
        .unwrap()
}

async fn open_keys(pool: &srv::db::DbPool) -> Vec<(String, String)> {
    srv::alerts::list(pool, &Default::default())
        .await
        .unwrap()
        .into_iter()
        .map(|a| (a.key, a.level))
        .collect()
}

#[tokio::test]
async fn ca_certified_renewal_and_expiry_alerts() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::auth::init(&pool, &c.layout()).await.unwrap();
    let _stop = start(&c).await;
    let adir = tmp.path().join("agent");

    // The token names the root key; a token with a different root is refused at enrollment.
    let good = token(&pool, &c, "WS-01").await;
    let t = itcmdr_agentlink::Token::decode(&good).unwrap();
    assert_eq!(t.root.len(), 64, "token carries the root public key");
    let mut bad = t.clone();
    bad.root = "11".repeat(32);
    let e = link::enroll(&adir, &bad.encode(), Some("WS-01"), "workstation")
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("root key"), "{e:#}");

    // Short lifetime: 60 min -> expiry alert is critical.
    srv::settings::set(&pool, "agent.certLifetimeMins", "60", None)
        .await
        .unwrap();
    let good = token(&pool, &c, "WS-01").await;
    let st = link::enroll(&adir, &good, Some("WS-01"), "workstation")
        .await
        .unwrap();
    srv::agents::tick(&pool).await.unwrap();
    assert!(open_keys(&pool)
        .await
        .contains(&("agent-cert:WS-01".into(), "critical".into())));

    // The agent renews at 2/3 of the lifetime: pretend it was issued 200 minutes ago.
    srv::settings::set(&pool, "agent.certLifetimeMins", "129600", None)
        .await
        .unwrap();
    let mut old = st.clone();
    old.cert_issued_ms -= 200 * 60_000; // lifetime now 260 min, 200 > 2/3
    itcmdr_agent::state::save(&adir, &old).unwrap();
    let (mut w, h) = spawn_session(&adir, &old);
    assert_eq!(welcome(&mut w).await, 256);
    let end = tokio::time::timeout(Duration::from_secs(10), h)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(end, End::Renewed);
    let st2 = itcmdr_agent::state::load(&adir).unwrap().unwrap();
    assert_ne!(st2.cert_pem, st.cert_pem);
    assert!(st2.cert_expires_ms > st.cert_expires_ms + 80 * 86_400_000);
    let a = agent(&pool, "WS-01").await;
    assert_eq!(
        a.cert_fp,
        itcmdr_agentlink::tls::pem_fingerprint(&st2.cert_pem).unwrap()
    );
    assert!(!open_keys(&pool)
        .await
        .iter()
        .any(|(k, _)| k == "agent-cert:WS-01"));

    // The new certificate links; then an admin-requested renewal (agent renew) renews again.
    let (mut w, h) = spawn_session(&adir, &st2);
    welcome(&mut w).await;
    srv::agents::request_renew(&pool, "WS-01").await.unwrap();
    assert!(agent(&pool, "WS-01").await.renew_requested);
    // Next Pong triggers RenewNow; speed up with a short heartbeat is not possible mid-session,
    // so reconnect: the Welcome path sends RenewNow at once.
    h.abort();
    let (mut w, h) = spawn_session(&adir, &st2);
    welcome(&mut w).await;
    let end = tokio::time::timeout(Duration::from_secs(10), h)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(end, End::Renewed);
    let st3 = itcmdr_agent::state::load(&adir).unwrap().unwrap();
    assert!(!agent(&pool, "WS-01").await.renew_requested);

    // The first certificate (two renewals ago) is no longer allowed in the handshake; the
    // previous one (st2) is, during the overlap; the current one links.
    let serial = |s: &itcmdr_agent::state::State| {
        itcmdr_agentlink::tls::serial_of(
            itcmdr_agentlink::tls::pem_to_der(&s.cert_pem)
                .unwrap()
                .as_ref(),
        )
        .unwrap()
    };
    assert!(srv::agents::serial_revoked(&pool, &serial(&st)));
    assert!(!srv::agents::serial_revoked(&pool, &serial(&st2)));
    assert!(!srv::agents::serial_revoked(&pool, &serial(&st3)));
    let (mut w, h) = spawn_session(&adir, &st3);
    welcome(&mut w).await;
    h.abort();
}

#[tokio::test]
async fn revoked_serial_refused_in_handshake() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let _stop = start(&c).await;
    let adir = tmp.path().join("agent");
    let st = link::enroll(
        &adir,
        &token(&pool, &c, "WS-03").await,
        Some("WS-03"),
        "workstation",
    )
    .await
    .unwrap();
    srv::agents::revoke(&pool, "WS-03").await.unwrap().unwrap();
    // Raw TLS: the handshake itself fails (no RPC is reached).
    let key = String::from_utf8(
        itcmdr_agent::state::read_secret(&adir.join("keys").join("tls.key")).unwrap(),
    )
    .unwrap();
    let pem = itcmdr_agentlink::tls::Pem {
        cert: st.cert_pem.clone(),
        key,
    };
    let cfg = itcmdr_agentlink::tls::client_config(&st.pin, Some(&pem)).unwrap();
    let tcp = tokio::net::TcpStream::connect(&st.address).await.unwrap();
    let conn = tokio_rustls::TlsConnector::from(std::sync::Arc::new(cfg));
    let name = rustls::pki_types::ServerName::try_from("itcmdr-server").unwrap();
    let r = async {
        let mut s = conn.connect(name, tcp).await?;
        // TLS 1.3: the client's cert is judged after its Finished; the alert arrives on read.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        s.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await?;
        let mut b = [0u8; 64];
        let n = s.read(&mut b).await?;
        std::io::Result::Ok(n)
    }
    .await;
    let e = format!("{r:?}");
    assert!(e.contains("Revoked") || e.contains("revoked"), "{e}");
    let end = link::session(&adir, &st, |_| {}).await;
    assert!(
        matches!(end, End::Refused(ref m) if m.contains("revoked")),
        "{end:?}"
    );
}

#[tokio::test]
async fn office_hours_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let _stop = start(&c).await;
    let adir = tmp.path().join("agent");
    let st = link::enroll(
        &adir,
        &token(&pool, &c, "WS-04").await,
        Some("WS-04"),
        "workstation",
    )
    .await
    .unwrap();
    let (mut w, h) = spawn_session(&adir, &st);
    welcome(&mut w).await;
    h.abort();
    assert!(
        srv::settings::set(&pool, "agent.officeHours", "nonsense", Some("WS-04"))
            .await
            .is_err()
    );
    // Hours that never include now: gone for hours, still no alert (only "late").
    let now = chrono::Local::now();
    use chrono::Datelike;
    let other = now.weekday().succ().succ();
    srv::settings::set(
        &pool,
        "agent.officeHours",
        &format!("{other:?} 00:00-23:59"),
        Some("WS-04"),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE agents SET last_seen_ms = last_seen_ms - 36000000")
        .execute(&pool)
        .await
        .unwrap();
    srv::agents::tick(&pool).await.unwrap();
    assert_eq!(agent(&pool, "WS-04").await.state, "late");
    assert!(open_keys(&pool).await.is_empty());
    // Hours covering all day today: counted from the start of the hours (00:00) -> alert.
    srv::settings::set(
        &pool,
        "agent.officeHours",
        &format!("{:?} 00:00-23:59", now.weekday()),
        Some("WS-04"),
    )
    .await
    .unwrap();
    if now.time() > chrono::NaiveTime::from_hms_opt(0, 10, 0).unwrap()
        && now.time() < chrono::NaiveTime::from_hms_opt(23, 58, 0).unwrap()
    {
        srv::agents::tick(&pool).await.unwrap();
        assert_eq!(agent(&pool, "WS-04").await.state, "non_responsive");
    }
    // Unit check of the window maths.
    let h = srv::schedule::parse_hours("Mon-Fri 07:00-18:00").unwrap();
    use chrono::TimeZone;
    let mon9 = chrono::Local
        .with_ymd_and_hms(2026, 11, 2, 9, 0, 0)
        .unwrap();
    let start = srv::agents::office_start_ms(&h, mon9).unwrap();
    assert_eq!(start, mon9.timestamp_millis() - 2 * 3_600_000);
    assert!(srv::agents::office_start_ms(&h, mon9 + chrono::Duration::hours(10)).is_none());
    assert!(srv::agents::office_start_ms(&h, mon9 - chrono::Duration::days(1)).is_none());
}

#[tokio::test]
async fn queued_alerts_delivered_once_after_reconnect() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let stop = start(&c).await;
    let adir = tmp.path().join("agent");
    let st = link::enroll(
        &adir,
        &token(&pool, &c, "WS-05").await,
        Some("WS-05"),
        "workstation",
    )
    .await
    .unwrap();
    // Server down: alerts queue up.
    let _ = stop.send(());
    tokio::time::sleep(Duration::from_millis(300)).await;
    let item = |key: &str, level: &str| itcmdr_agent::queue::Item {
        seq: 0,
        key: key.into(),
        level: level.into(),
        title: format!("{key} happened"),
        detail: "from the test".into(),
        ts_ms: chrono::Utc::now().timestamp_millis() - 600_000,
        resolve: false,
        report: None,
    };
    itcmdr_agent::queue::spool(&adir, &item("disk-low", "critical")).unwrap();
    itcmdr_agent::queue::spool(&adir, &item("note", "informational")).unwrap();
    let end = link::session(&adir, &st, |_| {}).await;
    assert!(matches!(end, End::Lost(_)), "{end:?}");
    // Server back (same data): both are delivered and acked.
    let _stop2 = start(&c).await;
    let (mut w, h) = spawn_session(&adir, &st);
    welcome(&mut w).await;
    for _ in 0..50 {
        if itcmdr_agent::queue::ingest(&adir, 256).unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(itcmdr_agent::queue::ingest(&adir, 256).unwrap().is_empty());
    let all = srv::alerts::list(
        &pool,
        &srv::alerts::Filter {
            state: Some("all".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let crit = all
        .iter()
        .find(|a| a.key == "agent:WS-05:disk-low")
        .unwrap();
    assert_eq!(crit.level, "critical");
    assert!(
        crit.detail.contains("queued on the agent"),
        "{}",
        crit.detail
    );
    assert!(all.iter().any(|a| a.key == "agent:WS-05:note"));
    // Resolve from the agent.
    let mut r = item("disk-low", "critical");
    r.resolve = true;
    itcmdr_agent::queue::spool(&adir, &r).unwrap();
    for _ in 0..50 {
        if !open_keys(&pool)
            .await
            .iter()
            .any(|(k, _)| k == "agent:WS-05:disk-low")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(!open_keys(&pool)
        .await
        .iter()
        .any(|(k, _)| k == "agent:WS-05:disk-low"));
    h.abort();
    let a = agent(&pool, "WS-05").await;
    assert!(a.last_seq >= 3);
}

/// v1.3.3: a revoke written by another process (itcmdr-cli) reaches the handshake allow-list only
/// at the server's next refresh. Until then Link must still refuse the certificate before it reads
/// any message: never "the first message must be Hello" for a revoked agent.
#[tokio::test]
async fn link_refuses_a_revoked_certificate_before_the_allow_list_reloads() {
    use itcmdr_agentlink::pb::{self, agent_service_client::AgentServiceClient};
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let _stop = start(&c).await;
    let adir = tmp.path().join("agent");
    let st = link::enroll(
        &adir,
        &token(&pool, &c, "WS-13").await,
        Some("WS-13"),
        "workstation",
    )
    .await
    .unwrap();
    let key = String::from_utf8(
        itcmdr_agent::state::read_secret(&adir.join("keys").join("tls.key")).unwrap(),
    )
    .unwrap();
    let pem = itcmdr_agentlink::tls::Pem {
        cert: st.cert_pem.clone(),
        key,
    };
    // Like `itcmdr-cli agent revoke`: the database only, no in-process refresh.
    sqlx::query("UPDATE agents SET state = 'revoked', revoked_ms = 1 WHERE machine = 'WS-13'")
        .execute(&pool)
        .await
        .unwrap();
    let r = async {
        let ch = itcmdr_agentlink::tls::connect(&st.address, &st.pin, Some(&pem)).await?;
        let msgs = tokio_stream::iter(vec![pb::AgentMessage::default()]);
        AgentServiceClient::new(ch)
            .link(msgs)
            .await
            .map(|_| ())
            .map_err(anyhow::Error::from)
    }
    .await;
    let e = format!("{r:?}");
    assert!(r.is_err(), "a revoked certificate opened a link");
    assert!(!e.contains("first message must be Hello"), "{e}");
    assert!(e.contains("revoked") || e.contains("Revoked"), "{e}");
}
