//! Rust agents (v0.17.0, docs/historical/1.0/design/SERVER-AGENT.md sections 2 and 4).
//!
//! - Enrollment: an admin creates a one-time token (`CreateEnrollToken`, `itcmdr-cli agent
//!   enroll-token`). It carries the agent link address and the SHA-256 of the server's TLS
//!   certificate (pinned by the agent); only a hash of its secret is stored. `Enroll` takes the
//!   secret, the agent's Ed25519 public key and a CSR, and returns a client certificate issued by
//!   the site CA (CN = agent id, SAN = machine, 90 days).
//! - Link: mTLS (the client certificate must be the one issued to that agent and not revoked),
//!   a signed `Hello` bound to the certificate, then `Ping` every `agent.heartbeatSecs` and `Pong`.
//! - Heartbeats: `tick` moves agents online -> late -> non_responsive by missed beats
//!   (`agent.lateHeartbeats`, `agent.missedHeartbeats`, per machine overridable) and raises
//!   `agent-nonresponsive:<machine>` at `agent.nonresponsiveLevel` (`agent.serverNonresponsiveLevel`
//!   for agents enrolled with role server). Coming back resolves it and notes "back online".
//! - TLS material lives in the keys folder: `agent-ca.pem`/`agent-ca.key` (the site CA) and
//!   `agent-server.pem`/`agent-server.key`; keys are secret files (DPAPI on Windows, 0600 elsewhere).
use crate::alerts::{self, Raise};
use crate::config::ServerConfig;
use crate::db::DbPool;
use crate::storage::Layout;
use anyhow::{bail, ensure, Context, Result};
use itcmdr_agentlink::pb;
use itcmdr_agentlink::tls::{self, Pem};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;
use sqlx::Row;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

pub const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS agents (
        id TEXT PRIMARY KEY,
        machine TEXT NOT NULL,
        role TEXT NOT NULL,
        public_key TEXT NOT NULL,
        cert_fp TEXT NOT NULL,
        cert_serial TEXT NOT NULL,
        cert_expires_ms INTEGER NOT NULL,
        state TEXT NOT NULL,
        enrolled_ms INTEGER NOT NULL,
        last_seen_ms INTEGER,
        version TEXT NOT NULL DEFAULT '',
        os TEXT NOT NULL DEFAULT '',
        peer TEXT NOT NULL DEFAULT '',
        maintenance INTEGER NOT NULL DEFAULT 0,
        revoked_ms INTEGER
    );
    CREATE INDEX IF NOT EXISTS ix_agents_machine ON agents(machine, state);
    CREATE TABLE IF NOT EXISTS enroll_tokens (
        id TEXT PRIMARY KEY,
        secret_hash TEXT NOT NULL UNIQUE,
        machine TEXT NOT NULL,
        created_ms INTEGER NOT NULL,
        expires_ms INTEGER NOT NULL,
        used_ms INTEGER,
        created_by TEXT NOT NULL
    )
"#;

pub const STATES: [&str; 6] = [
    "enrolled",
    "online",
    "late",
    "non_responsive",
    "maintenance",
    "revoked",
];

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// The site CA and server certificate (created on first use).
#[derive(Clone)]
pub struct Material {
    pub ca: Pem,
    pub server: Pem,
    pub fingerprint: String,
    /// v0.17.1: (root public key hex, signature hex) certifying the CA; None until a root key exists.
    pub ca_sig: Option<(String, String)>,
}

/// Certify the site CA with the server root key (itself certified by the master key), once.
fn ca_signature(layout: &Layout, ca: &Pem) -> Option<(String, String)> {
    let p = layout.keys().join("agent-ca.sig");
    if let Ok(t) = std::fs::read_to_string(&p) {
        let v: serde_json::Value = serde_json::from_str(&t).ok()?;
        return Some((
            v["root_public_key"].as_str()?.to_string(),
            v["signature"].as_str()?.to_string(),
        ));
    }
    let root = crate::auth::load_root_key(layout).ok()?;
    let der = tls::pem_to_der(&ca.cert).ok()?;
    use ed25519_dalek::Signer;
    let sig = hex::encode(
        root.sign(&itcmdr_agentlink::ca_cert_bytes(der.as_ref()))
            .to_bytes(),
    );
    let pk = hex::encode(root.verifying_key().to_bytes());
    let j = serde_json::json!({"root_public_key": pk, "signature": sig});
    if std::fs::write(&p, j.to_string()).is_ok() {
        tracing::info!("site CA certified by the server root key");
    }
    Some((pk, sig))
}

fn read_pair(layout: &Layout, name: &str) -> Result<Option<Pem>> {
    let c = layout.keys().join(format!("{name}.pem"));
    let k = layout.keys().join(format!("{name}.key"));
    if !c.exists() || !k.exists() {
        return Ok(None);
    }
    let cert = std::fs::read_to_string(&c)?;
    let key = String::from_utf8(crate::storage::read_secret(&k)?).context("key file")?;
    Ok(Some(Pem { cert, key }))
}

fn write_pair(layout: &Layout, name: &str, p: &Pem) -> Result<()> {
    std::fs::create_dir_all(layout.keys())?;
    crate::storage::write_secret(&layout.keys().join(format!("{name}.key")), p.key.as_bytes())?;
    std::fs::write(layout.keys().join(format!("{name}.pem")), &p.cert)?;
    Ok(())
}

/// Load the site CA and server certificate, creating them when missing. The server certificate
/// is reissued when it expires within 30 days (agents then need a new token: it is pinned).
pub fn ensure_material(layout: &Layout) -> Result<Material> {
    let ca = match read_pair(layout, "agent-ca")? {
        Some(c) => c,
        None => {
            let c = tls::new_ca("itcmdr site CA")?;
            write_pair(layout, "agent-ca", &c)?;
            tracing::info!("created the site CA for agent certificates");
            c
        }
    };
    let server = match read_pair(layout, "agent-server")? {
        Some(s) => s,
        None => {
            let host = crate::runtime::hostname();
            let s = tls::issue_server(&ca, &[host, "localhost".into(), "127.0.0.1".into()], 3650)?;
            write_pair(layout, "agent-server", &s)?;
            s
        }
    };
    let fingerprint = tls::pem_fingerprint(&server.cert)?;
    let ca_sig = ca_signature(layout, &ca);
    Ok(Material {
        ca,
        server,
        fingerprint,
        ca_sig,
    })
}

