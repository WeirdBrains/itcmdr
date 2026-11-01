//! SiteLink: sites dial in (pinned TLS + mTLS client certificate from the adminsvr CA). adminsvr
//! stores what the site sends (already redacted on the site), pings, sends queued commands and
//! raises a Critical "site unreachable" alert after the missed threshold.
use crate::config::Config;
use crate::material::Material;
use crate::pb;
use crate::store::Pool;
use anyhow::{anyhow, Context, Result};
use parking_lot::{Mutex, RwLock};
use prost::Message;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

static REVOKED: once_cell_lite::Lazy = once_cell_lite::Lazy::new();

mod once_cell_lite {
    use parking_lot::RwLock;
    use std::collections::HashSet;
    use std::sync::OnceLock;
    pub struct Lazy(OnceLock<RwLock<HashSet<String>>>);
    impl Lazy {
        pub const fn new() -> Self {
            Lazy(OnceLock::new())
        }
        pub fn get(&self) -> &RwLock<HashSet<String>> {
            self.0.get_or_init(Default::default)
        }
    }
}

/// Reload the revoked client certificate serials (refused in the TLS handshake).
pub async fn refresh_revoked(pool: &Pool) {
    if let Ok(v) = sqlx::query_scalar::<_, String>(
        "SELECT cert_serial FROM sites WHERE status = 'revoked' AND cert_serial IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    {
        *REVOKED.get().write() = v.into_iter().collect::<HashSet<_>>();
    }
}

type Tx = mpsc::Sender<Result<pb::AdminMessage, Status>>;

#[derive(Clone, Default)]
pub struct Hub {
    inner: Arc<HubInner>,
}

#[derive(Default)]
struct HubInner {
    sessions: RwLock<HashMap<String, (u64, Tx)>>,
    kicks: Mutex<HashMap<String, Arc<Notify>>>,
    next: Mutex<u64>,
}

impl Hub {
    fn notify(&self, site: &str) -> Arc<Notify> {
        self.inner
            .kicks
            .lock()
            .entry(site.to_string())
            .or_default()
            .clone()
    }
    /// Wake a site's session (new commands).
    pub fn kick(&self, site: &str) {
        self.notify(site).notify_one();
    }
    pub fn connected(&self, site: &str) -> bool {
        self.inner.sessions.read().contains_key(site)
    }
    /// Close a site's live stream (revoked).
    pub fn drop_site(&self, site: &str) {
        self.inner.sessions.write().remove(site);
        self.kick(site);
    }
    fn add(&self, site: &str, tx: mpsc::Sender<Result<pb::AdminMessage, Status>>) -> u64 {
        let mut n = self.inner.next.lock();
        *n += 1;
        self.inner
            .sessions
            .write()
            .insert(site.to_string(), (*n, tx));
        *n
    }
    fn remove(&self, site: &str, id: u64) {
        let mut s = self.inner.sessions.write();
        if s.get(site).is_some_and(|(i, _)| *i == id) {
            s.remove(site);
        }
    }
    fn alive(&self, site: &str, id: u64) -> bool {
        self.inner
            .sessions
            .read()
            .get(site)
            .is_some_and(|(i, _)| *i == id)
    }
}

pub struct Link {
    pool: Pool,
    cfg: Config,
    m: Material,
    hub: Hub,
}

async fn take_token(pool: &Pool, secret: &str, burn: bool) -> Result<String> {
    let h = itcmdr_adminlink::sha256_hex(secret.as_bytes());
    let r =
        sqlx::query("SELECT site_id, expires_ms, used_ms FROM site_tokens WHERE secret_hash = $1")
            .bind(&h)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| anyhow!("unknown site token"))?;
    if r.get::<Option<i64>, _>("used_ms").is_some()
        || r.get::<i64, _>("expires_ms") < crate::now_ms()
    {
        anyhow::bail!("site token used or expired");
    }
    if burn {
        let n = sqlx::query(
            "UPDATE site_tokens SET used_ms = $1 WHERE secret_hash = $2 AND used_ms IS NULL",
        )
        .bind(crate::now_ms())
        .bind(&h)
        .execute(pool)
        .await?
        .rows_affected();
        if n == 0 {
            anyhow::bail!("site token used");
        }
    }
    Ok(r.get("site_id"))
}

type Out = Pin<Box<dyn Stream<Item = Result<pb::AdminMessage, Status>> + Send>>;

