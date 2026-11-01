//! v0.19.1 passkeys / security keys as a second factor (ADMINSVR.md "Passkeys"), with a software
//! authenticator, on SQLite and Postgres. Required: step-up refused without a touch and when the
//! challenge belongs to another request; a replayed assertion is refused; owner-only enforcement;
//! a lost-passkey reset is audited; the local device key unlocks only with its passkey.
mod common;
use common::{owner, start};
use itcmdr_admin::passkey::{soft::Soft, Authenticator};
use itcmdr_admin::{proto as pb, signed, Client};
use std::sync::atomic::Ordering;
use std::sync::Arc;

async fn user(o: &Client, email: &str, role: &str) -> (Client, itcmdr_admin::Enrolled) {
    let r = o.create_user(email, "User U", role).await.unwrap();
    let e = itcmdr_admin::enroll(&r.enroll_code, "test", false, None)
        .await
        .unwrap();
    o.approve_device(email, &e.user.devices[0].fingerprint)
        .await
        .unwrap();
    (Client::connect(&e.identity).await.unwrap(), e)
}

const SET_ROLE: &str = "/itcmdr.admin.AdminApi/SetRole";

async fn set_role_with(
    c: &Client,
    key: &ed25519_dalek::SigningKey,
    req: pb::SetRoleRequest,
    header: Option<&str>,
) -> Result<pb::UserInfo, tonic::Status> {
    let mut r = signed(key, SET_ROLE, req);
    if let Some(h) = header {
        r.metadata_mut()
            .insert("x-itcmdr-stepup", h.parse().unwrap());
    }
    c.api.clone().set_role(r).await.map(|r| r.into_inner())
}

async fn audit_actions(o: &Client) -> Vec<(String, String)> {
    o.list_audit("", 1000)
        .await
        .unwrap()
        .entries
        .into_iter()
        .map(|e| (e.action, e.target))
        .collect()
}

both!(passkey_second_factor, passkeys_sqlite, passkeys_postgres);

