//! v0.19.0: client-side encrypted user key backup (ADMINSVR.md decision 17). The device key seed
//! is encrypted on the device with a key derived from the user's 24 words (HKDF over the BIP39
//! seed, info `itcmdr/user-key-wrap/v1`), XChaCha20-Poly1305 with a random nonce and the user id
//! and device fingerprint as AAD. adminsvr only ever stores the resulting blob.
use anyhow::{anyhow, bail, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub const INFO: &str = "itcmdr/user-key-wrap/v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Wrapped {
    pub v: u32,
    pub kdf: String, // "words-hkdf"
    pub user: String,
    pub device: String, // fingerprint of the wrapped key
    pub nonce: String,
    pub ct: String,
}

fn seed_of(words: &str) -> Result<Zeroizing<[u8; 64]>> {
    let normalized = words
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let m = bip39::Mnemonic::parse_normalized(&normalized)
        .map_err(|e| anyhow!("invalid recovery words: {e}"))?;
    if m.word_count() != crate::WORDS {
        bail!(
            "expected {} recovery words, got {}",
            crate::WORDS,
            m.word_count()
        );
    }
    Ok(Zeroizing::new(m.to_seed("")))
}

fn aad(user: &str, device: &str) -> Vec<u8> {
    format!("itcmdr-user-key-wrap/v1\n{user}\n{device}").into_bytes()
}

/// Encrypt a 32-byte device key seed with the words.
pub fn wrap(words: &str, user: &str, seed: &[u8; 32]) -> Result<Wrapped> {
    let s = seed_of(words)?;
    let key = crate::hkdf32(b"itcmdr", s.as_ref(), INFO);
    let device = crate::fingerprint(&crate::key_from_seed(seed)?.verifying_key());
    let mut nonce = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ct = XChaCha20Poly1305::new(key.as_ref().into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: seed,
                aad: &aad(user, &device),
            },
        )
        .map_err(|_| anyhow!("key wrap failed"))?;
    Ok(Wrapped {
        v: 1,
        kdf: "words-hkdf".into(),
        user: user.into(),
        device,
        nonce: hex::encode(nonce),
        ct: hex::encode(ct),
    })
}

/// Decrypt a backup blob with the words. Fails on other words or any change to the blob.
pub fn unwrap(words: &str, w: &Wrapped) -> Result<Zeroizing<[u8; 32]>> {
    if w.v != 1 || w.kdf != "words-hkdf" {
        bail!("unsupported key backup format");
    }
    let s = seed_of(words)?;
    let key = crate::hkdf32(b"itcmdr", s.as_ref(), INFO);
    let nonce = hex::decode(&w.nonce).context("bad nonce")?;
    if nonce.len() != 24 {
        bail!("bad nonce");
    }
    let pt = Zeroizing::new(
        XChaCha20Poly1305::new(key.as_ref().into())
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &hex::decode(&w.ct).context("bad ciphertext")?,
                    aad: &aad(&w.user, &w.device),
                },
            )
            .map_err(|_| anyhow!("these recovery words do not open this key backup"))?,
    );
    let seed: [u8; 32] = pt
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("bad key length"))?;
    let out = Zeroizing::new(seed);
    if crate::fingerprint(&crate::key_from_seed(out.as_ref())?.verifying_key()) != w.device {
        bail!("key backup does not match its device fingerprint");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_refusals() {
        let words = crate::generate_words();
        let seed = crate::random_seed();
        let w = wrap(&words, "u1", &seed).unwrap();
        assert!(!w.ct.contains(&hex::encode(seed.as_ref())));
        assert_eq!(unwrap(&words, &w).unwrap().as_ref(), seed.as_ref());
        assert!(unwrap(&crate::generate_words(), &w).is_err());
        let mut t = w.clone();
        t.user = "u2".into();
        assert!(unwrap(&words, &t).is_err());
        let mut t = w.clone();
        let mut ct = hex::decode(&t.ct).unwrap();
        ct[0] ^= 1;
        t.ct = hex::encode(ct);
        assert!(unwrap(&words, &t).is_err());
    }
}
