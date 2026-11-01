//! v0.19.1 passkeys / FIDO2 security keys as a second factor (ADMINSVR.md "Passkeys").
//! The 24-word key stays the signing identity. Methods on the step-up list need a fresh touch
//! bound to that exact request, from users who have a passkey (or whose role requires one).
use crate::audit::{self, Entry};
use crate::auth::Principal;
use crate::config::Config;
use crate::pb;
use crate::rbac::Role;
use crate::schema::*;
use crate::store::{Db, Pool};
use anyhow::Result;
use base64::Engine;
use diesel::prelude::*;
use itcmdr_adminlink::webauthn as wa;
use parking_lot::Mutex;
use rand::RngCore;
use std::collections::HashMap;
use tonic::{metadata::MetadataMap, Status};

pub const CHALLENGE_TTL_MS: i64 = 5 * 60 * 1000;
pub const HEADER: &str = "x-itcmdr-stepup";
/// Status message prefix clients look for to run a step-up and retry.
pub const STEPUP_REQUIRED: &str = "stepup-required";

/// Default step-up list: owner/role changes, user and device enrollment and revocation, key
/// backup retrieval, site (key) operations and revocation (the kill switch), passkey and policy changes.
pub const DEFAULT_ACTIONS: &[&str] = &[
    "SetRole",
    "SetUserStatus",
    "CreateUser",
    "NewEnrollCode",
    "ApproveDevice",
    "RevokeDevice",
    "AddDevice",
    "GetKeyBackup",
    "SetKeyBackupOptOut",
    "CreateSite",
    "RevokeSite",
    "FinishPasskeyRegistration",
    "RemovePasskey",
    "SetAuthPolicy",
];

/// Every AdminApi method a policy may name.
pub const METHODS: &[&str] = &[
    "EnrollDevice",
    "WhoAmI",
    "ListUsers",
    "CreateUser",
    "SetRole",
    "SetUserStatus",
    "SetScopes",
    "ApproveDevice",
    "NewEnrollCode",
    "ListDevices",
    "RevokeDevice",
    "PutKeyBackup",
    "GetKeyBackup",
    "SetKeyBackupOptOut",
    "AddDevice",
    "CreateSite",
    "ListSites",
    "RevokeSite",
    "ListAlerts",
    "AckAlert",
    "ResolveAlert",
    "StartBackup",
    "ListCommands",
    "ListAudit",
    "VerifyAudit",
    "BeginPasskeyRegistration",
    "FinishPasskeyRegistration",
    "ListPasskeys",
    "RemovePasskey",
    "BeginStepUp",
    "GetAuthPolicy",
    "SetAuthPolicy",
];
/// Never on the step-up list (they are how a step-up happens, or how the list is read).
const NEVER: &[&str] = &[
    "EnrollDevice",
    "WhoAmI",
    "BeginStepUp",
    "BeginPasskeyRegistration",
    "GetAuthPolicy",
];

fn internal(e: impl std::fmt::Display) -> Status {
    Status::internal(e.to_string())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Purpose {
    Register,
    StepUp,
}

struct Pending {
    user: String,
    purpose: Purpose,
    challenge: Vec<u8>,
    exp: i64,
}

/// Outstanding challenges (single use, 5 minutes; in memory: a restart only voids them).
#[derive(Default)]
pub struct Challenges(Mutex<HashMap<String, Pending>>);

impl Challenges {
    fn add(&self, user: &str, purpose: Purpose, challenge: Vec<u8>) -> (String, i64) {
        let mut b = [0u8; 24];
        rand::rngs::OsRng.fill_bytes(&mut b);
        let nonce = hex::encode(b);
        let now = crate::now_ms();
        let exp = now + CHALLENGE_TTL_MS;
        let mut m = self.0.lock();
        m.retain(|_, p| p.exp > now);
        m.insert(
            nonce.clone(),
            Pending {
                user: user.into(),
                purpose,
                challenge,
                exp,
            },
        );
        (nonce, exp)
    }
    /// Take (and so burn) a nonce for this user and purpose.
    fn take(&self, nonce: &str, user: &str, purpose: Purpose) -> Result<Vec<u8>, &'static str> {
        let p = self
            .0
            .lock()
            .remove(nonce)
            .ok_or("unknown or already used step-up nonce")?;
        if p.user != user || p.purpose != purpose {
            return Err("nonce belongs to another user or purpose");
        }
        if p.exp < crate::now_ms() {
            return Err("challenge expired");
        }
        Ok(p.challenge)
    }
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub actions: Vec<String>,
    pub required_roles: Vec<String>,
    pub require_uv: bool,
}