async fn passkey_second_factor(db: String) {
    let s = start(&db).await;
    let (o0, oe) = owner(&s).await;
    let (_sre, _) = user(&o0, "sre@example.com", "sre").await;
    let soft = Arc::new(Soft::default());
    let auth: Arc<dyn Authenticator> = soft.clone();
    let o = Client::connect_with(&oe.identity, Some(auth.clone()))
        .await
        .unwrap();

    // Optional: no passkey yet, no touch needed.
    o0.set_role("sre@example.com", "admin").await.unwrap();
    // Register (the first one needs no touch); a second one needs a touch from the first.
    let (lp, l) = o.add_passkey("yubikey", true).await.unwrap();
    assert_eq!(l.passkeys.len(), 1);
    assert_eq!(l.passkeys[0].attestation, "self");
    assert!(
        o0.add_passkey("x", false).await.is_err(),
        "no authenticator, no registration"
    );
    let (_lp2, l) = o.add_passkey("backup key", false).await.unwrap();
    assert_eq!(l.passkeys.len(), 2);
    assert_eq!(o.whoami().await.unwrap().passkeys, 2);

    // Now step-up actions need a touch: without an authenticator they are refused...
    let e = o0
        .set_role("sre@example.com", "sre")
        .await
        .unwrap_err()
        .to_string();
    assert!(e.starts_with("stepup-required"), "{e}");
    // ...with one they pass, and the assertion is in the audit chain.
    o.set_role("sre@example.com", "sre").await.unwrap();
    let a = audit_actions(&o).await;
    assert!(
        a.iter()
            .any(|(x, t)| x == "passkey.stepup" && t == "SetRole"),
        "{a:?}"
    );
    // Non-step-up actions don't ask.
    soft.no_touch.store(true, Ordering::SeqCst);
    o.list_users().await.unwrap();

    // No touch (user presence flag missing): refused.
    let e = o
        .set_role("sre@example.com", "admin")
        .await
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("step-up refused") && e.contains("presence"),
        "{e}"
    );
    soft.no_touch.store(false, Ordering::SeqCst);

    // Replay: the same assertion twice; and an assertion for another request.
    let key = oe.identity.key().unwrap();
    let req = pb::SetRoleRequest {
        user: "sre@example.com".into(),
        role: "admin".into(),
    };
    let body = prost::Message::encode_to_vec(&req);
    let h = o.stepup(SET_ROLE, &body).await.unwrap();
    set_role_with(&o, &key, req.clone(), Some(&h))
        .await
        .unwrap();
    let e = set_role_with(&o, &key, req.clone(), Some(&h))
        .await
        .unwrap_err();
    assert!(e.message().contains("already used"), "{e:?}");
    let other = pb::SetRoleRequest {
        user: "sre@example.com".into(),
        role: "sre".into(),
    };
    let h = o.stepup(SET_ROLE, &body).await.unwrap(); // touched for "admin"...
    let e = set_role_with(&o, &key, other.clone(), Some(&h))
        .await
        .unwrap_err(); // ...sent for "sre"
    assert!(e.message().contains("challenge does not match"), "{e:?}");
    // A frozen counter (cloned key) is refused (first a good touch, so adminsvr has the count).
    o.set_role("sre@example.com", "admin").await.unwrap();
    soft.freeze_counter.store(true, Ordering::SeqCst);
    let e = o
        .set_role("sre@example.com", "sre")
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("sign count"), "{e}");
    soft.freeze_counter.store(false, Ordering::SeqCst);
    let a = audit_actions(&o).await;
    assert!(
        a.iter()
            .filter(|(x, _)| x == "passkey.stepup.refused")
            .count()
            >= 4,
        "{a:?}"
    );

    // Policy: owners only; required for owners -> another owner without a passkey is refused.
    let (adm, _) = user(&o, "adm@example.com", "admin").await;
    assert!(adm
        .set_auth_policy(pb::AuthPolicy::default())
        .await
        .is_err());
    let mut p = o.get_auth_policy().await.unwrap();
    assert!(
        p.stepup_actions.contains(&"SetRole".to_string()) && p.passkey_required_roles.is_empty()
    );
    p.passkey_required_roles = vec!["owner".into()];
    assert!(o
        .set_auth_policy(pb::AuthPolicy {
            stepup_actions: vec!["WhoAmI".into()],
            ..p.clone()
        })
        .await
        .is_err());
    assert!(o
        .set_auth_policy(pb::AuthPolicy {
            stepup_actions: vec!["Nope".into()],
            ..p.clone()
        })
        .await
        .is_err());
    o.set_auth_policy(p.clone()).await.unwrap();
    let (o2, o2e) = user(&o, "owner2@example.com", "admin").await;
    o.set_role("owner2@example.com", "owner").await.unwrap();
    let e = o2
        .set_role("adm@example.com", "sre")
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("needs a passkey"), "{e}");
    // Admins are not owners: the requirement does not apply to them.
    adm.set_role("sre@example.com", "sre").await.unwrap();
    // The second owner registers one (allowed without a touch), then may act with a touch.
    let soft2: Arc<dyn Authenticator> = Arc::new(Soft::default());
    let o2 = Client::connect_with(&o2e.identity, Some(soft2))
        .await
        .unwrap();
    o2.add_passkey("phone", false).await.unwrap();
    o2.set_role("adm@example.com", "sre").await.unwrap();

    // Lost passkey: an owner resets another user's (with their own touch); audited. Admins can't
    // reset owners'.
    let pk2 = o2.list_passkeys("").await.unwrap().passkeys[0]
        .credential_id
        .clone();
    o.set_role("adm@example.com", "admin").await.unwrap();
    assert!(adm
        .remove_passkey("owner2@example.com", &pk2)
        .await
        .is_err());
    assert!(o
        .remove_passkey("owner2@example.com", &pk2)
        .await
        .unwrap()
        .passkeys
        .is_empty());
    let a = audit_actions(&o).await;
    assert!(a.iter().any(|(x, _)| x == "passkey.reset"), "{a:?}");

    // Unlock: the identity file holds only ciphertext; another key can't open it.
    let mut id = oe.identity.clone();
    id.passkeys.push(lp.clone());
    id.locked = Some(itcmdr_admin::passkey::lock(auth.as_ref(), &lp, &id.device_seed).unwrap());
    let seed = std::mem::take(&mut id.device_seed);
    let json = serde_json::to_string(&id).unwrap();
    assert!(!json.contains(&seed));
    assert!(id.key().is_err());
    assert!(id.key_with(Some(&Soft::default())).is_err());
    assert_eq!(
        hex::encode(id.key_with(Some(auth.as_ref())).unwrap().to_bytes()),
        seed
    );
    let ol = Client::connect_with(&id, Some(auth.clone())).await.unwrap();
    assert_eq!(ol.whoami().await.unwrap().email, "owner@example.com");

    // Key backup retrieval with the words also needs the touch once passkeys exist.
    assert!(itcmdr_admin::recover(
        &s.cfg.public_api,
        &oe.identity.pin,
        "owner@example.com",
        &oe.words,
        true,
        "t",
        None
    )
    .await
    .is_err());
    let r = itcmdr_admin::recover(
        &s.cfg.public_api,
        &oe.identity.pin,
        "owner@example.com",
        &oe.words,
        true,
        "t",
        Some(auth.clone()),
    )
    .await
    .unwrap();
    assert_eq!(r.device_seed, seed);

    // Console recover-owner (words and keys lost) removes the owner's passkeys.
    itcmdr_adminsvr::api::recover_owner(&s.cfg, "owner@example.com")
        .await
        .unwrap();
    assert!(
        o2.list_passkeys("owner@example.com").await.is_err()
            || o2
                .list_passkeys("owner@example.com")
                .await
                .unwrap()
                .passkeys
                .is_empty()
    );
    assert!(o2.verify_audit().await.unwrap().ok);
}
