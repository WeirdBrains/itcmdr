//! v0.19.1 WebAuthn (passkeys / FIDO2 security keys) as a second factor: the parts adminsvr and
//! the clients share. adminsvr verifies registrations and assertions; clients build the client
//! data and the step-up challenge. Algorithms: ES256 (-7) and EdDSA (-8). Attestation formats:
//! `none` and `packed` (self or x5c; the certificate chain is recorded, not trusted to a root).
//!
//! Step-up binding (ADMINSVR.md "Passkeys"): the challenge is
//! `sha256("itcmdr-stepup/v1" || payload_hash || nonce)`, where
//! `payload_hash = sha256(method || "\n" || request body)` and `nonce` is single use, from adminsvr.
use anyhow::{anyhow, bail, ensure, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64URL;
use base64::Engine;
use ciborium::value::Value;
use sha2::{Digest, Sha256};

pub const STEPUP_DOMAIN: &[u8] = b"itcmdr-stepup/v1";
/// Origin the CLI puts in its client data (it talks CTAP to the key, no browser).
pub const CLI_ORIGIN: &str = "itcmdr-admin:cli";
pub const ES256: i64 = -7;
pub const EDDSA: i64 = -8;
pub const FLAG_UP: u8 = 0x01;
pub const FLAG_UV: u8 = 0x04;
pub const FLAG_AT: u8 = 0x40;

fn sha(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}

pub fn payload_hash(method: &str, body: &[u8]) -> [u8; 32] {
    sha(&[method.as_bytes(), b"\n", body])
}

pub fn stepup_challenge(method: &str, body: &[u8], nonce: &str) -> [u8; 32] {
    sha(&[STEPUP_DOMAIN, &payload_hash(method, body), nonce.as_bytes()])
}

pub fn b64url(b: &[u8]) -> String {
    B64URL.encode(b)
}
pub fn from_b64url(s: &str) -> Result<Vec<u8>> {
    Ok(B64URL.decode(s.trim_end_matches('='))?)
}

/// clientDataJSON as a browser makes it (`kind` is webauthn.create or webauthn.get).
pub fn client_data(kind: &str, challenge: &[u8], origin: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": kind, "challenge": b64url(challenge), "origin": origin, "crossOrigin": false
    }))
    .expect("json")
}

pub fn client_data_hash(cd: &[u8]) -> [u8; 32] {
    sha(&[cd])
}

/// What adminsvr accepts: RP id, allowed origins, whether user verification is required.
#[derive(Debug, Clone)]
pub struct Rp {
    pub id: String,
    pub origins: Vec<String>,
    pub require_uv: bool,
}

fn check_client_data(rp: &Rp, cd: &[u8], kind: &str, challenge: &[u8]) -> Result<()> {
    let v: serde_json::Value =
        serde_json::from_slice(cd).map_err(|_| anyhow!("client data is not JSON"))?;
    ensure!(v["type"] == kind, "client data type is not {kind}");
    let c = from_b64url(v["challenge"].as_str().unwrap_or_default())?;
    ensure!(
        c == challenge,
        "challenge does not match (another request or an old challenge)"
    );
    let o = v["origin"].as_str().unwrap_or_default();
    ensure!(
        rp.origins.iter().any(|x| x == o),
        "origin {o} is not allowed"
    );
    Ok(())
}

/// Authenticator data (the raw bytes; `unwrap_cbor_bytes` first if a tool CBOR-wrapped them).
#[derive(Debug, Clone)]
pub struct AuthData {
    pub rp_id_hash: [u8; 32],
    pub flags: u8,
    pub sign_count: u32,
    pub aaguid: [u8; 16],
    pub credential_id: Vec<u8>,
    pub cose_key: Option<Value>,
}

pub fn parse_auth_data(b: &[u8]) -> Result<AuthData> {
    ensure!(b.len() >= 37, "authenticator data too short");
    let mut a = AuthData {
        rp_id_hash: b[..32].try_into()?,
        flags: b[32],
        sign_count: u32::from_be_bytes(b[33..37].try_into()?),
        aaguid: [0; 16],
        credential_id: vec![],
        cose_key: None,
    };
    if a.flags & FLAG_AT != 0 {
        ensure!(b.len() >= 55, "attested credential data too short");
        a.aaguid = b[37..53].try_into()?;
        let n = u16::from_be_bytes(b[53..55].try_into()?) as usize;
        ensure!(
            n > 0 && n <= 1023 && b.len() >= 55 + n,
            "bad credential id length"
        );
        a.credential_id = b[55..55 + n].to_vec();
        let mut cur = std::io::Cursor::new(&b[55 + n..]);
        let v: Value = ciborium::de::from_reader(&mut cur)
            .map_err(|_| anyhow!("bad credential public key"))?;
        a.cose_key = Some(v);
    }
    Ok(a)
}