impl Policy {
    pub fn to_pb(&self) -> pb::AuthPolicy {
        pb::AuthPolicy {
            stepup_actions: self.actions.clone(),
            passkey_required_roles: self.required_roles.clone(),
            require_user_verification: self.require_uv,
        }
    }
}

fn split(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect()
}

pub fn load_policy(c: &mut Db) -> Result<Policy> {
    let rows: Vec<(String, String)> = meta::table.filter(meta::k.like("auth.%")).load(c)?;
    let get = |k: &str| rows.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    Ok(Policy {
        actions: get("auth.stepup_actions")
            .map(|v| split(&v))
            .unwrap_or_else(|| DEFAULT_ACTIONS.iter().map(|s| s.to_string()).collect()),
        required_roles: get("auth.passkey_required_roles")
            .map(|v| split(&v))
            .unwrap_or_default(),
        require_uv: get("auth.passkey_require_uv").is_some_and(|v| v == "true"),
    })
}

fn put_meta(c: &mut Db, key: &str, val: &str) -> Result<()> {
    let n = diesel::update(meta::table.filter(meta::k.eq(key)))
        .set(meta::v.eq(val))
        .execute(c)?;
    if n == 0 {
        diesel::insert_into(meta::table)
            .values((meta::k.eq(key), meta::v.eq(val)))
            .execute(c)?;
    }
    Ok(())
}

/// Validate and store a policy (owners only; the caller checks the role).
pub fn save_policy(c: &mut Db, p: &pb::AuthPolicy) -> Result<Policy, String> {
    let mut actions = vec![];
    for a in &p.stepup_actions {
        let a = a.trim();
        if !METHODS.contains(&a) {
            return Err(format!("unknown method {a:?}"));
        }
        if NEVER.contains(&a) {
            return Err(format!("{a} cannot need a step-up"));
        }
        if !actions.iter().any(|x: &String| x == a) {
            actions.push(a.to_string());
        }
    }
    let mut roles = vec![];
    for r in &p.passkey_required_roles {
        let r = Role::parse(r.trim()).ok_or_else(|| format!("unknown role {r:?}"))?;
        if !roles.iter().any(|x: &String| x == r.as_str()) {
            roles.push(r.as_str().to_string());
        }
    }
    let r: Result<()> = (|| {
        put_meta(c, "auth.stepup_actions", &actions.join(","))?;
        put_meta(c, "auth.passkey_required_roles", &roles.join(","))?;
        put_meta(
            c,
            "auth.passkey_require_uv",
            if p.require_user_verification {
                "true"
            } else {
                "false"
            },
        )?;
        Ok(())
    })();
    r.map_err(|e| e.to_string())?;
    Ok(Policy {
        actions,
        required_roles: roles,
        require_uv: p.require_user_verification,
    })
}

type PkRow = (
    String,
    String,
    i64,
    Vec<u8>,
    String,
    String,
    i64,
    bool,
    i64,
    i64,
);

fn info(r: PkRow) -> pb::PasskeyInfo {
    pb::PasskeyInfo {
        credential_id: r.0,
        label: r.1,
        alg: r.2,
        aaguid: r.4,
        attestation: r.5,
        sign_count: r.6,
        hmac_secret: r.7,
        created_ms: r.8,
        last_used_ms: r.9,
    }
}

fn rows(c: &mut Db, user: &str) -> Result<Vec<PkRow>> {
    Ok(passkeys::table
        .filter(passkeys::user_id.eq(user))
        .order(passkeys::created_ms)
        .select((
            passkeys::credential_id,
            passkeys::label,
            passkeys::alg,
            passkeys::public_key,
            passkeys::aaguid,
            passkeys::attestation,
            passkeys::sign_count,
            passkeys::hmac_secret,
            passkeys::created_ms,
            passkeys::last_used_ms,
        ))
        .load(c)?)
}

pub async fn list(pool: &Pool, user: &str) -> Result<pb::PasskeyList> {
    let u = user.to_string();
    pool.run(move |c| {
        Ok(pb::PasskeyList {
            passkeys: rows(c, &u)?.into_iter().map(info).collect(),
        })
    })
    .await
}

pub async fn count(pool: &Pool, user: &str) -> Result<i64> {
    let u = user.to_string();
    pool.run(move |c| {
        Ok(passkeys::table
            .filter(passkeys::user_id.eq(u))
            .count()
            .get_result(c)?)
    })
    .await
}

