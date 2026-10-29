mod config;
mod api;
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
use tonic_web::GrpcWebLayer;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let cfg = ServerConfig::load()?;
    let addr = cfg.bind_addr.parse().context("invalid bind_addr")?;

    let svc = ItcmdrService::new(cfg.clone()).into_router();

    // Enable gRPC-Web and permissive CORS for simplicity; tighten via config later.
    let cors = CorsLayer::new().allow_headers(Any).allow_methods(Any).allow_origin(Any);
    let router = svc.layer(GrpcWebLayer::new()).layer(cors);

    tracing::info!(%addr, "starting itcmdr-server");
    Server::builder()
        .accept_http1(true) // for gRPC-Web
        .add_service(router)
        .serve(addr)
        .await?;

    Ok(())
}

