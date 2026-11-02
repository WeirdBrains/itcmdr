//! The agent <-> server link (v0.17.0), shared by `itcmdr-server` and `itcmdr-agent`:
//! the generated `agent.proto`, enrollment tokens, the site CA and certificates (rcgen),
//! the pinned-TLS client (the server leaf certificate is pinned by its SHA-256, no public CA),
//! and the canonical bytes of the signed `Hello`.
pub mod pb {
    #![allow(clippy::all)]
    tonic::include_proto!("itcmdr.agent");
}
pub mod tls;
pub use rustls;
pub use tokio_rustls;
pub mod token;

pub use token::Token;

/// Default agent link port (the app/CLI port is 50051).
pub const DEFAULT_PORT: u16 = 50052;
/// Default agent client certificate lifetime (setting `agent.certLifetimeMins`).
pub const CERT_DAYS: i64 = 90;

/// Bytes the server root key signs to certify the site CA: SHA-256 of the CA certificate DER.
pub fn ca_cert_bytes(ca_der: &[u8]) -> Vec<u8> {
    let mut v = b"itcmdr-site-ca/v1\n".to_vec();
    v.extend_from_slice(sha256_hex(ca_der).as_bytes());
    v
}

/// Bytes the server root key signs to endorse a plugin package for agents (v0.18.0).
pub fn plugin_bytes(name: &str, version: &str, manifest_sha256: &str) -> Vec<u8> {
    format!("itcmdr-agent-plugin/v1\n{name}\n{version}\n{manifest_sha256}").into_bytes()
}

/// v0.21.0: bytes the server root key signs for an UpgradeOffer: it vouches that the package
/// (manifest hash) is an agent release signed by an owner key (fingerprint).
pub fn upgrade_bytes(version: &str, manifest_sha256: &str, signer: &str) -> Vec<u8> {
    format!("itcmdr-agent-upgrade/v1\n{version}\n{manifest_sha256}\n{signer}").into_bytes()
}

/// v0.21.0: compare dotted versions numerically (pre-release suffixes after '-' sort lower).
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn parts(v: &str) -> (Vec<u64>, bool) {
        let v = v.trim().trim_start_matches('v');
        let (core, pre) = match v.split_once('-') {
            Some((c, _)) => (c, true),
            None => (v, false),
        };
        (
            core.split('.').map(|p| p.parse().unwrap_or(0)).collect(),
            pre,
        )
    }
    let (mut x, xp) = parts(a);
    let (mut y, yp) = parts(b);
    let n = x.len().max(y.len());
    x.resize(n, 0);
    y.resize(n, 0);
    x.cmp(&y).then(yp.cmp(&xp))
}

/// v0.20.1: bytes the server root key signs for a ConfigUpdate (the agent checks them).
pub fn config_bytes(version: &str, json: &str) -> Vec<u8> {
    format!(
        "itcmdr-agent-config/v1\n{version}\n{}",
        sha256_hex(json.as_bytes())
    )
    .into_bytes()
}

/// v0.20.1: version of a config document (first 16 hex of its sha256).
pub fn config_version(json: &str) -> String {
    sha256_hex(json.as_bytes())[..16].to_string()
}

/// Renew at 2/3 of the lifetime.
pub fn renew_due(issued_ms: i64, expires_ms: i64, now_ms: i64) -> bool {
    expires_ms > issued_ms && now_ms >= issued_ms + (expires_ms - issued_ms) * 2 / 3
}

/// Bytes an agent signs for its `Hello` (Ed25519, key registered at enrollment).
/// `cert_fp` is the SHA-256 (hex) of the TLS client certificate on the same connection, so a
/// stolen Hello cannot be replayed over another agent's TLS session.
pub fn hello_bytes(
    agent_id: &str,
    machine: &str,
    ts_ms: i64,
    nonce: &str,
    cert_fp: &str,
) -> Vec<u8> {
    format!("itcmdr-agent-hello/v1\n{agent_id}\n{machine}\n{ts_ms}\n{nonce}\n{cert_fp}")
        .into_bytes()
}

/// SHA-256 hex of DER bytes (certificate fingerprints, token secrets).
pub fn sha256_hex(b: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(b))
}

/// Valid machine name for agents (same rule as the settings registry).
pub fn valid_machine(m: &str) -> bool {
    !m.is_empty()
        && m.len() <= 63
        && m.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_order() {
        use std::cmp::Ordering::*;
        assert_eq!(version_cmp("0.21.0", "0.20.1"), Greater);
        assert_eq!(version_cmp("0.21.0", "0.21"), Equal);
        assert_eq!(version_cmp("0.21.0-pre1", "0.21.0"), Less);
        assert_eq!(version_cmp("v0.9.9", "0.10.0"), Less);
    }

    #[test]
    fn hello_bytes_bind_every_field() {
        let a = hello_bytes("a", "WS-01", 1, "n", "fp");
        assert_ne!(a, hello_bytes("a", "WS-02", 1, "n", "fp"));
        assert_ne!(a, hello_bytes("a", "WS-01", 1, "n", "fp2"));
        assert!(!renew_due(0, 300, 199) && renew_due(0, 300, 200));
        assert!(valid_machine("WS-01") && !valid_machine("a b") && !valid_machine(""));
    }
}