/// The agent link address put into tokens: `explicit`, else this host with the bind port.
pub fn advertised_address(cfg: &ServerConfig, explicit: &str) -> String {
    if !explicit.trim().is_empty() {
        return explicit.trim().to_string();
    }
    let port = cfg
        .agent_bind_addr
        .rsplit_once(':')
        .and_then(|(_, p)| p.parse::<u16>().ok())
        .unwrap_or(itcmdr_agentlink::DEFAULT_PORT);
    format!("{}:{port}", crate::runtime::hostname())
}

pub struct NewToken {
    pub token: String,
    pub expires_ms: i64,
    pub address: String,
    pub fingerprint: String,
}

/// Create a one-time enrollment token.
pub async fn create_token(
    pool: &DbPool,
    layout: &Layout,
    cfg: &ServerConfig,
    machine: &str,
    ttl_mins: i64,
    address: &str,
    actor: &str,
) -> Result<NewToken> {
    ensure!(
        machine.is_empty() || itcmdr_agentlink::valid_machine(machine),
        "invalid machine name {machine:?}"
    );
    let ttl = if ttl_mins <= 0 {
        24 * 60
    } else {
        ttl_mins.min(30 * 24 * 60)
    };
    let m = ensure_material(layout)?;
    let mut secret = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut secret);
    let secret = hex::encode(secret);
    let t = now();
    let expires_ms = t + ttl * 60_000;
    sqlx::query("INSERT INTO enroll_tokens(id, secret_hash, machine, created_ms, expires_ms, created_by) VALUES (?,?,?,?,?,?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(itcmdr_agentlink::sha256_hex(secret.as_bytes()))
        .bind(machine)
        .bind(t)
        .bind(expires_ms)
        .bind(actor)
        .execute(pool)
        .await?;
    let address = advertised_address(cfg, address);
    let token = itcmdr_agentlink::Token {
        v: 1,
        addr: address.clone(),
        pin: m.fingerprint.clone(),
        secret,
        machine: machine.into(),
        root: m
            .ca_sig
            .as_ref()
            .map(|(pk, _)| pk.clone())
            .unwrap_or_default(),
    }
    .encode();
    Ok(NewToken {
        token,
        expires_ms,
        address,
        fingerprint: m.fingerprint,
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Agent {
    pub id: String,
    pub machine: String,
    pub role: String,
    pub state: String,
    pub enrolled_ms: i64,
    pub last_seen_ms: i64,
    pub version: String,
    pub os: String,
    pub peer: String,
    pub cert_expires_ms: i64,
    pub cert_fp: String,
    pub public_key: String,
    pub cert_issued_ms: i64,
    pub cert_serial: String,
    pub renew_requested: bool,
    #[serde(skip)]
    pub prev_cert_fp: String,
    #[serde(skip)]
    pub prev_until_ms: i64,
    #[serde(skip)]
    pub last_seq: i64,
}

fn row(r: &sqlx::sqlite::SqliteRow) -> Agent {
    Agent {
        id: r.get("id"),
        machine: r.get("machine"),
        role: r.get("role"),
        state: r.get("state"),
        enrolled_ms: r.get("enrolled_ms"),
        last_seen_ms: r.get::<Option<i64>, _>("last_seen_ms").unwrap_or(0),
        version: r.get("version"),
        os: r.get("os"),
        peer: r.get("peer"),
        cert_expires_ms: r.get("cert_expires_ms"),
        cert_fp: r.get("cert_fp"),
        public_key: r.get("public_key"),
        cert_issued_ms: r.get("cert_issued_ms"),
        cert_serial: r.get("cert_serial"),
        renew_requested: r.get::<i64, _>("renew_requested") != 0,
        prev_cert_fp: r
            .get::<Option<String>, _>("prev_cert_fp")
            .unwrap_or_default(),
        prev_until_ms: r.get::<Option<i64>, _>("prev_until_ms").unwrap_or(0),
        last_seq: r.get("last_seq"),
    }
}

/// Every agent, newest enrollment first. `include_revoked`: also old/revoked enrollments.
pub async fn list(pool: &DbPool, include_revoked: bool) -> Result<Vec<Agent>> {
    let q = if include_revoked {
        "SELECT * FROM agents ORDER BY machine, enrolled_ms DESC"
    } else {
        "SELECT * FROM agents WHERE state != 'revoked' ORDER BY machine"
    };
    Ok(sqlx::query(q)
        .fetch_all(pool)
        .await?
        .iter()
        .map(row)
        .collect())
}

pub async fn get(pool: &DbPool, id: &str) -> Result<Option<Agent>> {
    Ok(sqlx::query("SELECT * FROM agents WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(row))
}

/// The live (not revoked) enrollment of a machine.
pub async fn by_machine(pool: &DbPool, machine: &str) -> Result<Option<Agent>> {
    Ok(sqlx::query("SELECT * FROM agents WHERE machine = ? AND state != 'revoked' ORDER BY enrolled_ms DESC LIMIT 1")
        .bind(machine)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(row))
}

/// Open streams, by agent id: sending `true` ends the stream (revoke, re-enroll).
type Live = HashMap<String, (u64, watch::Sender<bool>)>;
static LIVE: Lazy<Mutex<Live>> = Lazy::new(Default::default);
static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn kick(agent_id: &str) {
    if let Some((_, tx)) = LIVE.lock().remove(agent_id) {
        let _ = tx.send(true);
    }
}

async fn set_state(pool: &DbPool, a: &Agent, state: &str, why: &str) -> Result<()> {
    if a.state == state {
        return Ok(());
    }
    sqlx::query("UPDATE agents SET state = ? WHERE id = ?")
        .bind(state)
        .bind(&a.id)
        .execute(pool)
        .await?;
    crate::db::insert_audit(
        pool,
        "itcmdr-server",
        "agent-state",
        &serde_json::json!({"machine": a.machine, "agent": a.id, "from": a.state, "to": state, "why": why}).to_string(),
    )
    .await?;
    Ok(())
}

/// Revoke the machine's enrollment (its certificate is refused from now on; the stream closes).
pub async fn revoke(pool: &DbPool, machine: &str) -> Result<Option<Agent>> {
    let Some(a) = by_machine(pool, machine).await? else {
        return Ok(None);
    };
    sqlx::query("UPDATE agents SET state = 'revoked', revoked_ms = ? WHERE id = ?")
        .bind(now())
        .bind(&a.id)
        .execute(pool)
        .await?;
    kick(&a.id);
    refresh_revoked(pool).await;
    alerts::resolve_key(pool, &format!("agent-nonresponsive:{}", a.machine)).await?;
    get(pool, &a.id).await
}

/// Maintenance on: no heartbeat alerts (e.g. a PC off for the holidays). Off: back to the
/// state its heartbeats give it.
pub async fn set_maintenance(pool: &DbPool, machine: &str, on: bool) -> Result<Option<Agent>> {
    let Some(a) = by_machine(pool, machine).await? else {
        return Ok(None);
    };
    sqlx::query("UPDATE agents SET maintenance = ? WHERE id = ?")
        .bind(on as i64)
        .bind(&a.id)
        .execute(pool)
        .await?;
    if on {
        set_state(pool, &a, "maintenance", "admin").await?;
        alerts::resolve_key(pool, &format!("agent-nonresponsive:{}", a.machine)).await?;
    } else {
        let st = if LIVE.lock().contains_key(&a.id) {
            "online"
        } else {
            "enrolled"
        };
        set_state(pool, &a, st, "admin").await?;
    }
    get(pool, &a.id).await
}

/// v0.18.0 Prepare: the CA and its certification for a valid (unused, unexpired) token, without
/// using it, so the agent can check the root key before the server records an enrollment.
pub async fn prepare(pool: &DbPool, layout: &Layout, secret: &str) -> Result<pb::PrepareResponse> {
    let hash = itcmdr_agentlink::sha256_hex(secret.trim().as_bytes());
    let row: Option<(i64, Option<i64>)> =
        sqlx::query_as("SELECT expires_ms, used_ms FROM enroll_tokens WHERE secret_hash = ?")
            .bind(&hash)
            .fetch_optional(pool)
            .await?;
    let Some((exp, used)) = row else {
        bail!("unknown enrollment token")
    };
    ensure!(used.is_none(), "this enrollment token was already used");
    ensure!(exp > now(), "this enrollment token has expired");
    let m = ensure_material(layout)?;
    let (pk, sig) = m.ca_sig.unwrap_or_default();
    Ok(pb::PrepareResponse {
        ca_cert_pem: m.ca.cert,
        ca_signature: sig,
        root_public_key: pk,
    })
}

/// v0.18.0 Check: is this agent id revoked? (No client certificate: a revoked agent's handshake
/// fails, so this is how it learns to stop retrying.)
///
/// v1.3.3: answered only to the enrolled agent itself. The request must name a certificate issued
/// to that enrollment (current or previous) and carry a fresh, unreplayed Ed25519 signature by the
/// agent key registered at enrollment over `check_bytes`. Anything else, an unknown id included,
/// gets the same empty answer, so Check is not an oracle for which ids exist or were revoked. The
/// nonce is only recorded after the signature verifies, so others cannot burn an agent's nonces.
pub async fn check(
    pool: &DbPool,
    nonces: &crate::auth::NonceCache,
    r: &pb::CheckRequest,
    skew_ms: i64,
) -> Result<pb::CheckResponse> {
    let row: Option<(String, String, Option<String>, String)> =
        sqlx::query_as("SELECT state, cert_fp, prev_cert_fp, public_key FROM agents WHERE id = ?")
            .bind(&r.agent_id)
            .fetch_optional(pool)
            .await?;
    let Some((state, cur, prev, pk)) = row else {
        return Ok(pb::CheckResponse::default());
    };
    let fp_ok = fp_matches(&r.cert_fingerprint, &cur)
        || fp_matches(&r.cert_fingerprint, prev.as_deref().unwrap_or(""));
    let fresh = (now() - r.ts_ms).abs() <= skew_ms;
    let nonce_ok = (16..=128).contains(&r.nonce.len())
        && r.nonce
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_');
    let sig_ok = (|| -> Option<()> {
        let pk: [u8; 32] = hex::decode(&pk).ok()?.try_into().ok()?;
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk).ok()?;
        let sig: [u8; 64] = hex::decode(&r.signature).ok()?.try_into().ok()?;
        vk.verify_strict(
            &itcmdr_agentlink::check_bytes(&r.agent_id, &r.cert_fingerprint, r.ts_ms, &r.nonce),
            &ed25519_dalek::Signature::from_bytes(&sig),
        )
        .ok()
    })()
    .is_some();
    if !(fp_ok && fresh && nonce_ok && sig_ok)
        || !nonces.check_and_insert_ttl(
            &format!("check:{}", r.agent_id),
            &r.nonce,
            now(),
            2 * skew_ms,
        )
    {
        return Ok(pb::CheckResponse::default());
    }
    Ok(pb::CheckResponse {
        revoked: state == "revoked",
        known: true,
    })
}

/// Case-insensitive comparison of two hex fingerprints in constant time; empty never matches.
fn fp_matches(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim().to_ascii_lowercase(), b.trim().to_ascii_lowercase());
    if a.is_empty() || a.len() != b.len() {
        return false;
    }
    a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// v1.3.3: the enrollment a client certificate was issued to (current certificate, or the
/// previous one during its renewal overlap), revoked ones included.
pub async fn by_cert_fp(pool: &DbPool, fp: &str) -> Result<Option<Agent>> {
    let fp = fp.to_ascii_lowercase();
    Ok(sqlx::query(
        "SELECT * FROM agents WHERE lower(cert_fp) = ? OR (lower(prev_cert_fp) = ? AND prev_until_ms > ?) ORDER BY enrolled_ms DESC LIMIT 1",
    )
    .bind(&fp)
    .bind(&fp)
    .bind(now())
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(row))
}

/// Enroll: check and burn the token, register the key, sign the CSR.
pub async fn enroll(
    pool: &DbPool,
    layout: &Layout,
    r: &pb::EnrollRequest,
    peer: &str,
) -> Result<pb::EnrollResponse> {
    let machine = r.machine.trim();
    ensure!(
        itcmdr_agentlink::valid_machine(machine),
        "invalid machine name {machine:?}"
    );
    let pk = hex::decode(r.public_key.trim()).context("public_key is not hex")?;
    ensure!(
        pk.len() == 32
            && ed25519_dalek::VerifyingKey::from_bytes(pk.as_slice().try_into().unwrap()).is_ok(),
        "public_key is not an Ed25519 key"
    );
    let role = match r.role.as_str() {
        "" | "workstation" => "workstation",
        "server" => "server",
        o => bail!("unknown role {o:?}"),
    };
    let t = now();
    let hash = itcmdr_agentlink::sha256_hex(r.token_secret.trim().as_bytes());
    let mut tx = pool.begin().await?;
    let tok = sqlx::query(
        "SELECT id, machine, expires_ms, used_ms FROM enroll_tokens WHERE secret_hash = ?",
    )
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(tok) = tok else {
        bail!("unknown enrollment token")
    };
    ensure!(
        tok.get::<Option<i64>, _>("used_ms").is_none(),
        "this enrollment token was already used"
    );
    ensure!(
        tok.get::<i64, _>("expires_ms") > t,
        "this enrollment token has expired"
    );
    let tm: String = tok.get("machine");
    ensure!(
        tm.is_empty() || tm.eq_ignore_ascii_case(machine),
        "this enrollment token is for {tm}, not {machine}"
    );
    let m = ensure_material(layout)?;
    let id = uuid::Uuid::new_v4().to_string();
    let life = crate::settings::get_i64(pool, None, "agent.certLifetimeMins").await;
    let issued = tls::sign_csr(&m.ca, &r.csr_pem, &id, machine, life)?;
    sqlx::query("UPDATE enroll_tokens SET used_ms = ? WHERE id = ?")
        .bind(t)
        .bind(tok.get::<String, _>("id"))
        .execute(&mut *tx)
        .await?;
    // A re-enrollment replaces the machine's previous one.
    let old: Vec<String> =
        sqlx::query_scalar("SELECT id FROM agents WHERE machine = ? AND state != 'revoked'")
            .bind(machine)
            .fetch_all(&mut *tx)
            .await?;
    sqlx::query("UPDATE agents SET state = 'revoked', revoked_ms = ? WHERE machine = ? AND state != 'revoked'").bind(t).bind(machine).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO agents(id, machine, role, public_key, cert_fp, cert_serial, cert_expires_ms, state, enrolled_ms, version, os, peer, cert_issued_ms) VALUES (?,?,?,?,?,?,?,'enrolled',?,?,?,?,?)")
        .bind(&id)
        .bind(machine)
        .bind(role)
        .bind(hex::encode(&pk))
        .bind(&issued.fingerprint)
        .bind(&issued.serial)
        .bind(issued.not_after_ms)
        .bind(t)
        .bind(r.agent_version.chars().take(40).collect::<String>())
        .bind(r.os.chars().take(120).collect::<String>())
        .bind(peer)
        .bind(t)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    refresh_revoked(pool).await;
    for o in old {
        kick(&o);
    }
    crate::db::insert_audit(
        pool,
        "itcmdr-server",
        "agent-enroll",
        &serde_json::json!({"machine": machine, "agent": id, "role": role, "cert_fp": issued.fingerprint}).to_string(),
    )
    .await?;
    Ok(pb::EnrollResponse {
        agent_id: id,
        client_cert_pem: issued.cert_pem,
        ca_cert_pem: m.ca.cert.clone(),
        cert_expires_ms: issued.not_after_ms,
        heartbeat_secs: crate::settings::get_i64(pool, None, "agent.heartbeatSecs").await as i32,
        ca_signature: m
            .ca_sig
            .as_ref()
            .map(|(_, s)| s.clone())
            .unwrap_or_default(),
        root_public_key: m.ca_sig.map(|(pk, _)| pk).unwrap_or_default(),
    })
}

/// A machine-scoped integer setting (falls back to the global value, then the default).
pub async fn machine_i64(pool: &DbPool, key: &str, machine: &str) -> i64 {
    machine_value(pool, key, machine)
        .await
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| {
            crate::settings::def(key)
                .ok()
                .and_then(|d| d.default.as_i64())
                .unwrap_or_default()
        })
}

/// A text setting resolved for one machine (None when unset/empty).
pub async fn machine_text(pool: &DbPool, key: &str, machine: &str) -> Option<String> {
    machine_value(pool, key, machine)
        .await
        .and_then(|v| v.as_str().map(str::to_string))
        .filter(|s| !s.trim().is_empty())
}

async fn machine_value(pool: &DbPool, key: &str, machine: &str) -> Option<serde_json::Value> {
    crate::settings::resolve_all(pool, None, Some(machine))
        .await
        .ok()?
        .into_iter()
        .find(|r| r.def.key == key)
        .map(|r| r.value)
}

/// Heartbeat state machine pass (called every few seconds).
pub async fn tick(pool: &DbPool) -> Result<()> {
    let t = now();
    refresh_revoked(pool).await;
    for a in list(pool, false).await? {
        cert_alert(pool, &a, t).await?;
        if a.state == "maintenance" || a.state == "enrolled" && a.last_seen_ms == 0 {
            continue;
        }
        let interval = machine_i64(pool, "agent.heartbeatSecs", &a.machine)
            .await
            .max(1)
            * 1000;
        let late = machine_i64(pool, "agent.lateHeartbeats", &a.machine)
            .await
            .max(1);
        let lost = machine_i64(pool, "agent.missedHeartbeats", &a.machine)
            .await
            .max(late);
        // Office-hours profile: outside the hours nothing escalates to an alert, and misses
        // count from the start of today's hours (a PC switched on in the morning is not "lost").
        let (since, outside) = match machine_value(pool, "agent.officeHours", &a.machine)
            .await
            .and_then(|v| v.as_str().map(str::to_string))
            .filter(|t| !t.trim().is_empty())
            .and_then(|t| crate::schedule::parse_hours(&t).ok())
        {
            Some(h) => match office_start_ms(&h, chrono::Local::now()) {
                Some(start) => (a.last_seen_ms.max(start), false),
                None => (a.last_seen_ms, true),
            },
            None => (a.last_seen_ms, false),
        };
        let missed = (t - since) / interval;
        let key = format!("agent-nonresponsive:{}", a.machine);
        if missed >= lost && !outside {
            if a.state != "non_responsive" {
                set_state(
                    pool,
                    &a,
                    "non_responsive",
                    &format!("{missed} heartbeats missed"),
                )
                .await?;
                let lvl_key = if a.role == "server" {
                    "agent.serverNonresponsiveLevel"
                } else {
                    "agent.nonresponsiveLevel"
                };
                let level = machine_value(pool, lvl_key, &a.machine)
                    .await
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| "critical".into());
                alerts::raise(
                    pool,
                    &Raise {
                        key,
                        level,
                        source: "agent".into(),
                        machine: a.machine.clone(),
                        title: format!("{} is not responding", a.machine),
                        detail: format!(
                            "No heartbeat for {missed} intervals (last seen {}).",
                            ms_text(a.last_seen_ms)
                        ),
                        run_id: None,
                    },
                )
                .await?;
            }
        } else if missed >= late {
            if a.state == "online" {
                set_state(pool, &a, "late", &format!("{missed} heartbeats missed")).await?;
            }
        } else if a.state == "late" || a.state == "non_responsive" {
            back_online(pool, &a).await?;
        }
    }
    Ok(())
}

fn renew_now(ack: u64, reason: &str) -> pb::ServerMessage {
    pb::ServerMessage {
        ack_seq: ack,
        body: Some(pb::server_message::Body::RenewNow(pb::RenewNow {
            reason: reason.into(),
        })),
    }
}

/// How long the replaced certificate keeps working after a renewal (the agent reconnects at once).
const RENEW_OVERLAP_MS: i64 = 24 * 3_600_000;

/// Sign a renewal CSR received over an authenticated link and make it the current certificate.
pub async fn renew(
    pool: &DbPool,
    layout: &Layout,
    a: &Agent,
    csr_pem: &str,
) -> Result<pb::Renewed> {
    let m = ensure_material(layout)?;
    let life = crate::settings::get_i64(pool, None, "agent.certLifetimeMins").await;
    let i = tls::sign_csr(&m.ca, csr_pem, &a.id, &a.machine, life)?;
    let t = now();
    sqlx::query("UPDATE agents SET prev_cert_fp = cert_fp, prev_serial = cert_serial, prev_until_ms = ?, cert_fp = ?, cert_serial = ?, cert_expires_ms = ?, cert_issued_ms = ?, renew_requested = 0 WHERE id = ? AND state != 'revoked'")
        .bind(t + RENEW_OVERLAP_MS)
        .bind(&i.fingerprint)
        .bind(&i.serial)
        .bind(i.not_after_ms)
        .bind(t)
        .bind(&a.id)
        .execute(pool)
        .await?;
    refresh_revoked(pool).await;
    alerts::resolve_key(pool, &format!("agent-cert:{}", a.machine)).await?;
    crate::db::insert_audit(
        pool,
        "itcmdr-server",
        "agent-renew",
        &serde_json::json!({"machine": a.machine, "agent": a.id, "cert_fp": i.fingerprint})
            .to_string(),
    )
    .await?;
    tracing::info!(machine=%a.machine, "agent certificate renewed");
    Ok(pb::Renewed {
        cert_pem: i.cert_pem,
        expires_ms: i.not_after_ms,
    })
}

/// Ask the machine's agent to renew its certificate at its next heartbeat (`agent renew`).
pub async fn request_renew(pool: &DbPool, machine: &str) -> Result<Option<Agent>> {
    let Some(a) = by_machine(pool, machine).await? else {
        return Ok(None);
    };
    sqlx::query("UPDATE agents SET renew_requested = 1 WHERE id = ?")
        .bind(&a.id)
        .execute(pool)
        .await?;
    get(pool, &a.id).await
}

/// A queued alert from the agent (`itcmdr-agent alert`), delivered once even across reconnects.
/// Record a durable queue seq; false when it was already delivered.
pub async fn claim_seq(pool: &DbPool, agent_id: &str, seq: u64) -> Result<bool> {
    let seq = seq as i64;
    let r = sqlx::query("UPDATE agents SET last_seq = ? WHERE id = ? AND last_seq < ?")
        .bind(seq)
        .bind(agent_id)
        .bind(seq)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

async fn agent_alert(pool: &DbPool, a: &Agent, seq: u64, al: &pb::AgentAlert) -> Result<()> {
    if !claim_seq(pool, &a.id, seq).await? {
        return Ok(()); // already delivered
    }
    let k: String = al
        .key
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-_.:".contains(*c))
        .take(80)
        .collect();
    ensure!(!k.is_empty(), "alert key is empty");
    let key = format!("agent:{}:{k}", a.machine);
    if al.resolve {
        alerts::resolve_key(pool, &key).await?;
        return Ok(());
    }
    let level = if alerts::LEVELS.contains(&al.level.as_str()) {
        al.level.clone()
    } else {
        "cautious".into()
    };
    let mut detail: String = al.detail.chars().take(4000).collect();
    if al.ts_ms > 0 && now() - al.ts_ms > 120_000 {
        detail = format!("{detail}\n(queued on the agent at {})", ms_text(al.ts_ms));
    }
    alerts::raise(
        pool,
        &Raise {
            key,
            level,
            source: "agent".into(),
            machine: a.machine.clone(),
            title: al.title.chars().take(200).collect(),
            detail,
            run_id: None,
        },
    )
    .await?;
    Ok(())
}

/// Start (unix ms) of the office-hours window `at` is in, or None outside the hours.
pub fn office_start_ms(
    h: &crate::schedule::Hours,
    at: chrono::DateTime<chrono::Local>,
) -> Option<i64> {
    use chrono::{Datelike, TimeZone};
    let (wd, t) = (at.weekday(), at.time());
    if !h.days.contains(&wd) || t < h.start || t >= h.end {
        return None;
    }
    chrono::Local
        .from_local_datetime(&at.date_naive().and_time(h.start))
        .earliest()
        .map(|d| d.timestamp_millis())
}

/// Certificate expiry alert: Cautious within 14 days, Critical within 3, resolved after renewal.
async fn cert_alert(pool: &DbPool, a: &Agent, t: i64) -> Result<()> {
    let key = format!("agent-cert:{}", a.machine);
    let left = a.cert_expires_ms - t;
    let level = if left < 3 * 86_400_000 {
        "critical"
    } else if left < 14 * 86_400_000 {
        "cautious"
    } else {
        alerts::resolve_key(pool, &key).await?;
        return Ok(());
    };
    alerts::raise(
        pool,
        &Raise {
            key,
            level: level.into(),
            source: "agent".into(),
            machine: a.machine.clone(),
            title: format!("{} agent certificate expires {}", a.machine, ms_text(a.cert_expires_ms)),
            detail: "The agent renews it over the link at 2/3 of its lifetime; if it is offline, bring it online or enroll it again.".into(),
            run_id: None,
        },
    )
    .await?;
    Ok(())
}

/// Serials of agent client certificates still allowed in the TLS handshake: the current one of
/// every non-revoked agent, plus the previous one during its renewal overlap. Anything else signed
/// by the site CA (a revoked agent, or a certificate replaced by renewal) is refused at once.
/// Keyed by database file, so several servers in one process (tests) do not share a set.
type Allowed = HashMap<String, std::collections::HashSet<String>>;
static ALLOWED: Lazy<parking_lot::RwLock<Allowed>> = Lazy::new(Default::default);

fn db_key(pool: &DbPool) -> String {
    pool.connect_options()
        .get_filename()
        .to_string_lossy()
        .to_string()
}

/// Reload the allowed serial set (every heartbeat pass, and after revoke/enroll/renew).
pub async fn refresh_revoked(pool: &DbPool) {
    type Row3 = (String, Option<String>, Option<i64>);
    let r: Result<Vec<Row3>, _> = sqlx::query_as(
        "SELECT cert_serial, prev_serial, prev_until_ms FROM agents WHERE state != 'revoked'",
    )
    .fetch_all(pool)
    .await;
    match r {
        Ok(rows) => {
            let t = now();
            let mut set = std::collections::HashSet::new();
            for (s, p, until) in rows {
                set.insert(s.to_lowercase());
                if let (Some(p), Some(u)) = (p, until) {
                    if u > t {
                        set.insert(p.to_lowercase());
                    }
                }
            }
            ALLOWED.write().insert(db_key(pool), set);
        }
        Err(e) => tracing::warn!("agent serial refresh: {e}"),
    }
}

/// True when a client certificate with this serial (hex) must be refused in the handshake.
pub fn serial_revoked(pool: &DbPool, serial: &str) -> bool {
    match ALLOWED.read().get(&db_key(pool)) {
        Some(set) => !set.contains(&serial.to_lowercase()),
        None => true,
    }
}

fn ms_text(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}

async fn back_online(pool: &DbPool, a: &Agent) -> Result<()> {
    let was = a.state.clone();
    set_state(pool, a, "online", "heartbeat").await?;
    if alerts::resolve_key(pool, &format!("agent-nonresponsive:{}", a.machine)).await? > 0
        || was == "non_responsive"
    {
        let id = alerts::raise(
            pool,
            &Raise {
                key: format!("agent-online:{}", a.machine),
                level: "informational".into(),
                source: "agent".into(),
                machine: a.machine.clone(),
                title: format!("{} is back online", a.machine),
                detail: String::new(),
                run_id: None,
            },
        )
        .await?;
        // A note, not a problem: closed at once so it does not count as open.
        alerts::resolve(pool, &id, "auto").await?;
    }
    Ok(())
}

async fn seen(pool: &DbPool, id: &str, peer: &str) -> Result<()> {
    sqlx::query("UPDATE agents SET last_seen_ms = ?, peer = CASE WHEN ? = '' THEN peer ELSE ? END WHERE id = ?")
        .bind(now())
        .bind(peer)
        .bind(peer)
        .bind(id)
        .execute(pool)
        .await?;
    if let Some(a) = get(pool, id).await? {
        if a.state == "enrolled" {
            set_state(pool, &a, "online", "hello").await?;
        } else if a.state == "late" || a.state == "non_responsive" {
            back_online(pool, &a).await?;
        }
    }
    Ok(())
}

/// The AgentService on the TLS listener.
pub struct AgentLink {
    pool: DbPool,
    layout: Layout,
    nonces: crate::auth::NonceCache,
}

impl AgentLink {
    pub fn new(pool: DbPool, layout: Layout) -> Self {
        Self {
            pool,
            layout,
            nonces: Default::default(),
        }
    }
}

fn peer_of<T>(r: &Request<T>) -> String {
    r.remote_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_default()
}

fn client_fp<T>(r: &Request<T>) -> Option<String> {
    r.peer_certs()
        .and_then(|c| c.first().map(|d| itcmdr_agentlink::sha256_hex(d.as_ref())))
}

type LinkStream = Pin<Box<dyn Stream<Item = Result<pb::ServerMessage, Status>> + Send>>;

#[tonic::async_trait]
impl pb::agent_service_server::AgentService for AgentLink {
    async fn prepare(
        &self,
        req: Request<pb::PrepareRequest>,
    ) -> Result<Response<pb::PrepareResponse>, Status> {
        prepare(&self.pool, &self.layout, &req.get_ref().token_secret)
            .await
            .map(Response::new)
            .map_err(|e| Status::permission_denied(format!("enrollment refused: {e}")))
    }

    async fn check(
        &self,
        req: Request<pb::CheckRequest>,
    ) -> Result<Response<pb::CheckResponse>, Status> {
        let skew = crate::settings::get_i64(&self.pool, None, "auth.max_skew_secs").await * 1000;
        check(&self.pool, &self.nonces, req.get_ref(), skew)
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn enroll(
        &self,
        req: Request<pb::EnrollRequest>,
    ) -> Result<Response<pb::EnrollResponse>, Status> {
        let peer = peer_of(&req);
        match enroll(&self.pool, &self.layout, req.get_ref(), &peer).await {
            Ok(r) => Ok(Response::new(r)),
            Err(e) => {
                tracing::warn!(error=%e, %peer, "agent enrollment refused");
                Err(Status::permission_denied(format!(
                    "enrollment refused: {e}"
                )))
            }
        }
    }

    type LinkStream = LinkStream;

    async fn link(
        &self,
        req: Request<Streaming<pb::AgentMessage>>,
    ) -> Result<Response<LinkStream>, Status> {
        let peer = peer_of(&req);
        let fp = client_fp(&req)
            .ok_or_else(|| Status::unauthenticated("a client certificate is required"))?;
        // v1.3.3: the certificate must belong to a live enrollment before anything is read. The
        // handshake allow-list already refuses revoked serials; this also covers the moment after
        // a revoke by another process (itcmdr-cli) and before this server reloads that list.
        match by_cert_fp(&self.pool, &fp)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            None => {
                return Err(Status::unauthenticated(
                    "the client certificate is not the one issued to this agent",
                ))
            }
            Some(a) if a.state == "revoked" => {
                return Err(Status::permission_denied(
                    "this agent was revoked; enroll it again",
                ))
            }
            Some(_) => {}
        }
        let mut inbound = req.into_inner();
        let first = tokio::time::timeout(Duration::from_secs(15), inbound.next())
            .await
            .map_err(|_| Status::deadline_exceeded("no Hello"))?
            .ok_or_else(|| Status::invalid_argument("no Hello"))??;
        let Some(pb::agent_message::Body::Hello(h)) = first.body else {
            return Err(Status::invalid_argument("the first message must be Hello"));
        };
        let a = get(&self.pool, &h.agent_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::unauthenticated("unknown agent"))?;
        if a.state == "revoked" {
            return Err(Status::permission_denied(
                "this agent was revoked; enroll it again",
            ));
        }
        let prev_ok = a.prev_until_ms > now() && a.prev_cert_fp.eq_ignore_ascii_case(&fp);
        if !a.cert_fp.eq_ignore_ascii_case(&fp) && !prev_ok {
            return Err(Status::unauthenticated(
                "the client certificate is not the one issued to this agent",
            ));
        }
        if a.cert_expires_ms < now() && !prev_ok {
            return Err(Status::unauthenticated(
                "the client certificate has expired; enroll again",
            ));
        }
        let skew = crate::settings::get_i64(&self.pool, None, "auth.max_skew_secs").await * 1000;
        if (now() - h.ts_ms).abs() > skew {
            return Err(Status::unauthenticated(
                "Hello timestamp outside the allowed clock skew",
            ));
        }
        let ok = (|| -> Option<()> {
            let pk: [u8; 32] = hex::decode(&a.public_key).ok()?.try_into().ok()?;
            let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk).ok()?;
            let sig: [u8; 64] = hex::decode(&h.signature).ok()?.try_into().ok()?;
            vk.verify_strict(
                &itcmdr_agentlink::hello_bytes(&h.agent_id, &h.machine, h.ts_ms, &h.nonce, &fp),
                &ed25519_dalek::Signature::from_bytes(&sig),
            )
            .ok()
        })();
        if ok.is_none() || !h.machine.eq_ignore_ascii_case(&a.machine) {
            return Err(Status::unauthenticated("bad Hello signature"));
        }
        if !self
            .nonces
            .check_and_insert_ttl(&a.id, &h.nonce, now(), 2 * skew)
        {
            return Err(Status::unauthenticated("replayed Hello"));
        }
        let _ = sqlx::query("UPDATE agents SET version = ?, os = ? WHERE id = ?")
            .bind(h.agent_version.chars().take(40).collect::<String>())
            .bind(h.os.chars().take(120).collect::<String>())
            .bind(&a.id)
            .execute(&self.pool)
            .await;
        seen(&self.pool, &a.id, &peer)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        tracing::info!(machine=%a.machine, %peer, "agent connected");

        let (stop_tx, mut stop_rx) = watch::channel(false);
        let gen = GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some((_, old)) = LIVE.lock().insert(a.id.clone(), (gen, stop_tx)) {
            let _ = old.send(true);
        }
        let hb = machine_i64(&self.pool, "agent.heartbeatSecs", &a.machine)
            .await
            .max(1);
        let reconnect = crate::settings::get_i64(&self.pool, None, "agent.reconnectMaxSecs").await;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<pb::ServerMessage, Status>>(16);
        let _ = tx
            .send(Ok(pb::ServerMessage {
                ack_seq: first.seq,
                body: Some(pb::server_message::Body::Welcome(pb::Welcome {
                    server_ts_ms: now(),
                    heartbeat_secs: hb as i32,
                    reconnect_max_secs: reconnect as i32,
                    queue_max_mib: machine_i64(&self.pool, "agent.queueMaxMib", &a.machine).await
                        as i32,
                    root_public_key: ensure_material(&self.layout)
                        .ok()
                        .and_then(|m| m.ca_sig.map(|(pk, _)| pk))
                        .unwrap_or_default(),
                })),
            }))
            .await;
        let mut renew_sent = false;
        if a.renew_requested || prev_ok {
            renew_sent = true;
            let _ = tx
                .send(Ok(renew_now(
                    first.seq,
                    if prev_ok {
                        "connected with the replaced certificate"
                    } else {
                        "requested by an administrator"
                    },
                )))
                .await;
        }
        let pool = self.pool.clone();
        let layout = self.layout.clone();
        let agent = a.clone();
        let mut have = crate::agent_plugins::parse_have(&h.plugins);
        let mut sched_ver = h.schedule_version.clone();
        let mut cfg_ver = h.config_version.clone();
        let running = h.agent_version.clone();
        let mut upgrade_sent = String::new();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(hb as u64));
            ticker.tick().await;
            let mut last_seq = first.seq;
            // v0.18.0: plugin/schedule sync now and every 30 s; RunNow requests every 2 s.
            let mut sync_t = tokio::time::interval(Duration::from_secs(30));
            let mut req_t = tokio::time::interval(Duration::from_secs(2));
            loop {
                tokio::select! {
                    _ = sync_t.tick() => {
                        match crate::agent_plugins::sync_messages(&pool, &layout, &agent.machine, &mut have, &mut sched_ver, &mut cfg_ver, &running, &mut upgrade_sent).await {
                            Ok(msgs) => for b in msgs {
                                if tx.send(Ok(pb::ServerMessage { ack_seq: last_seq, body: Some(b) })).await.is_err() { break; }
                            },
                            Err(e) => tracing::warn!(machine=%agent.machine, error=%format!("{e:#}"), "agent plugin sync"),
                        }
                    }
                    _ = req_t.tick() => {
                        if let Ok(rs) = crate::agent_plugins::take_requests(&pool, &agent.machine).await {
                            for r in rs {
                                let _ = tx.send(Ok(pb::ServerMessage { ack_seq: last_seq, body: Some(pb::server_message::Body::RunNow(r)) })).await;
                            }
                        }
                    }
                    _ = stop_rx.changed() => {
                        let _ = tx.send(Ok(pb::ServerMessage { ack_seq: last_seq, body: Some(pb::server_message::Body::Bye(pb::Bye { reason: "revoked or replaced".into() })) })).await;
                        break;
                    }
                    _ = ticker.tick() => {
                        let ping = pb::Ping { nonce: uuid::Uuid::new_v4().to_string(), server_ts_ms: now() };
                        if tx.send(Ok(pb::ServerMessage { ack_seq: last_seq, body: Some(pb::server_message::Body::Ping(ping)) })).await.is_err() { break; }
                    }
                    m = inbound.next() => {
                        match m {
                            Some(Ok(msg)) => {
                                // Queued alerts carry the agent's durable seq: dedupe against the DB, ack each.
                                if let Some(pb::agent_message::Body::Alert(al)) = &msg.body {
                                    match agent_alert(&pool, &agent, msg.seq, al).await {
                                        Ok(()) => { let _ = tx.send(Ok(pb::ServerMessage { ack_seq: msg.seq, body: None })).await; }
                                        Err(e) => tracing::warn!(machine=%agent.machine, error=%e, "agent alert"),
                                    }
                                    continue;
                                }
                                if let Some(pb::agent_message::Body::Run(rep)) = &msg.body {
                                    match crate::agent_plugins::store_report(&pool, &layout, &agent, msg.seq, rep).await {
                                        Ok(_) => { let _ = tx.send(Ok(pb::ServerMessage { ack_seq: msg.seq, body: None })).await; }
                                        Err(e) => tracing::warn!(machine=%agent.machine, error=%format!("{e:#}"), "agent run report"),
                                    }
                                    continue;
                                }
                                if msg.seq <= last_seq { continue; } // replay/duplicate
                                if let Some(pb::agent_message::Body::Applied(ap)) = &msg.body {
                                    last_seq = msg.seq;
                                    if let Err(e) = crate::agent_plugins::applied(&pool, &agent, ap).await {
                                        tracing::warn!(machine=%agent.machine, error=%format!("{e:#}"), "agent applied");
                                    }
                                    continue;
                                }
                                last_seq = msg.seq;
                                if let Some(pb::agent_message::Body::Renew(r)) = &msg.body {
                                    match renew(&pool, &layout, &agent, &r.csr_pem).await {
                                        Ok(rn) => {
                                            renew_sent = true;
                                            let _ = tx.send(Ok(pb::ServerMessage { ack_seq: last_seq, body: Some(pb::server_message::Body::Renewed(rn)) })).await;
                                        }
                                        Err(e) => tracing::warn!(machine=%agent.machine, error=%e, "agent renewal refused"),
                                    }
                                    continue;
                                }
                                if let Some(pb::agent_message::Body::Pong(p)) = msg.body {
                                    if !renew_sent && get(&pool, &agent.id).await.ok().flatten().is_some_and(|a| a.renew_requested) {
                                        renew_sent = true;
                                        let _ = tx.send(Ok(renew_now(last_seq, "requested by an administrator"))).await;
                                    }
                                    // Revoked from another process (itcmdr-cli): end at the next beat.
                                    if get(&pool, &agent.id).await.ok().flatten().is_some_and(|a| a.state == "revoked") {
                                        let _ = tx.send(Ok(pb::ServerMessage { ack_seq: last_seq, body: Some(pb::server_message::Body::Bye(pb::Bye { reason: "revoked".into() })) })).await;
                                        break;
                                    }
                                    let _ = seen(&pool, &agent.id, "").await;
                                    skew_alert(&pool, &agent.machine, now() - p.agent_ts_ms).await;
                                }
                            }
                            _ => break,
                        }
                    }
                }
            }
            {
                let mut live = LIVE.lock();
                if live.get(&agent.id).is_some_and(|(g, _)| *g == gen) {
                    live.remove(&agent.id);
                }
            }
            tracing::info!(machine=%agent.machine, "agent disconnected");
        });
        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        )))
    }
}

