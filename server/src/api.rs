use crate::config::ServerConfig;
use crate::generated::itcmdr::itcmdr_server::Itcmdr;
use crate::generated::itcmdr::*;
use tonic::{Request, Response, Status};

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
}

#[tonic::async_trait]
impl Itcmdr for ItcmdrService {
    async fn list_machines(
        &self,
        _req: Request<ListMachinesRequest>,
    ) -> Result<Response<ListMachinesResponse>, Status> {
        let _bind = &self.cfg.bind_addr; // read to mark field used
        // Placeholder: return empty list; server will populate from DB in future commits.
        Ok(Response::new(ListMachinesResponse { machines: vec![] }))
    }

    async fn trigger_runs(
        &self,
        _req: Request<TriggerRunsRequest>,
    ) -> Result<Response<TriggerRunsResponse>, Status> {
        // Placeholder implementation that returns a random request_id.
        let id = uuid::Uuid::new_v4().to_string();
        Ok(Response::new(TriggerRunsResponse { request_id: id }))
    }

    async fn get_request_status(
        &self,
        _req: Request<GetRequestStatusRequest>,
    ) -> Result<Response<GetRequestStatusResponse>, Status> {
        Ok(Response::new(GetRequestStatusResponse {
            status: request_status::Status::Queued as i32,
            message: "queued".to_string(),
        }))
    }

    async fn get_metrics(
        &self,
        _req: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        Ok(Response::new(GetMetricsResponse { points: vec![] }))
    }

    async fn list_exceptions(
        &self,
        _req: Request<ListExceptionsRequest>,
    ) -> Result<Response<ListExceptionsResponse>, Status> {
        Ok(Response::new(ListExceptionsResponse { rules: vec![] }))
    }

    async fn save_exception(
        &self,
        req: Request<SaveExceptionRequest>,
    ) -> Result<Response<SaveExceptionResponse>, Status> {
        // Echo back with a placeholder id for now.
        let mut rule = req.into_inner().rule.unwrap_or_default();
        if rule.id.is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        Ok(Response::new(SaveExceptionResponse { rule: Some(rule) }))
    }

    async fn expire_exception(
        &self,
        _req: Request<ExpireExceptionRequest>,
    ) -> Result<Response<ExpireExceptionResponse>, Status> {
        Ok(Response::new(ExpireExceptionResponse {}))
    }
}

