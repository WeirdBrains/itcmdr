//! v0.19.0: shared pieces of the adminsvr link (site <-> adminsvr) and the admin API.
use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod pb {
    #![allow(clippy::all)]
    tonic::include_proto!("itcmdr.admin");
}

pub const SITE_PORT: u16 = 8443;
pub const API_PORT: u16 = 8444;

/// Command kinds adminsvr may send. Anything else is refused by the site (unknown = denied).
pub const COMMANDS: &[&str] = &["backup.start", "alert.ack", "alert.resolve"];
/// Never accepted remotely, whatever a site's allowlist says (ADMINSVR.md section 5, decision 18).
pub const NEVER_REMOTE: &[&str] = &[
    "restore",
    "backup.restore",
    "backup.download",
    "auth.mode",
    "keys.register",
    "keys.revoke",
    "words",
    "cutover.switch",
    "adminsvr.unlink",
    "adminsvr.allowlist",
];

pub mod webauthn;

pub fn sha256_hex(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}
fn unb64(s: &str) -> Result<Vec<u8>> {
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s.trim())?)
}

fn check_pin(pin: &str) -> Result<()> {
    if pin.len() != 64 || hex::decode(pin).is_err() {
        bail!("no valid server fingerprint");
    }
    Ok(())
}

/// One-time site enrollment token: `itcmdr-site:` + base64url(JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteToken {
    pub v: u32,
    /// host:port of the SiteLink
    pub addr: String,
    /// SHA-256 hex of adminsvr's SiteLink TLS certificate (pinned)
    pub pin: String,
    pub secret: String,
    /// hex adminsvr root public key (signs commands and user endorsements)
    pub root: String,
}

impl SiteToken {
    const PREFIX: &'static str = "itcmdr-site:";
    pub fn encode(&self) -> String {
        format!(
            "{}{}",
            Self::PREFIX,
            b64(&serde_json::to_vec(self).unwrap())
        )
    }
    pub fn decode(s: &str) -> Result<Self> {
        let b = s
            .trim()
            .strip_prefix(Self::PREFIX)
            .ok_or_else(|| anyhow!("not an adminsvr site token (expected itcmdr-site:...)"))?;
        let t: Self = serde_json::from_slice(&unb64(b)?).context("site token is not valid")?;
        if t.v != 1 {
            bail!("unsupported site token version {}", t.v);
        }
        check_pin(&t.pin)?;
        itcmdr_keys::parse_public_key(&t.root).context("site token root key")?;
        if t.addr.is_empty() || t.secret.is_empty() {
            bail!("site token is incomplete");
        }
        Ok(t)
    }
}

/// One-time device enrollment code for a user: `itcmdr-admin:` + base64url(JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrollCode {
    pub v: u32,
    /// host:port of the admin API
    pub addr: String,
    pub pin: String,
    pub secret: String,
    pub email: String,
}

impl EnrollCode {
    const PREFIX: &'static str = "itcmdr-admin:";
    pub fn encode(&self) -> String {
        format!(
            "{}{}",
            Self::PREFIX,
            b64(&serde_json::to_vec(self).unwrap())
        )
    }
    pub fn decode(s: &str) -> Result<Self> {
        let b = s.trim().strip_prefix(Self::PREFIX).ok_or_else(|| {
            anyhow!("not an adminsvr enrollment code (expected itcmdr-admin:...)")
        })?;
        let t: Self = serde_json::from_slice(&unb64(b)?).context("enrollment code is not valid")?;
        if t.v != 1 {
            bail!("unsupported enrollment code version {}", t.v);
        }
        check_pin(&t.pin)?;
        if t.addr.is_empty() || t.secret.is_empty() {
            bail!("enrollment code is incomplete");
        }
        Ok(t)
    }
}

/// Bytes the site key signs for every SiteMessage.
pub fn site_bytes(seq: u64, ts_ms: i64, body: &[u8]) -> Vec<u8> {
    format!("itcmdr-site-msg/v1\n{seq}\n{ts_ms}\n{}", sha256_hex(body)).into_bytes()
}

/// Encoded body of a SiteMessage (the oneof only), for signing.
pub fn body_bytes(m: &pb::SiteMessage) -> Vec<u8> {
    use prost::Message;
    let mut c = m.clone();
    c.seq = 0;
    c.ts_ms = 0;
    c.sig = String::new();
    c.encode_to_vec()
}

pub fn sign_site(key: &SigningKey, mut m: pb::SiteMessage) -> pb::SiteMessage {
    let b = body_bytes(&m);
    m.sig = hex::encode(key.sign(&site_bytes(m.seq, m.ts_ms, &b)).to_bytes());
    m
}

fn sig_of(h: &str) -> Result<Signature> {
    let b: [u8; 64] = hex::decode(h)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| anyhow!("bad signature encoding"))?;
    Ok(Signature::from_bytes(&b))
}

