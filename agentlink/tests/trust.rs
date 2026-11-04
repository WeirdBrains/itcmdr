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

/// A private CA and a leaf it signed for `name` (stands in for a publicly trusted, e.g. ACME, cert).
fn ca_and_leaf(name: &str) -> (String, Pem) {
    let mut cp = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    cp.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, "test root");
    cp.distinguished_name = dn;
    let ck = rcgen::KeyPair::generate().unwrap();
    let ca = cp.self_signed(&ck).unwrap();
    let lp = rcgen::CertificateParams::new(vec![name.to_string()]).unwrap();
    let lk = rcgen::KeyPair::generate().unwrap();
    let leaf = lp.signed_by(&lk, &ca, &ck).unwrap();
    (
        ca.pem(),
        Pem {
            cert: leaf.pem(),
            key: lk.serialize_pem(),
        },
    )
}

#[tokio::test]
async fn system_valid_certificate_wins_over_an_older_self_signed_pin() {
    // First the server was self-signed and the user pinned it.
    let old = self_signed(&["test.example.com"], 365);
    let old_fp = tls::pem_fingerprint(&old.cert).unwrap();
    // Then it got a certificate the system store accepts (ACME). The pin stays as a fallback.
    let (root, leaf) = ca_and_leaf("test.example.com");
    let addr = serve(&leaf).await;
    let mut t = named("test.example.com");
    t.pin = Some(old_fp.clone());
    t.extra_roots.push(root.clone());
    let r = trust::inspect(&addr, &t).await.unwrap();
    assert_eq!(r.problem, "ok", "{r:?}");
    assert!(r.detail.starts_with(trust::SYSTEM_OVER_PIN), "{r:?}");
    assert!(
        r.detail.contains(&old_fp),
        "the old pin is named, not dropped: {r:?}"
    );
    // Same pin, wrong name for the system-valid certificate: not system-valid, so the pin
    // decides and it does not match.
    let mut wrong = named("other.example.com");
    wrong.pin = Some(old_fp.clone());
    wrong.extra_roots.push(root);
    assert_eq!(
        trust::inspect(&addr, &wrong).await.unwrap().problem,
        "pin-changed"
    );
}

#[tokio::test]
async fn changed_certificate_the_system_refuses_still_blocks() {
    let old = self_signed(&["test.example.com"], 365);
    let new = self_signed(&["test.example.com"], 365);
    let addr = serve(&new).await;
    let mut t = named("test.example.com");
    t.pin = Some(tls::pem_fingerprint(&old.cert).unwrap());
    let r = trust::inspect(&addr, &t).await.unwrap();
    assert_eq!(r.problem, "pin-changed");
    assert!(trust::connect(&addr, &t, None).await.is_err());
    // Back on the pinned certificate the pin alone is enough (no system trust needed).
    let addr_old = serve(&old).await;
    assert_eq!(
        trust::inspect(&addr_old, &t).await.unwrap().detail,
        "pinned"
    );
}

#[tokio::test]
async fn enrollment_pins_stay_exact_even_for_system_valid_certificates() {
    // `internal` clients (adminsvr enrollment pins) validate as the reserved pinned name, so a
    // public certificate never replaces their pin.
    let (root, leaf) = ca_and_leaf("test.example.com");
    let addr = serve(&leaf).await;
    let mut t = Trust::pinned(&"00".repeat(32));
    t.internal = true;
    t.extra_roots.push(root);
    assert_eq!(
        trust::inspect(&addr, &t).await.unwrap().problem,
        "pin-changed"
    );
}

#[tokio::test]
async fn mixed_listener_plain_only_when_allowed() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_stream::StreamExt;
    for allow in [false, true] {
        let pem = self_signed(&["test.example.com"], 365);
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let mut inc = Box::pin(tls::incoming_mixed(
            l,
            tls::api_server_config(&pem).unwrap(),
            Arc::new(move |_| allow),
            Arc::new(|_, _| {}),
        ));
        let mut c = tokio::net::TcpStream::connect(&addr).await.unwrap();
        c.write_all(b"PRI * HTTP/2.0\r\n").await.unwrap();
        let got = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            if allow {
                matches!(inc.next().await, Some(Ok(tls::MaybeTls::Plain(_))))
            } else {
                // Refused: the server closes the connection and yields nothing.
                let mut b = [0u8; 1];
                matches!(c.read(&mut b).await, Ok(0) | Err(_))
            }
        })
        .await
        .unwrap();
        assert!(got, "plain allowed={allow}");
        // HTTPS always works.
        let fp = tls::pem_fingerprint(&pem.cert).unwrap();
        let h = tokio::spawn(async move { inc.next().await.map(|r| r.is_ok()) });
        assert_eq!(
            trust::inspect(&addr, &Trust::pinned(&fp))
                .await
                .unwrap()
                .problem,
            "ok"
        );
        assert_eq!(h.await.unwrap(), Some(true));
    }
}
