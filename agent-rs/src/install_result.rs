//! v1.2.1: the agent package's Windows installer result (installer/common-code.iss writes
//! `%ProgramData%\itcmdr\install-logs\last-agent.json`). A failed upgrade is queued as a Critical
//! alert `installer-agent` (step, error, log path) on the durable queue, a good one resolves it;
//! each result is reported once (renamed to `last-agent.reported.json`).
use std::path::{Path, PathBuf};

fn candidates(dir: &Path) -> Vec<PathBuf> {
    let mut v = vec![dir.join("install-logs")];
    if let Some(p) = dir.parent() {
        v.push(p.join("install-logs"));
    }
    v
}

/// Returns true when a result was found and queued.
pub fn report(dir: &Path) -> bool {
    for d in candidates(dir) {
        let p = d.join("last-agent.json");
        let Ok(b) = std::fs::read(&p) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&b)) else {
            continue;
        };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
        if ok {
            crate::config::alert(dir, "installer-agent", "informational", "", "", true);
        } else {
            let from = if s("from").is_empty() {
                "nothing".into()
            } else {
                s("from")
            };
            crate::config::alert(
                dir,
                "installer-agent",
                "critical",
                &format!("agent upgrade to {} failed at {}; still on {from}", s("to"), s("step")),
                &format!(
                    "{} (exit code {}). The services were started again on the old version. Log: {}",
                    s("error"),
                    s("exit"),
                    s("log")
                ),
                false,
            );
        }
        let _ = std::fs::rename(&p, d.join("last-agent.reported.json"));
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn failed_install_is_queued_once() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().join("agent");
        std::fs::create_dir_all(t.path().join("install-logs")).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            t.path().join("install-logs/last-agent.json"),
            r#"{"part":"agent","from":"1.2.0","to":"1.2.1","ok":false,"step":"file locks","error":"agent\\current\\itcmdr-agent.exe is locked","exit":"7","log":"C:\\ProgramData\\itcmdr\\install-logs\\agent-1.2.1-x.log","time":"t"}"#,
        )
        .unwrap();
        assert!(super::report(&dir));
        assert!(!super::report(&dir), "reported once");
        assert!(t
            .path()
            .join("install-logs/last-agent.reported.json")
            .exists());
        let q = crate::queue::ingest(&dir, 100).unwrap();
        let it = q
            .iter()
            .find(|i| i.key == "installer-agent")
            .expect("queued");
        assert_eq!(it.level, "critical");
        assert!(
            it.title.contains("failed at file locks; still on 1.2.0"),
            "{}",
            it.title
        );
        assert!(it.detail.contains("exit code 7") && it.detail.contains("agent-1.2.1-x.log"));
    }
}
