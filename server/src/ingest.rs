use anyhow::Context;
use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecord {
    pub run_id: String,
    pub computer: String,
    pub category: String,
    pub check_id: String,
    pub severity: String,
    pub status: String,
    pub action: Option<String>,
}

pub async fn start_watcher(dir: PathBuf, mut on_line: impl FnMut(String) + Send + 'static) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<Event>(64);
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Ok(ev) = res {
                let _ = tx.blocking_send(ev);
            }
        },
        NotifyConfig::default(),
    )?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    info!(?dir, "watching log folder");
    tokio::spawn(async move {
        while let Some(_ev) = rx.recv().await {
            // In a later commit: read appended lines, process new records, and update DB.
        }
    });
    Ok(())
}

