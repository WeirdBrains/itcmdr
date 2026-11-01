//! Configuration: flags or environment (ITCMDR_ADMINSVR_*). Nothing site-specific is built in.
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// Database URL: postgres://… (Postgres) or sqlite://<path> (SQLite)
    pub db_url: String,
    /// Data dir: keys/root.key, tls/ (0700)
    pub data_dir: PathBuf,
    /// Admin API (CLI and app), host:port
    pub api_addr: String,
    /// SiteLink, host:port
    pub site_addr: String,
    /// Address put into enrollment codes (how clients reach the API), host:port
    pub public_api: String,
    /// Address put into site tokens (how sites reach the SiteLink), host:port
    pub public_site: String,
    /// Extra names for the TLS certificate
    pub tls_names: Vec<String>,
    pub heartbeat_secs: i64,
    pub site_missed: i64,
    /// Allowed request clock skew
    pub max_skew_secs: i64,
    /// WebAuthn RP id (v0.19.1); empty = the host of `public_api`
    pub rp_id: String,
    /// Extra WebAuthn origins (the web app's https://…); the CLI origin is always allowed
    pub web_origins: Vec<String>,
}

impl Config {
    /// The WebAuthn relying party (RP id and allowed origins); user verification comes from the policy.
    pub fn rp(&self, require_uv: bool) -> itcmdr_adminlink::webauthn::Rp {
        let id = if self.rp_id.is_empty() {
            let h = self
                .public_api
                .rsplit_once(':')
                .map(|(h, _)| h)
                .unwrap_or(&self.public_api);
            let h = h.trim_matches(|c| c == '[' || c == ']');
            if h.is_empty() {
                "itcmdr-adminsvr".to_string()
            } else {
                h.to_string()
            }
        } else {
            self.rp_id.clone()
        };
        let mut origins = vec![
            itcmdr_adminlink::webauthn::CLI_ORIGIN.to_string(),
            format!("https://{id}"),
        ];
        origins.extend(self.web_origins.iter().cloned());
        itcmdr_adminlink::webauthn::Rp {
            id,
            origins,
            require_uv,
        }
    }
}

impl Config {
    pub fn for_test(data_dir: PathBuf, db_url: &str, api_port: u16, site_port: u16) -> Self {
        Config {
            db_url: db_url.into(),
            data_dir,
            api_addr: format!("127.0.0.1:{api_port}"),
            site_addr: format!("127.0.0.1:{site_port}"),
            public_api: format!("127.0.0.1:{api_port}"),
            public_site: format!("127.0.0.1:{site_port}"),
            tls_names: vec![],
            heartbeat_secs: 60,
            site_missed: 5,
            max_skew_secs: 300,
            rp_id: String::new(),
            web_origins: vec![],
        }
    }
}
