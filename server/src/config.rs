use anyhow::Context;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

static DEFAULT_BIND: &str = "127.0.0.1:50051";
const DEFAULT_RETENTION_DAYS: i64 = 400;
const DEFAULT_INGEST_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    /// Folder holding JSON Lines agent logs (the share's maintenanceRuns folder).
    pub logs_dir: PathBuf,
    /// SQLite database file path.
    pub database_path: PathBuf,
    /// Optional folder for the on-demand request queue (defaults to logs_dir/requests).
    #[serde(default)]
    pub requests_dir: Option<PathBuf>,
    /// SHA-256 hex of the admin token. Required for mutating RPCs unless
    /// `allow_insecure_no_auth` is true.
    #[serde(default)]
    pub admin_token_hash: Option<String>,
    /// Explicit opt-out: allow mutating RPCs without any token (lab/demo only).
    #[serde(default)]
    pub allow_insecure_no_auth: bool,
    /// Retain DB data for this many days (default 400).
    #[serde(default = "default_retention_days")]
    pub retention_days: i64,
    /// Seconds between ingest/request-sync passes (default 30).
    #[serde(default = "default_ingest_interval")]
    pub ingest_interval_secs: u64,
    /// Where exception rules are exported for agents (defaults to logs_dir/exceptions.json).
    #[serde(default)]
    pub exceptions_export_path: Option<PathBuf>,
    /// Metrics windows (days) for trends. Defaults: 7,30,90,365
    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_windows")]
    pub trend_windows_days: Vec<i64>,
    /// Rollups supported: daily, weekly, block4 (4 ISO weeks), yearly (ISO year)
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
    vec!["daily".into(), "weekly".into(), "block4".into(), "yearly".into()]
}

fn default_retention_days() -> i64 {
    DEFAULT_RETENTION_DAYS
}

fn default_ingest_interval() -> u64 {
    DEFAULT_INGEST_INTERVAL_SECS
}

impl Default for ServerConfig {
    fn default() -> Self {
        let proj = project_dirs();
        let data_dir = proj.data_dir().to_path_buf();
        Self {
            bind_addr: DEFAULT_BIND.to_string(),
            logs_dir: data_dir.join("maintenanceRuns"),
            database_path: data_dir.join("itcmdr.sqlite"),
            requests_dir: None,
            admin_token_hash: None,
            allow_insecure_no_auth: false,
            retention_days: DEFAULT_RETENTION_DAYS,
            ingest_interval_secs: DEFAULT_INGEST_INTERVAL_SECS,
            exceptions_export_path: None,
            metrics: MetricsConfig::default(),
        }
    }
}

pub fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "itcmdr", "itcmdr").expect("unable to get project directories")
}

impl ServerConfig {
    /// Precedence:
    /// 1. ITCMDR_SERVER_CONFIG env var (path to JSON)
    /// 2. ./config/server.json
    /// 3. %ProgramData%\itcmdr\server.json (Windows service installs)
    /// 4. server.json next to the executable
    /// 5. built-in defaults
    pub fn load() -> anyhow::Result<Self> {
        if let Ok(p) = std::env::var("ITCMDR_SERVER_CONFIG") {
            return Self::from_path(p.into());
        }
        let mut candidates = vec![PathBuf::from("config").join("server.json")];
        if let Ok(pd) = std::env::var("ProgramData") {
            candidates.push(PathBuf::from(pd).join("itcmdr").join("server.json"));
        }
        if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
            candidates.push(dir.join("server.json"));
        }
        for c in candidates {
            if c.exists() {
                return Self::from_path(c);
            }
        }
        Ok(Self::default())
    }

    pub fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let s = fs::read_to_string(&path).with_context(|| format!("reading {:?}", path))?;
        let mut cfg: ServerConfig = serde_json::from_str(&s).with_context(|| format!("parsing {:?}", path))?;
        if cfg.bind_addr.trim().is_empty() {
            cfg.bind_addr = DEFAULT_BIND.to_string();
        }
        Ok(cfg)
    }

    pub fn requests_dir(&self) -> PathBuf {
        self.requests_dir.clone().unwrap_or_else(|| self.logs_dir.join("requests"))
    }

    /// Pollers write `<request_id>.<COMPUTER>.json` into the sibling `responses` folder.
    pub fn responses_dir(&self) -> PathBuf {
        let req = self.requests_dir();
        req.parent().map(|p| p.join("responses")).unwrap_or_else(|| self.logs_dir.join("responses"))
    }

    pub fn exceptions_export_path(&self) -> PathBuf {
        self.exceptions_export_path.clone().unwrap_or_else(|| self.logs_dir.join("exceptions.json"))
    }

    /// Token hash, trimmed and lowercased; None when unset/blank.
    pub fn token_hash(&self) -> Option<String> {
        self.admin_token_hash
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
    }
}
