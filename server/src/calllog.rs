//! Call log (v0.15.0, docs/design/SERVER-AGENT.md section 7): one `call_log` row for **every** RPC,
//! written by a tower layer around the gRPC service (inside gRPC-Web, so app calls are plain gRPC
//! here). A row has the time, service and method, kind (unary/server_stream/...), peer address,
//! principal (the verified key, the token actor, or anonymous), gRPC status, duration, request and
//! response bytes (framing included) and the error message (cut to 512 bytes). Bodies, words, keys
//! and package bytes are never logged.
//!
//! Rows go through a bounded queue to one writer task, so a slow disk never slows a call (a full
//! queue drops rows and counts them). Each call runs inside a task-local [`CallCtx`]: auth sets the
//! principal there, and audit rows store the call id, so an audited call links to its call row.
//! Retention: `calllog.retention_days` (30), pruned by the daily jobs pass after the finished days
//! are exported as itcmdr.log/v1 `event: "call"` lines to `<data>/server-logs/calls/`.
use crate::db::DbPool;
use anyhow::Result;
use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};
use std::time::Instant;
use tonic::body::BoxBody;

pub const MAX_ERROR: usize = 512;
const QUEUE: usize = 10_000;

tokio::task_local! {
    static CALL: Arc<CallCtx>;
}

/// Per-call context, available to handlers through the task-local.
#[derive(Debug, Default)]
pub struct CallCtx {
    pub id: String,
    principal: Mutex<Option<String>>,
}

/// Who made the current call (set by auth). No-op outside a logged call.
pub fn set_principal(p: impl Into<String>) {
    let p = p.into();
    let _ = CALL.try_with(|c| *c.principal.lock() = Some(p));
}

/// Id of the current call's `call_log` row, if this runs inside a logged call.
pub fn current_id() -> Option<String> {
    CALL.try_with(|c| c.id.clone()).ok()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CallRecord {
    pub id: String,
    pub ts_ms: i64,
    pub service: String,
    pub method: String,
    pub kind: String,
    pub peer: String,
    pub principal: String,
    /// gRPC status code (0 = OK).
    pub status: i32,
    pub duration_ms: i64,
    pub req_bytes: i64,
    pub resp_bytes: i64,
    pub error: String,
}

/// Method kinds from `proto/itcmdr.proto` (`rpc X(stream A) returns (stream B)`).
fn kinds() -> &'static HashMap<String, &'static str> {
    static K: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    K.get_or_init(|| {
        let mut m = HashMap::new();
        for (pkg, text) in [
            ("itcmdr.Itcmdr", include_str!("../../proto/itcmdr.proto")),
            (
                "itcmdr.plugin.PluginService",
                include_str!("../../proto/plugin.proto"),
            ),
        ] {
            for l in text.lines() {
                let l = l.trim();
                let Some(rest) = l.strip_prefix("rpc ") else {
                    continue;
                };
                let Some((name, sig)) = rest.split_once('(') else {
                    continue;
                };
                let (req, resp) = sig.split_once("returns").unwrap_or((sig, ""));
                let kind = match (req.contains("stream "), resp.contains("stream ")) {
                    (false, false) => "unary",
                    (false, true) => "server_stream",
                    (true, false) => "client_stream",
                    (true, true) => "bidi",
                };
                m.insert(format!("/{pkg}/{}", name.trim()), kind);
            }
        }
        m
    })
}

pub fn kind_of(path: &str) -> &'static str {
    kinds().get(path).copied().unwrap_or("unary")
}

/// The writer side: a queue into the database.
#[derive(Clone)]
pub struct CallLog {
    tx: tokio::sync::mpsc::Sender<CallRecord>,
    dropped: Arc<AtomicU64>,
}

