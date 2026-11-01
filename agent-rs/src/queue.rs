//! Offline alert queue (v0.17.1).
//!
//! `itcmdr-agent alert` (any process) writes a file into `spool/`. The service moves spool files
//! into `queue/` with a durable, increasing seq, sends them over the link and deletes each one
//! when the server acks that seq; the server ignores a seq it already has, so a reconnect never
//! duplicates an alert. The queue is capped (`agent.queueMaxMib`): the oldest informational
//! items go first, then the oldest of any level, and the drop itself is queued as an alert.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    #[serde(default)]
    pub seq: u64,
    pub key: String,
    pub level: String,
    pub title: String,
    #[serde(default)]
    pub detail: String,
    pub ts_ms: i64,
    #[serde(default)]
    pub resolve: bool,
    /// v0.18.0: a RunReport (prost bytes, base64) instead of an alert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
}

/// Queue a plugin RunReport for delivery (same durable path as alerts).
pub fn spool_report(dir: &Path, rep: &itcmdr_agentlink::pb::RunReport) -> Result<PathBuf> {
    use base64::Engine;
    use prost::Message;
    spool(
        dir,
        &Item {
            seq: 0,
            key: format!("run:{}", rep.plugin),
            level: "report".into(),
            title: rep.status.clone(),
            detail: String::new(),
            ts_ms: rep.finished_ms,
            resolve: false,
            report: Some(base64::engine::general_purpose::STANDARD.encode(rep.encode_to_vec())),
        },
    )
}

/// The agent message body for a queued item.
pub fn body(it: Item) -> Option<itcmdr_agentlink::pb::agent_message::Body> {
    use itcmdr_agentlink::pb;
    match it.report {
        Some(b) => {
            use base64::Engine;
            use prost::Message;
            let bytes = base64::engine::general_purpose::STANDARD.decode(b).ok()?;
            Some(pb::agent_message::Body::Run(
                pb::RunReport::decode(bytes.as_slice()).ok()?,
            ))
        }
        None => Some(pb::agent_message::Body::Alert(pb::AgentAlert {
            key: it.key,
            level: it.level,
            title: it.title,
            detail: it.detail,
            ts_ms: it.ts_ms,
            resolve: it.resolve,
        })),
    }
}

pub fn spool_dir(dir: &Path) -> PathBuf {
    dir.join("spool")
}
pub fn queue_dir(dir: &Path) -> PathBuf {
    dir.join("queue")
}

fn write_atomic(p: &Path, b: &[u8]) -> Result<()> {
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, b)?;
    std::fs::rename(&tmp, p)?;
    Ok(())
}

/// Drop an alert into the spool (what the `alert` subcommand does).
pub fn spool(dir: &Path, it: &Item) -> Result<PathBuf> {
    let d = spool_dir(dir);
    std::fs::create_dir_all(&d).with_context(|| format!("creating {}", d.display()))?;
    let p = d.join(format!(
        "{:016}-{}.json",
        it.ts_ms,
        hex::encode(rand::random::<[u8; 6]>())
    ));
    write_atomic(&p, &serde_json::to_vec(it)?)?;
    Ok(p)
}

fn next_seq(q: &Path) -> Result<u64> {
    let p = q.join("next_seq");
    let n = std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(1);
    // Never below the newest item on disk (a lost counter must not reuse seqs).
    let n = n.max(list(q).last().map(|i| i.seq + 1).unwrap_or(1));
    write_atomic(&p, (n + 1).to_string().as_bytes())?;
    Ok(n)
}

fn item_path(q: &Path, seq: u64) -> PathBuf {
    q.join(format!("{seq:020}.json"))
}

fn list(q: &Path) -> Vec<Item> {
    let mut v: Vec<Item> = std::fs::read_dir(q)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok())
        .collect();
    v.sort_by_key(|i| i.seq);
    v
}

fn push(q: &Path, mut it: Item) -> Result<u64> {
    it.seq = next_seq(q)?;
    write_atomic(&item_path(q, it.seq), &serde_json::to_vec(&it)?)?;
    Ok(it.seq)
}