/// Remove every passkey of a user (console recover-owner).
pub fn remove_all(c: &mut Db, user: &str) -> Result<usize> {
    Ok(diesel::delete(passkeys::table.filter(passkeys::user_id.eq(user))).execute(c)?)
}

pub struct Passkeys {
    pub pool: Pool,
    pub cfg: Config,
    pub ch: std::sync::Arc<Challenges>,
}

impl Passkeys {
    async fn policy(&self) -> Result<Policy, Status> {
        self.pool.run(load_policy).await.map_err(internal)
    }

    async fn audit_stepup(
        &self,
        p: &Principal,
        action: &str,
        target: &str,
        payload: serde_json::Value,
    ) {
        let _ = audit::append(
            &self.pool,
            &Entry {
                actor: p.user_id.clone(),
                actor_key: p.key_fp.clone(),
                action: action.into(),
                target: target.into(),
                payload: payload.to_string().into_bytes(),
                request_sig: p.sig.clone(),
            },
        )
        .await;
    }

    /// Enforce the step-up policy for an already signature-checked request.
    pub async fn check(
        &self,
        md: &MetadataMap,
        method: &str,
        body: &[u8],
        p: &Principal,
    ) -> Result<(), Status> {
        let short = method.rsplit('/').next().unwrap_or(method);
        let pol = self.policy().await?;
        if !pol.actions.iter().any(|a| a == short) {
            return Ok(());
        }
        let n = count(&self.pool, &p.user_id).await.map_err(internal)?;
        if n == 0 {
            if pol.required_roles.iter().any(|r| r == p.role.as_str())
                && short != "FinishPasskeyRegistration"
            {
                return Err(Status::failed_precondition(format!(
                    "{STEPUP_REQUIRED}: your role ({}) needs a passkey for {short}; register one with `itcmdr-admin passkey add`",
                    p.role.as_str()
                )));
            }
            return Ok(()); // optional second factor: not set up for this user
        }
        let Some(h) = md.get(HEADER).and_then(|v| v.to_str().ok()) else {
            return Err(Status::failed_precondition(format!(
                "{STEPUP_REQUIRED}: {short} needs a touch of your passkey"
            )));
        };
        let refuse = |why: String| Status::unauthenticated(format!("step-up refused: {why}"));
        let a = base64::engine::general_purpose::STANDARD
            .decode(h)
            .ok()
            .and_then(|b| <pb::StepUpAssertion as prost::Message>::decode(b.as_slice()).ok())
            .ok_or_else(|| refuse("bad x-itcmdr-stepup header".into()))?;
        let cid = wa::b64url(&a.credential_id);
        let challenge = wa::stepup_challenge(method, body, &a.nonce);
        let res: Result<wa::Verified, String> = async {
            self.ch
                .take(&a.nonce, &p.user_id, Purpose::StepUp)
                .map_err(String::from)?;
            let (u, c2) = (p.user_id.clone(), cid.clone());
            let cred: Option<(i64, Vec<u8>, i64)> = self
                .pool
                .run(move |c| {
                    Ok(passkeys::table
                        .filter(passkeys::credential_id.eq(c2).and(passkeys::user_id.eq(u)))
                        .select((passkeys::alg, passkeys::public_key, passkeys::sign_count))
                        .first(c)
                        .optional()?)
                })
                .await
                .map_err(|e| e.to_string())?;
            let (alg, pk, cnt) = cred.ok_or("not one of your passkeys")?;
            let v = wa::verify_assertion(
                &self.cfg.rp(pol.require_uv),
                alg,
                &pk,
                cnt as u32,
                &challenge,
                &a.client_data_json,
                &a.auth_data,
                &a.signature,
            )
            .map_err(|e| e.to_string())?;
            let c3 = cid.clone();
            self.pool
                .run(move |c| {
                    Ok(
                        diesel::update(passkeys::table.filter(passkeys::credential_id.eq(c3)))
                            .set((
                                passkeys::sign_count.eq(v.sign_count as i64),
                                passkeys::last_used_ms.eq(crate::now_ms()),
                            ))
                            .execute(c)?,
                    )
                })
                .await
                .map_err(|e| e.to_string())?;
            Ok(v)
        }
        .await;
        let mut j = serde_json::json!({
            "method": short, "credential_id": cid, "challenge_sha256": itcmdr_adminlink::sha256_hex(&challenge),
        });
        match res {
            Ok(v) => {
                j["flags"] = v.flags.into();
                j["sign_count"] = v.sign_count.into();
                j["result"] = "ok".into();
                self.audit_stepup(p, "passkey.stepup", short, j).await;
                Ok(())
            }
            Err(e) => {
                j["result"] = e.clone().into();
                self.audit_stepup(p, "passkey.stepup.refused", short, j)
                    .await;
                Err(refuse(e))
            }
        }
    }

