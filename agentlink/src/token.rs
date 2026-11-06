//! Enrollment tokens: `itcmdr-enroll:` + base64url(JSON). Only a hash of the secret is stored on
//! the server; the token carries the address and the pinned server certificate fingerprint.
use anyhow::{bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};

const PREFIX: &str = "itcmdr-enroll:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub v: u32,
    /// host:port of the agent link
    pub addr: String,
    /// SHA-256 hex of the server's TLS certificate (pinned)
    pub pin: String,
    /// one-time secret (hex)
    pub secret: String,
    /// machine the token is for ("" = any)
    pub machine: String,
    /// v0.17.1: hex public key of the server root key that certifies the site CA ("" = not checked)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub root: String,
}

impl Token {
    pub fn encode(&self) -> String {
        let j = serde_json::to_vec(self).expect("token json");
        format!(
            "{PREFIX}{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(j)
        )
    }
    pub fn decode(s: &str) -> Result<Token> {
        match Self::decode_one(s.trim()) {
            Ok(t) => Ok(t),
            Err(first) => {
                for line in s.lines() {
                    let line = line.trim().trim_matches('"').trim_matches('\'');
                    if line.is_empty() {
                        continue;
                    }
                    let candidate = line.find(PREFIX).map(|i| &line[i..]).unwrap_or(line);
                    if let Ok(t) = Self::decode_one(candidate) {
                        return Ok(t);
                    }
                }
                Err(first)
            }
        }
    }

    fn decode_one(s: &str) -> Result<Token> {
        let Some(b) = s.strip_prefix(PREFIX) else {
            bail!("not an itcmdr enrollment token (expected {PREFIX}...)")
        };
        let j = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(b)
            .context("enrollment token is not valid base64url")?;
        let t: Token = serde_json::from_slice(&j).context("enrollment token is not valid")?;
        if t.v != 1 {
            bail!("unsupported enrollment token version {}", t.v);
        }
        if t.pin.len() != 64 || hex::decode(&t.pin).is_err() {
            bail!("enrollment token has no valid server fingerprint");
        }
        if t.addr.is_empty() || t.secret.is_empty() {
            bail!("enrollment token is incomplete");
        }
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_rejects() {
        let t = Token {
            v: 1,
            addr: "SERVER01:50052".into(),
            pin: "ab".repeat(32),
            secret: "00ff".into(),
            machine: "WS-01".into(),
            root: "cd".repeat(32),
        };
        let enc = t.encode();
        assert_eq!(Token::decode(&enc).unwrap(), t);
        assert_eq!(Token::decode(&format!("  {enc} \n")).unwrap(), t);
        assert_eq!(
            Token::decode(&format!(
                "one-time token for WS-01 until later; agents dial SERVER01:50052\n{enc}\n"
            ))
            .unwrap(),
            t
        );
        assert!(Token::decode("junk").is_err());
        let bad = Token {
            pin: "zz".into(),
            ..t
        };
        assert!(Token::decode(&bad.encode()).is_err());
    }
}
