//! v0.19.0 adminsvr: the offsite admin server (docs/design/ADMINSVR.md).
//! - Postgres store (`store`), signed admin API (`api`, same request signing as the site),
//!   owner/admin/sre RBAC (`rbac`), hash-chained audit of every signed change (`audit`).
//! - SiteLink (`link`): sites dial in over pinned TLS + mTLS and send redacted status and alerts;
//!   adminsvr sends commands signed by the user and the adminsvr root key.
//! Backups never come here: adminsvr only sees backup age/size/result and may ask a site to start
//! one; there is no restore.
pub mod api;
pub mod audit;
pub mod auth;
pub mod config;
pub mod link;
pub mod material;
pub mod rbac;
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
    let hub = link::Hub::default();
    let (tx, rx) = tokio::sync::watch::channel(false);
    let api = api::serve(pool.clone(), cfg.clone(), m.clone(), hub.clone(), {
        let mut r = rx.clone();
        async move {
            let _ = r.changed().await;
        }
    });
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
