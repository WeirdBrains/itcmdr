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
pub const COMMANDS: &[&str] = &[
    "backup.start",
    "alert.ack",
    "alert.resolve",
    // v0.20.0
    "setting.set",
    "schedule.run",
    "schedule.pause",
    "schedule.resume",
    "plugin.enable",
    "plugin.disable",
    "plugin.install",
];
/// Code-changing kinds: a requester and a different owner or admin approver (ADMINSVR.md section
/// 5). adminsvr always holds them for approval; a site checks the approver when its
/// `adminsvr.requireTwoPerson` is on (default).
pub const TWO_PERSON: &[&str] = &["plugin.enable", "plugin.disable", "plugin.install"];
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
    .into_iter()
    // v0.20.0: a dry run is part of what is signed (v1 bytes unchanged otherwise).
    .chain(if c.dry_run {
        b"\ndry-run".to_vec()
    } else {
        Vec::new()
    })
    .collect()
}

/// What the second person signs: the command bytes plus who approves.
pub fn approve_bytes(c: &pb::Command) -> Vec<u8> {
    format!(
        "itcmdr-admin-approve/v1\n{}\n{}\n{}\n{}",
        sha256_hex(&command_bytes(c)),
        c.approver_id,
        c.approver_role,
        c.approver_key
    )
    .into_bytes()
}

/// Bytes a site signs to fetch a package (v0.20.0).
pub fn fetch_bytes(site_id: &str, sha256: &str, ts_ms: i64) -> Vec<u8> {
    format!("itcmdr-admin-fetch/v1\n{site_id}\n{sha256}\n{ts_ms}").into_bytes()
}

pub fn sign_approver(key: &SigningKey, c: &mut pb::Command) {
    c.approver_key = hex::encode(key.verifying_key().as_bytes());
    c.approver_sig = hex::encode(key.sign(&approve_bytes(c)).to_bytes());
}

/// adminsvr endorses the approver's key like the user's.
pub fn endorse_approver(root: &SigningKey, c: &mut pb::Command) {
    c.approver_endorsement = hex::encode(
        root.sign(&endorse_bytes(
            &c.approver_id,
            &c.approver_role,
            &c.approver_key,
        ))
        .to_bytes(),
    );
}

/// The second person's part: present, endorsed, signed, an owner or admin, not the requester.
/// Ok(false) = no approver on the command.
pub fn check_approver(root: &VerifyingKey, c: &pb::Command) -> Result<bool> {
    if c.approver_id.is_empty() && c.approver_sig.is_empty() {
        return Ok(false);
    }
    if c.approver_id == c.user_id || c.approver_key == c.user_key {
        bail!("approver is the requester");
    }
    if !matches!(c.approver_role.as_str(), "owner" | "admin") {
        bail!("approver must be an owner or admin");
    }
    let ak = itcmdr_keys::parse_public_key(&c.approver_key)?;
    root.verify(
        &endorse_bytes(&c.approver_id, &c.approver_role, &c.approver_key),
        &sig_of(&c.approver_endorsement)?,
    )
    .map_err(|_| anyhow!("approver endorsement"))?;
    ak.verify(&approve_bytes(c), &sig_of(&c.approver_sig)?)
        .map_err(|_| anyhow!("approver signature"))?;
    Ok(true)
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
    if check_approver(root, c).is_err() {
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

    #[test]
    fn dry_run_is_signed_and_v1_bytes_unchanged() {
        let c = pb::Command {
            id: "c".into(),
            kind: "setting.set".into(),
            ..Default::default()
        };
        let mut d = c.clone();
        d.dry_run = true;
        assert!(!command_bytes(&c).ends_with(b"dry-run"));
        assert!(command_bytes(&d).ends_with(b"\ndry-run"));
        let root = itcmdr_keys::new_keypair();
        let user = itcmdr_keys::new_keypair();
        let mut d = pb::Command {
            issued_ms: 1,
            expires_ms: 10_000,
            user_id: "u".into(),
            user_role: "admin".into(),
            ..d
        };
        sign_user(&user, &mut d);
        sign_root(&root, &mut d);
        assert_eq!(check_command(&root.verifying_key(), &d, 5), "ok");
        // flipping the dry-run flag breaks both signatures
        d.dry_run = false;
        assert_eq!(check_command(&root.verifying_key(), &d, 5), "bad-signature");
    }

    #[test]
    fn approver_rules() {
        let root = itcmdr_keys::new_keypair();
        let rp = root.verifying_key();
        let user = itcmdr_keys::new_keypair();
        let appr = itcmdr_keys::new_keypair();
        let mut c = pb::Command {
            id: "c".into(),
            kind: "plugin.enable".into(),
            user_id: "u1".into(),
            user_role: "admin".into(),
            issued_ms: 1,
            expires_ms: 10_000,
            ..Default::default()
        };
        c.args.insert("name".into(), "itcmdr-network".into());
        sign_user(&user, &mut c);
        sign_root(&root, &mut c);
        assert!(!check_approver(&rp, &c).unwrap());
        let approve = |c: &pb::Command, id: &str, role: &str, k: &SigningKey| {
            let mut a = c.clone();
            a.approver_id = id.into();
            a.approver_role = role.into();
            sign_approver(k, &mut a);
            endorse_approver(&root, &mut a);
            a
        };
        let ok = approve(&c, "u2", "owner", &appr);
        assert!(check_approver(&rp, &ok).unwrap());
        assert_eq!(check_command(&rp, &ok, 5), "ok");
        // the requester can't be their own second person (other id, same key; or same id)
        assert!(check_approver(&rp, &approve(&c, "u1", "owner", &appr)).is_err());
        assert!(check_approver(&rp, &approve(&c, "u9", "owner", &user)).is_err());
        // SREs can't approve
        assert!(check_approver(&rp, &approve(&c, "u2", "sre", &appr)).is_err());
        // the approval is for this exact command
        let mut t = ok.clone();
        t.args.insert("name".into(), "other".into());
        assert_eq!(check_command(&rp, &t, 5), "bad-signature");
        // an approver endorsement not by the root
        let mut t = ok.clone();
        let other = itcmdr_keys::new_keypair();
        endorse_approver(&other, &mut t);
        assert!(check_approver(&rp, &t).is_err());
        // role changed after signing
        let mut t = ok.clone();
        t.approver_role = "admin".into();
        assert!(check_approver(&rp, &t).is_err());
    }
}