impl CallLog {
    /// Start the writer task. Rows are inserted in batches.
    pub fn start(pool: DbPool) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<CallRecord>(QUEUE);
        let dropped = Arc::new(AtomicU64::new(0));
        let d2 = dropped.clone();
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(64);
            while rx.recv_many(&mut batch, 64).await > 0 {
                if let Err(e) = insert(&pool, &batch).await {
                    tracing::warn!(error=%e, rows = batch.len(), "call log write failed");
                }
                batch.clear();
                let n = d2.swap(0, Ordering::Relaxed);
                if n > 0 {
                    tracing::warn!(dropped = n, "call log queue was full: rows dropped");
                }
            }
        });
        Self { tx, dropped }
    }

    pub fn record(&self, r: CallRecord) {
        if self.tx.try_send(r).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Wrap a gRPC service (pass the result to `tonic_web::enable` / `add_service`).
    pub fn wrap<S>(&self, inner: S) -> CallLogService<S> {
        CallLogService {
            inner,
            log: self.clone(),
        }
    }
}

async fn insert(pool: &DbPool, rows: &[CallRecord]) -> Result<()> {
    let mut tx = pool.begin().await?;
    for r in rows {
        sqlx::query(
            "INSERT OR IGNORE INTO call_log(id, ts_ms, service, method, kind, peer, principal, status, duration_ms, req_bytes, resp_bytes, error) \
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&r.id)
        .bind(r.ts_ms)
        .bind(&r.service)
        .bind(&r.method)
        .bind(&r.kind)
        .bind(&r.peer)
        .bind(&r.principal)
        .bind(r.status)
        .bind(r.duration_ms)
        .bind(r.req_bytes)
        .bind(r.resp_bytes)
        .bind(&r.error)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Clone)]
pub struct CallLogService<S> {
    inner: S,
    log: CallLog,
}

impl<S: tonic::server::NamedService> tonic::server::NamedService for CallLogService<S> {
    const NAME: &'static str = S::NAME;
}

/// Everything needed to finish the row once the response body ends (or is dropped).
struct Pending {
    log: CallLog,
    rec: CallRecord,
    started: Instant,
    ctx: Arc<CallCtx>,
    req_bytes: Arc<AtomicU64>,
    claimed_key: Option<String>,
    done: bool,
}

impl Pending {
    fn finish(&mut self, status: Option<i32>, message: Option<String>, resp_bytes: u64) {
        if self.done {
            return;
        }
        self.done = true;
        let mut r = self.rec.clone();
        r.duration_ms = self.started.elapsed().as_millis() as i64;
        r.req_bytes = self.req_bytes.load(Ordering::Relaxed) as i64;
        r.resp_bytes = resp_bytes as i64;
        // Body ended without a grpc-status: the client went away (CANCELLED).
        r.status = status.unwrap_or(1);
        r.error = itcmdr_common::truncate(&message.unwrap_or_default(), MAX_ERROR);
        r.principal = self
            .ctx
            .principal
            .lock()
            .clone()
            .or_else(|| {
                self.claimed_key
                    .as_ref()
                    .map(|k| format!("unverified key {k}"))
            })
            .unwrap_or_else(|| "anonymous".into());
        self.log.record(r);
    }
}

fn grpc_status(h: &http::HeaderMap) -> Option<(i32, String)> {
    let code = h.get("grpc-status")?.to_str().ok()?.parse().ok()?;
    let msg = h
        .get("grpc-message")
        .and_then(|v| v.to_str().ok())
        .map(percent_decode)
        .unwrap_or_default();
    Some((code, msg))
}

/// grpc-message is percent-encoded.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Request body that counts bytes.
struct CountReq {
    inner: BoxBody,
    n: Arc<AtomicU64>,
}

impl Body for CountReq {
    type Data = Bytes;
    type Error = tonic::Status;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, tonic::Status>>> {
        let r = Pin::new(&mut self.inner).poll_frame(cx);
        if let Poll::Ready(Some(Ok(f))) = &r {
            if let Some(d) = f.data_ref() {
                self.n.fetch_add(d.len() as u64, Ordering::Relaxed);
            }
        }
        r
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

/// Response body that counts bytes, reads the trailers' grpc-status and finishes the row.
struct CountResp {
    inner: BoxBody,
    n: u64,
    pending: Pending,
}

impl Body for CountResp {
    type Data = Bytes;
    type Error = tonic::Status;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, tonic::Status>>> {
        let r = Pin::new(&mut self.inner).poll_frame(cx);
        match &r {
            Poll::Ready(Some(Ok(f))) => {
                if let Some(d) = f.data_ref() {
                    self.n += d.len() as u64;
                } else if let Some(t) = f.trailers_ref() {
                    let (code, msg) = grpc_status(t).unwrap_or((0, String::new()));
                    let n = self.n;
                    self.pending.finish(Some(code), Some(msg), n);
                }
            }
            Poll::Ready(Some(Err(s))) => {
                let (n, code, msg) = (self.n, s.code() as i32, s.message().to_string());
                self.pending.finish(Some(code), Some(msg), n);
            }
            Poll::Ready(None) => {
                let n = self.n;
                self.pending.finish(None, None, n);
            }
            Poll::Pending => {}
        }
        r
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl Drop for CountResp {
    fn drop(&mut self) {
        let n = self.n;
        self.pending
            .finish(None, Some("response dropped before it ended".into()), n);
    }
}

impl<S> tonic::codegen::Service<http::Request<BoxBody>> for CallLogService<S>
where
    S: tonic::codegen::Service<http::Request<BoxBody>, Response = http::Response<BoxBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = http::Response<BoxBody>;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Self::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<BoxBody>) -> Self::Future {
        // The service that was polled ready handles this call (tower's clone-and-swap pattern).
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let log = self.log.clone();
        let started = Instant::now();
        let path = req.uri().path().to_string();
        let (service, method) = path
            .trim_start_matches('/')
            .split_once('/')
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .unwrap_or_else(|| (String::new(), path.clone()));
        let peer = req
            .extensions()
            .get::<tonic::transport::server::TcpConnectInfo>()
            .and_then(|i| i.remote_addr())
            .map(|a| a.to_string())
            .unwrap_or_default();
        let claimed_key = req
            .headers()
            .get("x-itcmdr-key")
            .and_then(|v| v.to_str().ok())
            .map(|k| k.chars().take(16).collect::<String>());
        let ctx = Arc::new(CallCtx {
            id: uuid::Uuid::new_v4().to_string(),
            principal: Mutex::new(None),
        });
        let req_bytes = Arc::new(AtomicU64::new(0));
        let (parts, body) = req.into_parts();
        let req = http::Request::from_parts(
            parts,
            tonic::body::boxed(CountReq {
                inner: body,
                n: req_bytes.clone(),
            }),
        );
        let rec = CallRecord {
            id: ctx.id.clone(),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            kind: kind_of(&path).into(),
            service,
            method,
            peer,
            principal: String::new(),
            status: 0,
            duration_ms: 0,
            req_bytes: 0,
            resp_bytes: 0,
            error: String::new(),
        };
        Box::pin(async move {
            let resp = CALL.scope(ctx.clone(), inner.call(req)).await?;
            let mut pending = Pending {
                log,
                rec,
                started,
                ctx,
                req_bytes,
                claimed_key,
                done: false,
            };
            let (parts, body) = resp.into_parts();
            // Trailers-only responses (most errors) carry the status in the headers.
            if let Some((code, msg)) = grpc_status(&parts.headers) {
                pending.finish(Some(code), Some(msg), 0);
            }
            let body = tonic::body::boxed(CountResp {
                inner: body,
                n: 0,
                pending,
            });
            Ok(http::Response::from_parts(parts, body))
        })
    }
}

// ---- Queries, retention and export ----

#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Substring of the method name.
    pub method: Option<String>,
    /// Substring of the principal.
    pub principal: Option<String>,
    /// Exact gRPC status code.
    pub status: Option<i32>,
    /// Only errors (status != 0).
    pub errors_only: bool,
    pub since_ms: Option<i64>,
    pub limit: i64,
}

type Row = (
    String,
    i64,
    String,
    String,
    String,
    String,
    String,
    i32,
    i64,
    i64,
    i64,
    String,
);

fn from_row(r: Row) -> CallRecord {
    CallRecord {
        id: r.0,
        ts_ms: r.1,
        service: r.2,
        method: r.3,
        kind: r.4,
        peer: r.5,
        principal: r.6,
        status: r.7,
        duration_ms: r.8,
        req_bytes: r.9,
        resp_bytes: r.10,
        error: r.11,
    }
}

const COLS: &str = "id, ts_ms, service, method, kind, COALESCE(peer,''), COALESCE(principal,''), status, duration_ms, req_bytes, resp_bytes, COALESCE(error,'')";

/// Newest first.
pub async fn list(pool: &DbPool, f: &Filter) -> Result<Vec<CallRecord>> {
    let like = |s: &Option<String>| s.as_ref().map(|x| format!("%{x}%"));
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM call_log WHERE (?1 IS NULL OR method LIKE ?1) AND (?2 IS NULL OR principal LIKE ?2) \
         AND (?3 IS NULL OR status = ?3) AND (?4 = 0 OR status != 0) AND (?5 IS NULL OR ts_ms >= ?5) \
         ORDER BY ts_ms DESC LIMIT ?6"
    ))
    .bind(like(&f.method))
    .bind(like(&f.principal))
    .bind(f.status)
    .bind(f.errors_only as i32)
    .bind(f.since_ms)
    .bind(if f.limit <= 0 { 200 } else { f.limit.min(5000) })
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(from_row).collect())
}

