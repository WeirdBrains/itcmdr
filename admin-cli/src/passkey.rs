//! v0.19.1 passkeys / FIDO2 security keys for the CLI: registration, step-up for actions on the
//! adminsvr policy, and unlocking the local device key with the key's hmac-secret.
//! The CLI talks to a USB/NFC security key through libfido2's tools (`fido2-token`, `fido2-cred`,
//! `fido2-assert`, from the libfido2 package); platform passkeys (phone, Windows Hello) are app-only.
use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::process::Command;
use zeroize::Zeroizing;

/// A new credential as the authenticator returns it.
#[derive(Debug, Clone)]
pub struct Made {
    pub fmt: String,
    pub auth_data: Vec<u8>,
    pub sig: Vec<u8>,
    pub x5c: Vec<Vec<u8>>,
}

/// An assertion (and the hmac-secret output when a salt was given).
#[derive(Debug, Clone)]
pub struct Asserted {
    pub auth_data: Vec<u8>,
    pub sig: Vec<u8>,
    pub hmac_secret: Option<Zeroizing<Vec<u8>>>,
}

/// A FIDO2 authenticator (CTAP). `client_data_hash` is sha256 of the client data JSON.
pub trait Authenticator: Send + Sync {
    fn make_credential(
        &self,
        rp_id: &str,
        user_id: &[u8],
        user_name: &str,
        client_data_hash: &[u8; 32],
        hmac_secret: bool,
    ) -> Result<Made>;
    fn get_assertion(
        &self,
        rp_id: &str,
        credential_id: &[u8],
        client_data_hash: &[u8; 32],
        hmac_salt: Option<&[u8; 32]>,
    ) -> Result<Asserted>;
}

/// libfido2's command line tools (installed with the libfido2 package).
pub struct Fido2Tools {
    /// Device path (`fido2-token -L`); empty = the first security key found.
    pub device: String,
}

impl Fido2Tools {
    pub fn new() -> Self {
        Fido2Tools {
            device: std::env::var("ITCMDR_FIDO2_DEVICE").unwrap_or_default(),
        }
    }
    fn device(&self) -> Result<String> {
        if !self.device.is_empty() {
            return Ok(self.device.clone());
        }
        let out = Command::new("fido2-token").arg("-L").output().context(
            "fido2-token not found: install libfido2 (its tools drive the security key)",
        )?;
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_once(": ").map(|(d, _)| d.trim().to_string()))
            .find(|d| !d.is_empty() && !d.starts_with("windows://hello"))
            .ok_or_else(|| anyhow!("no security key found (plug it in; fido2-token -L lists none)"))
    }
    fn run(&self, tool: &str, args: &[&str], input: &[String]) -> Result<Vec<String>> {
        let dir = tempfile::tempdir()?;
        let (i, o) = (dir.path().join("in"), dir.path().join("out"));
        std::fs::write(&i, input.join("\n") + "\n")?;
        let dev = self.device()?;
        eprintln!("Touch your security key…");
        let st = Command::new(tool)
            .args(args)
            .arg("-i")
            .arg(&i)
            .arg("-o")
            .arg(&o)
            .arg(&dev)
            .args(if tool == "fido2-cred" {
                &["es256"][..]
            } else {
                &[][..]
            })
            .status()
            .with_context(|| format!("{tool} not found: install libfido2"))?;
        ensure!(
            st.success(),
            "{tool} failed (no touch, wrong PIN, or not this key)"
        );
        Ok(std::fs::read_to_string(&o)?
            .lines()
            .map(str::to_string)
            .collect())
    }
}

impl Default for Fido2Tools {
    fn default() -> Self {
        Self::new()
    }
}

fn b64(s: Option<&String>) -> Result<Vec<u8>> {
    Ok(B64.decode(
        s.ok_or_else(|| anyhow!("short output from the fido2 tool"))?
            .trim(),
    )?)
}