/// Clock skew > 5 min in a Pong: Cautious alert; back within: resolved.
async fn skew_alert(pool: &DbPool, machine: &str, skew_ms: i64) {
    let key = format!("agent-clock:{machine}");
    let r = if skew_ms.abs() > 5 * 60_000 {
        alerts::raise(
            pool,
            &Raise {
                key,
                level: "cautious".into(),
                source: "agent".into(),
                machine: machine.into(),
                title: format!("{machine} clock is off by {} s", skew_ms / 1000),
                detail: "Check the PC's time sync.".into(),
                run_id: None,
            },
        )
        .await
        .map(|_| ())
    } else {
        alerts::resolve_key(pool, &key).await.map(|_| ())
    };
    if let Err(e) = r {
        tracing::warn!(error=%e, "agent clock alert failed");
    }
}

/// Serve the agent link on `cfg.agent_bind_addr` until `shutdown`. Errors are logged, never fatal.
pub async fn serve(
    pool: DbPool,
    cfg: ServerConfig,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) {
    if cfg.agent_bind_addr.trim().is_empty() {
        tracing::info!("agent link disabled (agent_bind_addr is empty)");
        return;
    }
    let r = async {
        let addr: std::net::SocketAddr = cfg.agent_bind_addr.parse().context("invalid agent_bind_addr")?;
        let layout = cfg.layout();
        let m = ensure_material(&layout)?;
        tracing::info!(%addr, fingerprint=%m.fingerprint, "starting the agent link (TLS + client certificates)");
        refresh_revoked(&pool).await;
        let tls_cfg = tls::server_config(&m.server, &m.ca.cert, {
            let p = pool.clone();
            std::sync::Arc::new(move |s: &str| serial_revoked(&p, s))
        })?;
        let listener = tokio::net::TcpListener::bind(addr).await.context("bind agent_bind_addr")?;
        let on_fail = std::sync::Arc::new(|peer: std::net::SocketAddr, e: String| {
            tracing::warn!(peer=%peer.ip(), error=%e, "agent link handshake refused");
        });
        tonic::transport::Server::builder()
            .http2_keepalive_interval(Some(Duration::from_secs(30)))
            .add_service(
                pb::agent_service_server::AgentServiceServer::new(AgentLink::new(pool, layout))
                    .max_decoding_message_size(32 * 1024 * 1024)
                    .max_encoding_message_size(128 * 1024 * 1024),
            )
            .serve_with_incoming_shutdown(tls::incoming(listener, tls_cfg, on_fail), shutdown)
            .await?;
        anyhow::Ok(())
    }
    .await;
    if let Err(e) = r {
        tracing::error!(error=%e, "the agent link stopped");
    }
}
