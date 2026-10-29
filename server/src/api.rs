use crate::config::ServerConfig;
use crate::generated::itcmdr::itcmdr_server::{Itcmdr, ItcmdrServer};
use crate::generated::itcmdr::*;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct ItcmdrService {
    cfg: ServerConfig,
}

impl ItcmdrService {
    pub fn new(cfg: ServerConfig) -> Self {
        Self { cfg }
    }

    pub fn into_router(self) -> tonic::transport::server::Router {
        tonic::transport::Server::builder().add_service(ItcmdrServer::new(self))
    }
}

#[tonic::async_trait]
impl Itcmdr for ItcmdrService {
    async fn list_machines(
        &self,
        _req: Request<ListMachinesRequest>,
    ) -> Result<Response<ListMachinesResponse>, Status> {
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
}

