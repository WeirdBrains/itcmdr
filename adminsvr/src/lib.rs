//! v0.19.0 adminsvr: the offsite admin server (docs/historical/1.0/design/ADMINSVR.md).
//! - Postgres store (`store`), signed admin API (`api`, same request signing as the site),
//!   owner/admin/sre RBAC (`rbac`), hash-chained audit of every signed change (`audit`).
//! - SiteLink (`link`): sites dial in over pinned TLS + mTLS and send redacted status and alerts;
//!   adminsvr sends commands signed by the user and the adminsvr root key.
//!
//! Backups never come here: adminsvr only sees backup age/size/result and may ask a site to start
//! one; there is no restore.
// tonic::Status is the error type of every handler.
#![allow(clippy::result_large_err)]
pub mod acme;
pub mod api;
pub mod audit;
pub mod auth;
pub mod config;
pub mod link;
pub mod material;
pub mod passkeys;
pub mod push;
pub mod rbac;
pub mod schema;
pub mod store;

pub use itcmdr_adminlink::pb;

pub fn now_ms() -> i64 {
    itcmdr_keys::now_ms()
}

/// Run both listeners until `shutdown`.
pub async fn run(
    cfg: config::Config,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let pool = store::connect(&cfg.db_url).await?;
    store::migrate(&pool).await?;
    let m = material::load(&cfg)?;
    let hub = link::Hub::with_push(push::Push::new(&cfg.push)?);
    let (tx, rx) = tokio::sync::watch::channel(false);
    // v1.1.0: the admin API's certificate resolver (internal cert, plus ACME when configured)
    let internal = itcmdr_agentlink::tls::certified_key(&m.server.cert, &m.server.key)?;
    let domains = cfg
        .acme
        .as_ref()
        .map(|a| a.domains.clone())
        .unwrap_or_default();
    let resolver = acme::Resolver::new(internal, &domains);
    if let Some(a) = cfg.acme.clone() {
        a.validate().map_err(|e| anyhow::anyhow!(e))?;
        let tokens: acme::Tokens = Default::default();
        if a.challenge == config::Challenge::Http01 {
            let (addr, t) = (a.http_addr.clone(), tokens.clone());
            tokio::spawn(async move {
                if let Err(e) = acme::serve_http01(addr, t).await {
                    tracing::error!(error=%e, "ACME HTTP-01 listener");
                }
            });
        }
        tokio::spawn(acme::run(
            pool.clone(),
            hub.clone(),
            a,
            resolver.clone(),
            tokens,
            rx.clone(),
        ));
    }
    let api = api::serve(
        pool.clone(),
        cfg.clone(),
        m.clone(),
        hub.clone(),
        resolver,
        {
            let mut r = rx.clone();
            async move {
                let _ = r.changed().await;
            }
        },
    );
    let sl = link::serve(pool.clone(), cfg.clone(), m.clone(), hub.clone(), {
        let mut r = rx.clone();
        async move {
            let _ = r.changed().await;
        }
    });
    let mon = link::monitor(pool.clone(), cfg.clone(), hub.clone(), rx.clone());
    tokio::spawn(async move {
        shutdown.await;
        let _ = tx.send(true);
    });
    let (a, b, _) = tokio::join!(api, sl, mon);
    a?;
    b?;
    Ok(())
}
