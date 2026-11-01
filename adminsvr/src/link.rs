//! SiteLink: sites dial in (pinned TLS + mTLS client certificate from the adminsvr CA). adminsvr
//! stores what the site sends (already redacted on the site), pings, sends queued commands and
//! raises a Critical "site unreachable" alert after the missed threshold.
use crate::config::Config;
use crate::material::Material;
use crate::pb;
use crate::schema::*;
use crate::store::Pool;
use anyhow::{anyhow, Context, Result};
use diesel::prelude::*;
use parking_lot::{Mutex, RwLock};
use prost::Message;
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
    if let Ok(v) = pool
        .run(|c| {
            Ok(sites::table
                .filter(
                    sites::status
                        .eq("revoked")
                        .and(sites::cert_serial.is_not_null()),
                )
                .select(sites::cert_serial.assume_not_null())
                .load::<String>(c)?)
        })
        .await
    {
        *REVOKED.get().write() = v.into_iter().collect::<HashSet<_>>();
    }
}

pub const METRICS_KEEP_MS: i64 = 30 * 24 * 3600 * 1000;

type Tx = mpsc::Sender<Result<pb::AdminMessage, Status>>;

#[derive(Clone, Default)]
pub struct Hub {
    inner: Arc<HubInner>,
    /// v0.19.2 alert pushes (none configured = off)
    pub push: Arc<crate::push::Push>,
}

#[derive(Default)]
struct HubInner {
    sessions: RwLock<HashMap<String, (u64, Tx)>>,
    kicks: Mutex<HashMap<String, Arc<Notify>>>,
    next: Mutex<u64>,
}

