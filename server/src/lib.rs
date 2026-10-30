#![allow(clippy::result_large_err, clippy::field_reassign_with_default)]
pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod events;
pub mod exceptions;
pub mod ingest;
pub mod keys;
pub mod metrics;
pub mod requests;
pub mod search;
pub mod storage;
pub mod generated {
    pub mod itcmdr {
        include!("generated/itcmdr.rs");
    }
}

use anyhow::Context;
use config::ServerConfig;
use std::future::Future;
use std::time::Duration;

/// One maintenance pass: ingest new/changed logs and fold on-demand responses in.
pub async fn tick(pool: &db::DbPool, cfg: &ServerConfig) {
    match ingest::ingest_all(pool, &cfg.logs_dir).await {
        Ok(s) if s.files > 0 => tracing::info!(
            files = s.files,
            runs = s.new_runs,
            checks = s.new_checks,
            "ingested"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!(error=%e, "ingest failed"),
    }
    if let Err(e) =
        requests::sync_requests(pool, &cfg.responses_dir(), cfg.request_timeout_ms()).await
    {
        tracing::warn!(error=%e, "request sync failed");
    }
}

/// Run the gRPC server and the background ingest loop until `shutdown` resolves.
pub async fn run(
    cfg: ServerConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    events::init();
    let pool = db::init_db(&cfg).await?;
    db::apply_retention(&pool, cfg.retention_days).await?;
    if cfg.token_hash().is_none() {
        if cfg.allow_insecure_no_auth {
            tracing::warn!(
                "allow_insecure_no_auth=true: mutating calls are accepted WITHOUT a token"
            );
        } else {
            tracing::warn!(
                "no admin_token_hash configured: on-demand runs and exception changes are disabled"
            );
        }
    }
    if let Err(e) = exceptions::export(&pool, &cfg.exceptions_export_path()).await {
        tracing::warn!(error=%e, "initial exceptions export failed");
    }

    let (bg_pool, bg_cfg) = (pool.clone(), cfg.clone());
    tokio::spawn(async move {
        let mut every =
            tokio::time::interval(Duration::from_secs(bg_cfg.ingest_interval_secs.max(5)));
        let mut last_retention = std::time::Instant::now();
        loop {
            every.tick().await;
            tick(&bg_pool, &bg_cfg).await;
            if last_retention.elapsed() > Duration::from_secs(24 * 3600) {
                last_retention = std::time::Instant::now();
                if let Err(e) = db::apply_retention(&bg_pool, bg_cfg.retention_days).await {
                    tracing::warn!(error=%e, "retention failed");
                }
            }
        }
    });

    let addr = cfg.bind_addr.parse().context("invalid bind_addr")?;
    let svc = tonic_web::enable(generated::itcmdr::itcmdr_server::ItcmdrServer::new(
        api::ItcmdrService::new(cfg.clone(), pool),
    ));
    tracing::info!(%addr, "starting itcmdr-server");
    tonic::transport::Server::builder()
        .accept_http1(true) // gRPC-Web
        .add_service(svc)
        .serve_with_shutdown(addr, shutdown)
        .await?;
    tracing::info!("itcmdr-server stopped");
    Ok(())
}