/// libfido2's tools print authenticator data as a CBOR byte string; take the bytes out.
pub fn unwrap_cbor_bytes(b: &[u8]) -> Vec<u8> {
    if let Ok(Value::Bytes(inner)) = ciborium::de::from_reader::<Value, _>(b) {
        let enc = {
            let mut v = vec![];
            ciborium::ser::into_writer(&Value::Bytes(inner.clone()), &mut v).ok();
            v
        };
        if enc == b && inner.len() >= 37 {
            return inner;
        }
    }
    b.to_vec()
}

fn int(v: &Value) -> Option<i128> {
    v.as_integer().map(i128::from)
}
fn get(m: &[(Value, Value)], k: i128) -> Option<&Value> {
    m.iter().find(|(a, _)| int(a) == Some(k)).map(|(_, b)| b)
}
fn get_s<'a>(m: &'a [(Value, Value)], k: &str) -> Option<&'a Value> {
    m.iter()
        .find(|(a, _)| a.as_text() == Some(k))
        .map(|(_, b)| b)
}

/// (alg, raw public key): ES256 = SEC1 uncompressed point (65 bytes), EdDSA = 32 bytes.
pub fn cose_public_key(v: &Value) -> Result<(i64, Vec<u8>)> {
    let m = v.as_map().ok_or_else(|| anyhow!("COSE key is not a map"))?;
    let kty = get(m, 1).and_then(int);
    let alg = get(m, 3).and_then(int);
    let crv = get(m, -1).and_then(int);
    let x = get(m, -2)
        .and_then(|v| v.as_bytes())
        .cloned()
        .unwrap_or_default();
    match (kty, alg, crv) {
        (Some(2), Some(-7), Some(1)) => {
            let y = get(m, -3)
                .and_then(|v| v.as_bytes())
                .cloned()
                .unwrap_or_default();
            ensure!(x.len() == 32 && y.len() == 32, "bad P-256 point");
            let mut k = vec![4u8];
            k.extend(x);
            k.extend(y);
            p256::ecdsa::VerifyingKey::from_sec1_bytes(&k).map_err(|_| anyhow!("bad P-256 key"))?;
            Ok((ES256, k))
        }
        (Some(1), Some(-8), Some(6)) => {
            ensure!(x.len() == 32, "bad Ed25519 key");
            Ok((EDDSA, x))
        }
        _ => bail!("unsupported key type (ES256 or EdDSA only)"),
    }
}

pub fn verify_sig(alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> Result<()> {
    match alg {
        ES256 => {
            use p256::ecdsa::signature::Verifier;
            let k =
                p256::ecdsa::VerifyingKey::from_sec1_bytes(pk).map_err(|_| anyhow!("bad key"))?;
            let s = p256::ecdsa::Signature::from_der(sig)
                .map_err(|_| anyhow!("bad signature encoding"))?;
            k.verify(msg, &s).map_err(|_| anyhow!("bad signature"))
        }
        EDDSA => {
            let k = ed25519_dalek::VerifyingKey::from_bytes(
                pk.try_into().map_err(|_| anyhow!("bad key"))?,
            )
            .map_err(|_| anyhow!("bad key"))?;
            let s =
                ed25519_dalek::Signature::from_slice(sig).map_err(|_| anyhow!("bad signature"))?;
            k.verify_strict(msg, &s)
                .map_err(|_| anyhow!("bad signature"))
        }
        _ => bail!("unsupported algorithm"),
    }
}

/// The parts of a registration, from an attestationObject (browser) or from a tool's output.
#[derive(Debug, Clone, Default)]
pub struct Attestation {
    pub fmt: String,
    pub auth_data: Vec<u8>,
    pub alg: Option<i64>,
    pub sig: Vec<u8>,
    pub x5c: Vec<Vec<u8>>,
}

pub fn parse_attestation_object(b: &[u8]) -> Result<Attestation> {
    let v: Value = ciborium::de::from_reader(b).map_err(|_| anyhow!("bad attestation object"))?;
    let m = v
        .as_map()
        .ok_or_else(|| anyhow!("bad attestation object"))?;
    let fmt = get_s(m, "fmt")
        .and_then(|v| v.as_text())
        .unwrap_or_default()
        .to_string();
    let auth_data = get_s(m, "authData")
        .and_then(|v| v.as_bytes())
        .cloned()
        .ok_or_else(|| anyhow!("authData missing"))?;
    let st = get_s(m, "attStmt")
        .and_then(|v| v.as_map())
        .cloned()
        .unwrap_or_default();
    Ok(Attestation {
        fmt,
        auth_data,
        alg: get_s(&st, "alg").and_then(int).map(|a| a as i64),
        sig: get_s(&st, "sig")
            .and_then(|v| v.as_bytes())
            .cloned()
            .unwrap_or_default(),
        x5c: get_s(&st, "x5c")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|c| c.as_bytes().cloned()).collect())
            .unwrap_or_default(),
    })
}

