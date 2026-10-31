//! v0.12.0: the 24 words for an app-triggered restore are sealed on the app, end to end, for a
//! one-time X25519 key that only lives in the server's memory (docs/SIGNING.md "Sealed restore words").
//!
//! key  = HKDF (SHA-256)(salt = eph_pub || server_pub, ikm = X25519(eph, server), info = "itcmdr/restore-words/v1")
//! aad  = "itcmdr-restore-words/v1\n{key_id}\n{backup}"
//! ct   = XChaCha20-Poly1305(key, nonce(24), words UTF8, aad)
//!
//! The one-time key store (single use, 10 min expiry) stays in the server (`restore_seal`).
use anyhow::{anyhow, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroizing;

pub use x25519_dalek::StaticSecret;
const INFO: &str = "itcmdr/restore-words/v1";

pub fn aad(key_id: &str, backup: &str) -> Vec<u8> {
    format!("itcmdr-restore-words/v1\n{key_id}\n{backup}").into_bytes()
}

fn derive(shared: &[u8], eph: &[u8; 32], server: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let mut salt = Vec::with_capacity(64);
    salt.extend_from_slice(eph);
    salt.extend_from_slice(server);
    crate::hkdf32(&salt, shared, INFO)
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
