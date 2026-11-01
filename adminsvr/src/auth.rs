//! Signed requests (same canonical form and headers as the site server, docs/SIGNING.md):
//! the key is a user's device key (certified by the user's master key) or, for a few recovery
//! calls, the user's master key itself. Unsigned or wrongly signed calls are refused.
use crate::rbac::Role;
use crate::store::Pool;
use diesel::prelude::*;
use parking_lot::Mutex;
use std::collections::HashMap;
use tonic::{metadata::MetadataMap, Status};

#[derive(Debug, Clone)]
pub struct Principal {
    pub user_id: String,
    pub email: String,
    pub role: Role,
    pub key_fp: String,
    pub key_hex: String,
    /// signed with the user's master key (24 words), not a device key
    pub via_master: bool,
    pub sig: String,
}

#[derive(Default)]
pub struct Nonces(Mutex<HashMap<String, i64>>);

impl Nonces {
    /// true if fresh (and remembers it until `exp_ms`)
    pub fn fresh(&self, key: &str, nonce: &str, now: i64, exp_ms: i64) -> bool {
        let mut m = self.0.lock();
        if m.len() > 50_000 {
            m.retain(|_, e| *e > now);
        }
        let k = format!("{key}/{nonce}");
        if m.get(&k).is_some_and(|e| *e > now) {
            return false;
        }
        m.insert(k, now + exp_ms);
        true
    }
}

fn meta<'a>(m: &'a MetadataMap, k: &str) -> Result<&'a str, Status> {
    m.get(k)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Status::unauthenticated(format!("unsigned request (missing {k})")))
}

pub struct Signed<'a> {
    pub fp: &'a str,
    pub ts: i64,
    pub nonce: &'a str,
    pub sig: &'a str,
}

/// Parse and check the signature headers against `pk` (clock window, nonce, signature).
pub fn check<'a>(
    nonces: &Nonces,
    m: &'a MetadataMap,
    pk: &ed25519_dalek::VerifyingKey,
    method: &str,
    body: &[u8],
    skew_ms: i64,
) -> Result<Signed<'a>, Status> {
    let s = parse(m)?;
    if itcmdr_keys::fingerprint(pk) != s.fp {
        return Err(Status::unauthenticated("key does not match x-itcmdr-key"));
    }
    let now = crate::now_ms();
    if (now - s.ts).abs() > skew_ms {
        return Err(Status::unauthenticated(
            "request timestamp outside the allowed window (check the clock)",
        ));
    }
    itcmdr_keys::verify_request(pk, method, s.ts, s.nonce, body, s.sig)
        .map_err(|_| Status::unauthenticated("bad signature"))?;
    if !nonces.fresh(s.fp, s.nonce, now, 2 * skew_ms) {
        return Err(Status::unauthenticated(
            "replayed request (nonce already used)",
        ));
    }
    Ok(s)
}

pub fn parse(m: &MetadataMap) -> Result<Signed<'_>, Status> {
    let fp = meta(m, "x-itcmdr-key")?;
    let ts = meta(m, "x-itcmdr-ts")?
        .parse()
        .map_err(|_| Status::unauthenticated("bad x-itcmdr-ts"))?;
    let nonce = meta(m, "x-itcmdr-nonce")?;
    if !(16..=128).contains(&nonce.len())
        || !nonce
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Status::unauthenticated("bad x-itcmdr-nonce"));
    }
    let sig = meta(m, "x-itcmdr-sig")?;
    Ok(Signed { fp, ts, nonce, sig })
}

fn internal(e: impl std::fmt::Display) -> Status {
    Status::internal(e.to_string())
}

