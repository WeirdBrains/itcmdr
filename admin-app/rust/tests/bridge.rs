//! The app's bridge end to end against an in-process adminsvr (SQLite): enroll, reopen, users,
//! servers, alerts, metrics, policy, audit, push (off), forget. Same client code as the CLI.
use itcmdr_admin_bridge::api::admin as a;
use std::time::Duration;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

#[test]
fn bridge_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let db = format!("sqlite://{}", tmp.path().join("adminsvr.db").display());
    let cfg = itcmdr_adminsvr::config::Config::for_test(tmp.path().join("data"), &db, free_port(), free_port());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let c2 = cfg.clone();
    rt.spawn(async move {
        let _ = itcmdr_adminsvr::run(c2, std::future::pending()).await;
    });
    for _ in 0..100 {
        if std::net::TcpStream::connect(&cfg.api_addr).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let code = rt.block_on(itcmdr_adminsvr::api::init(&cfg, "owner@example.com", "Owner O")).unwrap();
    let dir = tmp.path().join("app").display().to_string();

    assert!(a::open(dir.clone()).unwrap().is_none(), "not enrolled yet");
    let e = a::enroll(dir.clone(), code, false).unwrap();
    assert_eq!(e.words.split_whitespace().count(), 24);
    assert_eq!((e.me.role.as_str(), e.me.email.as_str()), ("owner", "owner@example.com"));
    assert!(a::enroll(dir.clone(), "x".into(), false).is_err(), "already enrolled");
    // Reopen from the stored identity.
    let me = a::open(dir.clone()).unwrap().unwrap();
    assert_eq!(me.id, e.me.id);
    assert_eq!(a::me().unwrap().name, "Owner O");

    // Users per role.
    let code = a::create_user("sre@example.com".into(), "Sre S".into(), "sre".into()).unwrap();
    assert!(code.starts_with("itcmdr-admin:"));
    assert!(a::create_user("x@example.com".into(), "X".into(), "root".into()).is_err());
    let us = a::users().unwrap();
    assert_eq!(us.len(), 2);
    let sre = us.iter().find(|u| u.role == "sre").unwrap();
    let u = a::set_role(sre.id.clone(), "admin".into()).unwrap();
    assert_eq!(u.role, "admin");
    assert_eq!(a::set_disabled(sre.id.clone(), true).unwrap().status, "disabled");

    // Servers.
    let tok = a::create_site("Office A".into()).unwrap();
    assert!(!tok.is_empty());
    let s = a::sites().unwrap();
    assert_eq!((s.len(), s[0].status.as_str()), (1, "pending"));
    assert!(a::metrics(s[0].id.clone(), 24).unwrap().is_empty());
    assert!(a::alerts("".into(), "open".into()).unwrap().is_empty());
    assert!(a::commands(s[0].id.clone()).unwrap().is_empty());

    // Policy, passkeys, audit, push (not configured).
    let p = a::policy().unwrap();
    assert!(p.stepup_actions.contains(&"SetRole".to_string()));
    assert!(a::passkeys("".into()).unwrap().is_empty());
    assert!(a::verify_audit().unwrap().starts_with("ok"));
    assert!(a::audit(50).unwrap().iter().any(|r| r.action.contains("site")));
    a::register_push("fcm".into(), "tok".into(), "android".into()).unwrap();
    let r = a::test_push().unwrap();
    assert_eq!((r.devices, r.sent), (1, 0));
    a::unregister_push().unwrap();

    a::forget(dir.clone()).unwrap();
    assert!(a::me().is_err());
    assert!(a::open(dir).unwrap().is_none());
}