pub async fn store_alert(pool: &Pool, site: &str, origin: &str, a: &pb::SiteAlert) -> Result<()> {
    sqlx::query("INSERT INTO alerts (site_id, alert_id, origin, state, severity, last_seen_ms, body) VALUES ($1,$2,$3,$4,$5,$6,$7) \
                 ON CONFLICT (site_id, alert_id) DO UPDATE SET state = EXCLUDED.state, severity = EXCLUDED.severity, last_seen_ms = EXCLUDED.last_seen_ms, body = EXCLUDED.body")
        .bind(site).bind(&a.id).bind(origin).bind(&a.state).bind(&a.severity).bind(a.last_seen_ms).bind(a.encode_to_vec())
        .execute(pool).await?;
    Ok(())
}

async fn unreachable_alert(pool: &Pool, site: &str, open: bool) {
    let now = crate::now_ms();
    let a = pb::SiteAlert {
        id: "site-unreachable".into(),
        code: "site-unreachable".into(),
        severity: "critical".into(),
        state: if open { "open" } else { "resolved" }.into(),
        source: "adminsvr".into(),
        first_seen_ms: now,
        last_seen_ms: now,
        count: 1,
        ..Default::default()
    };
    let exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM alerts WHERE site_id = $1 AND alert_id = 'site-unreachable' AND state = 'open'")
        .bind(site).fetch_one(pool).await.unwrap_or(false);
    if open != exists {
        if let Err(e) = store_alert(pool, site, "adminsvr", &a).await {
            tracing::warn!(error=%e, "site unreachable alert");
        }
    }
}

impl Link {
    async fn handle(
        &self,
        site: &str,
        pk: &ed25519_dalek::VerifyingKey,
        msg: &pb::SiteMessage,
    ) -> Result<Option<u64>> {
        itcmdr_adminlink::verify_site(pk, msg)?;
        let last: i64 = sqlx::query_scalar("SELECT last_seq FROM sites WHERE id = $1")
            .bind(site)
            .fetch_one(&self.pool)
            .await?;
        let now = crate::now_ms();
        sqlx::query("UPDATE sites SET last_seen_ms = $1, status = 'online' WHERE id = $2 AND status <> 'revoked'").bind(now).bind(site).execute(&self.pool).await?;
        if (msg.seq as i64) <= last {
            return Ok(Some(msg.seq)); // replay/duplicate: ack again, ignore
        }
        sqlx::query("UPDATE sites SET last_seq = $1 WHERE id = $2")
            .bind(msg.seq as i64)
            .bind(site)
            .execute(&self.pool)
            .await?;
        use pb::site_message::Body;
        match &msg.body {
            Some(Body::Hello(h)) => {
                sqlx::query("UPDATE sites SET version = $1, share_names = $2 WHERE id = $3")
                    .bind(&h.version)
                    .bind(h.share_display_names)
                    .bind(site)
                    .execute(&self.pool)
                    .await?;
            }
            Some(Body::Status(s)) => {
                sqlx::query("UPDATE sites SET last_status = $1, version = $2 WHERE id = $3")
                    .bind(s.encode_to_vec())
                    .bind(&s.version)
                    .bind(site)
                    .execute(&self.pool)
                    .await?;
            }
            Some(Body::Alert(a)) => store_alert(&self.pool, site, "site", a).await?,
            Some(Body::Result(r)) => {
                sqlx::query("UPDATE commands SET state = $1, result_code = $2 WHERE id = $3 AND site_id = $4")
                    .bind(if r.ok { "applied" } else { "failed" }).bind(&r.code).bind(&r.command_id).bind(site).execute(&self.pool).await?;
            }
            Some(Body::Pong(_)) | None => {}
        }
        Ok(Some(msg.seq))
    }
}

#[tonic::async_trait]
impl pb::site_link_server::SiteLink for Link {
    async fn prepare(
        &self,
        req: Request<pb::SitePrepareRequest>,
    ) -> Result<Response<pb::SitePrepareResponse>, Status> {
        take_token(&self.pool, &req.get_ref().token_secret, false)
            .await
            .map_err(|e| Status::permission_denied(e.to_string()))?;
        Ok(Response::new(pb::SitePrepareResponse {
            root_public_key: self.m.root_hex(),
            ca_cert_pem: self.m.ca.cert.clone(),
        }))
    }

