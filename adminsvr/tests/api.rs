//! v0.19.0 adminsvr admin API: owner/admin/sre rules, signed requests (unsigned, wrong key,
//! replay, revoked, disabled refused), device approval, the audit hash chain (tamper caught),
//! client-side encrypted key backup and new devices from the 24 words, CLI parity.
mod common;
use common::{owner, start};
use itcmdr_admin::{proto as pb, Client};

/// Create + enroll + approve a user; returns their client.
async fn user(o: &Client, email: &str, role: &str) -> (Client, itcmdr_admin::Enrolled) {
    let r = o.create_user(email, "User U", role).await.unwrap();
    let e = itcmdr_admin::enroll(&r.enroll_code, "test", false, None)
        .await
        .unwrap();
    let fp = e.user.devices[0].fingerprint.clone();
    assert_eq!(e.user.devices[0].status, "pending");
    o.approve_device(email, &fp).await.unwrap();
    (Client::connect(&e.identity).await.unwrap(), e)
}

both!(roles_and_signed_requests, roles_sqlite, roles_postgres);

async fn roles_and_signed_requests(db: String) {
    let s = start(&db).await;
    let (o, oe) = owner(&s).await;
    assert!(
        itcmdr_adminsvr::api::init(&s.cfg, "x@example.com", "X")
            .await
            .is_err(),
        "init twice"
    );

    // New users default to sre; a pending device can only ask who it is.
    let r = o
        .create_user("admin@example.com", "Admin A", "admin")
        .await
        .unwrap();
    let e = itcmdr_admin::enroll(&r.enroll_code, "test", false, None)
        .await
        .unwrap();
    let pending = Client::connect(&e.identity).await.unwrap();
    assert_eq!(pending.whoami().await.unwrap().devices[0].status, "pending");
    let err = pending.list_users().await.unwrap_err().to_string();
    assert!(err.contains("waiting for approval"), "{err}");
    // the code is single use
    assert!(itcmdr_admin::enroll(&r.enroll_code, "test", false, None)
        .await
        .is_err());
    o.approve_device("admin@example.com", &e.user.devices[0].fingerprint)
        .await
        .unwrap();
    let a = pending;
    let sre_default = a.create_user("sre@example.com", "Sre S", "").await.unwrap();
    assert_eq!(sre_default.user.unwrap().role, "sre");

    // Admins can't create or promote owners, or touch owners; can't promote themselves.
    assert!(a
        .create_user("o2@example.com", "O2", "owner")
        .await
        .is_err());
    assert!(a
        .set_role("sre@example.com", "owner")
        .await
        .unwrap_err()
        .to_string()
        .contains("can't grant owner"));
    assert!(a.set_role("owner@example.com", "sre").await.is_err());
    assert!(a.set_disabled("owner@example.com", true).await.is_err());
    assert!(a
        .set_role("", "owner")
        .await
        .unwrap_err()
        .to_string()
        .contains("promote yourself"));
    assert!(a.set_role("sre@example.com", "admin").await.is_ok());
    assert!(a.set_role("sre@example.com", "sre").await.is_ok());
    // The last owner can't be demoted or disabled, even by themselves.
    assert!(o
        .set_role("", "admin")
        .await
        .unwrap_err()
        .to_string()
        .contains("last owner"));
    assert!(o.set_disabled("", true).await.is_err());

    // SREs: only themselves in Users, no site management.
    let (sre, _) = user(&o, "sre2@example.com", "sre").await;
    let us = sre.list_users().await.unwrap().users;
    assert_eq!(us.len(), 1);
    assert_eq!(us[0].email, "sre2@example.com");
    assert!(sre.create_site("Office").await.is_err());
    assert!(sre.create_user("z@example.com", "Z", "sre").await.is_err());
    assert!(sre.verify_audit().await.is_err());
    assert!(
        sre.list_sites().await.unwrap().sites.is_empty(),
        "SREs start with no sites"
    );
    let site = o.create_site("Office").await.unwrap().site.unwrap();
    o.set_scopes("sre2@example.com", vec![site.id.clone()])
        .await
        .unwrap();
    assert_eq!(sre.list_sites().await.unwrap().sites.len(), 1);

    // A second owner, then self-demotion is fine.
    let (o2, _) = user(&o, "owner2@example.com", "owner").await;
    o2.set_role("", "admin").await.unwrap();
    assert_eq!(o2.whoami().await.unwrap().role, "admin");

    // Unsigned: refused.
    let id = oe.identity.clone();
    let mut raw = pb::admin_api_client::AdminApiClient::new(
        itcmdr_admin::channel(&id.addr, &id.pin).await.unwrap(),
    );
    let st = raw.list_users(pb::Empty {}).await.unwrap_err();
    assert_eq!(st.code(), tonic::Code::Unauthenticated, "{st:?}");
    // Signed by another key under the owner's fingerprint: refused.
    let other = itcmdr_keys::new_keypair();
    let mut req = itcmdr_admin::signed(&other, "/itcmdr.admin.AdminApi/ListUsers", pb::Empty {});
    let ofp = o.whoami().await.unwrap().devices[0].fingerprint.clone();
    req.metadata_mut()
        .insert("x-itcmdr-key", ofp.parse().unwrap());
    let st = raw.list_users(req).await.unwrap_err();
    assert_eq!(st.code(), tonic::Code::Unauthenticated, "{st:?}");
    // Signature over another method: refused. Replayed request: refused.
    let k = o.key().clone();
    let st = raw
        .list_users(itcmdr_admin::signed(
            &k,
            "/itcmdr.admin.AdminApi/ListSites",
            pb::Empty {},
        ))
        .await
        .unwrap_err();
    assert_eq!(st.code(), tonic::Code::Unauthenticated);
    let r1 = itcmdr_admin::signed(&k, "/itcmdr.admin.AdminApi/ListUsers", pb::Empty {});
    let meta = r1.metadata().clone();
    raw.list_users(r1).await.unwrap();
    let mut r2 = tonic::Request::new(pb::Empty {});
    *r2.metadata_mut() = meta;
    assert!(raw
        .list_users(r2)
        .await
        .unwrap_err()
        .message()
        .contains("replayed"));
    // Revoked device: refused at once. Disabled user: refused.
    let sfp = sre.whoami().await.unwrap().devices[0].fingerprint.clone();
    o.revoke_device("sre2@example.com", &sfp).await.unwrap();
    assert!(sre
        .whoami()
        .await
        .unwrap_err()
        .to_string()
        .contains("revoked"));
    o.set_disabled("admin@example.com", true).await.unwrap();
    assert!(a
        .whoami()
        .await
        .unwrap_err()
        .to_string()
        .contains("disabled"));
    o.set_disabled("admin@example.com", false).await.unwrap();
    a.whoami().await.unwrap();

    // Every change is in the chain, and the chain verifies.
    let v = o.verify_audit().await.unwrap();
    assert!(v.ok && v.entries >= 15, "{v:?}");
    let acts: Vec<String> = o
        .list_audit("", 500)
        .await
        .unwrap()
        .entries
        .into_iter()
        .map(|e| e.action)
        .collect();
    for want in [
        "init",
        "user.create",
        "device.enroll",
        "device.approve",
        "user.role",
        "user.disable",
        "user.scopes",
        "site.create",
        "device.revoke",
    ] {
        assert!(acts.iter().any(|a| a == want), "missing {want} in {acts:?}");
    }
    // SREs see only their own entries.
    let (sre3, e3) = user(&o, "sre3@example.com", "sre").await;
    assert!(sre3
        .list_audit("", 100)
        .await
        .unwrap()
        .entries
        .iter()
        .all(|x| x.actor == e3.user.id));
}

