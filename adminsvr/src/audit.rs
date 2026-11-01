//! Append-only, hash-chained audit of every signed change (ADMINSVR.md decision 14).
//! hash = sha256(prev_hash \n seq \n ts \n actor \n actor_key \n action \n target \n payload_sha \n sig)
use crate::store::Pool;
use anyhow::Result;
use sqlx::Row;

pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub actor: String,
    pub actor_key: String,
    pub action: String,
    pub target: String,
    pub payload: Vec<u8>,
    pub request_sig: String,
}

#[allow(clippy::too_many_arguments)]
pub fn link_hash(
    prev: &str,
    seq: i64,
    ts: i64,
    actor: &str,
    key: &str,
    action: &str,
    target: &str,
    payload_sha: &str,
    sig: &str,
) -> String {
    itcmdr_adminlink::sha256_hex(
        format!("{prev}\n{seq}\n{ts}\n{actor}\n{key}\n{action}\n{target}\n{payload_sha}\n{sig}")
            .as_bytes(),
    )
}

/// Append one entry (serialized with an advisory lock). Returns the new seq.
pub async fn append(pool: &Pool, e: &Entry) -> Result<i64> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(7319)")
        .execute(&mut *tx)
        .await?;
    let last = sqlx::query("SELECT seq, hash FROM audit_chain ORDER BY seq DESC LIMIT 1")
        .fetch_optional(&mut *tx)
        .await?;
    let (seq, prev) = match last {
        Some(r) => (r.get::<i64, _>("seq") + 1, r.get::<String, _>("hash")),
        None => (1, GENESIS.to_string()),
    };
    let ts = crate::now_ms();
    let psha = itcmdr_adminlink::sha256_hex(&e.payload);
    let h = link_hash(
        &prev,
        seq,
        ts,
        &e.actor,
        &e.actor_key,
        &e.action,
        &e.target,
        &psha,
        &e.request_sig,
    );
    sqlx::query("INSERT INTO audit_chain (seq, ts_ms, actor, actor_key, action, target, payload_sha256, request_sig, prev_hash, hash) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
        .bind(seq).bind(ts).bind(&e.actor).bind(&e.actor_key).bind(&e.action).bind(&e.target)
        .bind(&psha).bind(&e.request_sig).bind(&prev).bind(&h)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(seq)
}

pub fn row_entry(r: &sqlx::postgres::PgRow) -> crate::pb::AuditEntry {
    crate::pb::AuditEntry {
        seq: r.get("seq"),
        ts_ms: r.get("ts_ms"),
        actor: r.get("actor"),
        actor_key: r.get("actor_key"),
        action: r.get("action"),
        target: r.get("target"),
        payload_sha256: r.get("payload_sha256"),
        request_sig: r.get("request_sig"),
        prev_hash: r.get("prev_hash"),
        hash: r.get("hash"),
    }
}

pub async fn list(
    pool: &Pool,
    actor: Option<&str>,
    limit: i64,
) -> Result<Vec<crate::pb::AuditEntry>> {
    let rows = match actor {
        Some(a) => {
            sqlx::query("SELECT * FROM audit_chain WHERE actor = $1 ORDER BY seq DESC LIMIT $2")
                .bind(a)
                .bind(limit)
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query("SELECT * FROM audit_chain ORDER BY seq DESC LIMIT $1")
                .bind(limit)
                .fetch_all(pool)
                .await?
        }
    };
    Ok(rows.iter().map(row_entry).collect())
}

/// Walk the chain: seqs contiguous from 1, each prev_hash = previous hash, each hash recomputed.
pub async fn verify(pool: &Pool) -> Result<crate::pb::AuditVerify> {
    let rows = sqlx::query("SELECT * FROM audit_chain ORDER BY seq")
        .fetch_all(pool)
        .await?;
    let mut prev = GENESIS.to_string();
    for (want, r) in (1i64..).zip(rows.iter()) {
        let e = row_entry(r);
        let bad = |p: &str| crate::pb::AuditVerify {
            ok: false,
            entries: rows.len() as i64,
            first_bad_seq: e.seq,
            problem: p.into(),
        };
        if e.seq != want {
            return Ok(bad("gap or reordered sequence"));
        }
        if e.prev_hash != prev {
            return Ok(bad("prev_hash does not match the previous entry"));
        }
        let h = link_hash(
            &e.prev_hash,
            e.seq,
            e.ts_ms,
            &e.actor,
            &e.actor_key,
            &e.action,
            &e.target,
            &e.payload_sha256,
            &e.request_sig,
        );
        if h != e.hash {
            return Ok(bad("entry was changed (hash mismatch)"));
        }
        prev = e.hash;
    }
    Ok(crate::pb::AuditVerify {
        ok: true,
        entries: rows.len() as i64,
        first_bad_seq: 0,
        problem: String::new(),
    })
}
