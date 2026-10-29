use crate::config::ServerConfig;
use crate::generated::itcmdr::itcmdr_server::Itcmdr;
use crate::generated::itcmdr::*;
use tonic::{Request, Response, Status};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct ItcmdrService {
    cfg: ServerConfig,
}

impl ItcmdrService {
    pub fn new(cfg: ServerConfig) -> Self {
        // Touch a field to avoid dead_code warning until used by real handlers.
        let _ = cfg.bind_addr.as_str();
        Self { cfg }
    }

    fn check_auth(&self, meta: &tonic::metadata::MetadataMap) -> Result<(), Status> {
        if let Some(expected) = self.cfg.admin_token_hash.as_ref().and_then(|s| if s.trim().is_empty() { None } else { Some(s) }) {
            let auth = meta.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
            let token = auth.strip_prefix("Bearer ").unwrap_or("");
            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            let hash_hex = format!("{:x}", hasher.finalize());
            if &hash_hex != expected {
                return Err(Status::unauthenticated("invalid token"));
            }
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl Itcmdr for ItcmdrService {
    async fn list_machines(
        &self,
        req: Request<ListMachinesRequest>,
    ) -> Result<Response<ListMachinesResponse>, Status> {
        let pool = crate::db::init_db(&self.cfg).await.map_err(internal_err)?;
        let items = crate::db::list_machines_overview(&pool).await.map_err(internal_err)?;
        let machines = items.into_iter().map(|(name, status, open_incidents, last_run)| Machine {
            name,
            status,
            open_incidents: open_incidents as i32,
            last_run_ts: last_run,
        }).collect();
        Ok(Response::new(ListMachinesResponse { machines }))
    }

    async fn trigger_runs(
        &self,
        req: Request<TriggerRunsRequest>,
    ) -> Result<Response<TriggerRunsResponse>, Status> {
        self.check_auth(req.metadata())?;
        // Write a request file under requests_dir for agents to poll
        let id = uuid::Uuid::new_v4().to_string();
        let body = req.into_inner();
        let options = serde_json::json!({
            "machines": body.machines,
            "reportOnly": body.report_only,
            "repair": body.repair,
            "force": body.force,
            "categories": body.categories
        }).to_string();
        let pool = crate::db::init_db(&self.cfg).await.map_err(internal_err)?;
        crate::db::insert_request(&pool, &id, &options).await.map_err(internal_err)?;
        let dir = self.cfg.requests_dir.clone().unwrap_or(self.cfg.logs_dir.join("requests"));
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join(format!("{}.json", id));
        std::fs::write(&path, &options).map_err(internal_err)?;
        Ok(Response::new(TriggerRunsResponse { request_id: id }))
    }

    async fn get_request_status(
        &self,
        req: Request<GetRequestStatusRequest>,
    ) -> Result<Response<GetRequestStatusResponse>, Status> {
        let id = req.into_inner().request_id;
        let pool = crate::db::init_db(&self.cfg).await.map_err(internal_err)?;
        let status = crate::db::get_request_status(&pool, &id).await.map_err(internal_err)?;
        let mapped = match status.as_str() {
            "queued" => request_status::Status::Queued as i32,
            "running" => request_status::Status::Running as i32,
            "done" => request_status::Status::Done as i32,
            "failed" => request_status::Status::Failed as i32,
            _ => request_status::Status::Unknown as i32,
        };
        Ok(Response::new(GetRequestStatusResponse { status: mapped, message: status }))
    }

    async fn get_metrics(
        &self,
        req: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let _ = req.into_inner();
        // Placeholder: return empty until implemented fully in metrics module
        Ok(Response::new(GetMetricsResponse { points: vec![] }))
    }

    async fn list_exceptions(
        &self,
        _req: Request<ListExceptionsRequest>,
    ) -> Result<Response<ListExceptionsResponse>, Status> {
        // Return empty for now; full mapping from DB JSON to prost types can be added later.
        Ok(Response::new(ListExceptionsResponse { rules: vec![] }))
    }

    async fn save_exception(
        &self,
        req: Request<SaveExceptionRequest>,
    ) -> Result<Response<SaveExceptionResponse>, Status> {
        self.check_auth(req.metadata())?;
        let mut rule = req.into_inner().rule.unwrap_or_default();
        if rule.id.is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        let pool = crate::db::init_db(&self.cfg).await.map_err(internal_err)?;
        // Persist as JSON using a neutral schema
        let s = serde_json::json!({
            "id": rule.id,
            "scope": {
                "machines": rule.scope.as_ref().map(|s| s.machines.clone()).unwrap_or_default(),
                "role": rule.scope.as_ref().and_then(|s| if s.role.is_empty() { None } else { Some(s.role.clone()) })
            },
            "check_id": rule.check_id,
            "check_pattern": rule.check_pattern,
            "action": rule.action,
            "reason": rule.reason,
            "expires": rule.expires,
            "active": rule.active
        }).to_string();
        crate::db::save_exception(&pool, &rule.id, &s).await.map_err(internal_err)?;
        Ok(Response::new(SaveExceptionResponse { rule: Some(rule) }))
    }

    async fn expire_exception(
        &self,
        req: Request<ExpireExceptionRequest>,
    ) -> Result<Response<ExpireExceptionResponse>, Status> {
        self.check_auth(req.metadata())?;
        let id = req.into_inner().id;
        let pool = crate::db::init_db(&self.cfg).await.map_err(internal_err)?;
        crate::db::expire_exception(&pool, &id).await.map_err(internal_err)?;
        Ok(Response::new(ExpireExceptionResponse {}))
    }
}

fn internal_err<E: std::fmt::Display>(e: E) -> Status {
    Status::internal(e.to_string())
}
