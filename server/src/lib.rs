#![allow(clippy::result_large_err, clippy::field_reassign_with_default)]
pub mod api;
pub mod auth;
pub mod backup;
pub mod config;
pub mod cutover;
pub mod db;
pub mod events;
pub mod exceptions;
pub mod ingest;
pub mod jobs;
pub mod keys;
pub mod metrics;
pub mod packages;
pub mod plugin_logs;
pub mod requests;
pub mod restore;
pub mod restore_seal;
pub mod runtime;
pub mod schedule;
pub mod search;
pub mod settings;
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
    match plugin_logs::ingest(pool, &cfg.layout().plugin_logs()).await {
        Ok(s) if s.rejects > 0 => tracing::warn!(
            lines = s.lines,
            rejects = s.rejects,
            "plugin log lines rejected"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!(error=%e, "plugin log ingest failed"),
    }
    let timeout_ms = settings::get_i64(pool, Some(cfg), "server.request_timeout_secs").await * 1000;
    if let Err(e) = requests::sync_requests(pool, &cfg.responses_dir(), timeout_ms).await {
        tracing::warn!(error=%e, "request sync failed");
    }
}

/// Run the gRPC server and the background ingest loop until `shutdown` resolves.
pub async fn run(
    mut cfg: ServerConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    events::init();
    let pool = db::init_db(&cfg).await?;
    auth::settle_auth_mode(&pool, &mut cfg).await?;
    tracing::info!(auth_mode = %cfg.auth_mode, source = %cfg.auth_mode_source, "auth mode");
    if cfg.auth_mode_source == "kept-both" {
        tracing::warn!(
            "auth_mode is not set in server.json and this install predates v0.12.0: staying on \"both\" \
             (shared token still accepted). Register the app's key, then run `itcmdr-cli auth set-mode signed`"
        );
    }
    db::apply_retention(
        &pool,
        settings::get_i64(&pool, Some(&cfg), "server.retention_days").await,
    )
    .await?;
    // Built-in default schedules (decision 14, v0.11.0): idempotent on every start/upgrade.
    if let Err(e) = cutover::seed_defaults(&pool, &cfg).await {
        tracing::warn!(error=%e, "seeding the default schedules failed");
    }
    if let Err(e) = settings::export_agent(&pool, &cfg).await {
        tracing::warn!(error=%e, "agent settings export failed");
    }
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
        let mut last_retention = std::time::Instant::now();
        let mut first = true;
        loop {
            if !first {
                // Re-read every pass so a changed interval applies without a restart.
                let secs =
                    settings::get_i64(&bg_pool, Some(&bg_cfg), "server.ingest_interval_secs").await;
                tokio::time::sleep(Duration::from_secs(secs.max(5) as u64)).await;
            }
            first = false;
            tick(&bg_pool, &bg_cfg).await;
            if let Err(e) = schedule::tick(&bg_pool, &bg_cfg, chrono::Utc::now()).await {
                tracing::warn!(error=%e, "scheduler failed");
            }
            if let Err(e) = cutover::export_state(&bg_pool, &bg_cfg).await {
                tracing::warn!(error=%e, "schedule state export failed");
            }
            if last_retention.elapsed() > Duration::from_secs(24 * 3600) {
                last_retention = std::time::Instant::now();
                let days =
                    settings::get_i64(&bg_pool, Some(&bg_cfg), "server.retention_days").await;
                if let Err(e) = db::apply_retention(&bg_pool, days).await {
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