    async fn enroll(
        &self,
        req: Request<pb::SiteEnrollRequest>,
    ) -> Result<Response<pb::SiteEnrollResponse>, Status> {
        let r = req.into_inner();
        itcmdr_keys::parse_public_key(&r.site_public_key)
            .map_err(|_| Status::invalid_argument("bad site key"))?;
        let site = take_token(&self.pool, &r.token_secret, true)
            .await
            .map_err(|e| Status::permission_denied(e.to_string()))?;
        let issued = itcmdr_agentlink::tls::sign_csr(
            &self.m.ca,
            &r.csr_pem,
            &site,
            "itcmdr-site",
            90 * 24 * 60,
        )
        .map_err(|e| Status::invalid_argument(e.to_string()))?;
        sqlx::query("UPDATE sites SET site_pub = $1, cert_serial = $2, version = $3, status = 'late', last_seq = 0 WHERE id = $4")
            .bind(&r.site_public_key).bind(&issued.serial).bind(&r.version).bind(&site).execute(&self.pool).await.map_err(|e| Status::internal(e.to_string()))?;
        crate::audit::append(
            &self.pool,
            &crate::audit::Entry {
                actor: format!("site:{site}"),
                actor_key: itcmdr_keys::sha256_hex(r.site_public_key.as_bytes()),
                action: "site.enroll".into(),
                target: site.clone(),
                payload: r.site_public_key.as_bytes().to_vec(),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(pb::SiteEnrollResponse {
            site_id: site,
            client_cert_pem: issued.cert_pem,
            ca_cert_pem: self.m.ca.cert.clone(),
            cert_expires_ms: issued.not_after_ms,
            root_public_key: self.m.root_hex(),
            heartbeat_secs: self.cfg.heartbeat_secs as i32,
        }))
    }

    type LinkStream = Out;

    async fn link(
        &self,
        req: Request<Streaming<pb::SiteMessage>>,
    ) -> Result<Response<Out>, Status> {
        let der = req
            .peer_certs()
            .and_then(|c| c.first().map(|d| d.as_ref().to_vec()))
            .ok_or_else(|| Status::unauthenticated("client certificate required"))?;
        let serial = itcmdr_agentlink::tls::serial_of(&der)
            .map_err(|_| Status::unauthenticated("bad client certificate"))?;
        let row = sqlx::query("SELECT id, site_pub, status FROM sites WHERE cert_serial = $1")
            .bind(&serial)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::unauthenticated("unknown site certificate"))?;
        if row.get::<String, _>("status") == "revoked" {
            return Err(Status::permission_denied("site revoked"));
        }
        let site: String = row.get("id");
        let pk = itcmdr_keys::parse_public_key(
            &row.get::<Option<String>, _>("site_pub").unwrap_or_default(),
        )
        .map_err(|_| Status::internal("site key"))?;
        let mut inbound = req.into_inner();
        let first = inbound
            .message()
            .await?
            .ok_or_else(|| Status::invalid_argument("hello first"))?;
        match &first.body {
            Some(pb::site_message::Body::Hello(h)) if h.site_id == site => {}
            _ => return Err(Status::invalid_argument("hello for this site first")),
        }
        let (tx, rx) = mpsc::channel(64);
        let me = Link {
            pool: self.pool.clone(),
            cfg: self.cfg.clone(),
            m: self.m.clone(),
            hub: self.hub.clone(),
        };
        match me.handle(&site, &pk, &first).await {
            Ok(Some(seq)) => {
                let _ = tx
                    .send(Ok(pb::AdminMessage {
                        ack_seq: seq,
                        body: None,
                    }))
                    .await;
            }
            Ok(None) => {}
            Err(e) => return Err(Status::unauthenticated(e.to_string())),
        }
        unreachable_alert(&self.pool, &site, false).await;
        let sid = self.hub.add(&site, tx.clone());
        tracing::info!(site=%site, "site linked");
        let hub = self.hub.clone();
        let kick = hub.notify(&site);
        let hb = Duration::from_secs(self.cfg.heartbeat_secs.max(1) as u64);
        tokio::spawn(async move {
            let mut ping = tokio::time::interval(hb);
            let mut poll = tokio::time::interval(Duration::from_secs(2));
            loop {
                if !hub.alive(&site, sid) {
                    break;
                }
                tokio::select! {
                    m = inbound.next() => match m {
                        Some(Ok(msg)) => match me.handle(&site, &pk, &msg).await {
                            Ok(Some(seq)) => { if tx.send(Ok(pb::AdminMessage { ack_seq: seq, body: None })).await.is_err() { break; } }
                            Ok(None) => {}
                            Err(e) => { tracing::warn!(site=%site, error=%e, "site message refused"); let _ = tx.send(Err(Status::unauthenticated(e.to_string()))).await; break; }
                        },
                        _ => break,
                    },
                    _ = ping.tick() => {
                        if tx.send(Ok(pb::AdminMessage { ack_seq: 0, body: Some(pb::admin_message::Body::Ping(pb::Ping { ts_ms: crate::now_ms() })) })).await.is_err() { break; }
                    }
                    _ = kick.notified() => { if send_pending(&me.pool, &site, &tx).await.is_err() { break; } }
                    _ = poll.tick() => { if send_pending(&me.pool, &site, &tx).await.is_err() { break; } }
                }
            }
            hub.remove(&site, sid);
            tracing::info!(site=%site, "site link closed");
        });
        Ok(Response::new(
            Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)) as Out,
        ))
    }
}