    pub async fn begin(
        &self,
        p: &Principal,
        register: bool,
    ) -> Result<pb::PasskeyChallenge, Status> {
        let pol = self.policy().await?;
        let rp = self.cfg.rp(pol.require_uv);
        let mut chal = vec![0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut chal);
        let (nonce, exp) = self.ch.add(
            &p.user_id,
            if register {
                Purpose::Register
            } else {
                Purpose::StepUp
            },
            chal.clone(),
        );
        let u = p.user_id.clone();
        let ids: Vec<String> = self
            .pool
            .run(move |c| {
                Ok(passkeys::table
                    .filter(passkeys::user_id.eq(u))
                    .select(passkeys::credential_id)
                    .load(c)?)
            })
            .await
            .map_err(internal)?;
        Ok(pb::PasskeyChallenge {
            nonce,
            challenge: if register { chal } else { vec![] },
            rp_id: rp.id,
            user_handle: p.user_id.as_bytes().to_vec(),
            user_name: p.email.clone(),
            credential_ids: ids.iter().filter_map(|i| wa::from_b64url(i).ok()).collect(),
            expires_ms: exp,
            require_user_verification: pol.require_uv,
        })
    }

    pub async fn finish(
        &self,
        p: &Principal,
        r: &pb::PasskeyRegistration,
    ) -> Result<pb::PasskeyList, Status> {
        let chal = self
            .ch
            .take(&r.nonce, &p.user_id, Purpose::Register)
            .map_err(Status::permission_denied)?;
        let label = r.label.trim();
        if label.is_empty() || label.len() > 64 {
            return Err(Status::invalid_argument(
                "label is required (up to 64 characters)",
            ));
        }
        let att = if r.attestation_object.is_empty() {
            wa::Attestation {
                fmt: r.fmt.clone(),
                auth_data: wa::unwrap_cbor_bytes(&r.auth_data),
                alg: None,
                sig: r.att_sig.clone(),
                x5c: r.x5c.clone(),
            }
        } else {
            wa::parse_attestation_object(&r.attestation_object)
                .map_err(|e| Status::invalid_argument(e.to_string()))?
        };
        let pol = self.policy().await?;
        let cred = wa::verify_registration(
            &self.cfg.rp(pol.require_uv),
            &chal,
            &r.client_data_json,
            &att,
        )
        .map_err(|e| Status::invalid_argument(format!("registration refused: {e}")))?;
        let (u, l, hm) = (p.user_id.clone(), label.to_string(), r.hmac_secret);
        let cid = wa::b64url(&cred.id);
        let c2 = cid.clone();
        self.pool
            .run(move |c| {
                let dup: i64 = passkeys::table
                    .filter(passkeys::credential_id.eq(&c2))
                    .count()
                    .get_result(c)?;
                if dup > 0 {
                    anyhow::bail!("this credential is already registered");
                }
                diesel::insert_into(passkeys::table)
                    .values((
                        passkeys::credential_id.eq(&c2),
                        passkeys::user_id.eq(&u),
                        passkeys::label.eq(l),
                        passkeys::alg.eq(cred.alg),
                        passkeys::public_key.eq(cred.public_key.clone()),
                        passkeys::aaguid.eq(hex::encode(cred.aaguid)),
                        passkeys::attestation.eq(&cred.attestation),
                        passkeys::sign_count.eq(cred.sign_count as i64),
                        passkeys::hmac_secret.eq(hm),
                        passkeys::created_ms.eq(crate::now_ms()),
                    ))
                    .execute(c)?;
                Ok(())
            })
            .await
            .map_err(|e| Status::already_exists(e.to_string()))?;
        list(&self.pool, &p.user_id).await.map_err(internal)
    }

    pub async fn remove(&self, owner_of: &str, cred: &str) -> Result<bool, Status> {
        let (u, c2) = (owner_of.to_string(), cred.to_string());
        let n = self
            .pool
            .run(move |c| {
                Ok(diesel::delete(
                    passkeys::table
                        .filter(passkeys::user_id.eq(u).and(passkeys::credential_id.eq(c2))),
                )
                .execute(c)?)
            })
            .await
            .map_err(internal)?;
        Ok(n > 0)
    }
}