pub fn verify_site(pk: &VerifyingKey, m: &pb::SiteMessage) -> Result<()> {
    let b = body_bytes(m);
    pk.verify(&site_bytes(m.seq, m.ts_ms, &b), &sig_of(&m.sig)?)
        .map_err(|_| anyhow!("bad site message signature"))
}

/// Root endorsement of a user's device key, with the role at the time.
pub fn endorse_bytes(user_id: &str, role: &str, user_key_hex: &str) -> Vec<u8> {
    format!("itcmdr-admin-user/v1\n{user_id}\n{role}\n{user_key_hex}").into_bytes()
}

/// Canonical bytes of a command, signed by the user and by the adminsvr root key.
pub fn command_bytes(c: &pb::Command) -> Vec<u8> {
    let mut args: Vec<_> = c.args.iter().collect();
    args.sort();
    let args: Vec<String> = args
        .iter()
        .map(|(k, v)| format!("{}={}", hex::encode(k), hex::encode(v)))
        .collect();
    format!(
        "itcmdr-admin-cmd/v1\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        c.id,
        c.site_id,
        c.kind,
        args.join(","),
        c.user_id,
        c.user_role,
        c.user_key,
        c.issued_ms,
        c.expires_ms
    )
    .into_bytes()
}

pub fn sign_user(key: &SigningKey, c: &mut pb::Command) {
    c.user_key = hex::encode(key.verifying_key().as_bytes());
    c.user_sig = hex::encode(key.sign(&command_bytes(c)).to_bytes());
}

pub fn sign_root(root: &SigningKey, c: &mut pb::Command) {
    c.user_endorsement = hex::encode(
        root.sign(&endorse_bytes(&c.user_id, &c.user_role, &c.user_key))
            .to_bytes(),
    );
    c.root_sig = hex::encode(root.sign(&command_bytes(c)).to_bytes());
}

/// Site-side check of a command: both signatures, the endorsement, expiry, never-remote and the
/// known kinds. Returns the result code to send back ("ok" = may run, then the allowlist).
pub fn check_command(root: &VerifyingKey, c: &pb::Command, now_ms: i64) -> &'static str {
    if NEVER_REMOTE.contains(&c.kind.as_str()) || !COMMANDS.contains(&c.kind.as_str()) {
        return "not-allowed";
    }
    let ok = (|| -> Result<()> {
        let uk = itcmdr_keys::parse_public_key(&c.user_key)?;
        root.verify(
            &endorse_bytes(&c.user_id, &c.user_role, &c.user_key),
            &sig_of(&c.user_endorsement)?,
        )
        .map_err(|_| anyhow!("endorsement"))?;
        let b = command_bytes(c);
        uk.verify(&b, &sig_of(&c.user_sig)?)
            .map_err(|_| anyhow!("user"))?;
        root.verify(&b, &sig_of(&c.root_sig)?)
            .map_err(|_| anyhow!("root"))?;
        Ok(())
    })();
    if ok.is_err() {
        return "bad-signature";
    }
    if c.expires_ms < now_ms {
        return "expired";
    }
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_round_trip() {
        let root = itcmdr_keys::new_keypair();
        let t = SiteToken {
            v: 1,
            addr: "adminsvr.example.com:8443".into(),
            pin: "ab".repeat(32),
            secret: "s".into(),
            root: hex::encode(root.verifying_key().as_bytes()),
        };
        assert_eq!(SiteToken::decode(&t.encode()).unwrap(), t);
        assert!(SiteToken::decode("itcmdr-site:xx").is_err());
        let c = EnrollCode {
            v: 1,
            addr: "a:1".into(),
            pin: "cd".repeat(32),
            secret: "x".into(),
            email: "sre@example.com".into(),
        };
        assert_eq!(EnrollCode::decode(&c.encode()).unwrap(), c);
    }

    #[test]
    fn command_signatures() {
        let root = itcmdr_keys::new_keypair();
        let user = itcmdr_keys::new_keypair();
        let mut c = pb::Command {
            id: "c1".into(),
            kind: "backup.start".into(),
            user_id: "u1".into(),
            user_role: "sre".into(),
            issued_ms: 1,
            expires_ms: 10_000,
            ..Default::default()
        };
        sign_user(&user, &mut c);
        sign_root(&root, &mut c);
        let rp = root.verifying_key();
        assert_eq!(check_command(&rp, &c, 5), "ok");
        assert_eq!(check_command(&rp, &c, 20_000), "expired");
        let mut t = c.clone();
        t.args.insert("x".into(), "y".into());
        assert_eq!(check_command(&rp, &t, 5), "bad-signature");
        let mut t = c.clone();
        t.kind = "restore".into();
        assert_eq!(check_command(&rp, &t, 5), "not-allowed");
        let other = itcmdr_keys::new_keypair();
        assert_eq!(
            check_command(&other.verifying_key(), &c, 5),
            "bad-signature"
        );
        // user-only signature (no valid root) is refused
        let mut t = c.clone();
        t.root_sig = t.user_sig.clone();
        assert_eq!(check_command(&rp, &t, 5), "bad-signature");
    }
}