/// A verified new credential.
#[derive(Debug, Clone)]
pub struct NewCredential {
    pub id: Vec<u8>,
    pub alg: i64,
    pub public_key: Vec<u8>,
    pub aaguid: [u8; 16],
    pub sign_count: u32,
    /// none | self | x5c:<sha256 of the leaf certificate>
    pub attestation: String,
    pub flags: u8,
}

pub fn verify_registration(
    rp: &Rp,
    challenge: &[u8],
    client_data_json: &[u8],
    att: &Attestation,
) -> Result<NewCredential> {
    check_client_data(rp, client_data_json, "webauthn.create", challenge)?;
    let ad = parse_auth_data(&att.auth_data)?;
    ensure!(
        ad.rp_id_hash == sha(&[rp.id.as_bytes()]),
        "credential is for another RP id"
    );
    ensure!(ad.flags & FLAG_UP != 0, "user presence (touch) missing");
    ensure!(
        !rp.require_uv || ad.flags & FLAG_UV != 0,
        "user verification (PIN or biometric) required"
    );
    let key = ad
        .cose_key
        .as_ref()
        .ok_or_else(|| anyhow!("no credential in the authenticator data"))?;
    let (alg, pk) = cose_public_key(key)?;
    let signed = [
        att.auth_data.as_slice(),
        &client_data_hash(client_data_json),
    ]
    .concat();
    let attestation = match att.fmt.as_str() {
        "none" => "none".to_string(),
        "packed" if att.x5c.is_empty() => {
            ensure!(
                att.alg.is_none_or(|a| a == alg),
                "self attestation algorithm mismatch"
            );
            verify_sig(alg, &pk, &signed, &att.sig)
                .map_err(|_| anyhow!("attestation signature invalid"))?;
            "self".to_string()
        }
        "packed" => {
            let (_, cert) = x509_parser::parse_x509_certificate(&att.x5c[0])
                .map_err(|_| anyhow!("bad attestation certificate"))?;
            let spki = cert.public_key().subject_public_key.data.to_vec();
            ensure!(
                att.alg.is_none_or(|a| a == ES256),
                "attestation algorithm not supported"
            );
            verify_sig(ES256, &spki, &signed, &att.sig)
                .map_err(|_| anyhow!("attestation signature invalid"))?;
            format!("x5c:{}", hex::encode(&sha(&[&att.x5c[0]])[..8]))
        }
        f => bail!("attestation format {f:?} not supported (none or packed)"),
    };
    Ok(NewCredential {
        id: ad.credential_id,
        alg,
        public_key: pk,
        aaguid: ad.aaguid,
        sign_count: ad.sign_count,
        attestation,
        flags: ad.flags,
    })
}

