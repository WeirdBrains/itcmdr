use anyhow::Context;
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

static DEFAULT_BIND: &str = "127.0.0.1:50051";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    /// Folder to watch for JSON Lines agent logs.
    pub logs_dir: PathBuf,
    /// SQLite database file path.
    pub database_path: PathBuf,
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
        Ok(cfg)
    }
}

