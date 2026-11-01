//! Signed requests (same canonical form and headers as the site server, docs/SIGNING.md):
//! the key is a user's device key (certified by the user's master key) or, for a few recovery
//! calls, the user's master key itself. Unsigned or wrongly signed calls are refused.
use crate::rbac::Role;
use crate::store::Pool;
use parking_lot::Mutex;
use sqlx::Row;
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
    // device key?
    if let Some(r) = sqlx::query(
        "SELECT d.public_key, d.certificate, d.status AS dstatus, u.id, u.email, u.role, u.status, u.master_pub \
         FROM devices d JOIN users u ON u.id = d.user_id WHERE d.fingerprint = $1",
    )
    .bind(&fp)
    .fetch_optional(pool)
    .await
    .map_err(internal)?
    {
        let dstatus: String = r.get("dstatus");
        match dstatus.as_str() {
            "active" => {}
            "pending" if allow_pending => {}
            "pending" => return Err(Status::permission_denied("this device is waiting for approval by an owner or admin")),
            _ => return Err(deny("key revoked")),
        }
        if r.get::<String, _>("status") != "active" {
            return Err(deny("user disabled"));
        }
        let pk = itcmdr_keys::parse_public_key(&r.get::<String, _>("public_key"))
            .map_err(|_| deny("stored key invalid"))?;
        let mp = itcmdr_keys::parse_public_key(&r.get::<Option<String>, _>("master_pub").unwrap_or_default())
            .map_err(|_| deny("user has no master key"))?;
        let cert: itcmdr_keys::Certificate =
            serde_json::from_str(&r.get::<String, _>("certificate")).map_err(|_| deny("device has no certificate"))?;
        itcmdr_keys::verify_certificate(&mp, &cert).map_err(|_| deny("device certificate invalid"))?;
        if cert.fingerprint != fp {
            return Err(deny("certificate does not match the key"));
        }
        let s = check(nonces, m, &pk, method, body, skew_ms)?;
        let _ = sqlx::query("UPDATE devices SET last_used_ms = $1 WHERE fingerprint = $2")
            .bind(crate::now_ms())
            .bind(&fp)
            .execute(pool)
            .await;
        return Ok(Principal {
            user_id: r.get("id"),
            email: r.get("email"),
            role: Role::parse(&r.get::<String, _>("role")).ok_or_else(|| deny("unknown role"))?,
            key_fp: fp.clone(),
            key_hex: hex::encode(pk.as_bytes()),
            via_master: false,
            sig: s.sig.to_string(),
        });
    }
    // master key?
    let r =
        sqlx::query("SELECT id, email, role, status, master_pub FROM users WHERE master_fp = $1")
            .bind(&fp)
            .fetch_optional(pool)
            .await
            .map_err(internal)?
            .ok_or_else(|| deny("unknown key"))?;
    if !allow_master {
        return Err(Status::permission_denied(
            "the master key (24 words) may only recover a key backup or add a device; use a device key",
        ));
    }
    if r.get::<String, _>("status") != "active" {
        return Err(deny("user disabled"));
    }
    let pk = itcmdr_keys::parse_public_key(
        &r.get::<Option<String>, _>("master_pub").unwrap_or_default(),
    )
    .map_err(|_| deny("stored key invalid"))?;
    let s = check(nonces, m, &pk, method, body, skew_ms)?;
    Ok(Principal {
        user_id: r.get("id"),
        email: r.get("email"),
        role: Role::parse(&r.get::<String, _>("role")).ok_or_else(|| deny("unknown role"))?,
        key_fp: fp,
        key_hex: hex::encode(pk.as_bytes()),
        via_master: true,
        sig: s.sig.to_string(),
    })
}
