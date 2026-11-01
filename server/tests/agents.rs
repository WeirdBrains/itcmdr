//! v0.17.0 Rust agents: enrollment (one-time token, machine binding), pinned TLS (a wrong pin is
//! refused), mTLS on the link (no / foreign client certificate refused), the signed Hello,
//! heartbeat loss -> non_responsive + alert -> recovery, maintenance, and revocation.
use ::itcmdr_server as srv;
use itcmdr_agent::link::{self, End};
use itcmdr_agentlink::pb;
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

#[tokio::test]
async fn enroll_link_heartbeat_revoke() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let _stop = start(&c).await;
    let adir = tmp.path().join("agent");

    // A wrong pin is refused before anything is sent.
    let good = token(&pool, &c, "WS-01").await;
    let mut t = itcmdr_agentlink::Token::decode(&good).unwrap();
    t.pin = "00".repeat(32);
    let e = link::enroll(&adir, &t.encode(), Some("WS-01"), "workstation")
        .await
        .unwrap_err();
    assert!(
        format!("{e:#}").contains("does not match the pinned"),
        "{e:#}"
    );
    assert!(itcmdr_agent::state::load(&adir).unwrap().is_none());

    // Token for another machine; then the right one; then reuse.
    let e = link::enroll(&adir, &good, Some("WS-02"), "workstation")
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("for WS-01"), "{e:#}");
    let st = link::enroll(&adir, &good, Some("WS-01"), "workstation")
        .await
        .unwrap();
    assert_eq!(st.machine, "WS-01");
    let e = link::enroll(
        &tmp.path().join("again"),
        &good,
        Some("WS-01"),
        "workstation",
    )
    .await
    .unwrap_err();
    assert!(format!("{e:#}").contains("already used"), "{e:#}");
    let a = srv::agents::by_machine(&pool, "WS-01")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.state, "enrolled");
    assert_eq!(
        a.cert_fp,
        itcmdr_agentlink::tls::pem_fingerprint(&st.cert_pem).unwrap()
    );

    // Link: Welcome, online.
    let (wtx, mut wrx) = tokio::sync::mpsc::unbounded_channel();
    let (d2, st2) = (adir.clone(), st.clone());
    let sess = tokio::spawn(async move {
        link::session(&d2, &st2, |w| {
            let _ = wtx.send(w.heartbeat_secs);
        })
        .await
    });
    let hb = tokio::time::timeout(Duration::from_secs(10), wrx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hb, 60);
    assert_eq!(
        srv::agents::by_machine(&pool, "WS-01")
            .await
            .unwrap()
            .unwrap()
            .state,
        "online"
    );

    // Heartbeat loss: last seen 10 minutes ago -> non_responsive + critical alert.
    sqlx::query("UPDATE agents SET last_seen_ms = last_seen_ms - 600000")
        .execute(&pool)
        .await
        .unwrap();
    srv::agents::tick(&pool).await.unwrap();
    assert_eq!(
        srv::agents::by_machine(&pool, "WS-01")
            .await
            .unwrap()
            .unwrap()
            .state,
        "non_responsive"
    );
    let open = srv::alerts::list(
        &pool,
        &srv::alerts::Filter {
            source: Some("agent".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(
        (open[0].level.as_str(), open[0].key.as_str()),
        ("critical", "agent-nonresponsive:WS-01")
    );
    // Late (2..4 missed): no alert of its own. Per-machine level override is honoured.
    srv::settings::set(&pool, "agent.nonresponsiveLevel", "cautious", Some("WS-01"))
        .await
        .unwrap();
    assert!(
        srv::settings::set(&pool, "agent.nonresponsiveLevel", "loud", None)
            .await
            .is_err()
    );

    // Recovery: a fresh beat (session reconnect = Hello) -> online, alert resolved, a closed "back online" note.
    sess.abort();
    let (wtx, mut wrx) = tokio::sync::mpsc::unbounded_channel();
    let (d2, st2) = (adir.clone(), st.clone());
    let sess = tokio::spawn(async move {
        link::session(&d2, &st2, |w| {
            let _ = wtx.send(w.heartbeat_secs);
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(10), wrx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        srv::agents::by_machine(&pool, "WS-01")
            .await
            .unwrap()
            .unwrap()
            .state,
        "online"
    );
    assert_eq!(srv::alerts::open_counts(&pool).await.unwrap(), [0, 0, 0]);
    let all = srv::alerts::list(
        &pool,
        &srv::alerts::Filter {
            state: Some("all".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(all
        .iter()
        .any(|a| a.key == "agent-online:WS-01" && a.level == "informational"));

    // Late: 3 missed -> late, no alert.
    sqlx::query("UPDATE agents SET last_seen_ms = last_seen_ms - 180000")
        .execute(&pool)
        .await
        .unwrap();
    srv::agents::tick(&pool).await.unwrap();
    assert_eq!(
        srv::agents::by_machine(&pool, "WS-01")
            .await
            .unwrap()
            .unwrap()
            .state,
        "late"
    );
    assert_eq!(srv::alerts::open_counts(&pool).await.unwrap(), [0, 0, 0]);
    // Maintenance: no alert even when long gone.
    srv::agents::set_maintenance(&pool, "WS-01", true)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET last_seen_ms = last_seen_ms - 6000000")
        .execute(&pool)
        .await
        .unwrap();
    srv::agents::tick(&pool).await.unwrap();
    assert_eq!(
        srv::agents::by_machine(&pool, "WS-01")
            .await
            .unwrap()
            .unwrap()
            .state,
        "maintenance"
    );
    assert_eq!(srv::alerts::open_counts(&pool).await.unwrap(), [0, 0, 0]);

    // Revoke: the open stream ends with Bye; reconnecting is refused.
    let a = srv::agents::revoke(&pool, "WS-01").await.unwrap().unwrap();
    assert_eq!(a.state, "revoked");
    let end = tokio::time::timeout(Duration::from_secs(10), sess)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(end, End::Bye(_)), "{end:?}");
    let end = link::session(&adir, &st, |_| {}).await;
    assert!(
        matches!(end, End::Refused(ref m) if m.contains("revoked")),
        "{end:?}"
    );
}

#[tokio::test]
async fn link_requires_the_issued_client_certificate() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let _stop = start(&c).await;
    let adir = tmp.path().join("agent");
    let st = link::enroll(&adir, &token(&pool, &c, "").await, Some("WS-09"), "server")
        .await
        .unwrap();
    assert_eq!(
        srv::agents::by_machine(&pool, "WS-09")
            .await
            .unwrap()
            .unwrap()
            .role,
        "server"
    );

    let hello = |fp: &str| pb::AgentMessage {
        seq: 1,
        body: Some(pb::agent_message::Body::Hello(pb::Hello {
            agent_id: st.agent_id.clone(),
            machine: st.machine.clone(),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            nonce: "n".into(),
            signature: "00".repeat(64),
            agent_version: fp.into(),
            ..Default::default()
        })),
    };
    // No client certificate: refused by Link.
    let ch = itcmdr_agentlink::tls::connect(&st.address, &st.pin, None)
        .await
        .unwrap();
    let e = pb::agent_service_client::AgentServiceClient::new(ch)
        .link(tokio_stream::iter(vec![hello("")]))
        .await
        .unwrap_err();
    assert_eq!(e.code(), tonic::Code::Unauthenticated);

    // A certificate from another CA: the TLS handshake fails.
    let other = itcmdr_agentlink::tls::new_ca("other").unwrap();
    let (csr, key) = itcmdr_agentlink::tls::new_csr("WS-09").unwrap();
    let forged = itcmdr_agentlink::tls::sign_csr(&other, &csr, &st.agent_id, "WS-09", 30).unwrap();
    let pem = itcmdr_agentlink::tls::Pem {
        cert: forged.cert_pem,
        key,
    };
    let r = async {
        let ch = itcmdr_agentlink::tls::connect(&st.address, &st.pin, Some(&pem)).await?;
        pb::agent_service_client::AgentServiceClient::new(ch)
            .link(tokio_stream::iter(vec![hello("")]))
            .await
            .map_err(anyhow::Error::from)?;
        anyhow::Ok(())
    }
    .await;
    assert!(r.is_err());

    // The right certificate but a bad Hello signature.
    let tls_key = String::from_utf8(
        itcmdr_agent::state::read_secret(&adir.join("keys").join("tls.key")).unwrap(),
    )
    .unwrap();
    let mine = itcmdr_agentlink::tls::Pem {
        cert: st.cert_pem.clone(),
        key: tls_key,
    };
    let ch = itcmdr_agentlink::tls::connect(&st.address, &st.pin, Some(&mine))
        .await
        .unwrap();
    let e = pb::agent_service_client::AgentServiceClient::new(ch)
        .link(tokio_stream::iter(vec![hello("")]))
        .await
        .unwrap_err();
    assert_eq!(e.code(), tonic::Code::Unauthenticated);
    assert!(e.message().contains("signature"), "{}", e.message());

    // And the real agent links fine.
    let (wtx, mut wrx) = tokio::sync::mpsc::unbounded_channel();
    let st2 = st.clone();
    tokio::spawn(async move {
        link::session(&adir, &st2, |_| {
            let _ = wtx.send(());
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(10), wrx.recv())
        .await
        .unwrap()
        .unwrap();
}
