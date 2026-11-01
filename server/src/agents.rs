//! Rust agents (v0.17.0, docs/design/SERVER-AGENT.md sections 2 and 4).
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
    Ok(Material {
        ca,
        server,
        fingerprint,
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
    let issued = tls::sign_csr(&m.ca, &r.csr_pem, &id, machine, itcmdr_agentlink::CERT_DAYS)?;
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
    sqlx::query("INSERT INTO agents(id, machine, role, public_key, cert_fp, cert_serial, cert_expires_ms, state, enrolled_ms, version, os, peer) VALUES (?,?,?,?,?,?,?,'enrolled',?,?,?,?)")
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
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
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
    for a in list(pool, false).await? {
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
        let missed = (t - a.last_seen_ms) / interval;
        let key = format!("agent-nonresponsive:{}", a.machine);
        if missed >= lost {
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
        if !a.cert_fp.eq_ignore_ascii_case(&fp) {
            return Err(Status::unauthenticated(
                "the client certificate is not the one issued to this agent",
            ));
        }
        if a.cert_expires_ms < now() {
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
                })),
            }))
            .await;
        let pool = self.pool.clone();
        let agent = a.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(hb as u64));
            ticker.tick().await;
            let mut last_seq = first.seq;
            loop {
                tokio::select! {
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
                                if msg.seq <= last_seq { continue; } // replay/duplicate
                                last_seq = msg.seq;
                                if let Some(pb::agent_message::Body::Pong(p)) = msg.body {
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
        tonic::transport::Server::builder()
            .tls_config(tls::server_tls(&m.server, &m.ca.cert))?
            .http2_keepalive_interval(Some(Duration::from_secs(30)))
            .add_service(pb::agent_service_server::AgentServiceServer::new(AgentLink::new(pool, layout)))
            .serve_with_shutdown(addr, shutdown)
            .await?;
        anyhow::Ok(())
    }
    .await;
    if let Err(e) = r {
        tracing::error!(error=%e, "the agent link stopped");
    }
}