both!(
    audit_tamper_is_caught,
    audit_tamper_sqlite,
    audit_tamper_postgres
);

async fn audit_tamper_is_caught(db: String) {
    let s = start(&db).await;
    let (o, _) = owner(&s).await;
    for i in 0..4 {
        o.create_user(&format!("u{i}@example.com"), "U", "sre")
            .await
            .unwrap();
    }
    assert!(o.verify_audit().await.unwrap().ok);
    let pool = itcmdr_adminsvr::store::connect(&db).await.unwrap();
    pool.exec("UPDATE audit_chain SET action = 'user.nothing' WHERE seq = 3")
        .await
        .unwrap();
    let v = o.verify_audit().await.unwrap();
    assert!(!v.ok && v.first_bad_seq == 3, "{v:?}");
    // recompute that row's hash: the next row's prev_hash no longer matches
    let r: (
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ) = pool
        .run(|c| {
            use diesel::prelude::*;
            use itcmdr_adminsvr::schema::audit_chain::dsl::*;
            Ok(audit_chain
                .filter(seq.eq(3i64))
                .select((
                    ts_ms,
                    actor,
                    actor_key,
                    action,
                    target,
                    payload_sha256,
                    request_sig,
                    prev_hash,
                    hash,
                ))
                .first(c)?)
        })
        .await
        .unwrap();
    let h = itcmdr_adminsvr::audit::link_hash(&r.7, 3, r.0, &r.1, &r.2, &r.3, &r.4, &r.5, &r.6);
    pool.exec(&format!(
        "UPDATE audit_chain SET hash = '{h}' WHERE seq = 3"
    ))
    .await
    .unwrap();
    let v = o.verify_audit().await.unwrap();
    assert!(!v.ok && v.first_bad_seq == 4, "{v:?}");
    pool.exec("DELETE FROM audit_chain WHERE seq = 2")
        .await
        .unwrap();
    let v = itcmdr_adminsvr::audit::verify(&pool).await.unwrap();
    assert!(
        !v.ok && v.first_bad_seq == 3 && v.problem.contains("gap"),
        "{v:?}"
    );
}

