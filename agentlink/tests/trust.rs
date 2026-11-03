//! v1.1.0 client trust against a self-signed HTTPS server.
use itcmdr_agentlink::tls::{self, Pem};
use itcmdr_agentlink::trust::{self, Trust};
use std::sync::Arc;

fn self_signed(names: &[&str], days: i64) -> Pem {
    let mut p =
        rcgen::CertificateParams::new(names.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap();
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, "test.example.com");
    p.distinguished_name = dn;
    let now = time::OffsetDateTime::now_utc();
    if days < 0 {
        p.not_before = now - time::Duration::days(30);
    }
    p.not_after = now + time::Duration::days(days);
    let k = rcgen::KeyPair::generate().unwrap();
    let c = p.self_signed(&k).unwrap();
    Pem {
        cert: c.pem(),
        key: k.serialize_pem(),
    }
}

/// A TLS server answering handshakes with `pem` (nothing else).
async fn serve(pem: &Pem) -> String {
    let cfg = tls::api_server_config(pem).unwrap();
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap().to_string();
    let acc = tokio_rustls::TlsAcceptor::from(Arc::new(cfg));
    tokio::spawn(async move {
        loop {
            let (s, _) = l.accept().await.unwrap();
            let acc = acc.clone();
            tokio::spawn(async move {
                let _ = acc.accept(s).await;
            });
        }
    });
    addr
}

fn named(n: &str) -> Trust {
    Trust {
        name: Some(n.into()),
        ..Default::default()
    }
}

#[tokio::test]
async fn self_signed_is_reported_then_pinned_exactly() {
    let pem = self_signed(&["test.example.com"], 365);
    let fp = tls::pem_fingerprint(&pem.cert).unwrap();
    let addr = serve(&pem).await;
    let r = trust::inspect(&addr, &named("test.example.com"))
        .await
        .unwrap();
    assert_eq!(
        (r.problem.as_str(), r.fingerprint.as_str()),
        ("self-signed", fp.as_str())
    );
    assert!(r.subject.contains("test.example.com"), "{r:?}");
    assert!(r.names.contains(&"test.example.com".to_string()));
    assert!(!r.needs_override());
    // Trusted (pinned) exactly: accepted.
    let ok = trust::inspect(&addr, &Trust::pinned(&fp)).await.unwrap();
    assert_eq!(ok.problem, "ok");
    // A different pin: refused as changed.
    let bad = trust::inspect(&addr, &Trust::pinned(&"00".repeat(32)))
        .await
        .unwrap();
    assert_eq!(bad.problem, "pin-changed");
    // As an extra root it validates (name and expiry are checked).
    let mut t = named("test.example.com");
    t.extra_roots.push(pem.cert.clone());
    assert_eq!(trust::inspect(&addr, &t).await.unwrap().problem, "ok");
    let mut wrong = named("other.example.com");
    wrong.extra_roots.push(pem.cert.clone());
    let w = trust::inspect(&addr, &wrong).await.unwrap();
    assert_eq!(w.problem, "name-mismatch");
    assert!(w.needs_override());
}

#[tokio::test]
async fn expired_needs_override() {
    let pem = self_signed(&["test.example.com"], -1);
    let addr = serve(&pem).await;
    let mut t = named("test.example.com");
    t.extra_roots.push(pem.cert.clone());
    let r = trust::inspect(&addr, &t).await.unwrap();
    assert_eq!(r.problem, "expired");
    assert!(r.needs_override());
    assert!(r.not_after_ms < itcmdr_agentlink_now());
}

fn itcmdr_agentlink_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