pub async fn get(pool: &DbPool, id: &str) -> Result<Option<CallRecord>> {
    let r: Option<Row> = sqlx::query_as(&format!("SELECT {COLS} FROM call_log WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(r.map(from_row))
}

/// gRPC code name for display.
pub fn code_name(c: i32) -> &'static str {
    [
        "OK",
        "CANCELLED",
        "UNKNOWN",
        "INVALID_ARGUMENT",
        "DEADLINE_EXCEEDED",
        "NOT_FOUND",
        "ALREADY_EXISTS",
        "PERMISSION_DENIED",
        "RESOURCE_EXHAUSTED",
        "FAILED_PRECONDITION",
        "ABORTED",
        "OUT_OF_RANGE",
        "UNIMPLEMENTED",
        "INTERNAL",
        "UNAVAILABLE",
        "DATA_LOSS",
        "UNAUTHENTICATED",
    ]
    .get(c as usize)
    .copied()
    .unwrap_or("UNKNOWN")
}

/// The itcmdr.log/v1 line for one call (`event: "call"`).
pub fn log_line(r: &CallRecord, machine: &str) -> serde_json::Value {
    let ts = chrono::DateTime::from_timestamp_millis(r.ts_ms)
        .unwrap_or_default()
        .with_timezone(&chrono::Local)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
    let level = match r.status {
        0 => "info",
        1 | 3 | 5 | 6 | 7 | 9 | 16 => "warn",
        _ => "error",
    };
    serde_json::json!({
        "schema": itcmdr_plugin::log::SCHEMA,
        "ts": ts,
        "level": level,
        "plugin": "itcmdr-server",
        "plugin_version": env!("CARGO_PKG_VERSION"),
        "run_id": r.id,
        "machine": machine,
        "stage": "call",
        "event": "call",
        "message": format!("{} {}", r.method, code_name(r.status)),
        "duration_ms": r.duration_ms,
        "service": r.service,
        "method": r.method,
        "kind": r.kind,
        "peer": r.peer,
        "principal": r.principal,
        "grpc_status": r.status,
        "req_bytes": r.req_bytes,
        "resp_bytes": r.resp_bytes,
        "error": r.error,
    })
}

pub fn export_dir(layout: &crate::storage::Layout) -> PathBuf {
    layout.root.join("server-logs").join("calls")
}

/// Export every finished local day after the last exported one (at most `max_days` back) to
/// `<dir>/calls-<yyyy-mm-dd>.jsonl`. Returns the days written. Idempotent: a day's file is
/// rewritten whole, and the last exported day is kept in `server_meta`.
pub async fn export_finished_days(
    pool: &DbPool,
    dir: &Path,
    machine: &str,
    max_days: i64,
) -> Result<Vec<String>> {
    let today = chrono::Local::now().date_naive();
    let last: Option<String> =
        sqlx::query_scalar("SELECT value FROM server_meta WHERE key = 'calllog_exported_day'")
            .fetch_optional(pool)
            .await?;
    let oldest: Option<i64> = sqlx::query_scalar("SELECT MIN(ts_ms) FROM call_log")
        .fetch_one(pool)
        .await?;
    let Some(oldest) = oldest else {
        return Ok(vec![]);
    };
    let first_with_rows = chrono::DateTime::from_timestamp_millis(oldest)
        .unwrap_or_default()
        .with_timezone(&chrono::Local)
        .date_naive();
    let mut day = match last.and_then(|s| s.parse::<chrono::NaiveDate>().ok()) {
        Some(d) => d.succ_opt().unwrap_or(d),
        None => first_with_rows,
    }
    .max(today - chrono::Duration::days(max_days.max(1)))
    .max(first_with_rows);
    let mut out = vec![];
    while day < today {
        let start = local_midnight_ms(day);
        let end = local_midnight_ms(day.succ_opt().unwrap_or(day));
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM call_log WHERE ts_ms >= ? AND ts_ms < ? ORDER BY ts_ms"
        ))
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await?;
        if !rows.is_empty() {
            std::fs::create_dir_all(dir)?;
            let mut text = String::new();
            for r in rows.into_iter().map(from_row) {
                text.push_str(&log_line(&r, machine).to_string());
                text.push('\n');
            }
            std::fs::write(dir.join(format!("calls-{day}.jsonl")), text)?;
            out.push(day.to_string());
        }
        sqlx::query("INSERT INTO server_meta(key, value) VALUES ('calllog_exported_day', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
            .bind(day.to_string())
            .execute(pool)
            .await?;
        day = match day.succ_opt() {
            Some(d) => d,
            None => break,
        };
    }
    Ok(out)
}

