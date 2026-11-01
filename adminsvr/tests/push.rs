//! v0.19.2 push notifications against a local FCM + APNs stub (no Google/Apple accounts needed):
//! - payloads carry only opaque ids and the severity: no site name, alert code, machine or user;
//! - only active devices of users who may see the site get a push (SRE scopes), once per opening;
//! - below the minimum severity nothing is sent; gone tokens are deleted; every send is audited.
mod common;
use itcmdr_adminsvr::config::PushConfig;
use itcmdr_adminsvr::pb;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone)]
struct Hit {
    path: String,
    headers: String,
    body: String,
}

/// A tiny HTTP/1.1 stub: /token (OAuth), /v1/projects/*/messages:send (FCM), /3/device/* (APNs).
async fn stub() -> (String, Arc<Mutex<Vec<Hit>>>) {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let hits: Arc<Mutex<Vec<Hit>>> = Arc::default();
    let h = hits.clone();
    tokio::spawn(async move {
        loop {
            let (mut s, _) = l.accept().await.unwrap();
            let h = h.clone();
            tokio::spawn(async move {
                loop {
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 4096];
                    let (head, rest) = loop {
                        let n = match s.read(&mut tmp).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => n,
                        };
                        buf.extend_from_slice(&tmp[..n]);
                        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            break (
                                String::from_utf8_lossy(&buf[..i]).to_string(),
                                buf[i + 4..].to_vec(),
                            );
                        }
                    };
                    let len: usize = head
                        .lines()
                        .find_map(|l| {
                            let (k, v) = l.split_once(':')?;
                            k.eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse().ok())?
                        })
                        .unwrap_or(0);
                    let mut body = rest;
                    while body.len() < len {
                        let n = s.read(&mut tmp).await.unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        body.extend_from_slice(&tmp[..n]);
                    }
                    let path = head
                        .lines()
                        .next()
                        .unwrap_or("")
                        .split(' ')
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    let body = String::from_utf8_lossy(&body).to_string();
                    let (code, resp) = if path == "/token" {
                        (
                            200,
                            r#"{"access_token":"stub-access","expires_in":3600}"#.to_string(),
                        )
                    } else if body.contains("gone-token") {
                        (404, r#"{"error":{"status":"NOT_FOUND","details":[{"errorCode":"UNREGISTERED"}]}}"#.to_string())
                    } else {
                        (200, "{}".to_string())
                    };
                    h.lock().unwrap().push(Hit {
                        path,
                        headers: head,
                        body,
                    });
                    let r = format!("HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{resp}", resp.len());
                    if s.write_all(r.as_bytes()).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    (url, hits)
}

fn push_config(dir: &std::path::Path, url: &str) -> PushConfig {
    use p256::pkcs8::EncodePrivateKey;
    let rk = rsa::RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
    let pem = rk.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF).unwrap();
    let sa = serde_json::json!({
        "project_id": "stub-project", "client_email": "push@stub.invalid",
        "private_key": pem.as_str(), "token_uri": format!("{url}/token"),
    });
    std::fs::write(dir.join("sa.json"), sa.to_string()).unwrap();
    let ek = p256::SecretKey::random(&mut rand::rngs::OsRng);
    std::fs::write(
        dir.join("apns.p8"),
        ek.to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    PushConfig {
        fcm_service_account: Some(dir.join("sa.json")),
        fcm_url: url.into(),
        apns_key: Some(dir.join("apns.p8")),
        apns_key_id: "KEYID12345".into(),
        apns_team_id: "TEAMID1234".into(),
        apns_topic: "example.itcmdr.admin".into(),
        apns_url: url.into(),
        min_severity: "cautious".into(),
    }
}

async fn wait_hits(h: &Arc<Mutex<Vec<Hit>>>, n: usize) {
    for _ in 0..100 {
        if sends(h).len() >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("expected {n} pushes, got {:?}", sends(h));
}

fn sends(h: &Arc<Mutex<Vec<Hit>>>) -> Vec<Hit> {
    h.lock()
        .unwrap()
        .iter()
        .filter(|x| x.path != "/token")
        .cloned()
        .collect()
}

fn alert(id: &str, code: &str, sev: &str, state: &str) -> pb::SiteAlert {
    pb::SiteAlert {
        id: id.into(),
        code: code.into(),
        severity: sev.into(),
        state: state.into(),
        source: "agent".into(),
        machine: "m-secretmachine".into(),
        machine_display_name: "SECRET-PC-NAME".into(),
        first_seen_ms: 1,
        last_seen_ms: 2,
        count: 1,
        ..Default::default()
    }
}

both!(
    push_is_redacted_scoped_and_audited,
    push_sqlite,
    push_postgres
);

async fn push_is_redacted_scoped_and_audited(db: String) {
    let (url, hits) = stub().await;
    let keys = tempfile::tempdir().unwrap();
    let pc = push_config(keys.path(), &url);
    let pc2 = pc.clone();
    let a = common::start_with(&db, move |c| c.push = pc2).await;
    let (o, _) = common::owner(&a).await;
    let site = o
        .create_site("Secret Office Name")
        .await
        .unwrap()
        .site
        .unwrap()
        .id;
    // An SRE without the site in scope.
    let cu = o
        .create_user("sre@example.com", "Sre S", "sre")
        .await
        .unwrap();
    let se = itcmdr_admin::enroll(&cu.enroll_code, "test", false, None)
        .await
        .unwrap();
    o.approve_device("sre@example.com", &se.user.devices[0].fingerprint)
        .await
        .unwrap();
    let sre = itcmdr_admin::Client::connect(&se.identity).await.unwrap();

    assert!(
        o.register_push("smtp", "x", "").await.is_err(),
        "unknown provider refused"
    );
    o.register_push("fcm", "owner-fcm-token", "android")
        .await
        .unwrap();
    o.register_push("apns", "aabbccdd00112233", "ios")
        .await
        .unwrap();
    o.register_push("fcm", "gone-token", "android")
        .await
        .unwrap();
    sre.register_push("fcm", "sre-fcm-token", "android")
        .await
        .unwrap();

    // Test push: own devices only; the gone token is reported and deleted.
    let r = o.test_push().await.unwrap();
    assert_eq!((r.devices, r.sent, r.failed), (3, 2, 1), "{r:?}");
    assert_eq!(r.providers, vec!["fcm", "apns"]);
    assert_eq!(o.test_push().await.unwrap().devices, 2);
    let n0 = sends(&hits).len();
    assert!(sends(&hits)
        .iter()
        .all(|h| !h.body.contains("sre-fcm-token")));

    // A new critical alert reaches the owner's two devices, not the out-of-scope SRE.
    let pool = itcmdr_adminsvr::store::connect(&a.cfg.db_url)
        .await
        .unwrap();
    let hub = itcmdr_adminsvr::link::Hub::with_push(itcmdr_adminsvr::push::Push::new(&pc).unwrap());
    let secret_code = "agent:m-secretmachine:disk-low-SECRETCODE";
    let crit = alert("al-1", secret_code, "critical", "open");
    itcmdr_adminsvr::link::store_and_push(&pool, &hub, &site, "site", &crit)
        .await
        .unwrap();
    wait_hits(&hits, n0 + 2).await;
    // The same alert again (still open): no second push.
    itcmdr_adminsvr::link::store_and_push(&pool, &hub, &site, "site", &crit)
        .await
        .unwrap();
    // Informational: below the minimum, nothing.
    itcmdr_adminsvr::link::store_and_push(
        &pool,
        &hub,
        &site,
        "site",
        &alert("al-2", "x", "informational", "open"),
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let s1 = sends(&hits);
    assert_eq!(s1.len(), n0 + 2, "{s1:?}");

    // Scope the SRE to the site: the next opening reaches the SRE too.
    o.set_scopes(&se.user.id, vec![site.clone()]).await.unwrap();
    itcmdr_adminsvr::link::store_and_push(
        &pool,
        &hub,
        &site,
        "site",
        &alert("al-3", secret_code, "cautious", "open"),
    )
    .await
    .unwrap();
    wait_hits(&hits, n0 + 5).await;
    assert!(sends(&hits)
        .iter()
        .any(|h| h.body.contains("sre-fcm-token")));
    // Resolve and reopen: pushed again.
    itcmdr_adminsvr::link::store_and_push(
        &pool,
        &hub,
        &site,
        "site",
        &alert("al-3", secret_code, "cautious", "resolved"),
    )
    .await
    .unwrap();
    itcmdr_adminsvr::link::store_and_push(
        &pool,
        &hub,
        &site,
        "site",
        &alert("al-3", secret_code, "cautious", "open"),
    )
    .await
    .unwrap();
    wait_hits(&hits, n0 + 8).await;

    // Disabled user: nothing more for the SRE.
    o.set_disabled(&se.user.id, true).await.unwrap();
    itcmdr_adminsvr::link::store_and_push(
        &pool,
        &hub,
        &site,
        "site",
        &alert("al-4", "y", "critical", "open"),
    )
    .await
    .unwrap();
    wait_hits(&hits, n0 + 10).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let all = sends(&hits);
    assert_eq!(all.len(), n0 + 10);
    assert_eq!(
        all[n0 + 8..]
            .iter()
            .filter(|h| h.body.contains("sre-fcm-token"))
            .count(),
        0
    );

    // Redaction: every payload has only t/site/alert/sev data and the fixed text.
    for h in &all {
        for bad in [
            "Secret Office Name",
            "SECRETCODE",
            "secretmachine",
            "SECRET-PC-NAME",
            "example.com",
            "Owner O",
            "Sre S",
        ] {
            assert!(!h.body.contains(bad), "{bad} leaked in push: {}", h.body);
        }
        let v: Value = serde_json::from_str(&h.body).unwrap();
        let data = if h
            .path
            .starts_with("/v1/projects/stub-project/messages:send")
        {
            assert!(h
                .headers
                .to_lowercase()
                .contains("authorization: bearer stub-access"));
            let m = &v["message"];
            assert!(m["notification"]["title"]
                .as_str()
                .unwrap()
                .starts_with("itcmdr: "));
            m["data"].clone()
        } else {
            assert!(
                h.path.starts_with("/3/device/aabbccdd00112233"),
                "{}",
                h.path
            );
            let hd = h.headers.to_lowercase();
            assert!(
                hd.contains("apns-topic: example.itcmdr.admin")
                    && hd.contains("authorization: bearer ")
            );
            let mut d = v.clone();
            d.as_object_mut().unwrap().remove("aps");
            d
        };
        let mut keys: Vec<&String> = data.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["alert", "sev", "site", "t"], "{data}");
        assert!(["", site.as_str()].contains(&data["site"].as_str().unwrap()));
    }

    // Every send is audited.
    let au = o.list_audit("", 500).await.unwrap().entries;
    let n = au.iter().filter(|e| e.action == "push.send").count();
    assert!(n >= 6, "push.send audited: {n}");
    assert!(au.iter().any(|e| e.action == "push.register"));
    assert!(o.verify_audit().await.unwrap().ok);

    // Unregister: this device's tokens are gone.
    o.unregister_push("").await.unwrap();
    assert_eq!(o.test_push().await.unwrap().devices, 0);
}

#[tokio::test]
async fn push_off_without_config() {
    let d = common::sqlite_db();
    let a = common::start(&d.url).await;
    let (o, _) = common::owner(&a).await;
    o.register_push("fcm", "t1", "android").await.unwrap();
    let r = o.test_push().await.unwrap();
    assert_eq!((r.devices, r.sent, r.failed), (1, 0, 1));
    assert!(r.providers.is_empty());
}
