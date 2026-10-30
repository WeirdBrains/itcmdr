//! Ingest of agent JSON Lines logs from the maintenanceRuns folder, including weekly,
//! 4-week block and yearly zips (zips nested inside zips are read recursively; file
//! names don't matter). Ingest is idempotent: runs are unique by run_id and checks by
//! (run_id, check_id), and files whose size and mtime haven't changed are skipped.
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::warn;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub run_id: String,
    pub computer: String,
    pub category: String,
    pub check_id: String,
    pub severity: String,
    pub status: String,
    pub attempted: Option<bool>,
    pub action: Option<String>,
    pub result: Option<String>,
    pub excepted: Option<bool>,
    pub exception_reason: Option<String>,
}

impl AgentRecord {
    pub fn attempted_repair(&self) -> bool {
        self.attempted.unwrap_or(false) || self.action.as_deref() == Some("repair")
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum LogLine {
    #[serde(rename_all = "camelCase")]
    RunStart {
        run_id: String,
        computer: String,
        role: Option<String>,
        report_only: Option<bool>,
        started: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Check {
        run_id: String,
        computer: String,
        category: String,
        check_id: String,
        severity: Option<String>,
        status: String,
        attempted_repair: Option<bool>,
        action: Option<String>,
        result: Option<String>,
        ts: Option<String>,
        excepted: Option<bool>,
        exception_reason: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    RunComplete {
        run_id: String,
        finished: Option<String>,
        exit_code: Option<i32>,
    },
    #[serde(other)]
    Unknown,
}

/// Parse an agent timestamp (PowerShell `ToString('o')`, RFC 3339). Naive timestamps
/// are taken as UTC.
pub fn parse_ts(s: Option<&str>) -> Option<i64> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|n| n.and_utc().timestamp_millis())
}

fn is_log_name(n: &str) -> bool {
    let n = n.to_ascii_lowercase();
    n.ends_with(".log") || n.ends_with(".jsonl")
}

fn read_zip_lines(bytes: Vec<u8>, out: &mut Vec<String>, depth: u32) {
    if depth > 4 {
        return;
    }
    let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        return;
    };
    for i in 0..z.len() {
        let Ok(mut f) = z.by_index(i) else { continue };
        let name = f.name().to_ascii_lowercase();
        let mut buf = Vec::new();
        if f.read_to_end(&mut buf).is_err() {
            continue;
        }
        if is_log_name(&name) {
            out.extend(String::from_utf8_lossy(&buf).lines().map(|s| s.to_string()));
        } else if name.ends_with(".zip") {
            read_zip_lines(buf, out, depth + 1);
        }
    }
}

/// Lines of one file (a log, or every log inside a possibly nested zip).
pub fn read_file_lines(p: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let name = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_log_name(&name) {
        if let Ok(b) = std::fs::read(p) {
            out.extend(String::from_utf8_lossy(&b).lines().map(|s| s.to_string()));
        }
    } else if name.ends_with(".zip") {
        if let Ok(b) = std::fs::read(p) {
            read_zip_lines(b, &mut out, 0);
        }
    }
    out
}

/// Log and zip files under `root`, skipping the on-demand request/response folders.
pub fn log_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy().to_ascii_lowercase();
            !(e.file_type().is_dir() && (n == "requests" || n == "responses"))
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            let n = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            is_log_name(&n) || n.ends_with(".zip")
        })
        .collect()
}

/// All raw lines under `root` (used by log search).
pub fn iter_log_lines(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut out = Vec::new();
    for p in log_files(root) {
        out.extend(read_file_lines(&p));
    }
    Ok(out)
}

enum Ev {
    Start {
        run_id: String,
        computer: String,
        role: Option<String>,
        report_only: bool,
    },
    Check(AgentRecord),
    Complete {
        run_id: String,
        exit_code: Option<i32>,
    },
}

