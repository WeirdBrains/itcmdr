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
    /// Push notifications (v0.19.2); off unless configured
    pub push: PushConfig,
    /// v1.1.0 ACME (Let's Encrypt) certificate for the admin API; None = off (default)
    pub acme: Option<AcmeConfig>,
}

/// ACME challenge type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Challenge {
    /// On the admin API listener (the CA connects to port 443 of each domain)
    TlsAlpn01,
    /// On `http_addr` (the CA connects to port 80 of each domain)
    Http01,
}

impl std::str::FromStr for Challenge {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "tls-alpn-01" => Ok(Challenge::TlsAlpn01),
            "http-01" => Ok(Challenge::Http01),
            o => Err(format!(
                "unknown ACME challenge {o:?} (tls-alpn-01 | http-01)"
            )),
        }
    }
}

/// v1.1.0: everything is per install (flags / env / env file); nothing is built in.
#[derive(Debug, Clone)]
pub struct AcmeConfig {
    /// DNS names for the certificate (the first is the main one)
    pub domains: Vec<String>,
    /// Contact e-mail(s) for the CA account
    pub contact: Vec<String>,
    /// production | staging | a directory URL
    pub directory: String,
    /// PEM root for the directory's own HTTPS (test CAs such as Pebble); None = system/webpki roots
    pub directory_root: Option<PathBuf>,
    pub challenge: Challenge,
    /// HTTP-01 listener (default 0.0.0.0:80)
    pub http_addr: String,
    /// Account key and certificate (0700; default <data dir>/acme)
    pub cache_dir: PathBuf,
    /// Renew this many days before expiry (default 30)
    pub renew_days: i64,
    /// Wait after a failed attempt (default 3600)
    pub retry_secs: u64,
}

impl AcmeConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.domains.is_empty() {
            return Err("ACME needs at least one domain".into());
        }
        for d in &self.domains {
            let ok = !d.is_empty()
                && d.len() <= 253
                && d.parse::<std::net::IpAddr>().is_err()
                && d.split('.').count() >= 2
                && d.split('.').all(|l| {
                    !l.is_empty()
                        && l.len() <= 63
                        && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                        && !l.starts_with('-')
                        && !l.ends_with('-')
                });
            if !ok {
                return Err(format!("not a DNS name for ACME: {d:?}"));
            }
        }
        if !(1..=60).contains(&self.renew_days) {
            return Err("ACME renew days: 1..60".into());
        }
        Ok(())
    }
}

/// FCM (Android, via a Firebase service account) and APNs (iOS/macOS, token auth with a .p8 key).
/// Each provider is on when its key file is set. The URLs can point at a relay or a test stub.
#[derive(Debug, Clone, Default)]
pub struct PushConfig {
    /// Firebase service account JSON (project_id, client_email, private_key, token_uri)
    pub fcm_service_account: Option<PathBuf>,
    /// FCM API base (default https://fcm.googleapis.com)
    pub fcm_url: String,
    /// APNs auth key (.p8)
    pub apns_key: Option<PathBuf>,
    pub apns_key_id: String,
    pub apns_team_id: String,
    /// The app's bundle id
    pub apns_topic: String,
    /// APNs base (default https://api.push.apple.com; https://api.sandbox.push.apple.com for development builds)
    pub apns_url: String,
    /// Lowest severity that is pushed: critical | cautious | informational (default cautious)
    pub min_severity: String,
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
            push: PushConfig::default(),
            acme: None,
        }
    }
}
