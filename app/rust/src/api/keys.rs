//! What the Flutter app calls. Every function is a thin wrapper over `itcmdr-keys`, so the app
//! signs and seals with exactly the server's code (native and WASM).
use anyhow::{anyhow, Result};
use flutter_rust_bridge::frb;
use itcmdr_keys as k;
use k::ed25519_dalek::Signer;

#[frb(init)]
pub fn init_app() {
    itcmdr_help::keep_stamp();
    flutter_rust_bridge::setup_default_user_utils();
}

/// A fresh random 32-byte private key (seed).
#[frb(sync)]
pub fn random_seed() -> Vec<u8> {
    k::random_seed().to_vec()
}

/// Ed25519 public key of a 32-byte seed.
#[frb(sync)]
pub fn public_key(seed: Vec<u8>) -> Result<Vec<u8>> {
    Ok(k::key_from_seed(&seed)?.verifying_key().to_bytes().to_vec())
}

/// Fingerprint (sha256 hex) of a 32-byte public key.
#[frb(sync)]
pub fn fingerprint(public_key: Vec<u8>) -> Result<String> {
    let b: [u8; 32] = public_key
        .try_into()
        .map_err(|_| anyhow!("public key must be 32 bytes"))?;
    let pk = k::ed25519_dalek::VerifyingKey::from_bytes(&b)
        .map_err(|e| anyhow!("invalid public key: {e}"))?;
    Ok(k::fingerprint(&pk))
}

/// Ed25519 signature (64 bytes) over `message`.
#[frb(sync)]
pub fn sign(seed: Vec<u8>, message: Vec<u8>) -> Result<Vec<u8>> {
    Ok(k::key_from_seed(&seed)?.sign(&message).to_bytes().to_vec())
}

/// `itcmdr-req/v1\n{method}\n{ts}\n{nonce}\n{sha256 hex of body}`
#[frb(sync)]
pub fn request_message(method: String, ts_ms: i64, nonce: String, body: Vec<u8>) -> Vec<u8> {
    k::request_message(&method, ts_ms, &nonce, &body)
}

#[frb(sync)]
pub fn sha256_hex(data: Vec<u8>) -> String {
    k::sha256_hex(&data)
}

pub struct SealedWordsOut {
    pub key_id: String,
    pub eph: String,
    pub nonce: String,
    pub ct: String,
}

/// Seal restore words for a server's one-time X25519 key. `eph_seed`/`nonce` only in tests.
#[frb(sync)]
pub fn seal_words(
    server_public: Vec<u8>,
    key_id: String,
    backup: String,
    words: String,
    eph_seed: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
) -> Result<SealedWordsOut> {
    let server: [u8; 32] = server_public
        .try_into()
        .map_err(|_| anyhow!("bad restore key"))?;
    let words = words.trim();
    let s = match (eph_seed, nonce) {
        (Some(e), Some(n)) => k::restore::seal_with(
            &server,
            &key_id,
            &backup,
            words,
            e.try_into()
                .map_err(|_| anyhow!("eph seed must be 32 bytes"))?,
            n.try_into()
                .map_err(|_| anyhow!("nonce must be 24 bytes"))?,
        )?,
        (None, None) => k::restore::seal(&server, &key_id, &backup, words)?,
        _ => return Err(anyhow!("pass both eph_seed and nonce, or neither")),
    };
    Ok(SealedWordsOut {
        key_id: s.key_id,
        eph: s.eph,
        nonce: s.nonce,
        ct: s.ct,
    })
}
