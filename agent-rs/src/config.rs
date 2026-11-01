//! v0.20.1: the agent's config document, sent by the server over the link (ConfigUpdate).
//!
//! It is stored in `<dir>/config.json` only when the server root key's endorsement checks, and
//! checked again every time it is loaded: a bad or tampered file is deleted, the agent stays
//! report-only, reports an empty config version (so the server resends it) and raises an alert.
//! In the `agent` cutover state the Rust agent runs plugins for real and keeps the PowerShell
//! agent's nightly task disabled (self-heal); on rollback it re-enables the task.

use anyhow::{anyhow, ensure, Context, Result};
use itcmdr_agentlink::pb;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE: &str = "config.json";
pub const TASK: &str = "itcmdr Agent Nightly";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stored {
    pub version: String,
    pub json: String,
    pub endorsement: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Doc {
    #[serde(default)]
    pub cutover: String,
    #[serde(default)]
    pub settings: serde_json::Map<String, serde_json::Value>,
}

static ALERTS: Mutex<Vec<pb::AgentAlert>> = Mutex::new(Vec::new());

/// Alerts waiting for the link (drained by the session).
pub fn take_alerts() -> Vec<pb::AgentAlert> {
    std::mem::take(&mut *ALERTS.lock().unwrap_or_else(|e| e.into_inner()))
}

pub fn alert(key: &str, level: &str, title: &str, detail: &str, resolve: bool) {
    ALERTS.lock().unwrap_or_else(|e| e.into_inner()).push(pb::AgentAlert {
        key: key.into(),
        level: level.into(),
        title: title.into(),
        detail: detail.into(),
        ts_ms: chrono::Utc::now().timestamp_millis(),
        resolve,
    });
}

fn path(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

fn root_key(dir: &Path) -> String {
    crate::state::load(dir)
        .ok()
        .flatten()
        .map(|s| s.root_public_key)
        .unwrap_or_default()
}

/// Check the endorsement and the document.
pub fn verify(root: &str, s: &Stored) -> Result<Doc> {
    ensure!(!root.is_empty(), "no server root key");
    ensure!(
        itcmdr_agentlink::config_version(&s.json) == s.version,
        "the config version does not match its contents"
    );
    let pk: [u8; 32] = hex::decode(root)?
        .try_into()
        .map_err(|_| anyhow!("bad root key"))?;
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk)?;
    let sig: [u8; 64] = hex::decode(&s.endorsement)
        .context("endorsement is not hex")?
        .try_into()
        .map_err(|_| anyhow!("endorsement has the wrong length"))?;
    vk.verify_strict(
        &itcmdr_agentlink::config_bytes(&s.version, &s.json),
        &ed25519_dalek::Signature::from_bytes(&sig),
    )
    .map_err(|_| anyhow!("the config is not endorsed by the server root key"))?;
    let d: Doc = serde_json::from_str(&s.json).context("config document")?;
    ensure!(
        matches!(d.cutover.as_str(), "task" | "schedule" | "agent"),
        "unknown cutover state {:?}",
        d.cutover
    );
    Ok(d)
}

/// Verify and store a ConfigUpdate from the server.
pub fn apply(dir: &Path, u: &pb::ConfigUpdate) -> Result<Doc> {
    let s = Stored {
        version: u.version.clone(),
        json: u.json.clone(),
        endorsement: u.endorsement.clone(),
    };
    let d = verify(&root_key(dir), &s)?;
    std::fs::create_dir_all(dir)?;
    let p = path(dir);
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&s)?)?;
    std::fs::rename(&tmp, &p)?;
    alert("config-rejected", "informational", "", "", true);
    Ok(d)
}

/// The stored config, verified. A bad file is deleted and alerted (the server resends it).
pub fn load(dir: &Path) -> Option<(Stored, Doc)> {
    let p = path(dir);
    let b = std::fs::read(&p).ok()?;
    let r = serde_json::from_slice::<Stored>(&b)
        .context("config.json")
        .and_then(|s| verify(&root_key(dir), &s).map(|d| (s, d)));
    match r {
        Ok(x) => Some(x),
        Err(e) => {
            let msg = format!("{e:#}");
            tracing::error!(error=%msg, "config.json rejected; running report-only until the server resends it");
            let _ = std::fs::remove_file(&p);
            alert(
                "config-rejected",
                "critical",
                "Agent config rejected",
                &format!("config.json was rejected ({msg}); the agent runs report-only until the server resends it"),
                false,
            );
            None
        }
    }
}

