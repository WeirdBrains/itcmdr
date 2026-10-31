//! v0.12.0: the 24 words for an app-triggered restore are sealed on the app, end to end, for a
//! one-time X25519 key that only lives in the server's memory (docs/SIGNING.md "Sealed restore words").
//!
//! key  = HKDF (SHA-256)(salt = eph_pub || server_pub, ikm = X25519(eph, server), info = "itcmdr/restore-words/v1")
//! aad  = "itcmdr-restore-words/v1\n{key_id}\n{backup}"
//! ct   = XChaCha20-Poly1305(key, nonce(24), words UTF8, aad)
//!
//! Keys are single use and expire after 10 minutes. A real restore also needs the id of a dry run
//! that passed for the same backup and the same signing key within 30 minutes.
use anyhow::{anyhow, bail, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use parking_lot::Mutex;
use rand::RngCore;
use std::collections::HashMap;
use zeroize::Zeroizing;

pub const KEY_TTL_MS: i64 = 10 * 60 * 1000;
pub const DRY_RUN_TTL_MS: i64 = 30 * 60 * 1000;
const MAX_KEYS: usize = 16;
const INFO: &str = "itcmdr/restore-words/v1";

pub fn aad(key_id: &str, backup: &str) -> Vec<u8> {
    format!("itcmdr-restore-words/v1\n{key_id}\n{backup}").into_bytes()
}

fn derive(shared: &[u8], eph: &[u8; 32], server: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let mut salt = Vec::with_capacity(64);
    salt.extend_from_slice(eph);
    salt.extend_from_slice(server);
    crate::keys::hkdf32(&salt, shared, INFO)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    pub key_id: String,
    pub eph: String,
    pub nonce: String,
    pub ct: String,
}

/// What the app does (also used by tests and the pinned vector). `eph_secret`/`nonce` are random
/// in real use; fixed only for test vectors.
pub fn seal_with(
    server_pub: &[u8; 32],
    key_id: &str,
    backup: &str,
    words: &str,
    eph_secret: [u8; 32],
    nonce: [u8; 24],
) -> Result<Sealed> {
    let eph = x25519_dalek::StaticSecret::from(eph_secret);
    let eph_pub = x25519_dalek::PublicKey::from(&eph);
    let shared = eph.diffie_hellman(&x25519_dalek::PublicKey::from(*server_pub));
    let key = derive(shared.as_bytes(), eph_pub.as_bytes(), server_pub);
    let ct = XChaCha20Poly1305::new(key.as_ref().into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: words.as_bytes(),
                aad: &aad(key_id, backup),
            },
        )
        .map_err(|_| anyhow!("sealing failed"))?;
    Ok(Sealed {
        key_id: key_id.into(),
        eph: hex::encode(eph_pub.as_bytes()),
        nonce: hex::encode(nonce),
        ct: hex::encode(ct),
    })
}

pub fn seal(server_pub: &[u8; 32], key_id: &str, backup: &str, words: &str) -> Result<Sealed> {
    let mut e = [0u8; 32];
    let mut n = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut e);
    rand::rngs::OsRng.fill_bytes(&mut n);
    seal_with(server_pub, key_id, backup, words, e, n)
}

pub fn open(
    secret: &x25519_dalek::StaticSecret,
    s: &Sealed,
    backup: &str,
) -> Result<Zeroizing<String>> {
    let h32 = |v: &str| -> Result<[u8; 32]> {
        hex::decode(v)?
            .try_into()
            .map_err(|_| anyhow!("bad sealed words"))
    };
    let eph = h32(&s.eph)?;
    let nonce: [u8; 24] = hex::decode(&s.nonce)?
        .try_into()
        .map_err(|_| anyhow!("bad sealed words"))?;
    let me = x25519_dalek::PublicKey::from(secret);
    let shared = secret.diffie_hellman(&x25519_dalek::PublicKey::from(eph));
    let key = derive(shared.as_bytes(), &eph, me.as_bytes());
    let pt = Zeroizing::new(
        XChaCha20Poly1305::new(key.as_ref().into())
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &hex::decode(&s.ct)?,
                    aad: &aad(&s.key_id, backup),
                },
            )
            .map_err(|_| {
                anyhow!("the sealed words could not be opened (wrong key, backup or data)")
            })?,
    );
    Ok(Zeroizing::new(
        String::from_utf8(pt.to_vec()).map_err(|_| anyhow!("sealed words are not UTF8"))?,
    ))
}

struct Entry {
    secret: x25519_dalek::StaticSecret,
    owner: String,
    expires: i64,
}

#[derive(Default)]
pub struct RestoreKeys {
    keys: Mutex<HashMap<String, Entry>>,
    dry_runs: Mutex<HashMap<String, (String, String, i64)>>,
}

