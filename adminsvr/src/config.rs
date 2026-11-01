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
        }
    }
}
