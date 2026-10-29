use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::info;
use walkdir::WalkDir;

#[allow(dead_code)]
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
    pub excepted: Option<bool>,
    pub exception_reason: Option<String>,
}

#[allow(dead_code)]
pub async fn start_watcher(dir: PathBuf, _on_line: impl FnMut(String) + Send + 'static) -> anyhow::Result<()> {
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

#[allow(dead_code)]
pub fn iter_log_lines(root: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let p = entry.into_path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
            match ext.as_str() {
                "log" | "jsonl" => {
                    if let Ok(s) = std::fs::read_to_string(&p) {
                        out.extend(s.lines().map(|s| s.to_string()));
                    }
                }
                "zip" => {
                    if let Ok(f) = std::fs::File::open(&p) {
                        if let Ok(mut z) = zip::ZipArchive::new(f) {
                            for i in 0..z.len() {
                                if let Ok(mut file) = z.by_index(i) {
                                    let name = file.name().to_string();
                                    let nlower = name.to_ascii_lowercase();
                                    if nlower.ends_with(".log") || nlower.ends_with(".jsonl") {
                                        let mut s = String::new();
                                        use std::io::Read;
                                        let _ = file.read_to_string(&mut s);
                                        out.extend(s.lines().map(|s| s.to_string()));
                                    } else if nlower.ends_with(".zip") {
                                        // Nested zip: extract to memory and recurse is expensive.
                                        // Keep it simple: write to a temp file and recurse.
                                        let mut buf = Vec::new();
                                        use std::io::Read;
                                        let _ = file.read_to_end(&mut buf);
                                        if let Ok(tmp) = tempfile::NamedTempFile::new() {
                                            let _ = std::fs::write(tmp.path(), &buf);
                                            if let Ok(f2) = std::fs::File::open(tmp.path()) {
                                                if let Ok(mut z2) = zip::ZipArchive::new(f2) {
                                                    for j in 0..z2.len() {
                                                        if let Ok(mut inner) = z2.by_index(j) {
                                                            let n = inner.name().to_ascii_lowercase();
                                                            if n.ends_with(".log") || n.ends_with(".jsonl") {
                                                                let mut s2 = String::new();
                                                                let _ = inner.read_to_string(&mut s2);
                                                                out.extend(s2.lines().map(|s| s.to_string()));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(out)
}

