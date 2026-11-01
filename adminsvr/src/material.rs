//! Key material in the data dir: the adminsvr root key (signs commands and user endorsements),
//! the CA that issues site client certificates, and the TLS server certificate.
use crate::config::Config;
use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use itcmdr_agentlink::tls::{self, Pem};
use std::path::Path;

#[derive(Clone)]
pub struct Material {
    pub root: SigningKey,
    pub ca: Pem,
    pub server: Pem,
    pub fingerprint: String,
}

impl Material {
    pub fn root_hex(&self) -> String {
        hex::encode(self.root.verifying_key().as_bytes())
    }
}

fn write_private(p: &Path, data: &str) -> Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(p, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Create anything missing, then load.
pub fn load(cfg: &Config) -> Result<Material> {
    let d = &cfg.data_dir;
    std::fs::create_dir_all(d).with_context(|| format!("data dir {}", d.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700));
    }
    let rk = d.join("keys/root.key");
    if !rk.exists() {
        write_private(&rk, &hex::encode(itcmdr_keys::random_seed().as_ref()))?;
    }
    let root = itcmdr_keys::key_from_seed(&hex::decode(std::fs::read_to_string(&rk)?.trim())?)?;
    let (cc, ck) = (d.join("tls/ca.pem"), d.join("tls/ca.key"));
    if !cc.exists() {
        let c = tls::new_ca("itcmdr adminsvr CA")?;
        write_private(&ck, &c.key)?;
        write_private(&cc, &c.cert)?;
    }
    let ca = Pem {
        cert: std::fs::read_to_string(&cc)?,
        key: std::fs::read_to_string(&ck)?,
    };
    let (sc, sk) = (d.join("tls/server.pem"), d.join("tls/server.key"));
    if !sc.exists() {
        let mut names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        for a in [&cfg.public_api, &cfg.public_site] {
            if let Some((h, _)) = a.rsplit_once(':') {
                names.push(h.trim_matches(|c| c == '[' || c == ']').to_string());
            }
        }
        names.extend(cfg.tls_names.iter().cloned());
        names.sort();
        names.dedup();
        let s = tls::issue_server(&ca, &names, 3650)?;
        write_private(&sk, &s.key)?;
        write_private(&sc, &s.cert)?;
    }
    let server = Pem {
        cert: std::fs::read_to_string(&sc)?,
        key: std::fs::read_to_string(&sk)?,
    };
    let fingerprint = tls::pem_fingerprint(&server.cert)?;
    Ok(Material {
        root,
        ca,
        server,
        fingerprint,
    })
}