fn local_midnight_ms(d: chrono::NaiveDate) -> i64 {
    use chrono::TimeZone;
    let naive = d.and_hms_opt(0, 0, 0).unwrap_or_default();
    chrono::Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|t| t.timestamp_millis())
        .unwrap_or_else(|| naive.and_utc().timestamp_millis())
}

/// Delete rows older than `days`. The daily pass exports finished days first, so only rows older
/// than the export window (e.g. after a long outage) go without an export.
pub async fn prune(pool: &DbPool, days: i64) -> Result<u64> {
    let cutoff = chrono::Utc::now().timestamp_millis() - days.max(1) * 86_400_000;
    let r = sqlx::query("DELETE FROM call_log WHERE ts_ms < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

/// Daily pass: export finished days, then prune.
pub async fn daily(pool: &DbPool, layout: &crate::storage::Layout, machine: &str, days: i64) {
    match export_finished_days(pool, &export_dir(layout), machine, days).await {
        Ok(d) if !d.is_empty() => tracing::info!(days = ?d, "call log exported"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error=%e, "call log export failed (not pruning)");
            return;
        }
    }
    match prune(pool, days).await {
        Ok(n) if n > 0 => tracing::info!(rows = n, "call log pruned"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error=%e, "call log prune failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_come_from_the_proto() {
        assert_eq!(kind_of("/itcmdr.Itcmdr/WatchUpdates"), "server_stream");
        assert_eq!(kind_of("/itcmdr.Itcmdr/ListAudit"), "unary");
        assert_eq!(kind_of("/itcmdr.plugin.PluginService/Execute"), "unary");
    }

    #[test]
    fn grpc_message_is_decoded() {
        assert_eq!(percent_decode("bad%20signature"), "bad signature");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn log_lines_validate() {
        let r = CallRecord {
            id: "c1".into(),
            ts_ms: 1_761_900_000_000,
            service: "itcmdr.Itcmdr".into(),
            method: "ListAudit".into(),
            kind: "unary".into(),
            peer: "127.0.0.1:5000".into(),
            principal: "Dr A".into(),
            status: 16,
            duration_ms: 3,
            req_bytes: 5,
            resp_bytes: 0,
            error: "unknown key".into(),
        };
        let l = log_line(&r, "SERVER01").to_string();
        let m = itcmdr_plugin::log::validate(&l).unwrap();
        assert_eq!(m["level"], "warn");
        assert_eq!(m["message"], "ListAudit UNAUTHENTICATED");
        assert_eq!(code_name(99), "UNKNOWN");
    }
}
