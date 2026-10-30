//! itcmdr service plugin SDK (decisions 6 and 11).
//!
//! A plugin is a separate process that serves `PluginService` over gRPC on loopback:
//!
//! ```no_run
//! use itcmdr_plugin::{proto, Plugin};
//! struct Hello;
//! impl Plugin for Hello {
//!     fn describe(&self) -> proto::DescribeResponse {
//!         itcmdr_plugin::info("hello", "1.0.0", &["verify", "execute"])
//!     }
//!     fn execute(&self, ctx: &proto::RunContext) -> Result<proto::StageResponse, String> {
//!         let log = itcmdr_plugin::log::Logger::for_ctx("hello", "1.0.0", ctx);
//!         log.info("hello", "said hello").map_err(|e| e.to_string())?;
//!         Ok(itcmdr_plugin::ok("done"))
//!     }
//! }
//! fn main() -> anyhow::Result<()> { itcmdr_plugin::serve(Hello) }
//! ```
//!
//! The server side (`host`) launches a plugin, reads the handshake line and calls it.
#![allow(clippy::result_large_err)]
pub mod host;
pub mod log;

pub mod proto {
    tonic::include_proto!("itcmdr.plugin");
}

use proto::plugin_service_server::{PluginService, PluginServiceServer};
use proto::*;
use std::sync::Arc;
use tonic::{Request, Response, Status};

/// Env var with the per-launch secret.
pub const ENV_SECRET: &str = "ITCMDR_PLUGIN_SECRET";
/// Env var with the run's itcmdr.log/v1 file.
pub const ENV_LOG_PATH: &str = "ITCMDR_LOG_PATH";
/// Metadata header that carries the secret on every call.
pub const SECRET_HEADER: &str = "x-itcmdr-plugin-secret";
/// Handshake line prefix: `ITCMDR-PLUGIN|1|tcp|127.0.0.1:<port>`.
pub const HANDSHAKE: &str = "ITCMDR-PLUGIN|1|tcp|";

/// What a plugin implements. Stages are plain (blocking) functions; they run on a blocking thread.
pub trait Plugin: Send + Sync + 'static {
    fn describe(&self) -> DescribeResponse;
    /// Pre-flight check. A failed Verify (Err or ok = false) stops Execute for the run.
    fn verify(&self, _ctx: &RunContext) -> Result<StageResponse, String> {
        Ok(ok("nothing to verify"))
    }
    fn execute(&self, ctx: &RunContext) -> Result<StageResponse, String>;
    /// Called when a stage failed, with the stage name and error.
    fn log(&self, _ctx: &RunContext, _failed_stage: &str, _error: &str) {}
}

pub fn info(name: &str, version: &str, features: &[&str]) -> DescribeResponse {
    DescribeResponse {
        name: name.into(),
        version: version.into(),
        features: features.iter().map(|s| s.to_string()).collect(),
        description: String::new(),
    }
}

pub fn ok(msg: &str) -> StageResponse {
    StageResponse {
        ok: true,
        message: msg.into(),
        results: vec![],
    }
}

pub fn fail(msg: &str) -> StageResponse {
    StageResponse {
        ok: false,
        message: msg.into(),
        results: vec![],
    }
}

struct Svc<P> {
    p: Arc<P>,
    secret: Arc<String>,
}

impl<P: Plugin> Svc<P> {
    fn check<T>(&self, r: &Request<T>) -> Result<(), Status> {
        let got = r
            .metadata()
            .get(SECRET_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if constant_eq(got.as_bytes(), self.secret.as_bytes()) {
            Ok(())
        } else {
            Err(Status::unauthenticated("bad plugin secret"))
        }
    }
    async fn stage(
        &self,
        f: fn(&P, &RunContext) -> Result<StageResponse, String>,
        ctx: RunContext,
    ) -> Result<Response<StageResponse>, Status> {
        let p = self.p.clone();
        let r = tokio::task::spawn_blocking(move || f(&p, &ctx))
            .await
            .map_err(|e| Status::internal(format!("plugin panicked: {e}")))?;
        match r {
            Ok(s) => Ok(Response::new(s)),
            Err(e) => Err(Status::aborted(e)),
        }
    }
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[tonic::async_trait]
impl<P: Plugin> PluginService for Svc<P> {
    async fn describe(
        &self,
        r: Request<DescribeRequest>,
    ) -> Result<Response<DescribeResponse>, Status> {
        self.check(&r)?;
        Ok(Response::new(self.p.describe()))
    }
    async fn verify(&self, r: Request<RunContext>) -> Result<Response<StageResponse>, Status> {
        self.check(&r)?;
        self.stage(P::verify, r.into_inner()).await
    }
    async fn execute(&self, r: Request<RunContext>) -> Result<Response<StageResponse>, Status> {
        self.check(&r)?;
        self.stage(P::execute, r.into_inner()).await
    }
    async fn log(&self, r: Request<LogRequest>) -> Result<Response<LogResponse>, Status> {
        self.check(&r)?;
        let l = r.into_inner();
        let p = self.p.clone();
        let _ = tokio::task::spawn_blocking(move || {
            p.log(&l.ctx.unwrap_or_default(), &l.failed_stage, &l.error)
        })
        .await;
        Ok(Response::new(LogResponse {}))
    }
}

/// Serve `plugin` until the parent closes stdin or kills the process. Needs `ITCMDR_PLUGIN_SECRET`.
pub fn serve<P: Plugin>(plugin: P) -> anyhow::Result<()> {
    let secret = std::env::var(ENV_SECRET).map_err(|_| {
        anyhow::anyhow!("{ENV_SECRET} is not set: plugins are started by itcmdr-server")
    })?;
    tokio::runtime::Runtime::new()?.block_on(serve_async(plugin, secret))
}

/// Async form of [`serve`] (bind, print the handshake, serve). Returns when stdin closes.
pub async fn serve_async<P: Plugin>(plugin: P, secret: String) -> anyhow::Result<()> {
    use std::io::Write;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    {
        let mut out = std::io::stdout().lock();
        writeln!(out, "{HANDSHAKE}{addr}")?;
        out.flush()?;
    }
    // Exit when the parent goes away (stdin EOF).
    let parent_gone = async {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 64];
        let mut stdin = tokio::io::stdin();
        while let Ok(n) = stdin.read(&mut buf).await {
            if n == 0 {
                break;
            }
        }
    };
    serve_on(listener, plugin, secret, parent_gone).await
}

/// Serve on an already bound listener until `shutdown` resolves (used by tests and hosts).
pub async fn serve_on<P: Plugin>(
    listener: tokio::net::TcpListener,
    plugin: P,
    secret: String,
    shutdown: impl std::future::Future<Output = ()>,
) -> anyhow::Result<()> {
    let svc = Svc {
        p: Arc::new(plugin),
        secret: Arc::new(secret),
    };
    tonic::transport::Server::builder()
        .add_service(PluginServiceServer::new(svc))
        .serve_with_incoming_shutdown(
            tokio_stream::wrappers::TcpListenerStream::new(listener),
            shutdown,
        )
        .await?;
    Ok(())
}
