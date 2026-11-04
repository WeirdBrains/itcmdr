#![allow(clippy::result_large_err, clippy::field_reassign_with_default)]
pub mod agent_plugins;
pub mod agents;
pub mod alerts;
pub mod api;
pub mod auth;
pub mod backup;
pub mod calllog;
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
pub mod remote;
pub mod requests;
pub mod restore;
pub mod restore_seal;
pub mod runtime;
pub mod schedule;
pub mod search;
pub mod server_upgrade;
pub mod settings;
pub mod setup;
pub mod sitelink;
pub mod storage;
pub mod update_window;
pub mod upgrades;
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

/// Test hook (VM break tests): while `<data>\health.pause` exists, no health file is written,
/// as if the service hung.
pub const HEALTH_PAUSE: &str = "health.pause";

async fn write_health(pool: &db::DbPool, cfg: &ServerConfig, connected: &mut i64) {
    let root = cfg.layout().root;
    if root.join(HEALTH_PAUSE).exists() {
        return;
    }
    let db_ok = sqlx::query("SELECT 1").execute(pool).await.is_ok();
    if db_ok && restore::grpc_answers(cfg).await {
        *connected = chrono::Utc::now().timestamp_millis();
    }
    let h = itcmdr_watchdog::Health {
        version: env!("CARGO_PKG_VERSION").into(),
        pid: std::process::id(),
        ts_ms: chrono::Utc::now().timestamp_millis(),
        connected_ms: *connected,
    };
    if let Err(e) = itcmdr_watchdog::write_health(&root, &h) {
        tracing::warn!(error=%e, "health file");
    }
}

fn calllog_machine() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "server".into())
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
            if let Err(e) = server_upgrade::tick(&bg_pool, &bg_cfg).await {
                tracing::warn!(error=%e, "server upgrade pass failed");
            }
            if let Err(e) = upgrades::tick(&bg_pool, chrono::Utc::now().timestamp_millis()).await {
                tracing::warn!(error=%e, "agent rollout pass failed");
            }
            if let Err(e) = alerts::expire_silences(&bg_pool).await {
                tracing::warn!(error=%e, "alert silence expiry failed");
            }
            if last_retention.elapsed() > Duration::from_secs(24 * 3600) {
                last_retention = std::time::Instant::now();
                let days =
                    settings::get_i64(&bg_pool, Some(&bg_cfg), "server.retention_days").await;
                if let Err(e) = db::apply_retention(&bg_pool, days).await {
                    tracing::warn!(error=%e, "retention failed");
                }
                let call_days =
                    settings::get_i64(&bg_pool, Some(&bg_cfg), "calllog.retention_days").await;
                calllog::daily(&bg_pool, &bg_cfg.layout(), &calllog_machine(), call_days).await;
            }
        }
    });

    // One shutdown signal for both listeners.
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        shutdown.await;
        let _ = stop_tx.send(true);
    });
    let wait = |mut rx: tokio::sync::watch::Receiver<bool>| async move {
        while !*rx.borrow() {
            if rx.changed().await.is_err() {
                break;
            }
        }
    };
    // v0.17.0: agent link (TLS + client certs) and the heartbeat state machine.
    tokio::spawn(agents::serve(
        pool.clone(),
        cfg.clone(),
        wait(stop_rx.clone()),
    ));
    // v0.19.0: the adminsvr link (only while enrolled and adminsvr.enabled).
    tokio::spawn(sitelink::run(pool.clone(), cfg.clone()));
    // v0.21.0: health.json for itcmdr-server-watchdog (gRPC answers and the DB opens).
    let (h_pool, h_cfg) = (pool.clone(), cfg.clone());
    tokio::spawn(async move {
        let mut connected = 0i64;
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            write_health(&h_pool, &h_cfg, &mut connected).await;
            tokio::time::sleep(Duration::from_secs(20)).await;
        }
    });
    let hb_pool = pool.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if let Err(e) = agents::tick(&hb_pool).await {
                tracing::warn!(error=%e, "agent heartbeat pass failed");
            }
        }
    });

    let addr: std::net::SocketAddr = cfg.bind_addr.parse().context("invalid bind_addr")?;
    // App plugin uploads (v0.13.0): allow one package of plugins.max_package_mib plus framing.
    let max_msg = (settings::get_i64(&pool, None, "plugins.max_package_mib")
        .await
        .max(4) as usize
        + 1)
        * 1024
        * 1024;
    // Call log (v0.15.0): inside gRPC-Web, so every call is logged as plain gRPC.
    let calls = calllog::CallLog::start(pool.clone());
    let svc = tonic_web::enable(
        calls.wrap(
            generated::itcmdr::itcmdr_server::ItcmdrServer::new(api::ItcmdrService::new(
                cfg.clone(),
                pool,
            ))
            .max_decoding_message_size(max_msg),
        ),
    );
    // v1.1.0: HTTPS with the server certificate (the one agents pin); plain HTTP only from this
    // machine or with the explicit `api_plain_http` opt-in.
    let mat = agents::ensure_material(&cfg.layout())?;
    let tls = itcmdr_agentlink::tls::api_server_config(&mat.server)?;
    let plain_any = cfg.api_plain_http;
    if plain_any {
        tracing::info!("API: HTTPS, and plain HTTP (api_plain_http)");
    }
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    tracing::info!(%addr, fingerprint=%mat.fingerprint, "starting itcmdr-server (HTTPS)");
    let incoming = itcmdr_agentlink::tls::incoming_mixed(
        listener,
        tls,
        std::sync::Arc::new(move |p: std::net::SocketAddr| plain_allowed(plain_any, p)),
        std::sync::Arc::new(
            |p: std::net::SocketAddr, e: String| tracing::debug!(peer=%p.ip(), error=%e, "api connection refused"),
        ),
    );
    tonic::transport::Server::builder()
        .accept_http1(true) // gRPC-Web
        .add_service(svc)
        .serve_with_incoming_shutdown(incoming, wait(stop_rx))
        .await?;
    tracing::info!("itcmdr-server stopped");
    Ok(())
}

/// v1.1.0 API policy: HTTPS always; plain HTTP from this machine, or from anywhere with
/// `api_plain_http` (v1.0 apps until they update, or a TLS-terminating proxy).
pub fn plain_allowed(api_plain_http: bool, peer: std::net::SocketAddr) -> bool {
    api_plain_http || peer.ip().is_loopback()
}