impl RestoreKeys {
    /// New one-time key for `owner` (the caller's key fingerprint). Returns (id, public hex, expires).
    pub fn issue(&self, owner: &str, now: i64) -> (String, String, i64) {
        let mut m = self.keys.lock();
        m.retain(|_, e| e.expires > now);
        while m.len() >= MAX_KEYS {
            let oldest = m
                .iter()
                .min_by_key(|(_, e)| e.expires)
                .map(|(k, _)| k.clone())
                .unwrap();
            m.remove(&oldest);
        }
        let secret = x25519_dalek::StaticSecret::random_from_rng(rand::rngs::OsRng);
        let public = hex::encode(x25519_dalek::PublicKey::from(&secret).as_bytes());
        let id = uuid::Uuid::new_v4().to_string();
        let expires = now + KEY_TTL_MS;
        m.insert(
            id.clone(),
            Entry {
                secret,
                owner: owner.into(),
                expires,
            },
        );
        (id, public, expires)
    }

    /// Open sealed words; the key is consumed whether or not opening works.
    pub fn open(
        &self,
        s: &Sealed,
        owner: &str,
        backup: &str,
        now: i64,
    ) -> Result<Zeroizing<String>> {
        let e = self
            .keys
            .lock()
            .remove(&s.key_id)
            .ok_or_else(|| anyhow!("unknown or already used restore key; ask for a new one"))?;
        if e.expires <= now {
            bail!("the restore key expired; ask for a new one");
        }
        if e.owner != owner {
            bail!("the restore key was issued to another signing key");
        }
        open(&e.secret, s, backup)
    }

    pub fn record_dry_run(&self, owner: &str, backup: &str, now: i64) -> String {
        let mut m = self.dry_runs.lock();
        m.retain(|_, v| v.2 > now);
        let id = uuid::Uuid::new_v4().to_string();
        m.insert(
            id.clone(),
            (owner.into(), backup.into(), now + DRY_RUN_TTL_MS),
        );
        id
    }

    /// Consume a dry-run id; it must match the owner and backup and still be fresh.
    pub fn take_dry_run(&self, id: &str, owner: &str, backup: &str, now: i64) -> Result<()> {
        let v = self
            .dry_runs
            .lock()
            .remove(id)
            .ok_or_else(|| anyhow!("run a dry run first (no passed dry run with that id)"))?;
        if v.2 <= now || v.0 != owner || v.1 != backup {
            bail!("the dry run expired or was for another backup or key; run it again");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_single_use() {
        let rk = RestoreKeys::default();
        let (id, pubhex, _) = rk.issue("fp", 0);
        let p: [u8; 32] = hex::decode(pubhex).unwrap().try_into().unwrap();
        let s = seal(&p, &id, "b1", "abandon ability").unwrap();
        assert!(
            rk.open(&s, "fp", "b2", 1).is_err(),
            "backup is bound in the aad"
        );
        let (id, pubhex, _) = rk.issue("fp", 0);
        let p: [u8; 32] = hex::decode(pubhex).unwrap().try_into().unwrap();
        let s = seal(&p, &id, "b1", "abandon ability").unwrap();
        assert!(rk.open(&s, "other", "b1", 1).is_err(), "owner bound");
        let (id, pubhex, _) = rk.issue("fp", 0);
        let p: [u8; 32] = hex::decode(pubhex).unwrap().try_into().unwrap();
        let s = seal(&p, &id, "b1", "abandon ability").unwrap();
        assert_eq!(
            rk.open(&s, "fp", "b1", 1).unwrap().as_str(),
            "abandon ability"
        );
        assert!(rk.open(&s, "fp", "b1", 1).is_err(), "single use");
        let (id, pubhex, _) = rk.issue("fp", 0);
        let p: [u8; 32] = hex::decode(pubhex).unwrap().try_into().unwrap();
        let s = seal(&p, &id, "b1", "x").unwrap();
        assert!(rk.open(&s, "fp", "b1", KEY_TTL_MS + 1).is_err(), "expiry");
    }

    #[test]
    fn dry_runs_bind_owner_and_backup() {
        let rk = RestoreKeys::default();
        let d = rk.record_dry_run("fp", "b1", 0);
        assert!(rk.take_dry_run(&d, "fp", "b2", 1).is_err());
        let d = rk.record_dry_run("fp", "b1", 0);
        assert!(rk.take_dry_run(&d, "fp", "b1", DRY_RUN_TTL_MS + 1).is_err());
        let d = rk.record_dry_run("fp", "b1", 0);
        rk.take_dry_run(&d, "fp", "b1", 1).unwrap();
        assert!(rk.take_dry_run(&d, "fp", "b1", 1).is_err(), "single use");
    }
}
