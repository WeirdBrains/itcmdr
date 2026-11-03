//! v1.1.0 TLS: client trust (system store, trust on first use, changed pins) and ACME against
//! Pebble (Let's Encrypt's test CA, run with podman; skipped when podman or the images are missing).
mod common;
use common::{free_port, owner, start, start_with};
use itcmdr_admin::{cert_problem, Client};
use itcmdr_agentlink::trust::{self, Trust};
use std::process::Command;
use std::time::Duration;

#[tokio::test]
async fn untrusted_cert_needs_an_explicit_trust_and_a_changed_one_is_refused() {
    let d = common::sqlite_db();
    let s = start(&d.url).await;
    let (_o, e) = owner(&s).await;
    // Enrollment-code pin: the internal certificate (sent with the pinned SNI).
    let internal = e.identity.pin.clone();
    assert!(!internal.is_empty() && !e.identity.pin_tofu);

    // Without a pin the system store decides: adminsvr's own CA is not in it.
    let mut id = e.identity.clone();
    id.forget_cert();
    let err = Client::connect(&id).await.err().expect("refused");
    let r = cert_problem(&err).expect("a certificate problem, not a generic error");
    assert_eq!(r.problem, "untrusted", "{r:?}");
    assert_eq!(r.fingerprint, internal);
    assert!(r.subject.contains("itcmdr") || !r.names.is_empty(), "{r:?}");
    assert!(r.not_after_ms > itcmdr_keys::now_ms());
    // "Trust this certificate" only accepts the certificate shown.
    assert!(trust::check_choice(&r, &"00".repeat(32)).is_err());
    let fp = trust::check_choice(&r, &r.fingerprint.to_uppercase()).unwrap();
    id.trust_cert(&fp);
    Client::connect(&id).await.expect("trusted on first use");

    // A different certificate than the trusted one: blocked, with the new one's details.
    let mut bad = id.clone();
    bad.trust_cert(&"ab".repeat(32));
    let err = Client::connect(&bad).await.err().expect("refused");
    let r = cert_problem(&err).unwrap();
    assert_eq!(r.problem, "pin-changed");
    assert_eq!(r.fingerprint, internal);
    assert!(err.to_string().contains("HAS CHANGED"), "{err}");

    // inspect() shows the certificate without trusting it.
    let r = itcmdr_admin::inspect(&s.cfg.public_api, "", false)
        .await
        .unwrap();
    assert_eq!(
        (r.problem.as_str(), r.fingerprint.as_str()),
        ("untrusted", internal.as_str())
    );

    // Plain HTTP is never implied.
    assert!(trust::connect(
        &format!("http://{}", s.cfg.public_api),
        &Trust::system(),
        None
    )
    .await
    .is_err());
}

// ---------- Pebble ----------

const PEBBLE: &str = "ghcr.io/letsencrypt/pebble:latest";
const CHALL: &str = "ghcr.io/letsencrypt/pebble-challtestsrv:latest";
const DOMAIN: &str = "adminsvr.example.com";

struct Ctr(Vec<String>);
impl Drop for Ctr {
    fn drop(&mut self) {
        for n in &self.0 {
            let _ = Command::new("podman").args(["rm", "-f", n]).output();
        }
    }
}