/// Move spool files into the queue, then enforce the size cap. Returns the queue (oldest first).
pub fn ingest(dir: &Path, max_mib: u64) -> Result<Vec<Item>> {
    let q = queue_dir(dir);
    std::fs::create_dir_all(&q)?;
    let mut spooled: Vec<PathBuf> = std::fs::read_dir(spool_dir(dir))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    spooled.sort();
    for p in spooled {
        match std::fs::read(&p)
            .ok()
            .and_then(|b| serde_json::from_slice::<Item>(&b).ok())
        {
            Some(it) => {
                push(&q, it)?;
            }
            None => tracing::warn!(file=%p.display(), "unreadable spooled alert dropped"),
        }
        let _ = std::fs::remove_file(&p);
    }
    let dropped = cap(&q, max_mib.max(1) * 1024 * 1024);
    if dropped > 0 {
        tracing::warn!(dropped, "offline queue over its cap; oldest items dropped");
        push(
            &q,
            Item {
                seq: 0,
                key: "queue-dropped".into(),
                level: "cautious".into(),
                title: format!("Agent offline queue full: {dropped} queued alert(s) dropped"),
                detail: format!("The queue is capped at {max_mib} MiB (agent.queueMaxMib)."),
                ts_ms: chrono::Utc::now().timestamp_millis(),
                resolve: false,
                report: None,
            },
        )?;
    }
    Ok(list(&q))
}

fn size_of(q: &Path, it: &Item) -> u64 {
    std::fs::metadata(item_path(q, it.seq))
        .map(|m| m.len())
        .unwrap_or(0)
}

/// Drop items until the queue fits `limit` bytes: informational first, then oldest.
fn cap(q: &Path, limit: u64) -> usize {
    let items = list(q);
    let mut total: u64 = items.iter().map(|i| size_of(q, i)).sum();
    let mut order: Vec<&Item> = items
        .iter()
        .filter(|i| i.level == "informational")
        .collect();
    order.extend(items.iter().filter(|i| i.level != "informational"));
    let mut n = 0;
    for it in order {
        if total <= limit {
            break;
        }
        total = total.saturating_sub(size_of(q, it));
        let _ = std::fs::remove_file(item_path(q, it.seq));
        n += 1;
    }
    n
}

/// The server acked this seq: the item is delivered.
pub fn ack(dir: &Path, seq: u64) {
    let _ = std::fs::remove_file(item_path(&queue_dir(dir), seq));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn it(level: &str, detail: usize) -> Item {
        Item {
            seq: 0,
            key: "k".into(),
            level: level.into(),
            title: "t".into(),
            detail: "x".repeat(detail),
            ts_ms: 1,
            resolve: false,
            report: None,
        }
    }
    #[test]
    fn spool_ingest_ack_and_cap() {
        let d = std::env::temp_dir().join(format!("itq-{}", rand::random::<u32>()));
        spool(&d, &it("critical", 10)).unwrap();
        spool(&d, &it("informational", 10)).unwrap();
        let q = ingest(&d, 256).unwrap();
        assert_eq!(q.len(), 2);
        assert!(q[0].seq < q[1].seq);
        ack(&d, q[0].seq);
        let q2 = ingest(&d, 256).unwrap();
        assert_eq!(q2.len(), 1);
        // seqs keep increasing after acks
        spool(&d, &it("critical", 10)).unwrap();
        let q3 = ingest(&d, 256).unwrap();
        assert!(q3.last().unwrap().seq > q[1].seq);
        // cap: 1 MiB, two big items; the informational one goes first
        spool(&d, &it("informational", 700_000)).unwrap();
        spool(&d, &it("critical", 700_000)).unwrap();
        let q4 = ingest(&d, 1).unwrap();
        assert!(q4.iter().all(|i| i.level != "informational"));
        assert!(q4.iter().any(|i| i.key == "queue-dropped"));
        assert!(q4.iter().any(|i| i.detail.len() == 700_000));
        let _ = std::fs::remove_dir_all(&d);
    }
}
