use anyhow::Context;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

static DEFAULT_BIND: &str = "127.0.0.1:50051";
const DEFAULT_RETENTION_DAYS: i64 = 400;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    /// Folder to watch for JSON Lines agent logs.
    pub logs_dir: PathBuf,
    /// SQLite database file path.
    pub database_path: PathBuf,
    /// Optional folder for request queue (defaults to logs_dir/requests)
    #[serde(default)]
    pub requests_dir: Option<PathBuf>,
    /// Admin token SHA-256 hex (optional). If empty, no auth required.
    #[serde(default)]
    pub admin_token_hash: Option<String>,
    /// Retain DB data for at least this many days (default 400)
    #[serde(default = "default_retention_days")]
    pub retention_days: i64,
    /// Metrics windows (days) for trends. Defaults: 7,30,90,365
    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_windows")]
    pub trend_windows_days: Vec<i64>,
    /// Rollups supported: daily, weekly, monthly, quarterly, yearly
    #[serde(default = "default_rollups")]
    pub rollups: Vec<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            trend_windows_days: default_windows(),
            rollups: default_rollups(),
        }
    }
}

fn default_windows() -> Vec<i64> {
    vec![7, 30, 90, 365]
}

fn default_rollups() -> Vec<String> {
    vec![
        "daily".into(),
        "weekly".into(),
        "monthly".into(),
        "quarterly".into(),
        "yearly".into(),
    ]
}

fn default_retention_days() -> i64 {
    DEFAULT_RETENTION_DAYS
}

impl Default for ServerConfig {
    fn default() -> Self {
        let proj = project_dirs();
        let data_dir = proj.data_dir().to_path_buf();
        let db = data_dir.join("itcmdr.sqlite");
        let logs = data_dir.join("maintenanceRuns");
        Self {
            bind_addr: DEFAULT_BIND.to_string(),
            logs_dir: logs,
            database_path: db,
            requests_dir: None,
            admin_token_hash: None,
            retention_days: DEFAULT_RETENTION_DAYS,
            metrics: MetricsConfig::default(),
        }
    }
}

pub fn project_dirs() -> ProjectDirs {
    // Org and app copied from product name; neutral and generic.
    ProjectDirs::from("com", "itcmdr", "itcmdr").expect("unable to get project directories")
}

impl ServerConfig {
    pub fn load() -> anyhow::Result<Self> {
        // Precedence:
        // 1. ITCMDR_SERVER_CONFIG env var (path to JSON)
        // 2. ./config/server.json
        // 3. platform data dir default
        if let Ok(p) = std::env::var("ITCMDR_SERVER_CONFIG") {
            return Self::from_path(p.into());
        }
        let repo_rel = PathBuf::from("config").join("server.json");
        if repo_rel.exists() {
            return Self::from_path(repo_rel);
        }
        Ok(Self::default())
    }

    fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let s = fs::read_to_string(&path).with_context(|| format!("reading {:?}", path))?;
        let mut cfg: ServerConfig =
            serde_json::from_str(&s).with_context(|| format!("parsing {:?}", path))?;
        if cfg.bind_addr.trim().is_empty() {
            cfg.bind_addr = DEFAULT_BIND.to_string();
        }
        if cfg.requests_dir.is_none() {
            cfg.requests_dir = Some(cfg.logs_dir.join("requests"));
        }
        Ok(cfg)
    }
}