fn podman_ok() -> bool {
    if std::env::var_os("ITCMDR_SKIP_PEBBLE").is_some() {
        return false;
    }
    let ok = |img: &str| {
        Command::new("podman")
            .args(["image", "exists", img])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    ok(PEBBLE) && ok(CHALL)
}

fn run(args: &[&str]) {
    let o = Command::new("podman").args(args).output().unwrap();
    assert!(
        o.status.success(),
        "podman {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

async fn get_root(mgmt: u16, minica: &std::path::Path) -> String {
    let c = reqwest::Client::builder()
        .add_root_certificate(
            reqwest::Certificate::from_pem(&std::fs::read(minica).unwrap()).unwrap(),
        )
        .build()
        .unwrap();
    for _ in 0..50 {
        if let Ok(r) = c
            .get(format!("https://localhost:{mgmt}/roots/0"))
            .send()
            .await
        {
            if r.status().is_success() {
                return r.text().await.unwrap();
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("pebble root not available");
}

async fn served_fp(api: &str, root: &str) -> trust::CertReport {
    let t = Trust {
        extra_roots: vec![root.to_string()],
        name: Some(DOMAIN.into()),
        ..Default::default()
    };
    trust::inspect(api, &t).await.unwrap()
}

#[tokio::test]
async fn acme_issues_renews_in_place_and_fails_safe_against_pebble() {
    if !podman_ok() {
        eprintln!(
            "skipped: podman or the pebble images are missing (podman pull {PEBBLE} {CHALL})"
        );
        return;
    }
    let tag = format!("itcmdr-test-{}-{}", std::process::id(), free_port());
    let tmp = tempfile::tempdir().unwrap();
    let (dir_port, mgmt, dns, api_port) = (free_port(), free_port(), free_port(), free_port());
    let mut ctr = Ctr(vec![]);
    // DNS: every name -> 127.0.0.1
    let dns_arg = format!(":{dns}");
    let chall_mgmt = format!(":{}", free_port());
    let chall = format!("{tag}-chall");
    run(&[
        "run",
        "-d",
        "--rm",
        "--network",
        "host",
        "--name",
        &chall,
        CHALL,
        "-defaultIPv4",
        "127.0.0.1",
        "-defaultIPv6",
        "",
        "-dnsserver",
        &dns_arg,
        "-http01",
        "",
        "-https01",
        "",
        "-tlsalpn01",
        "",
        "-doh",
        "",
        "-management",
        &chall_mgmt,
    ]);
    ctr.0.push(chall);
    // Pebble: TLS-ALPN-01 against the adminsvr API port; 30-day certificates so that a 31-day
    // renewal window makes every check renew.
    let cfg = serde_json::json!({"pebble": {
        "listenAddress": format!("0.0.0.0:{dir_port}"),
        "managementListenAddress": format!("0.0.0.0:{mgmt}"),
        "certificate": "/test/certs/localhost/cert.pem",
        "privateKey": "/test/certs/localhost/key.pem",
        "httpPort": free_port(), "tlsPort": api_port, "ocspResponderURL": "",
        "externalAccountBindingRequired": false,
        "profiles": {"default": {"description": "test", "validityPeriod": 2_592_000}}
    }});
    std::fs::write(tmp.path().join("pebble.json"), cfg.to_string()).unwrap();
    let minica = tmp.path().join("minica.pem");
    let cp = format!("{tag}-cp");
    run(&["create", "--name", &cp, PEBBLE]);
    ctr.0.push(cp.clone());
    run(&[
        "cp",
        &format!("{cp}:/test/certs/pebble.minica.pem"),
        minica.to_str().unwrap(),
    ]);
    let peb = format!("{tag}-pebble");
    let vol = format!("{}:/cfg:Z", tmp.path().display());
    let dnsserver = format!("127.0.0.1:{dns}");
    run(&[
        "run",
        "-d",
        "--rm",
        "--network",
        "host",
        "--name",
        &peb,
        "-v",
        &vol,
        "-e",
        "PEBBLE_VA_NOSLEEP=1",
        "-e",
        "PEBBLE_WFE_NONCEREJECT=0",
        "-e",
        "PEBBLE_AUTHZREUSE=0",
        PEBBLE,
        "-config",
        "/cfg/pebble.json",
        "-dnsserver",
        &dnsserver,
    ]);
    ctr.0.push(peb.clone());
    let root = get_root(mgmt, &minica).await;

    let d = common::sqlite_db();
    let cache = tmp.path().join("acme");
    let acme = itcmdr_adminsvr::config::AcmeConfig {
        domains: vec![DOMAIN.into()],
        contact: vec!["admin@example.com".into()],
        directory: format!("https://localhost:{dir_port}/dir"),
        directory_root: Some(minica.clone()),
        challenge: itcmdr_adminsvr::config::Challenge::TlsAlpn01,
        http_addr: String::new(),
        cache_dir: cache.clone(),
        renew_days: 31,
        retry_secs: 3600,
    };
    let s = start_with(&d.url, |c| {
        c.api_addr = format!("127.0.0.1:{api_port}");
        c.public_api = c.api_addr.clone();
        c.acme = Some(acme.clone());
    })
    .await;
    let (o, e) = owner(&s).await;
    // First certificate (the loop starts at once).
    let mut st = Default::default();
    for _ in 0..150 {
        st = itcmdr_adminsvr::acme::load_status(&common::pool(&d.url).await).await;
        if st.expires_ms > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(st.expires_ms > 0, "no certificate: {st:?}");
    let r1 = served_fp(&s.cfg.api_addr, &root).await;
    assert_eq!(r1.problem, "ok", "{r1:?}");
    assert_eq!(r1.fingerprint, st.fingerprint);
    assert!(r1.names.contains(&DOMAIN.to_string()));
    // Without Pebble's root the same certificate is untrusted (the user would be asked).
    let plain = trust::inspect(
        &s.cfg.api_addr,
        &Trust {
            name: Some(DOMAIN.into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        (plain.problem.as_str(), plain.fingerprint.as_str()),
        ("untrusted", r1.fingerprint.as_str())
    );
    // Enrolled devices still get the internal certificate they pinned.
    let i = o
        .api
        .clone()
        .info(itcmdr_admin::proto::Empty {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(i.tls_mode, "acme");
    assert_eq!(i.tls_domains, vec![DOMAIN.to_string()]);
    assert!(i.tls_expires_ms > 0 && !i.tls_renewal_failing);
    assert_ne!(i.internal_fingerprint, r1.fingerprint);
    // Secure cache: 0700 dir, 0600 files.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let m = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(m(&cache), 0o700);
        for f in ["account.json", "cert.pem", "key.pem"] {
            assert_eq!(m(&cache.join(f)), 0o600, "{f}");
        }
    }

    // Renewal in place: restarting adminsvr re-checks at once; inside the 31-day window it
    // issues a new certificate and serves it without another restart.
    let pool = common::pool(&d.url).await;
    let hub = itcmdr_adminsvr::link::Hub::default();
    let before = st.renewed_ms;
    drop(s);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let s2 = start_with(&d.url, |c| {
        c.api_addr = format!("127.0.0.1:{api_port}");
        c.public_api = c.api_addr.clone();
        c.acme = Some(acme.clone());
    })
    .await;
    let mut st2 = Default::default();
    for _ in 0..150 {
        st2 = itcmdr_adminsvr::acme::load_status(&pool).await;
        if st2.renewed_ms > before {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let r2 = served_fp(&s2.cfg.api_addr, &root).await;
    assert!(st2.renewed_ms > before, "{st2:?}");
    assert_ne!(
        r2.fingerprint, r1.fingerprint,
        "renewed certificate is served"
    );
    assert_eq!(r2.fingerprint, st2.fingerprint);

    // Fail safe: the CA is gone; the check fails, the current certificate stays, alert opens.
    run(&["rm", "-f", &peb]);
    let res3 = itcmdr_adminsvr::acme::Resolver::new(
        itcmdr_agentlink::tls::certified_key(
            &std::fs::read_to_string(s2.cfg.data_dir.join("tls/server.pem")).unwrap(),
            &std::fs::read_to_string(s2.cfg.data_dir.join("tls/server.key")).unwrap(),
        )
        .unwrap(),
        &acme.domains,
    );
    itcmdr_adminsvr::acme::load_cached(&acme, &res3).unwrap();
    let tokens: itcmdr_adminsvr::acme::Tokens = Default::default();
    let exp = itcmdr_adminsvr::acme::check_once(&pool, &hub, &acme, &res3, &tokens).await;
    assert_eq!(exp, st2.expires_ms, "keeps the current certificate");
    let st3 = itcmdr_adminsvr::acme::load_status(&pool).await;
    assert!(
        st3.last_error_ms > st3.renewed_ms && !st3.last_error.is_empty(),
        "{st3:?}"
    );
    assert_eq!(st3.fingerprint, st2.fingerprint);
    let r3 = served_fp(&s2.cfg.api_addr, &root).await;
    assert_eq!(
        r3.fingerprint, r2.fingerprint,
        "still serving the good certificate"
    );
    // The restarted server has its own data dir (new internal pin) but the same database.
    let mut id2 = e.identity.clone();
    id2.addr = s2.cfg.api_addr.clone();
    id2.pin = itcmdr_agentlink::tls::pem_fingerprint(
        &std::fs::read_to_string(s2.cfg.data_dir.join("tls/server.pem")).unwrap(),
    )
    .unwrap();
    let o2 = Client::connect(&id2).await.unwrap();
    let alerts = o2
        .api
        .clone()
        .list_alerts(itcmdr_admin::signed(
            &id2.key().unwrap(),
            "/itcmdr.admin.AdminApi/ListAlerts",
            itcmdr_admin::proto::AlertFilter::default(),
        ))
        .await
        .unwrap()
        .into_inner()
        .alerts;
    assert!(
        alerts.iter().any(|a| a.site == "adminsvr"
            && a.alert
                .as_ref()
                .is_some_and(|x| x.id == "tls-acme" && x.state == "open")),
        "{alerts:?}"
    );
    let i = o2
        .api
        .clone()
        .info(itcmdr_admin::proto::Empty {})
        .await
        .unwrap()
        .into_inner();
    assert!(i.tls_renewal_failing);
    drop(ctr);
}
