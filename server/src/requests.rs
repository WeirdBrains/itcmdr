//! On-demand request status: pollers write `<requestId>.<COMPUTER>.json` files with
//! `{"requestId","machine","status","ts"}` (status: running|done|failed) into the
//! responses folder next to the requests folder; the server folds them into the
//! request's status.
use crate::db::DbPool;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize)]
struct ResponseFile {
    machine: String,
    status: String,
}

/// Fold per-machine responses into (status, message).
/// `expected` empty means "all machines": done once anything answered and nothing runs.
pub fn aggregate(expected: &[String], responses: &[(String, String)]) -> (String, String) {
    if responses.is_empty() {
        return ("queued".into(), "queued".into());
    }
    let mut by_machine: BTreeMap<String, String> = BTreeMap::new();
    for (m, s) in responses {
        by_machine.insert(m.to_ascii_uppercase(), s.to_ascii_lowercase());
    }
    let missing: Vec<String> = expected
        .iter()
        .map(|m| m.to_ascii_uppercase())
        .filter(|m| !by_machine.contains_key(m))
        .collect();
    let mut msg = by_machine.iter().map(|(m, s)| format!("{m}={s}")).collect::<Vec<_>>().join(", ");
    if !missing.is_empty() {
        msg.push_str(&format!("; waiting: {}", missing.join(", ")));
    }
    let running = by_machine.values().any(|s| s == "running");
    let failed = by_machine.values().any(|s| s == "failed");
    let status = if running || !missing.is_empty() {
        "running"
    } else if failed {
        "failed"
    } else {
        "done"
    };
    (status.into(), msg)
}

fn read_responses(dir: &Path, request_id: &str) -> Vec<(String, String)> {
    let prefix = format!("{request_id}.");
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.filter_map(Result::ok)
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with(&prefix) && n.to_ascii_lowercase().ends_with(".json")
        })
        .filter_map(|e| std::fs::read(e.path()).ok())
        .filter_map(|b| {
            let s = String::from_utf8_lossy(&b);
            serde_json::from_str::<ResponseFile>(s.trim_start_matches('\u{feff}')).ok()
        })
        .map(|r| (r.machine, r.status))
        .collect()
}

/// Update every queued/running request from the response files. Returns how many changed.
pub async fn sync_requests(pool: &DbPool, responses_dir: &Path) -> anyhow::Result<usize> {
    let mut changed = 0;
    for (id, options) in crate::db::pending_requests(pool).await? {
        let expected: Vec<String> = serde_json::from_str::<serde_json::Value>(&options)
            .ok()
            .and_then(|v| v.get("machines").cloned())
            .and_then(|m| serde_json::from_value(m).ok())
            .unwrap_or_default();
        let (status, msg) = aggregate(&expected, &read_responses(responses_dir, &id));
        if crate::db::set_request_status(pool, &id, &status, &msg).await? {
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(m: &str, s: &str) -> (String, String) {
        (m.into(), s.into())
    }

    #[test]
    fn aggregates_statuses() {
        let exp = vec!["WS-01".to_string(), "WS-02".to_string()];
        assert_eq!(aggregate(&exp, &[]).0, "queued");
        assert_eq!(aggregate(&exp, &[r("WS-01", "done")]).0, "running");
        assert!(aggregate(&exp, &[r("WS-01", "done")]).1.contains("waiting: WS-02"));
        assert_eq!(aggregate(&exp, &[r("ws-01", "done"), r("WS-02", "running")]).0, "running");
        assert_eq!(aggregate(&exp, &[r("WS-01", "done"), r("WS-02", "done")]).0, "done");
        assert_eq!(aggregate(&exp, &[r("WS-01", "failed"), r("WS-02", "done")]).0, "failed");
        assert_eq!(aggregate(&[], &[r("WS-03", "done")]).0, "done");
    }
}