/// Verify a signed call. `allow_master`: the user's master key may sign this method.
/// `allow_pending`: a device waiting for approval may call (WhoAmI only).
#[allow(clippy::too_many_arguments)]
pub async fn verify(
    pool: &Pool,
    nonces: &Nonces,
    m: &MetadataMap,
    method: &str,
    body: &[u8],
    skew_ms: i64,
    allow_master: bool,
    allow_pending: bool,
) -> Result<Principal, Status> {
    let s = parse(m)?;
    let fp = s.fp.to_string();
    let deny = |x: &str| Status::unauthenticated(x.to_string());
    let f = fp.clone();
    let (dev, master) = pool
        .run(move |c| {
            use crate::schema::{devices as d, users as u};
            let dev: Option<DevRow> = d::table
                .inner_join(u::table.on(u::id.eq(d::user_id)))
                .filter(d::fingerprint.eq(&f))
                .select((
                    d::public_key,
                    d::certificate,
                    d::status,
                    u::id,
                    u::email,
                    u::role,
                    u::status,
                    u::master_pub,
                ))
                .first(c)
                .optional()?;
            let master: Option<(String, String, String, String, Option<String>)> = if dev.is_none()
            {
                u::table
                    .filter(u::master_fp.eq(&f))
                    .select((u::id, u::email, u::role, u::status, u::master_pub))
                    .first(c)
                    .optional()?
            } else {
                None
            };
            Ok((dev, master))
        })
        .await
        .map_err(internal)?;
    if let Some((public_key, certificate, dstatus, uid, email, role, ustatus, master_pub)) = dev {
        match dstatus.as_str() {
            "active" => {}
            "pending" if allow_pending => {}
            "pending" => {
                return Err(Status::permission_denied(
                    "this device is waiting for approval by an owner or admin",
                ))
            }
            _ => return Err(deny("key revoked")),
        }
        if ustatus != "active" {
            return Err(deny("user disabled"));
        }
        let pk =
            itcmdr_keys::parse_public_key(&public_key).map_err(|_| deny("stored key invalid"))?;
        let mp = itcmdr_keys::parse_public_key(&master_pub.unwrap_or_default())
            .map_err(|_| deny("user has no master key"))?;
        let cert: itcmdr_keys::Certificate =
            serde_json::from_str(&certificate).map_err(|_| deny("device has no certificate"))?;
        itcmdr_keys::verify_certificate(&mp, &cert)
            .map_err(|_| deny("device certificate invalid"))?;
        if cert.fingerprint != fp {
            return Err(deny("certificate does not match the key"));
        }
        let s = check(nonces, m, &pk, method, body, skew_ms)?;
        let f = fp.clone();
        let _ = pool
            .run(move |c| {
                use crate::schema::devices as d;
                diesel::update(d::table.filter(d::fingerprint.eq(f)))
                    .set(d::last_used_ms.eq(crate::now_ms()))
                    .execute(c)?;
                Ok(())
            })
            .await;
        return Ok(Principal {
            user_id: uid,
            email,
            role: Role::parse(&role).ok_or_else(|| deny("unknown role"))?,
            key_fp: fp.clone(),
            key_hex: hex::encode(pk.as_bytes()),
            via_master: false,
            sig: s.sig.to_string(),
        });
    }
    let (uid, email, role, ustatus, master_pub) = master.ok_or_else(|| deny("unknown key"))?;
    if !allow_master {
        return Err(Status::permission_denied(
            "the master key (24 words) may only recover a key backup or add a device; use a device key",
        ));
    }
    if ustatus != "active" {
        return Err(deny("user disabled"));
    }
    let pk = itcmdr_keys::parse_public_key(&master_pub.unwrap_or_default())
        .map_err(|_| deny("stored key invalid"))?;
    let s = check(nonces, m, &pk, method, body, skew_ms)?;
    Ok(Principal {
        user_id: uid,
        email,
        role: Role::parse(&role).ok_or_else(|| deny("unknown role"))?,
        key_fp: fp,
        key_hex: hex::encode(pk.as_bytes()),
        via_master: true,
        sig: s.sig.to_string(),
    })
}

type DevRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
);
