//! v1.2.1: results of the Windows installers on this machine. Every package writes
//! `<data dir>\install-logs\last-<part>.json` (and a step log next to it); see installer/common-code.iss
//! and docs/UPGRADES.md. The server reports the server package (the agent reports its own): a failed upgrade opens a
//! Critical alert (`installer-<part>`) with the step, the error and the log path; a later good install
//! resolves it. Each result is reported once (renamed to `last-<part>.reported.json`).
use crate::db::DbPool;
use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Shown by `setup check`. Only `server` is reported by the server; the agent reports its own
/// (agent-rs install_result.rs) so the result reaches the server from every PC.
pub const PARTS: [&str; 2] = ["server", "agent"];

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct InstallResult {
    pub part: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    pub ok: bool,
    #[serde(default)]
    pub step: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub exit: String,
    #[serde(default)]
    pub log: String,
    #[serde(default)]
    pub time: String,
}

pub fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join("install-logs")
}

fn parse(p: &Path) -> Option<InstallResult> {
    let b = std::fs::read(p).ok()?;
    // Inno writes ANSI; invalid bytes are replaced, the fields are ASCII anyway.
    serde_json::from_str(&String::from_utf8_lossy(&b)).ok()
}

/// Newest result per part (unreported first, else the reported one): for `setup check`.
pub fn latest(data_dir: &Path, part: &str) -> Option<InstallResult> {
    let d = dir(data_dir);
    parse(&d.join(format!("last-{part}.json")))
        .or_else(|| parse(&d.join(format!("last-{part}.reported.json"))))
}

pub fn title(r: &InstallResult) -> String {
    let from = if r.from.is_empty() {
        "nothing"
    } else {
        &r.from
    };
    format!(
        "{} upgrade to {} failed at {}; still on {from}",
        r.part, r.to, r.step
    )
}

/// Report new results as alerts. Returns how many were reported.
pub async fn report(pool: &DbPool, data_dir: &Path) -> Result<usize> {
    let d = dir(data_dir);
    let mut n = 0;
    for part in ["server"] {
        let p = d.join(format!("last-{part}.json"));
        let Some(r) = parse(&p) else { continue };
        let key = format!("installer-{part}");
        if r.ok {
            let _ = crate::alerts::resolve_key(pool, &key).await;
        } else {
            crate::alerts::raise(
                pool,
                &crate::alerts::Raise {
                    key,
                    level: "critical".into(),
                    source: "server".into(),
                    machine: String::new(),
                    title: title(&r),
                    detail: format!(
                        "{} (exit code {}). The services were started again on the old version. Log: {}",
                        r.error, r.exit, r.log
                    ),
                    run_id: None,
                },
            )
            .await?;
        }
        std::fs::rename(&p, d.join(format!("last-{part}.reported.json")))?;
        n += 1;
    }
    Ok(n)
}
