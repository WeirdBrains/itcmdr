mod config;
mod api;
mod db;
mod metrics;
mod ingest;
mod generated {
    // Prost/tonic generated code lives here after build.rs runs.
    pub mod itcmdr {
        include!("generated/itcmdr.rs");
    }
}

use crate::api::ItcmdrService;
use crate::config::ServerConfig;
use anyhow::Context;
use tonic::transport::Server;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let cfg = ServerConfig::load()?;
    let pool = crate::db::init_db(&cfg).await?;
    crate::db::apply_retention(&pool, cfg.retention_days).await?;
    // Initial ingest (best-effort)
    if let Err(e) = ingest::ingest_all(&pool, &cfg.logs_dir).await {
        tracing::warn!(error=%e, "initial ingest failed");
    }
    let addr = cfg.bind_addr.parse().context("invalid bind_addr")?;

    // Build service and enable gRPC-Web
    let base = crate::generated::itcmdr::itcmdr_server::ItcmdrServer::new(ItcmdrService::new(cfg.clone()));
    let svc = tonic_web::enable(base);

    tracing::info!(%addr, "starting itcmdr-server");
    Server::builder()
        .accept_http1(true) // for gRPC-Web
        .add_service(svc)
        .serve(addr)
        .await?;

    Ok(())
}

