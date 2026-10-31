//! v0.12.0: one-time X25519 keys for sealed restore words, held in the server's memory only.
//! The crypto lives in `itcmdr_keys::restore` (docs/SIGNING.md "Sealed restore words").
//!
//! Keys are single use and expire after 10 minutes. A real restore also needs the id of a dry run
//! that passed for the same backup and the same signing key within 30 minutes.
use anyhow::{anyhow, bail, Result};
pub use itcmdr_keys::restore::{aad, open, seal, seal_with, Sealed};
use parking_lot::Mutex;
use std::collections::HashMap;
use zeroize::Zeroizing;

pub const KEY_TTL_MS: i64 = 10 * 60 * 1000;
pub const DRY_RUN_TTL_MS: i64 = 30 * 60 * 1000;
const MAX_KEYS: usize = 16;

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