both!(
    key_backup_and_new_devices_from_words,
    key_backup_sqlite,
    key_backup_postgres
);

async fn key_backup_and_new_devices_from_words(db: String) {
    let s = start(&db).await;
    let (o, oe) = owner(&s).await;
    let pool = itcmdr_adminsvr::store::connect(&db).await.unwrap();
    // On by default: only ciphertext on adminsvr.
    let me = o.whoami().await.unwrap();
    assert!(me.key_backup && !me.key_backup_opt_out);
    let blob: String = pool
        .run(|c| {
            use diesel::prelude::*;
            use itcmdr_adminsvr::schema::key_backups::dsl::*;
            Ok(key_backups.select(blob).first(c)?)
        })
        .await
        .unwrap();
    assert!(!blob.contains(&oe.identity.device_seed));
    for w in oe.words.split_whitespace() {
        assert!(!blob.contains(&format!("\"{w}\"")));
    }
    // New device: restore the same key from the backup with the words.
    let id = itcmdr_admin::recover(
        &s.cfg.public_api,
        &oe.identity.pin,
        "owner@example.com",
        &oe.words,
        true,
        "t",
        None,
    )
    .await
    .unwrap();
    assert_eq!(id.device_seed, oe.identity.device_seed);
    // Wrong words: refused (the master key is unknown to adminsvr).
    assert!(itcmdr_admin::recover(
        &s.cfg.public_api,
        &oe.identity.pin,
        "owner@example.com",
        &itcmdr_keys::generate_words(),
        true,
        "t",
        None,
    )
    .await
    .is_err());
    // New device key certified by the master (AddDevice), active at once.
    let id2 = itcmdr_admin::recover(
        &s.cfg.public_api,
        &oe.identity.pin,
        "owner@example.com",
        &oe.words,
        false,
        "t",
        None,
    )
    .await
    .unwrap();
    let c2 = Client::connect(&id2).await.unwrap();
    assert_eq!(c2.whoami().await.unwrap().devices.len(), 2);
    // The master key only does recovery calls.
    let master = itcmdr_keys::Master::from_words(&oe.words).unwrap();
    let mc = Client::with_key(&s.cfg.public_api, &oe.identity.pin, master.signing.clone())
        .await
        .unwrap();
    assert!(mc
        .list_users()
        .await
        .unwrap_err()
        .to_string()
        .contains("master key"));
    // Opt out: the blob is deleted, audited.
    let u = o.key_backup_opt_out(true).await.unwrap();
    assert!(u.key_backup_opt_out && !u.key_backup);
    let n: i64 = pool
        .run(|c| {
            use diesel::prelude::*;
            Ok(itcmdr_adminsvr::schema::key_backups::table
                .count()
                .get_result(c)?)
        })
        .await
        .unwrap();
    assert_eq!(n, 0);
    // A user enrolled with --no-key-backup has none.
    let r = o.create_user("nb@example.com", "N", "sre").await.unwrap();
    let e = itcmdr_admin::enroll(&r.enroll_code, "t", true, None)
        .await
        .unwrap();
    assert!(e.user.key_backup_opt_out && !e.user.key_backup);
    let acts: Vec<String> = o
        .list_audit("", 100)
        .await
        .unwrap()
        .entries
        .into_iter()
        .map(|e| e.action)
        .collect();
    for want in ["keybackup.get", "device.add", "keybackup.opt-out"] {
        assert!(acts.iter().any(|a| a == want), "missing {want}");
    }
}

#[test]
fn cli_parity_covers_every_api_method() {
    let proto =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../proto/admin.proto"))
            .unwrap();
    let svc = &proto[proto.find("service AdminApi").unwrap()..];
    let svc = &svc[..svc.find("\n}").unwrap()];
    let methods: Vec<&str> = svc
        .lines()
        .filter_map(|l| l.trim().strip_prefix("rpc "))
        .map(|l| l.split('(').next().unwrap().trim())
        .collect();
    assert!(methods.len() >= 25);
    for m in &methods {
        assert!(
            itcmdr_admin::PARITY.iter().any(|(a, _)| a == m),
            "no CLI command for {m}"
        );
    }
    assert_eq!(methods.len(), itcmdr_admin::PARITY.len());
}