pub fn version(dir: &Path) -> String {
    load(dir).map(|(s, _)| s.version).unwrap_or_default()
}

/// Cutover state ("task" when there is no valid config).
pub fn cutover(dir: &Path) -> String {
    load(dir)
        .map(|(_, d)| d.cutover)
        .unwrap_or_else(|| "task".into())
}

/// Plugins run for real only in the `agent` state.
pub fn report_only(dir: &Path) -> bool {
    cutover(dir) != "agent"
}

/// Self-heal the PowerShell nightly task for `state` (Windows only). Returns what it did.
pub fn heal_task(dir: &Path, state: &str) -> Option<String> {
    let marker = dir.join("task-disabled-by-agent");
    let want_enabled = match state {
        "agent" => false,
        _ if marker.exists() => true, // rollback: re-enable what we disabled
        _ => return None,
    };
    let enabled = task_enabled()?;
    if enabled == want_enabled {
        if want_enabled {
            let _ = std::fs::remove_file(&marker);
        }
        return None;
    }
    if !set_task(want_enabled) {
        tracing::error!(task = TASK, enable = want_enabled, "could not change the nightly task");
        return None;
    }
    if want_enabled {
        let _ = std::fs::remove_file(&marker);
        Some("re-enabled the nightly task (rollback)".into())
    } else {
        let again = marker.exists();
        let _ = std::fs::write(&marker, b"1");
        if again {
            alert(
                "nightly-task-reenabled",
                "cautious",
                "PowerShell nightly task was re-enabled",
                "The agent owns the nightly run (cutover agent); it disabled the task again. Use itcmdr-cli cutover rollback to hand it back.",
                false,
            );
            Some("disabled the nightly task again (it was re-enabled)".into())
        } else {
            Some("disabled the nightly task (cutover agent)".into())
        }
    }
}

#[cfg(windows)]
fn task_enabled() -> Option<bool> {
    let o = std::process::Command::new("schtasks")
        .args(["/Query", "/TN", TASK, "/XML"])
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    let x = String::from_utf8_lossy(&o.stdout).to_string();
    let x = if x.contains("<Enabled>") {
        x
    } else {
        // schtasks /XML writes wide (little-endian) text
        let u: Vec<u16> = o
            .stdout
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u)
    };
    // The task-level <Settings><Enabled> (absent means enabled).
    let settings = x.split("<Settings>").nth(1).unwrap_or("");
    let settings = settings.split("</Settings>").next().unwrap_or("");
    Some(!settings.contains("<Enabled>false</Enabled>"))
}

#[cfg(windows)]
fn set_task(enable: bool) -> bool {
    std::process::Command::new("schtasks")
        .args([
            "/Change",
            "/TN",
            TASK,
            if enable { "/ENABLE" } else { "/DISABLE" },
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn task_enabled() -> Option<bool> {
    None
}

#[cfg(not(windows))]
fn set_task(_: bool) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn signed(k: &ed25519_dalek::SigningKey, json: &str) -> Stored {
        let version = itcmdr_agentlink::config_version(json);
        let sig = k.sign(&itcmdr_agentlink::config_bytes(&version, json));
        Stored {
            version,
            json: json.into(),
            endorsement: hex::encode(sig.to_bytes()),
        }
    }

    #[test]
    fn endorsed_config_verifies_and_forgeries_fail() {
        let k = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let root = hex::encode(k.verifying_key().to_bytes());
        let s = signed(&k, r#"{"v":1,"cutover":"agent","settings":{}}"#);
        assert_eq!(verify(&root, &s).unwrap().cutover, "agent");
        // tampered body
        let mut t = s.clone();
        t.json = t.json.replace("agent", "task\"");
        assert!(verify(&root, &t).is_err());
        // body + version recomputed, old signature
        let mut t2 = s.clone();
        t2.json = r#"{"v":1,"cutover":"task","settings":{}}"#.into();
        t2.version = itcmdr_agentlink::config_version(&t2.json);
        assert!(verify(&root, &t2).is_err());
        // signed by another key
        let other = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        assert!(verify(&root, &signed(&other, &s.json)).is_err());
        // unknown state
        assert!(verify(&root, &signed(&k, r#"{"cutover":"yolo"}"#)).is_err());
        assert!(verify("", &s).is_err());
    }
}
