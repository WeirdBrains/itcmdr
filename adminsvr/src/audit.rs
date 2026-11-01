//! Append-only, hash-chained audit of every signed change (ADMINSVR.md decision 14).
//! hash = sha256(prev_hash \n seq \n ts \n actor \n actor_key \n action \n target \n payload_sha \n sig)
use crate::schema::audit_chain::dsl as ac;
use crate::store::{Db, Pool};
use anyhow::Result;
use diesel::prelude::*;

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

static APPEND: parking_lot::Mutex<()> = parking_lot::const_mutex(());

#[derive(Queryable)]
struct Row {
    seq: i64,
    ts_ms: i64,
    actor: String,
    actor_key: String,
    action: String,
    target: String,
    payload_sha256: String,
    request_sig: String,
    prev_hash: String,
    hash: String,
}

fn entry(r: Row) -> crate::pb::AuditEntry {
    crate::pb::AuditEntry {
        seq: r.seq,
        ts_ms: r.ts_ms,
        actor: r.actor,
        actor_key: r.actor_key,
        action: r.action,
        target: r.target,
        payload_sha256: r.payload_sha256,
        request_sig: r.request_sig,
        prev_hash: r.prev_hash,
        hash: r.hash,
    }
}

/// Append one entry in a transaction (serialized in-process and, on Postgres, with an advisory
/// lock). Returns the new seq.
pub fn append_with(c: &mut Db, e: &Entry) -> Result<i64> {
    let _g = APPEND.lock();
    c.transaction::<_, anyhow::Error, _>(|c| {
        if let Db::Pg(pg) = c {
            diesel::sql_query("SELECT pg_advisory_xact_lock(7319)").execute(pg)?;
        }
        let last: Option<(i64, String)> = ac::audit_chain
            .select((ac::seq, ac::hash))
            .order(ac::seq.desc())
            .first(c)
            .optional()?;
        let (seq, prev) = match last {
            Some((s, h)) => (s + 1, h),
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
        diesel::insert_into(ac::audit_chain)
            .values((
                ac::seq.eq(seq),
                ac::ts_ms.eq(ts),
                ac::actor.eq(&e.actor),
                ac::actor_key.eq(&e.actor_key),
                ac::action.eq(&e.action),
                ac::target.eq(&e.target),
                ac::payload_sha256.eq(&psha),
                ac::request_sig.eq(&e.request_sig),
                ac::prev_hash.eq(&prev),
                ac::hash.eq(&h),
            ))
            .execute(c)?;
        Ok(seq)
    })
}

pub async fn append(pool: &Pool, e: &Entry) -> Result<i64> {
    let e = e.clone();
    pool.run(move |c| append_with(c, &e)).await
}

pub async fn list(
    pool: &Pool,
    actor: Option<&str>,
    limit: i64,
) -> Result<Vec<crate::pb::AuditEntry>> {
    let actor = actor.map(str::to_string);
    pool.run(move |c| {
        let mut q = ac::audit_chain.into_boxed();
        if let Some(a) = actor {
            q = q.filter(ac::actor.eq(a));
        }
        let rows: Vec<Row> = q.order(ac::seq.desc()).limit(limit).load(c)?;
        Ok(rows.into_iter().map(entry).collect())
    })
    .await
}

/// Walk the chain: seqs contiguous from 1, each prev_hash = previous hash, each hash recomputed.
pub async fn verify(pool: &Pool) -> Result<crate::pb::AuditVerify> {
    let rows: Vec<Row> = pool
        .run(|c| Ok(ac::audit_chain.order(ac::seq.asc()).load(c)?))
        .await?;
    let n = rows.len() as i64;
    let mut prev = GENESIS.to_string();
    for (want, r) in (1i64..).zip(rows) {
        let e = entry(r);
        let bad = |p: &str| crate::pb::AuditVerify {
            ok: false,
            entries: n,
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
        entries: n,
        first_bad_seq: 0,
        problem: String::new(),
    })
}