fn file_stamp(p: &Path) -> Option<(i64, i64)> {
    let md = std::fs::metadata(p).ok()?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Some((md.len() as i64, mtime))
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IngestStats {
    pub files: usize,
    pub new_runs: usize,
    pub new_checks: usize,
}

/// Ingest new or changed files under `dir`. Safe to call repeatedly.
pub async fn ingest_all(pool: &crate::db::DbPool, dir: &Path) -> anyhow::Result<IngestStats> {
    let mut stats = IngestStats::default();
    let mut events: Vec<(i64, u8, Ev)> = Vec::new();
    let mut touched = Vec::new();
    for p in log_files(dir) {
        let Some((size, mtime)) = file_stamp(&p) else {
            continue;
        };
        let key = p.to_string_lossy().to_string();
        if crate::db::ingested_file(pool, &key).await? == Some((size, mtime)) {
            continue;
        }
        stats.files += 1;
        touched.push((key, size, mtime));
        let mut run_ts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for line in read_file_lines(&p) {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<LogLine>(&line) {
                Ok(LogLine::RunStart {
                    run_id,
                    computer,
                    role,
                    report_only,
                    started,
                }) => {
                    let ts = parse_ts(started.as_deref()).unwrap_or(mtime);
                    run_ts.insert(run_id.clone(), ts);
                    events.push((
                        ts,
                        0,
                        Ev::Start {
                            run_id,
                            computer,
                            role,
                            report_only: report_only.unwrap_or(false),
                        },
                    ));
                }
                Ok(LogLine::Check {
                    run_id,
                    computer,
                    category,
                    check_id,
                    severity,
                    status,
                    attempted_repair,
                    action,
                    result,
                    ts,
                    excepted,
                    exception_reason,
                }) => {
                    let t = parse_ts(ts.as_deref())
                        .or_else(|| run_ts.get(&run_id).copied())
                        .unwrap_or(mtime);
                    events.push((
                        t,
                        1,
                        Ev::Check(AgentRecord {
                            run_id,
                            computer,
                            category,
                            check_id,
                            severity: severity.unwrap_or_else(|| "info".into()),
                            status,
                            attempted: attempted_repair,
                            action,
                            result,
                            excepted,
                            exception_reason,
                        }),
                    ));
                }
                Ok(LogLine::RunComplete {
                    run_id,
                    finished,
                    exit_code,
                }) => {
                    let t = parse_ts(finished.as_deref()).unwrap_or(mtime);
                    events.push((t, 2, Ev::Complete { run_id, exit_code }));
                }
                Ok(LogLine::Unknown) => {}
                Err(e) => warn!(error=%e, file=%p.display(), "skipping unparsable log line"),
            }
        }
    }
    // Apply in time order so incident open/close transitions are correct across files.
    events.sort_by_key(|(t, k, _)| (*t, *k));
    for (ts, _, ev) in events {
        match ev {
            Ev::Start {
                run_id,
                computer,
                role,
                report_only,
            } => {
                if crate::db::record_run_start(
                    pool,
                    &run_id,
                    &computer,
                    role.as_deref(),
                    report_only,
                    ts,
                )
                .await?
                {
                    stats.new_runs += 1;
                }
            }
            Ev::Check(rec) => {
                if crate::db::record_check(pool, &rec, ts).await? {
                    stats.new_checks += 1;
                }
            }
            Ev::Complete { run_id, exit_code } => {
                crate::db::record_run_complete(pool, &run_id, ts, exit_code).await?
            }
        }
    }
    for (k, s, m) in touched {
        crate::db::mark_ingested(pool, &k, s, m).await?;
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_powershell_round_trip_timestamps() {
        let a = parse_ts(Some("2026-10-29T19:00:00.1234567-04:00")).unwrap();
        let b = parse_ts(Some("2026-10-29T23:00:00.1234567Z")).unwrap();
        assert_eq!(a, b);
        assert_eq!(parse_ts(Some("2026-10-29T23:00:00")), Some(b - 123));
        assert_eq!(parse_ts(Some("")), None);
        assert_eq!(parse_ts(None), None);
    }
}