impl Hub {
    pub fn with_push(push: Arc<crate::push::Push>) -> Self {
        Hub {
            push,
            ..Default::default()
        }
    }
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
    /// Every connected site (after an unfreeze).
    pub fn kick_all(&self) {
        let sites: Vec<String> = self.inner.sessions.read().keys().cloned().collect();
        for s in sites {
            self.kick(&s);
        }
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
    pool.run(move |c| {
        let r: (String, i64, Option<i64>) = site_tokens::table
            .filter(site_tokens::secret_hash.eq(&h))
            .select((
                site_tokens::site_id,
                site_tokens::expires_ms,
                site_tokens::used_ms,
            ))
            .first(c)
            .optional()?
            .ok_or_else(|| anyhow!("unknown site token"))?;
        if r.2.is_some() || r.1 < crate::now_ms() {
            anyhow::bail!("site token used or expired");
        }
        if burn {
            let n = diesel::update(
                site_tokens::table.filter(
                    site_tokens::secret_hash
                        .eq(&h)
                        .and(site_tokens::used_ms.is_null()),
                ),
            )
            .set(site_tokens::used_ms.eq(crate::now_ms()))
            .execute(c)?;
            if n == 0 {
                anyhow::bail!("site token used");
            }
        }
        Ok(r.0)
    })
    .await
}

type Out = Pin<Box<dyn Stream<Item = Result<pb::AdminMessage, Status>> + Send>>;

/// Store an alert; true when it is newly open (new, or reopened).
pub async fn store_alert(pool: &Pool, site: &str, origin: &str, a: &pb::SiteAlert) -> Result<bool> {
    let (site, origin, a) = (site.to_string(), origin.to_string(), a.clone());
    pool.run(move |c| {
        let key2 = alerts::table.filter(alerts::site_id.eq(&site).and(alerts::alert_id.eq(&a.id)));
        let before: Option<String> = key2.select(alerts::state).first(c).optional()?;
        let opened = a.state == "open"
            && before.as_deref() != Some("open")
            && before.as_deref() != Some("acked")
            && before.as_deref() != Some("silenced");
        let key = alerts::table.filter(alerts::site_id.eq(&site).and(alerts::alert_id.eq(&a.id)));
        let n = diesel::update(key)
            .set((
                alerts::state.eq(&a.state),
                alerts::severity.eq(&a.severity),
                alerts::last_seen_ms.eq(a.last_seen_ms),
                alerts::body.eq(a.encode_to_vec()),
            ))
            .execute(c)?;
        if n == 0 {
            diesel::insert_into(alerts::table)
                .values((
                    alerts::site_id.eq(&site),
                    alerts::alert_id.eq(&a.id),
                    alerts::origin.eq(&origin),
                    alerts::state.eq(&a.state),
                    alerts::severity.eq(&a.severity),
                    alerts::last_seen_ms.eq(a.last_seen_ms),
                    alerts::body.eq(a.encode_to_vec()),
                ))
                .execute(c)?;
        }
        Ok(opened)
    })
    .await
}

/// Store an alert and push it when it is newly open.
pub async fn store_and_push(
    pool: &Pool,
    hub: &Hub,
    site: &str,
    origin: &str,
    a: &pb::SiteAlert,
) -> Result<()> {
    if store_alert(pool, site, origin, a).await? {
        crate::push::alert_opened(
            hub.push.clone(),
            pool.clone(),
            site.to_string(),
            a.id.clone(),
            a.severity.clone(),
        );
    }
    Ok(())
}

async fn unreachable_alert(pool: &Pool, hub: &Hub, site: &str, open: bool) {
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
    let s = site.to_string();
    let exists = pool
        .run(move |c| {
            let n: i64 = alerts::table
                .filter(
                    alerts::site_id
                        .eq(s)
                        .and(alerts::alert_id.eq("site-unreachable"))
                        .and(alerts::state.eq("open")),
                )
                .count()
                .get_result(c)?;
            Ok(n > 0)
        })
        .await
        .unwrap_or(false);
    if open != exists {
        if let Err(e) = store_and_push(pool, hub, site, "adminsvr", &a).await {
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
        let (site_id, seq) = (site, msg.seq);
        let (site, msg) = (site.to_string(), msg.clone());
        let pool = self.pool.clone();
        let alert = pool
            .run(move |c| {
                c.transaction::<_, anyhow::Error, _>(|c| {
                    let me = sites::table.filter(sites::id.eq(&site));
                    let last: i64 = me.select(sites::last_seq).first(c)?;
                    diesel::update(
                        sites::table.filter(sites::id.eq(&site).and(sites::status.ne("revoked"))),
                    )
                    .set((
                        sites::last_seen_ms.eq(crate::now_ms()),
                        sites::status.eq("online"),
                    ))
                    .execute(c)?;
                    if (msg.seq as i64) <= last {
                        return Ok(None); // replay/duplicate: ack again, ignore
                    }
                    diesel::update(me)
                        .set(sites::last_seq.eq(msg.seq as i64))
                        .execute(c)?;
                    use pb::site_message::Body;
                    match &msg.body {
                        Some(Body::Hello(h)) => {
                            diesel::update(me)
                                .set((
                                    sites::version.eq(&h.version),
                                    sites::share_names.eq(h.share_display_names),
                                ))
                                .execute(c)?;
                        }
                        Some(Body::Status(st)) => {
                            diesel::update(me)
                                .set((
                                    sites::last_status.eq(Some(st.encode_to_vec())),
                                    sites::version.eq(&st.version),
                                ))
                                .execute(c)?;
                            // v0.19.2 history for the app's metrics view (30 days)
                            let now = crate::now_ms();
                            diesel::delete(
                                site_metrics::table.filter(
                                    site_metrics::site_id
                                        .eq(&site)
                                        .and(site_metrics::ts_ms.lt(now - METRICS_KEEP_MS)),
                                ),
                            )
                            .execute(c)?;
                            let ts = msg.ts_ms.max(1);
                            let have: i64 = site_metrics::table
                                .filter(
                                    site_metrics::site_id
                                        .eq(&site)
                                        .and(site_metrics::ts_ms.eq(ts)),
                                )
                                .count()
                                .get_result(c)?;
                            if have == 0 {
                                diesel::insert_into(site_metrics::table)
                                    .values((
                                        site_metrics::site_id.eq(&site),
                                        site_metrics::ts_ms.eq(ts),
                                        site_metrics::body.eq(st.encode_to_vec()),
                                    ))
                                    .execute(c)?;
                            }
                        }
                        Some(Body::Alert(a)) => return Ok(Some(a.clone())),
                        Some(Body::Result(r)) => {
                            let this = commands::table.filter(
                                commands::id
                                    .eq(&r.command_id)
                                    .and(commands::site_id.eq(&site)),
                            );
                            let dry: Option<bool> =
                                this.select(commands::dry_run).first(c).optional()?;
                            let state = match (dry.unwrap_or(false), r.ok) {
                                (true, true) => "previewed",
                                (false, true) => "applied",
                                _ => "failed",
                            };
                            let preview = pb::CommandResult {
                                preview: r.preview.iter().take(200).cloned().collect(),
                                ..Default::default()
                            }
                            .encode_to_vec();
                            diesel::update(this)
                                .set((
                                    commands::state.eq(state),
                                    commands::result_code.eq(&r.code),
                                    commands::preview.eq(Some(preview)),
                                ))
                                .execute(c)?;
                        }
                        Some(Body::Config(d)) => {
                            diesel::update(me)
                                .set((
                                    sites::config.eq(Some(d.encode_to_vec())),
                                    sites::config_ms.eq(crate::now_ms()),
                                ))
                                .execute(c)?;
                        }
                        Some(Body::Plugins(d)) => {
                            diesel::update(me)
                                .set((
                                    sites::plugins.eq(Some(d.encode_to_vec())),
                                    sites::plugins_ms.eq(crate::now_ms()),
                                ))
                                .execute(c)?;
                        }
                        Some(Body::Pong(_)) | None => {}
                    }
                    Ok(None)
                })
            })
            .await?;
        if let Some(a) = alert {
            store_and_push(&self.pool, &self.hub, site_id, "site", &a).await?;
        }
        Ok(Some(seq))
    }
}

#[tonic::async_trait]
impl pb::site_link_server::SiteLink for Link {
    type FetchPackageStream = Pin<Box<dyn Stream<Item = Result<pb::PackageChunk, Status>> + Send>>;

    /// v0.20.0: a site fetches a catalog package for a plugin.install command it was sent.
    async fn fetch_package(
        &self,
        req: Request<pb::PackageRequest>,
    ) -> Result<Response<Self::FetchPackageStream>, Status> {
        let r = req.into_inner();
        if (r.ts_ms - crate::now_ms()).abs() > self.cfg.max_skew_secs * 1000 {
            return Err(Status::unauthenticated("request time out of range"));
        }
        let (sid, sha) = (r.site_id.clone(), r.sha256.to_lowercase());
        let found: Option<(Option<String>, String)> = self
            .pool
            .run(move |c| {
                Ok(sites::table
                    .filter(sites::id.eq(sid))
                    .select((sites::site_pub, sites::status))
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let Some((Some(pk), status)) = found else {
            return Err(Status::unauthenticated("unknown site"));
        };
        if status == "revoked" {
            return Err(Status::permission_denied("site revoked"));
        }
        let pk = itcmdr_keys::parse_public_key(&pk).map_err(|_| Status::internal("site key"))?;
        let sig: [u8; 64] = hex::decode(&r.sig)
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| Status::unauthenticated("bad signature"))?;
        use ed25519_dalek::Verifier;
        pk.verify(
            &itcmdr_adminlink::fetch_bytes(&r.site_id, &r.sha256, r.ts_ms),
            &ed25519_dalek::Signature::from_bytes(&sig),
        )
        .map_err(|_| Status::unauthenticated("bad signature"))?;
        // Only a package named by a plugin.install command for this site that was sent to it.
        let (sid, sh) = (r.site_id.clone(), sha.clone());
        let body: Option<Vec<u8>> = self
            .pool
            .run(move |c| {
                let cmds: Vec<Vec<u8>> = commands::table
                    .filter(
                        commands::site_id
                            .eq(&sid)
                            .and(commands::kind.eq("plugin.install"))
                            .and(commands::state.eq("sent")),
                    )
                    .select(commands::body)
                    .load(c)?;
                let named = cmds.iter().any(|b| {
                    pb::Command::decode(b.as_slice())
                        .is_ok_and(|c| c.args.get("sha256").is_some_and(|s| s.to_lowercase() == sh))
                });
                if !named {
                    return Ok(None);
                }
                Ok(catalog::table
                    .filter(catalog::sha256.eq(&sh))
                    .select(catalog::body)
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let body =
            body.ok_or_else(|| Status::permission_denied("no such package for this site"))?;
        let chunks: Vec<Result<pb::PackageChunk, Status>> = body
            .chunks(1024 * 1024)
            .map(|b| Ok(pb::PackageChunk { data: b.to_vec() }))
            .collect();
        Ok(Response::new(Box::pin(tokio_stream::iter(chunks))))
    }

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
        let (pk, ser, ver, sid) = (
            r.site_public_key.clone(),
            issued.serial.clone(),
            r.version.clone(),
            site.clone(),
        );
        self.pool
            .run(move |c| {
                Ok(diesel::update(sites::table.filter(sites::id.eq(sid)))
                    .set((
                        sites::site_pub.eq(Some(pk)),
                        sites::cert_serial.eq(Some(ser)),
                        sites::version.eq(ver),
                        sites::status.eq("late"),
                        sites::last_seq.eq(0i64),
                    ))
                    .execute(c)?)
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
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
        let ser = serial.clone();
        let row: (String, Option<String>, String) = self
            .pool
            .run(move |c| {
                Ok(sites::table
                    .filter(sites::cert_serial.eq(ser))
                    .select((sites::id, sites::site_pub, sites::status))
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::unauthenticated("unknown site certificate"))?;
        if row.2 == "revoked" {
            return Err(Status::permission_denied("site revoked"));
        }
        let site: String = row.0;
        let pk = itcmdr_keys::parse_public_key(&row.1.unwrap_or_default())
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
        unreachable_alert(&self.pool, &self.hub, &site, false).await;
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
    if frozen(pool).await? {
        return Ok(()); // global freeze: queued commands stay pending
    }
    let s = site.to_string();
    let rows: Vec<(String, Vec<u8>)> = pool
        .run(move |c| {
            Ok(commands::table
                .filter(commands::site_id.eq(s).and(commands::state.eq("pending")))
                .order(commands::created_ms)
                .select((commands::id, commands::body))
                .load(c)?)
        })
        .await?;
    for (id, body) in rows {
        let c = pb::Command::decode(body.as_slice())?;
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
        pool.run(move |c| {
            Ok(diesel::update(commands::table.filter(commands::id.eq(id)))
                .set(commands::state.eq(state))
                .execute(c)?)
        })
        .await?;
    }
    Ok(())
}

/// The global freeze (owners): commands are refused and nothing queued is sent.
pub async fn frozen(pool: &Pool) -> Result<bool> {
    pool.run(|c| {
        Ok(meta::table
            .filter(meta::k.eq("freeze"))
            .select(meta::v)
            .first::<String>(c)
            .optional()?
            .is_some_and(|v| v.starts_with('1')))
    })
    .await
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
        let rows: Vec<(String, i64, String)> = match pool
            .run(|c| {
                Ok(sites::table
                    .filter(sites::status.eq_any(["online", "late", "unreachable"]))
                    .select((sites::id, sites::last_seen_ms, sites::status))
                    .load(c)?)
            })
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };
        for (id, seen, cur) in rows {
            let age = now - seen;
            let st = if hub.connected(&id) && age <= 2 * hb {
                "online"
            } else if age > cfg.site_missed * hb {
                "unreachable"
            } else if age > 2 * hb {
                "late"
            } else {
                "online"
            };
            if st != cur {
                let i = id.clone();
                let _ = pool
                    .run(move |c| {
                        Ok(diesel::update(
                            sites::table.filter(sites::id.eq(i).and(sites::status.ne("revoked"))),
                        )
                        .set(sites::status.eq(st))
                        .execute(c)?)
                    })
                    .await;
            }
            if st == "unreachable" {
                unreachable_alert(&pool, &hub, &id, true).await;
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
