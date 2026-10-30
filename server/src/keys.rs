//! Key material: BIP39 words -> master keys, fingerprints, certificates, db.key escrow.
//!
//! Derivation (all v1, documented in server/PLAN.md decision 4/12):
//! - seed = BIP39 seed of the 24 words, empty passphrase (64 bytes)
//! - master Ed25519 secret = HKDF (SHA-256)(salt "itcmdr", ikm seed, info "itcmdr/master/ed25519/v1")
//! - master X25519 secret  = HKDF (SHA-256)(salt "itcmdr", ikm seed, info "itcmdr/master/x25519/v1")
//!
//! Certificates: the master signs
//! `itcmdr-cert/v1\n{kind}\n{fingerprint}\n{role}\n{name}\n{issued_ms}`.
//!
//! Escrow of a secret for the master X25519 public key: ephemeral X25519 DH, key =
//! HKDF (SHA-256)(salt eph_pub || recipient_pub, ikm shared, info "itcmdr/escrow/v1"),
//! XChaCha20-Poly1305 with a random 24-byte nonce and AAD "itcmdr-escrow/v1".
use anyhow::{anyhow, bail, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub const WORDS: usize = 24;

/// 24 fresh BIP39 words (256 bits of entropy).
pub fn generate_words() -> String {
    let mut entropy = Zeroizing::new([0u8; 32]);
    rand::rngs::OsRng.fill_bytes(entropy.as_mut());
    bip39::Mnemonic::from_entropy(entropy.as_ref())
        .expect("32 bytes is valid entropy")
        .to_string()
}

fn hkdf32(salt: &[u8], ikm: &[u8], info: &str) -> Zeroizing<[u8; 32]> {
    let mut out = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(Some(salt), ikm)
        .expand(info.as_bytes(), out.as_mut())
        .expect("32 bytes is a valid HKDF length");
    out
}

/// Keys rebuilt from the words. Never stored on the server.
pub struct Master {
    pub signing: SigningKey,
    pub x25519: x25519_dalek::StaticSecret,
}

impl Master {
    pub fn from_words(words: &str) -> Result<Self> {
        let normalized = words
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let m = bip39::Mnemonic::parse_normalized(&normalized)
            .map_err(|e| anyhow!("invalid recovery words: {e}"))?;
        if m.word_count() != WORDS {
            bail!("expected {WORDS} recovery words, got {}", m.word_count());
        }
        let seed = Zeroizing::new(m.to_seed(""));
        let ed = hkdf32(b"itcmdr", seed.as_ref(), "itcmdr/master/ed25519/v1");
        let x = hkdf32(b"itcmdr", seed.as_ref(), "itcmdr/master/x25519/v1");
        Ok(Self {
            signing: SigningKey::from_bytes(&ed),
            x25519: x25519_dalek::StaticSecret::from(*x),
        })
    }
    pub fn public(&self) -> MasterPublic {
        MasterPublic {
            ed25519: self.signing.verifying_key(),
            x25519: x25519_dalek::PublicKey::from(&self.x25519),
        }
    }
}

/// What the server keeps.
#[derive(Clone, Debug)]
pub struct MasterPublic {
    pub ed25519: VerifyingKey,
    pub x25519: x25519_dalek::PublicKey,
}

/// sha256(public key bytes), lowercase hex.
pub fn fingerprint(pk: &VerifyingKey) -> String {
    hex::encode(Sha256::digest(pk.as_bytes()))
}

pub fn parse_public_key(hex_s: &str) -> Result<VerifyingKey> {
    let b: [u8; 32] = hex::decode(hex_s.trim())
        .context("public key is not hex")?
        .try_into()
        .map_err(|_| anyhow!("public key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&b).map_err(|e| anyhow!("invalid Ed25519 public key: {e}"))
}

pub fn new_keypair() -> SigningKey {
    SigningKey::generate(&mut rand::rngs::OsRng)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Certificate {
    pub v: u32,
    pub kind: String, // "client" | "user"
    pub name: String,
    pub role: String,
    pub fingerprint: String,
    pub issued_ms: i64,
    pub sig: String, // hex Ed25519 signature by the master
}

fn cert_message(kind: &str, fp: &str, role: &str, name: &str, issued_ms: i64) -> Vec<u8> {
    format!("itcmdr-cert/v1\n{kind}\n{fp}\n{role}\n{name}\n{issued_ms}").into_bytes()
}

/// Sign a certificate with the master (offline: needs the words).
pub fn certify(
    master: &SigningKey,
    kind: &str,
    name: &str,
    role: &str,
    pk: &VerifyingKey,
) -> Certificate {
    let fp = fingerprint(pk);
    let issued_ms = chrono::Utc::now().timestamp_millis();
    let sig = master.sign(&cert_message(kind, &fp, role, name, issued_ms));
    Certificate {
        v: 1,
        kind: kind.into(),
        name: name.into(),
        role: role.into(),
        fingerprint: fp,
        issued_ms,
        sig: hex::encode(sig.to_bytes()),
    }
}

pub fn verify_certificate(master: &VerifyingKey, c: &Certificate) -> Result<()> {
    if c.v != 1 {
        bail!("unsupported certificate version {}", c.v);
    }
    let sig: [u8; 64] = hex::decode(&c.sig)
        .context("certificate signature is not hex")?
        .try_into()
        .map_err(|_| anyhow!("bad signature length"))?;
    master
        .verify(
            &cert_message(&c.kind, &c.fingerprint, &c.role, &c.name, c.issued_ms),
            &Signature::from_bytes(&sig),
        )
        .map_err(|_| anyhow!("certificate is not signed by this server's master key"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Escrow {
    pub v: u32,
    pub eph: String,
    pub nonce: String,
    pub ct: String,
}

const ESCROW_AAD: &[u8] = b"itcmdr-escrow/v1";

fn escrow_key(shared: &[u8], eph: &[u8; 32], recipient: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let mut salt = Vec::with_capacity(64);
    salt.extend_from_slice(eph);
    salt.extend_from_slice(recipient);
    hkdf32(&salt, shared, "itcmdr/escrow/v1")
}

/// Seal `secret` so only the holder of the words can open it.
pub fn seal(recipient: &x25519_dalek::PublicKey, secret: &[u8]) -> Result<Escrow> {
    let eph = x25519_dalek::StaticSecret::random_from_rng(rand::rngs::OsRng);
    let eph_pub = x25519_dalek::PublicKey::from(&eph);
    let shared = eph.diffie_hellman(recipient);
    let key = escrow_key(shared.as_bytes(), eph_pub.as_bytes(), recipient.as_bytes());
    let mut nonce = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ct = XChaCha20Poly1305::new(key.as_ref().into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: secret,
                aad: ESCROW_AAD,
            },
        )
        .map_err(|_| anyhow!("escrow encryption failed"))?;
    Ok(Escrow {
        v: 1,
        eph: hex::encode(eph_pub.as_bytes()),
        nonce: hex::encode(nonce),
        ct: hex::encode(ct),
    })
}

pub fn unseal(master: &Master, e: &Escrow) -> Result<Zeroizing<Vec<u8>>> {
    if e.v != 1 {
        bail!("unsupported escrow version {}", e.v);
    }
    let eph: [u8; 32] = hex::decode(&e.eph)?
        .try_into()
        .map_err(|_| anyhow!("bad escrow"))?;
    let nonce: [u8; 24] = hex::decode(&e.nonce)?
        .try_into()
        .map_err(|_| anyhow!("bad escrow"))?;
    let eph_pub = x25519_dalek::PublicKey::from(eph);
    let shared = master.x25519.diffie_hellman(&eph_pub);
    let me = x25519_dalek::PublicKey::from(&master.x25519);
    let key = escrow_key(shared.as_bytes(), &eph, me.as_bytes());
    let pt = XChaCha20Poly1305::new(key.as_ref().into())
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &hex::decode(&e.ct)?,
                aad: ESCROW_AAD,
            },
        )
        .map_err(|_| anyhow!("these recovery words do not open this escrow"))?;
    Ok(Zeroizing::new(pt))
}

/// Canonical request message signed by clients (see `auth`).
pub fn request_message(method: &str, ts_ms: i64, nonce: &str, body: &[u8]) -> Vec<u8> {
    format!(
        "itcmdr-req/v1\n{method}\n{ts_ms}\n{nonce}\n{}",
        hex::encode(Sha256::digest(body))
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_rebuild_the_same_master() {
        let w = generate_words();
        assert_eq!(w.split(' ').count(), 24);
        let a = Master::from_words(&w).unwrap();
        let b =
            Master::from_words(&format!("  {}  ", w.to_uppercase().replace(' ', "   "))).unwrap();
        assert_eq!(a.public().ed25519, b.public().ed25519);
        assert_eq!(a.public().x25519.as_bytes(), b.public().x25519.as_bytes());
        let other = Master::from_words(&generate_words()).unwrap();
        assert_ne!(a.public().ed25519, other.public().ed25519);
        assert!(Master::from_words("abandon abandon").is_err());
        // 12 valid words are refused (we require 24).
        assert!(Master::from_words("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").is_err());
    }

    #[test]
    fn known_vector_is_stable() {
        // Derivation must never change: words written down today must work in every later version.
        let w = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
        let m = Master::from_words(w).unwrap();
        let fp = fingerprint(&m.public().ed25519);
        assert_eq!(fp.len(), 64);
        // Pinned value: change only together with a documented derivation version bump.
        assert_eq!(
            fp, "cf6f7e6b7c4206b44e4590982041fae3c6c02f700450d1a000ec4fa25bb3ccc8",
            "master derivation changed!"
        );
    }

    #[test]
    fn certificates() {
        let m = Master::from_words(&generate_words()).unwrap();
        let k = new_keypair();
        let c = certify(&m.signing, "user", "Front Desk", "user", &k.verifying_key());
        verify_certificate(&m.public().ed25519, &c).unwrap();
        let mut t = c.clone();
        t.role = "owner".into();
        assert!(
            verify_certificate(&m.public().ed25519, &t).is_err(),
            "role can't be raised"
        );
        let other = Master::from_words(&generate_words()).unwrap();
        assert!(verify_certificate(&other.public().ed25519, &c).is_err());
    }

    #[test]
    fn escrow_round_trip() {
        let w = generate_words();
        let m = Master::from_words(&w).unwrap();
        let e = seal(&m.public().x25519, &[42u8; 32]).unwrap();
        assert_eq!(
            unseal(&Master::from_words(&w).unwrap(), &e)
                .unwrap()
                .as_slice(),
            &[42u8; 32]
        );
        assert!(unseal(&Master::from_words(&generate_words()).unwrap(), &e).is_err());
        let mut bad = e;
        bad.ct
            .replace_range(0..2, if &bad.ct[0..2] == "00" { "01" } else { "00" });
        assert!(unseal(&m, &bad).is_err(), "tampering is detected");
    }
}