impl Authenticator for Fido2Tools {
    fn make_credential(
        &self,
        rp_id: &str,
        user_id: &[u8],
        user_name: &str,
        cdh: &[u8; 32],
        hmac: bool,
    ) -> Result<Made> {
        let mut args = vec!["-M"];
        if hmac {
            args.push("-h");
        }
        let out = self.run(
            "fido2-cred",
            &args,
            &[
                B64.encode(cdh),
                rp_id.into(),
                user_name.into(),
                B64.encode(user_id),
            ],
        )?;
        // client data hash, rp id, fmt, authdata, credential id, signature, [x5c]
        Ok(Made {
            fmt: out.get(2).cloned().unwrap_or_default(),
            auth_data: b64(out.get(3))?,
            sig: b64(out.get(5))?,
            x5c: out
                .get(6)
                .filter(|s| !s.trim().is_empty())
                .map(|s| B64.decode(s.trim()))
                .transpose()?
                .into_iter()
                .collect(),
        })
    }
    fn get_assertion(
        &self,
        rp_id: &str,
        cred: &[u8],
        cdh: &[u8; 32],
        salt: Option<&[u8; 32]>,
    ) -> Result<Asserted> {
        let mut args = vec!["-G"];
        let mut input = vec![B64.encode(cdh), rp_id.into(), B64.encode(cred)];
        if let Some(s) = salt {
            args.push("-h");
            input.push(B64.encode(s));
        }
        let out = self.run("fido2-assert", &args, &input)?;
        // client data hash, rp id, authdata, signature, [hmac-secret]
        Ok(Asserted {
            auth_data: b64(out.get(2))?,
            sig: b64(out.get(3))?,
            hmac_secret: if salt.is_some() {
                Some(Zeroizing::new(b64(out.get(4))?))
            } else {
                None
            },
        })
    }
}

/// A passkey this device registered (so the CLI knows which credential to ask the key for).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocalPasskey {
    pub credential_id: String,
    pub rp_id: String,
    pub label: String,
    pub hmac_secret: bool,
}

/// The device key encrypted with a key from the passkey's hmac-secret (identity file, v0.19.1).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Locked {
    pub credential_id: String,
    pub rp_id: String,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

const LOCK_INFO: &[u8] = b"itcmdr-admin/identity-lock/v1";

fn lock_key(secret: &[u8], cred: &str) -> Zeroizing<[u8; 32]> {
    let hk = hkdf::Hkdf::<sha2::Sha256>::new(Some(cred.as_bytes()), secret);
    let mut k = Zeroizing::new([0u8; 32]);
    hk.expand(LOCK_INFO, k.as_mut()).expect("hkdf");
    k
}

fn random_cdh() -> [u8; 32] {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}

/// Encrypt `seed_hex` under the passkey (one touch).
pub fn lock(auth: &dyn Authenticator, pk: &LocalPasskey, seed_hex: &str) -> Result<Locked> {
    ensure!(
        pk.hmac_secret,
        "this passkey was registered without hmac-secret; add one with --unlock"
    );
    let mut salt = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let cred = itcmdr_adminlink::webauthn::from_b64url(&pk.credential_id)?;
    let a = auth.get_assertion(&pk.rp_id, &cred, &random_cdh(), Some(&salt))?;
    let secret = a
        .hmac_secret
        .ok_or_else(|| anyhow!("the key returned no hmac-secret"))?;
    let k = lock_key(&secret, &pk.credential_id);
    let mut n = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut n);
    let ct = XChaCha20Poly1305::new(k.as_ref().into())
        .encrypt(
            XNonce::from_slice(&n),
            Payload {
                msg: seed_hex.as_bytes(),
                aad: pk.credential_id.as_bytes(),
            },
        )
        .map_err(|_| anyhow!("encrypt"))?;
    Ok(Locked {
        credential_id: pk.credential_id.clone(),
        rp_id: pk.rp_id.clone(),
        salt: hex::encode(salt),
        nonce: hex::encode(n),
        ciphertext: hex::encode(ct),
    })
}

