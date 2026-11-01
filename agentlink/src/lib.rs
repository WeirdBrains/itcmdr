//! The agent <-> server link (v0.17.0), shared by `itcmdr-server` and `itcmdr-agent`:
//! the generated `agent.proto`, enrollment tokens, the site CA and certificates (rcgen),
//! the pinned-TLS client (the server leaf certificate is pinned by its SHA-256, no public CA),
//! and the canonical bytes of the signed `Hello`.
pub mod pb {
    #![allow(clippy::all)]
    tonic::include_proto!("itcmdr.agent");
}
pub mod tls;
pub mod token;

pub use token::Token;

/// Default agent link port (the app/CLI port is 50051).
pub const DEFAULT_PORT: u16 = 50052;
/// Agent client certificates live this long; renewal comes with the stream (later point release).
pub const CERT_DAYS: i64 = 90;

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
    fn hello_bytes_bind_every_field() {
        let a = hello_bytes("a", "WS-01", 1, "n", "fp");
        assert_ne!(a, hello_bytes("a", "WS-02", 1, "n", "fp"));
        assert_ne!(a, hello_bytes("a", "WS-01", 1, "n", "fp2"));
        assert!(valid_machine("WS-01") && !valid_machine("a b") && !valid_machine(""));
    }
}