async fn send_pending(
    pool: &Pool,
    site: &str,
    tx: &mpsc::Sender<Result<pb::AdminMessage, Status>>,
) -> Result<()> {
    let rows = sqlx::query("SELECT id, body FROM commands WHERE site_id = $1 AND state = 'pending' ORDER BY created_ms")
        .bind(site).fetch_all(pool).await?;
    for r in rows {
        let c = pb::Command::decode(r.get::<Vec<u8>, _>("body").as_slice())?;
        let state = if c.expires_ms < crate::now_ms() {
            "expired"
        } else {
            tx.send(Ok(pb::AdminMessage {
                ack_seq: 0,
                body: Some(pb::admin_message::Body::Command(c)),
            }))
            .await
            .map_err(|_| anyhow!("closed"))?;
            "sent"
        };
        sqlx::query("UPDATE commands SET state = $1 WHERE id = $2")
            .bind(state)
            .bind(r.get::<String, _>("id"))
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Late / unreachable states and the Critical "site unreachable" alert.
pub async fn monitor(
    pool: Pool,
    cfg: Config,
    hub: Hub,
    mut stop: tokio::sync::watch::Receiver<bool>,
) {
    let mut t = tokio::time::interval(Duration::from_secs(cfg.heartbeat_secs.clamp(1, 5) as u64));
    loop {
        tokio::select! {
            _ = t.tick() => {}
            _ = stop.changed() => return,
        }
        let hb = cfg.heartbeat_secs * 1000;
        let now = crate::now_ms();
        let rows = match sqlx::query("SELECT id, last_seen_ms, status FROM sites WHERE status IN ('online','late','unreachable')").fetch_all(&pool).await {
            Ok(r) => r,
            Err(_) => continue,
        };
        for r in rows {
            let id: String = r.get("id");
            let age = now - r.get::<i64, _>("last_seen_ms");
            let st = if hub.connected(&id) && age <= 2 * hb {
                "online"
            } else if age > cfg.site_missed * hb {
                "unreachable"
            } else if age > 2 * hb {
                "late"
            } else {
                "online"
            };
            if st != r.get::<String, _>("status") {
                let _ = sqlx::query(
                    "UPDATE sites SET status = $1 WHERE id = $2 AND status <> 'revoked'",
                )
                .bind(st)
                .bind(&id)
                .execute(&pool)
                .await;
            }
            if st == "unreachable" {
                unreachable_alert(&pool, &id, true).await;
            }
        }
    }
}

/// Serve the SiteLink on `cfg.site_addr`.
pub async fn serve(
    pool: Pool,
    cfg: Config,
    m: Material,
    hub: Hub,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let addr: std::net::SocketAddr = cfg.site_addr.parse().context("site_addr")?;
    refresh_revoked(&pool).await;
    let tls_cfg = itcmdr_agentlink::tls::server_config(
        &m.server,
        &m.ca.cert,
        Arc::new(|s: &str| REVOKED.get().read().contains(s)),
    )?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("bind site_addr")?;
    tracing::info!(%addr, "SiteLink listening");
    let on_fail = Arc::new(
        |peer: std::net::SocketAddr, e: String| tracing::warn!(peer=%peer.ip(), error=%e, "SiteLink handshake refused"),
    );
    tonic::transport::Server::builder()
        .http2_keepalive_interval(Some(Duration::from_secs(30)))
        .add_service(pb::site_link_server::SiteLinkServer::new(Link {
            pool,
            cfg,
            m,
            hub,
        }))
        .serve_with_incoming_shutdown(
            itcmdr_agentlink::tls::incoming(listener, tls_cfg, on_fail),
            shutdown,
        )
        .await?;
    Ok(())
}