/// Decrypt the device key (one touch).
pub fn unlock(auth: &dyn Authenticator, l: &Locked) -> Result<Zeroizing<String>> {
    let salt: [u8; 32] = hex::decode(&l.salt)?
        .try_into()
        .map_err(|_| anyhow!("bad salt"))?;
    let cred = itcmdr_adminlink::webauthn::from_b64url(&l.credential_id)?;
    let a = auth.get_assertion(&l.rp_id, &cred, &random_cdh(), Some(&salt))?;
    let secret = a
        .hmac_secret
        .ok_or_else(|| anyhow!("the key returned no hmac-secret"))?;
    let k = lock_key(&secret, &l.credential_id);
    let pt = XChaCha20Poly1305::new(k.as_ref().into())
        .decrypt(
            XNonce::from_slice(&hex::decode(&l.nonce)?),
            Payload {
                msg: &hex::decode(&l.ciphertext)?,
                aad: l.credential_id.as_bytes(),
            },
        )
        .map_err(|_| anyhow!("unlock failed: not the key this identity was locked with"))?;
    Ok(Zeroizing::new(String::from_utf8(pt)?))
}

#[cfg(feature = "soft-authenticator")]
pub mod soft {
    //! A software FIDO2 authenticator for tests: ES256, packed self-attestation, hmac-secret.
    use super::*;
    use hmac::Mac;
    use itcmdr_adminlink::webauthn as wa;
    use p256::ecdsa::{signature::Signer, Signature, SigningKey};
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct Cred {
        key: SigningKey,
        hmac: [u8; 32],
        count: u32,
    }

    #[derive(Default)]
    pub struct Soft {
        creds: Mutex<HashMap<Vec<u8>, Cred>>,
        /// Answer without user presence (no touch).
        pub no_touch: std::sync::atomic::AtomicBool,
        /// Keep the counter (a cloned key).
        pub freeze_counter: std::sync::atomic::AtomicBool,
    }

    impl Authenticator for Soft {
        fn make_credential(
            &self,
            rp_id: &str,
            _: &[u8],
            _: &str,
            cdh: &[u8; 32],
            _hmac: bool,
        ) -> Result<Made> {
            let key = SigningKey::random(&mut rand::rngs::OsRng);
            let mut id = vec![0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut id);
            let mut hm = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut hm);
            let sec1 = key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec();
            let ad = wa::build_auth_data(
                rp_id,
                wa::FLAG_UP,
                1,
                Some((&[0x5a; 16], &id, &wa::cose_es256(&sec1))),
            );
            let sig: Signature = key.sign(&[ad.as_slice(), cdh].concat());
            self.creds.lock().unwrap().insert(
                id,
                Cred {
                    key,
                    hmac: hm,
                    count: 1,
                },
            );
            // CBOR-wrapped like fido2-cred prints it
            let mut w = vec![];
            ciborium::ser::into_writer(&ciborium::value::Value::Bytes(ad), &mut w)?;
            Ok(Made {
                fmt: "packed".into(),
                auth_data: w,
                sig: sig.to_der().as_bytes().to_vec(),
                x5c: vec![],
            })
        }
        fn get_assertion(
            &self,
            rp_id: &str,
            cred: &[u8],
            cdh: &[u8; 32],
            salt: Option<&[u8; 32]>,
        ) -> Result<Asserted> {
            let mut m = self.creds.lock().unwrap();
            let c = m
                .get_mut(cred)
                .ok_or_else(|| anyhow!("no such credential on this key"))?;
            if !self
                .freeze_counter
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                c.count += 1;
            }
            let up = if self.no_touch.load(std::sync::atomic::Ordering::SeqCst) {
                0
            } else {
                wa::FLAG_UP
            };
            let ad = wa::build_auth_data(rp_id, up, c.count, None);
            let sig: Signature = c.key.sign(&[ad.as_slice(), cdh].concat());
            let hmac_secret = salt.map(|s| {
                let mut h =
                    <hmac::Hmac<sha2::Sha256> as Mac>::new_from_slice(&c.hmac).expect("hmac");
                h.update(s);
                Zeroizing::new(h.finalize().into_bytes().to_vec())
            });
            Ok(Asserted {
                auth_data: ad,
                sig: sig.to_der().as_bytes().to_vec(),
                hmac_secret,
            })
        }
    }
}

pub fn check_label(l: &str) -> Result<()> {
    if l.trim().is_empty() {
        bail!("label is required");
    }
    Ok(())
}
