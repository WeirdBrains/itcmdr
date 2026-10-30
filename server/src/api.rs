use crate::config::ServerConfig;
use crate::db::DbPool;
use crate::exceptions::ExceptionDoc;
use crate::generated::itcmdr::itcmdr_server::Itcmdr;
use crate::generated::itcmdr::*;
use sha2::{Digest, Sha256};
use std::pin::Pin;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct ItcmdrService {
    cfg: ServerConfig,
    pool: DbPool,
}

pub fn sha256_hex(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

/// Decide whether a mutating call may proceed. Returns the actor name for the audit log.
/// No token hash configured => refused, unless `allow_insecure_no_auth` is set.
pub fn authorize(cfg: &ServerConfig, meta: &tonic::metadata::MetadataMap) -> Result<String, Status> {
    let actor = meta
        .get("x-itcmdr-actor")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').take(64).collect::<String>())
        .filter(|s| !s.trim().is_empty());
    match cfg.token_hash() {
        Some(expected) => {
            let auth = meta.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
            let token = auth.strip_prefix("Bearer ").unwrap_or("").trim();
            if token.is_empty() {
                return Err(Status::unauthenticated("missing token: set the admin token in the app's Settings"));
            }
            if sha256_hex(token) != expected {
                return Err(Status::unauthenticated("invalid token"));
            }
            Ok(actor.unwrap_or_else(|| "admin".into()))
        }
        None if cfg.allow_insecure_no_auth => Ok(format!("{} (insecure)", actor.unwrap_or_else(|| "anonymous".into()))),
        None => Err(Status::failed_precondition(
            "server has no admin_token_hash configured; mutating calls are disabled. Set admin_token_hash in server.json \
             (itcmdr-server hash-token <token> prints it) or set allow_insecure_no_auth=true for a lab setup",
        )),
    }
}

impl ItcmdrService {
    pub fn new(cfg: ServerConfig, pool: DbPool) -> Self {
        Self { cfg, pool }
    }

    async fn audit(&self, actor: &str, action: &str, detail: &serde_json::Value) {
        if let Err(e) = crate::db::insert_audit(&self.pool, actor, action, &detail.to_string()).await {
            tracing::error!(error=%e, action, "failed to write audit log");
        }
    }

    async fn export_exceptions(&self) {
        let path = self.cfg.exceptions_export_path();
        if let Err(e) = crate::exceptions::export(&self.pool, &path).await {
            tracing::error!(error=%e, path=%path.display(), "failed to export exceptions");
        }
    }
}

#[tonic::async_trait]
impl Itcmdr for ItcmdrService {
    async fn list_machines(&self, _req: Request<ListMachinesRequest>) -> Result<Response<ListMachinesResponse>, Status> {
        let items = crate::db::list_machines_overview(&self.pool).await.map_err(internal_err)?;
        let machines = items
            .into_iter()
            .map(|(name, status, open_incidents, last_run)| Machine { name, status, open_incidents: open_incidents as i32, last_run_ts: last_run })
            .collect();
        Ok(Response::new(ListMachinesResponse { machines }))
    }

    async fn trigger_runs(&self, req: Request<TriggerRunsRequest>) -> Result<Response<TriggerRunsResponse>, Status> {
        let actor = authorize(&self.cfg, req.metadata())?;
        let id = uuid::Uuid::new_v4().to_string();
        let body = req.into_inner();
        let options = serde_json::json!({
            "requestId": id,
            "machines": body.machines,
            "reportOnly": body.report_only,
            "repair": body.repair,
            "force": body.force,
            "categories": body.categories
        });
        crate::db::insert_request(&self.pool, &id, &options.to_string()).await.map_err(internal_err)?;
        let dir = self.cfg.requests_dir();
        std::fs::create_dir_all(&dir).map_err(internal_err)?;
        std::fs::write(dir.join(format!("{id}.json")), options.to_string()).map_err(internal_err)?;
        self.audit(&actor, "trigger-runs", &options).await;
        Ok(Response::new(TriggerRunsResponse { request_id: id }))
    }

    async fn get_request_status(&self, req: Request<GetRequestStatusRequest>) -> Result<Response<GetRequestStatusResponse>, Status> {
        let id = req.into_inner().request_id;
        // Fold in any new responses before answering.
        let _ = crate::requests::sync_requests(&self.pool, &self.cfg.responses_dir()).await;
        let (status, message) = crate::db::get_request_status(&self.pool, &id).await.map_err(internal_err)?;
        let mapped = match status.as_str() {
            "queued" => request_status::Status::Queued,
            "running" => request_status::Status::Running,
            "done" => request_status::Status::Done,
            "failed" => request_status::Status::Failed,
            _ => request_status::Status::Unknown,
        } as i32;
        Ok(Response::new(GetRequestStatusResponse { status: mapped, message }))
    }

    async fn get_metrics(&self, req: Request<GetMetricsRequest>) -> Result<Response<GetMetricsResponse>, Status> {
        let p = req.into_inner();
        let pts = crate::metrics::get_metrics(&self.pool, p.window_days as i64, p.rollup()).await.map_err(internal_err)?;
        Ok(Response::new(GetMetricsResponse { points: pts }))
    }

    async fn list_exceptions(&self, req: Request<ListExceptionsRequest>) -> Result<Response<ListExceptionsResponse>, Status> {
        let inc = req.into_inner().include_inactive;
        let rules = crate::exceptions::load(&self.pool, inc).await.map_err(internal_err)?.into_iter().map(|(d, a)| d.to_proto(a)).collect();
        Ok(Response::new(ListExceptionsResponse { rules }))
    }

    async fn save_exception(&self, req: Request<SaveExceptionRequest>) -> Result<Response<SaveExceptionResponse>, Status> {
        let actor = authorize(&self.cfg, req.metadata())?;
        let rule = req.into_inner().rule.unwrap_or_default();
        let mut doc = ExceptionDoc::from_proto(&rule);
        doc.validate().map_err(Status::invalid_argument)?;
        let is_new = doc.id.is_empty();
        if is_new {
            doc.id = uuid::Uuid::new_v4().to_string();
        }
        let json = serde_json::to_value(&doc).map_err(internal_err)?;
        crate::db::save_exception(&self.pool, &doc.id, &json.to_string(), true).await.map_err(internal_err)?;
        self.audit(&actor, if is_new { "exception-add" } else { "exception-edit" }, &json).await;
        self.export_exceptions().await;
        Ok(Response::new(SaveExceptionResponse { rule: Some(doc.to_proto(true)) }))
    }

    async fn expire_exception(&self, req: Request<ExpireExceptionRequest>) -> Result<Response<ExpireExceptionResponse>, Status> {
        let actor = authorize(&self.cfg, req.metadata())?;
        let id = req.into_inner().id;
        if !crate::db::expire_exception(&self.pool, &id).await.map_err(internal_err)? {
            return Err(Status::not_found("no exception with that id"));
        }
        self.audit(&actor, "exception-expire", &serde_json::json!({ "id": id })).await;
        self.export_exceptions().await;
        Ok(Response::new(ExpireExceptionResponse {}))
    }

    async fn search_logs(&self, req: Request<SearchLogsRequest>) -> Result<Response<SearchLogsResponse>, Status> {
        let p = req.into_inner();
        let lines = crate::ingest::iter_log_lines(&self.cfg.logs_dir).map_err(internal_err)?;
        let page = crate::search::search(lines, &p);
        Ok(Response::new(SearchLogsResponse { lines: page.lines, total: page.total as i32, next_offset: page.next_offset as i32 }))
    }

    async fn list_audit(&self, req: Request<ListAuditRequest>) -> Result<Response<ListAuditResponse>, Status> {
        let limit = match req.into_inner().limit {
            n if n <= 0 => 200,
            n => n.min(5000) as i64,
        };
        let entries = crate::db::list_audit(&self.pool, limit)
            .await
            .map_err(internal_err)?
            .into_iter()
            .map(|(ts, actor, action, detail)| AuditEntry { ts, actor, action, detail })
            .collect();
        Ok(Response::new(ListAuditResponse { entries }))
    }

    type WatchUpdatesStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<Update, Status>> + Send + 'static>>;
    async fn watch_updates(&self, _req: Request<WatchUpdatesRequest>) -> Result<Response<Self::WatchUpdatesStream>, Status> {
        // Lagged receivers skip missed events instead of ending the stream.
        let stream = BroadcastStream::new(crate::events::subscribe()).filter_map(|item| item.ok().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }
}

fn internal_err<E: std::fmt::Display>(e: E) -> Status {
    Status::internal(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::metadata::MetadataMap;

    fn cfg(hash: Option<&str>, insecure: bool) -> ServerConfig {
        ServerConfig { admin_token_hash: hash.map(|s| s.to_string()), allow_insecure_no_auth: insecure, ..ServerConfig::default() }
    }

    fn meta(token: Option<&str>) -> MetadataMap {
        let mut m = MetadataMap::new();
        if let Some(t) = token {
            m.insert("authorization", format!("Bearer {t}").parse().unwrap());
        }
        m
    }

    #[test]
    fn refuses_without_configured_hash() {
        let e = authorize(&cfg(None, false), &meta(Some("x"))).unwrap_err();
        assert_eq!(e.code(), tonic::Code::FailedPrecondition);
        assert!(e.message().contains("admin_token_hash"));
        let e = authorize(&cfg(Some("  "), false), &meta(None)).unwrap_err();
        assert_eq!(e.code(), tonic::Code::FailedPrecondition);
    }

    #[test]
    fn insecure_opt_out_allows() {
        assert!(authorize(&cfg(None, true), &meta(None)).unwrap().contains("insecure"));
    }

    #[test]
    fn checks_token_hash() {
        let h = sha256_hex("s3cret").to_uppercase();
        let c = cfg(Some(&h), false);
        assert_eq!(authorize(&c, &meta(Some("s3cret"))).unwrap(), "admin");
        assert_eq!(authorize(&c, &meta(Some("nope"))).unwrap_err().code(), tonic::Code::Unauthenticated);
        assert_eq!(authorize(&c, &meta(None)).unwrap_err().code(), tonic::Code::Unauthenticated);
    }
}