/// A verified assertion: flags and the new sign count (checked against `stored_count`).
#[derive(Debug, Clone, Copy)]
pub struct Verified {
    pub flags: u8,
    pub sign_count: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn verify_assertion(
    rp: &Rp,
    alg: i64,
    public_key: &[u8],
    stored_count: u32,
    challenge: &[u8],
    client_data_json: &[u8],
    auth_data: &[u8],
    sig: &[u8],
) -> Result<Verified> {
    check_client_data(rp, client_data_json, "webauthn.get", challenge)?;
    let ad = parse_auth_data(auth_data)?;
    ensure!(
        ad.rp_id_hash == sha(&[rp.id.as_bytes()]),
        "assertion is for another RP id"
    );
    ensure!(ad.flags & FLAG_UP != 0, "user presence (touch) missing");
    ensure!(
        !rp.require_uv || ad.flags & FLAG_UV != 0,
        "user verification (PIN or biometric) required"
    );
    let signed = [auth_data, &client_data_hash(client_data_json)].concat();
    verify_sig(alg, public_key, &signed, sig)?;
    // Counters: if either side counts, the new value must grow (a cloned key shows up here).
    if (ad.sign_count != 0 || stored_count != 0) && ad.sign_count <= stored_count {
        bail!("sign count did not increase (cloned authenticator or replay)");
    }
    Ok(Verified {
        flags: ad.flags,
        sign_count: ad.sign_count,
    })
}

/// Authenticator data as an authenticator builds it (soft authenticators and tests).
pub fn build_auth_data(
    rp_id: &str,
    flags: u8,
    count: u32,
    cred: Option<(&[u8; 16], &[u8], &Value)>,
) -> Vec<u8> {
    let mut b = sha(&[rp_id.as_bytes()]).to_vec();
    b.push(flags | if cred.is_some() { FLAG_AT } else { 0 });
    b.extend(count.to_be_bytes());
    if let Some((aaguid, id, key)) = cred {
        b.extend(aaguid);
        b.extend((id.len() as u16).to_be_bytes());
        b.extend(id);
        ciborium::ser::into_writer(key, &mut b).expect("cbor");
    }
    b
}

/// COSE key for a P-256 point (SEC1 uncompressed).
pub fn cose_es256(sec1: &[u8]) -> Value {
    Value::Map(vec![
        (Value::Integer(1.into()), Value::Integer(2.into())),
        (Value::Integer(3.into()), Value::Integer((-7).into())),
        (Value::Integer((-1).into()), Value::Integer(1.into())),
        (
            Value::Integer((-2).into()),
            Value::Bytes(sec1[1..33].to_vec()),
        ),
        (
            Value::Integer((-3).into()),
            Value::Bytes(sec1[33..65].to_vec()),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{signature::Signer, Signature, SigningKey};

    fn rp() -> Rp {
        Rp {
            id: "admin.example.com".into(),
            origins: vec![CLI_ORIGIN.into()],
            require_uv: false,
        }
    }

    #[test]
    fn register_and_assert() {
        let k = SigningKey::from_slice(&[7u8; 32]).unwrap();
        let sec1 = k
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        let ch = [1u8; 32];
        let cd = client_data("webauthn.create", &ch, CLI_ORIGIN);
        let ad = build_auth_data(
            "admin.example.com",
            FLAG_UP,
            0,
            Some((&[9; 16], b"cred-1", &cose_es256(&sec1))),
        );
        let sig: Signature = k.sign(&[ad.as_slice(), &client_data_hash(&cd)].concat());
        let att = Attestation {
            fmt: "packed".into(),
            auth_data: ad.clone(),
            alg: Some(ES256),
            sig: sig.to_der().as_bytes().to_vec(),
            x5c: vec![],
        };
        let c = verify_registration(&rp(), &ch, &cd, &att).unwrap();
        assert_eq!(c.id, b"cred-1");
        assert_eq!(c.attestation, "self");
        // wrapped the way fido2-cred prints it
        let mut w = vec![];
        ciborium::ser::into_writer(&Value::Bytes(ad.clone()), &mut w).unwrap();
        assert_eq!(unwrap_cbor_bytes(&w), ad);
        assert_eq!(unwrap_cbor_bytes(&ad), ad);
        // wrong challenge, origin, RP
        assert!(verify_registration(&rp(), &[2u8; 32], &cd, &att).is_err());
        let bad = client_data("webauthn.create", &ch, "https://evil.example");
        assert!(verify_registration(&rp(), &ch, &bad, &att).is_err());
        let mut r2 = rp();
        r2.id = "other.example.com".into();
        assert!(verify_registration(&r2, &ch, &cd, &att).is_err());

        let sc = stepup_challenge("/itcmdr.admin.AdminApi/SetRole", b"body", "n1");
        let cd = client_data("webauthn.get", &sc, CLI_ORIGIN);
        let ad = build_auth_data("admin.example.com", FLAG_UP, 5, None);
        let sig: Signature = k.sign(&[ad.as_slice(), &client_data_hash(&cd)].concat());
        let der = sig.to_der().as_bytes().to_vec();
        let v = verify_assertion(&rp(), ES256, &c.public_key, 4, &sc, &cd, &ad, &der).unwrap();
        assert_eq!(v.sign_count, 5);
        // counter must grow; another request's challenge fails; no touch fails
        assert!(verify_assertion(&rp(), ES256, &c.public_key, 5, &sc, &cd, &ad, &der).is_err());
        let other = stepup_challenge("/itcmdr.admin.AdminApi/SetRole", b"other body", "n1");
        assert!(verify_assertion(&rp(), ES256, &c.public_key, 4, &other, &cd, &ad, &der).is_err());
        let ad0 = build_auth_data("admin.example.com", 0, 6, None);
        let sig0: Signature = k.sign(&[ad0.as_slice(), &client_data_hash(&cd)].concat());
        assert!(verify_assertion(
            &rp(),
            ES256,
            &c.public_key,
            4,
            &sc,
            &cd,
            &ad0,
            sig0.to_der().as_bytes()
        )
        .is_err());
        let mut uv = rp();
        uv.require_uv = true;
        assert!(verify_assertion(&uv, ES256, &c.public_key, 4, &sc, &cd, &ad, &der).is_err());
    }
}
