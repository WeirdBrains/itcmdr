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
    nonces: std::sync::Arc<crate::auth::NonceCache>,
    restore_keys: std::sync::Arc<crate::restore_seal::RestoreKeys>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AuthMode {
    Token,
    Both,
    Signed,
}

impl AuthMode {
    pub fn from_config(s: &str) -> AuthMode {
        match s.trim().to_ascii_lowercase().as_str() {
            "token" => AuthMode::Token,
            "both" => AuthMode::Both,
            // "signed", and anything unknown or unsettled: fail secure.
            _ => AuthMode::Signed,
        }
    }
}

const M_TRIGGER: &str = "/itcmdr.Itcmdr/TriggerRuns";
const M_SAVE_EXC: &str = "/itcmdr.Itcmdr/SaveException";
const M_EXPIRE_EXC: &str = "/itcmdr.Itcmdr/ExpireException";
const M_LIST_AUDIT: &str = "/itcmdr.Itcmdr/ListAudit";

pub fn sha256_hex(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

/// Decide whether a mutating call may proceed. Returns the actor name for the audit log.
/// No token hash configured => refused, unless `allow_insecure_no_auth` is set.
pub fn authorize(
    cfg: &ServerConfig,
    meta: &tonic::metadata::MetadataMap,
) -> Result<String, Status> {
    let actor = meta
        .get("x-itcmdr-actor")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.chars()
                .filter(|c| c.is_ascii_graphic() || *c == ' ')
                .take(64)
                .collect::<String>()
        })
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
        Self {
            cfg,
            pool,
            nonces: Default::default(),
            restore_keys: Default::default(),
        }
    }

    fn mode(&self) -> AuthMode {
        AuthMode::from_config(&self.cfg.auth_mode)
    }

    async fn signed<T: prost::Message>(
        &self,
        req: &Request<T>,
        method: &str,
        min: crate::auth::Role,
    ) -> Result<crate::auth::Principal, Status> {
        let body = req.get_ref().encode_to_vec();
        let p = crate::auth::verify(
            &self.pool,
            &self.nonces,
            req.metadata(),
            method,
            &body,
            chrono::Utc::now().timestamp_millis(),
        )
        .await?;
        crate::auth::require(&p, min)?;
        Ok(p)
    }

    /// Calls that never accept the shared token (restore, keys): signed in every auth mode.
    async fn signed_only<T: prost::Message>(
        &self,
        req: &Request<T>,
        method: &str,
        min: crate::auth::Role,
    ) -> Result<crate::auth::Principal, Status> {
        if !crate::auth::has_signature(req.metadata()) {
            return Err(Status::unauthenticated(
                "this call needs a signed request (register a key for the app; the shared token is not enough)",
            ));
        }
        self.signed(req, method, min).await
    }

    /// Mutating calls: a signed request with at least `min`, or (modes token/both) the legacy token.
    /// Returns the actor for the audit log.
    async fn authz<T: prost::Message>(
        &self,
        req: &Request<T>,
        method: &str,
        min: crate::auth::Role,
    ) -> Result<String, Status> {
        let mode = self.mode();
        if mode == AuthMode::Signed
            || (mode == AuthMode::Both && crate::auth::has_signature(req.metadata()))
        {
            return Ok(self.signed(req, method, min).await?.actor());
        }
        authorize(&self.cfg, req.metadata())
    }

    /// Read calls: open in modes token/both (a signature, if sent, must still be valid);
    /// in signed mode they need a key with at least `min`.
    async fn read_guard<T: prost::Message>(
        &self,
        req: &Request<T>,
        method: &str,
        min: crate::auth::Role,
    ) -> Result<(), Status> {
        if self.mode() == AuthMode::Signed || crate::auth::has_signature(req.metadata()) {
            self.signed(req, method, min).await?;
        }
        Ok(())
    }

    fn backup_info(b: crate::backup::BackupInfo) -> BackupInfo {
        BackupInfo {
            name: b.name,
            size: b.size as i64,
            created_ts: b.manifest.created_ms,
            kind: b.manifest.kind,
            itcmdr_version: b.manifest.itcmdr_version,
            schema_version: b.manifest.schema_version,
        }
    }

    /// Agent settings changed: rewrite agent-settings.json on the share.
    async fn after_settings_change(&self, key: &str) {
        if crate::settings::def(key).is_ok_and(|d| d.scope == crate::settings::Scope::Agent) {
            if let Err(e) = crate::settings::export_agent(&self.pool, &self.cfg).await {
                tracing::error!(error=%e, "agent settings export failed");
            }
        }
    }

    async fn audit(&self, actor: &str, action: &str, detail: &serde_json::Value) {
        if let Err(e) =
            crate::db::insert_audit(&self.pool, actor, action, &detail.to_string()).await
        {
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
    async fn list_machines(
        &self,
        req: Request<ListMachinesRequest>,
    ) -> Result<Response<ListMachinesResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/ListMachines",
            crate::auth::Role::Client,
        )
        .await?;
        let items = crate::db::list_machines_overview(&self.pool)
            .await
            .map_err(internal_err)?;
        let machines = items
            .into_iter()
            .map(|(name, status, open_incidents, last_run)| Machine {
                name,
                status,
                open_incidents: open_incidents as i32,
                last_run_ts: last_run,
            })
            .collect();
        Ok(Response::new(ListMachinesResponse { machines }))
    }

    async fn trigger_runs(
        &self,
        req: Request<TriggerRunsRequest>,
    ) -> Result<Response<TriggerRunsResponse>, Status> {
        let actor = self.authz(&req, M_TRIGGER, crate::auth::Role::User).await?;
        let body = req.into_inner();
        let r = crate::jobs::RunRequest {
            machines: body.machines,
            categories: body.categories,
            plugins: body.plugins,
            report_only: body.report_only,
            repair: body.repair,
            force: body.force,
            schedule_id: None,
        };
        let id = crate::jobs::queue_run(&self.pool, &self.cfg, &r, &actor)
            .await
            .map_err(internal_err)?;
        self.audit(
            &actor,
            "trigger-runs",
            &serde_json::json!({"requestId": id, "request": r}),
        )
        .await;
        Ok(Response::new(TriggerRunsResponse { request_id: id }))
    }

    async fn get_request_status(
        &self,
        req: Request<GetRequestStatusRequest>,
    ) -> Result<Response<GetRequestStatusResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/GetRequestStatus",
            crate::auth::Role::Client,
        )
        .await?;
        let id = req.into_inner().request_id;
        // Fold in any new responses before answering.
        let _ = crate::requests::sync_requests(
            &self.pool,
            &self.cfg.responses_dir(),
            crate::settings::get_i64(&self.pool, Some(&self.cfg), "server.request_timeout_secs")
                .await
                * 1000,
        )
        .await;
        let (status, message) = crate::db::get_request_status(&self.pool, &id)
            .await
            .map_err(internal_err)?;
        let mapped = match status.as_str() {
            "queued" => request_status::Status::Queued,
            "running" => request_status::Status::Running,
            "done" => request_status::Status::Done,
            "failed" => request_status::Status::Failed,
            "timed_out" => request_status::Status::TimedOut,
            _ => request_status::Status::Unknown,
        } as i32;
        Ok(Response::new(GetRequestStatusResponse {
            status: mapped,
            message,
        }))
    }

    async fn get_metrics(
        &self,
        req: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        self.read_guard(&req, "/itcmdr.Itcmdr/GetMetrics", crate::auth::Role::Client)
            .await?;
        let p = req.into_inner();
        let pts = crate::metrics::get_metrics(&self.pool, p.window_days as i64, p.rollup())
            .await
            .map_err(internal_err)?;
        Ok(Response::new(GetMetricsResponse { points: pts }))
    }

    async fn list_exceptions(
        &self,
        req: Request<ListExceptionsRequest>,
    ) -> Result<Response<ListExceptionsResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/ListExceptions",
            crate::auth::Role::Client,
        )
        .await?;
        let inc = req.into_inner().include_inactive;
        let rules = crate::exceptions::load(&self.pool, inc)
            .await
            .map_err(internal_err)?
            .into_iter()
            .map(|(d, a)| d.to_proto(a))
            .collect();
        Ok(Response::new(ListExceptionsResponse { rules }))
    }

    async fn save_exception(
        &self,
        req: Request<SaveExceptionRequest>,
    ) -> Result<Response<SaveExceptionResponse>, Status> {
        let actor = self
            .authz(&req, M_SAVE_EXC, crate::auth::Role::Admin)
            .await?;
        let rule = req.into_inner().rule.unwrap_or_default();
        let mut doc = ExceptionDoc::from_proto(&rule);
        doc.validate().map_err(Status::invalid_argument)?;
        let is_new = doc.id.is_empty();
        if is_new {
            doc.id = uuid::Uuid::new_v4().to_string();
        }
        let json = serde_json::to_value(&doc).map_err(internal_err)?;
        crate::db::save_exception(&self.pool, &doc.id, &json.to_string(), true)
            .await
            .map_err(internal_err)?;
        self.audit(
            &actor,
            if is_new {
                "exception-add"
            } else {
                "exception-edit"
            },
            &json,
        )
        .await;
        self.export_exceptions().await;
        Ok(Response::new(SaveExceptionResponse {
            rule: Some(doc.to_proto(true)),
        }))
    }

    async fn expire_exception(
        &self,
        req: Request<ExpireExceptionRequest>,
    ) -> Result<Response<ExpireExceptionResponse>, Status> {
        let actor = self
            .authz(&req, M_EXPIRE_EXC, crate::auth::Role::Admin)
            .await?;
        let id = req.into_inner().id;
        if !crate::db::expire_exception(&self.pool, &id)
            .await
            .map_err(internal_err)?
        {
            return Err(Status::not_found("no exception with that id"));
        }
        self.audit(&actor, "exception-expire", &serde_json::json!({ "id": id }))
            .await;
        self.export_exceptions().await;
        Ok(Response::new(ExpireExceptionResponse {}))
    }

    async fn search_logs(
        &self,
        req: Request<SearchLogsRequest>,
    ) -> Result<Response<SearchLogsResponse>, Status> {
        self.read_guard(&req, "/itcmdr.Itcmdr/SearchLogs", crate::auth::Role::User)
            .await?;
        let p = req.into_inner();
        let lines = crate::ingest::iter_log_lines(&self.cfg.logs_dir).map_err(internal_err)?;
        let dl = crate::settings::get_i64(&self.pool, None, "search.default_limit").await as usize;
        let ml = crate::settings::get_i64(&self.pool, None, "search.max_limit").await as usize;
        let page = crate::search::search_with(lines, &p, dl, ml.max(dl));
        Ok(Response::new(SearchLogsResponse {
            lines: page.lines,
            total: page.total as i32,
            next_offset: page.next_offset as i32,
        }))
    }

    async fn list_audit(
        &self,
        req: Request<ListAuditRequest>,
    ) -> Result<Response<ListAuditResponse>, Status> {
        self.read_guard(&req, M_LIST_AUDIT, crate::auth::Role::Admin)
            .await?;
        let limit = match req.into_inner().limit {
            n if n <= 0 => 200,
            n => n.min(5000) as i64,
        };
        let entries = crate::db::list_audit(&self.pool, limit)
            .await
            .map_err(internal_err)?
            .into_iter()
            .map(|(ts, actor, action, detail)| AuditEntry {
                ts,
                actor,
                action,
                detail,
            })
            .collect();
        Ok(Response::new(ListAuditResponse { entries }))
    }

    async fn create_backup(
        &self,
        req: Request<CreateBackupRequest>,
    ) -> Result<Response<CreateBackupResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/CreateBackup",
                crate::auth::Role::Admin,
            )
            .await?;
        let b = crate::backup::create(&self.pool, &self.cfg, "manual", None)
            .await
            .map_err(|e| Status::failed_precondition(format!("backup failed: {e:#}")))?;
        self.audit(
            &actor,
            "backup-create",
            &serde_json::json!({"name": b.name, "size": b.size}),
        )
        .await;
        Ok(Response::new(CreateBackupResponse {
            backup: Some(Self::backup_info(b)),
        }))
    }

    async fn list_backups(
        &self,
        req: Request<ListBackupsRequest>,
    ) -> Result<Response<ListBackupsResponse>, Status> {
        let actor = self
            .authz(&req, "/itcmdr.Itcmdr/ListBackups", crate::auth::Role::Admin)
            .await?;
        let list = crate::backup::list(
            &crate::settings::backup_dir(&self.pool, &self.cfg)
                .await
                .map_err(internal_err)?,
        )
        .map_err(internal_err)?;
        self.audit(
            &actor,
            "backup-list",
            &serde_json::json!({"count": list.len()}),
        )
        .await;
        Ok(Response::new(ListBackupsResponse {
            backups: list.into_iter().map(Self::backup_info).collect(),
        }))
    }

    async fn verify_backup(
        &self,
        req: Request<VerifyBackupRequest>,
    ) -> Result<Response<VerifyBackupResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/VerifyBackup",
                crate::auth::Role::Admin,
            )
            .await?;
        let name = req.into_inner().name;
        if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
            return Err(Status::invalid_argument(
                "name must be a plain archive file name",
            ));
        }
        let master = crate::auth::master_public(&self.pool)
            .await
            .map_err(internal_err)?
            .ok_or_else(|| Status::failed_precondition("server is not initialized"))?;
        let r = crate::backup::verify(
            &crate::settings::backup_dir(&self.pool, &self.cfg)
                .await
                .map_err(internal_err)?
                .join(&name),
            Some(&master.ed25519),
        );
        let (ok, message) = match r {
            Ok(_) => (true, "signature and contents hash are valid (decrypting needs the words: itcmdr-cli backup verify --deep)".to_string()),
            Err(e) => (false, format!("{e:#}")),
        };
        self.audit(
            &actor,
            "backup-verify",
            &serde_json::json!({"name": name, "ok": ok}),
        )
        .await;
        Ok(Response::new(VerifyBackupResponse { ok, message }))
    }

    async fn get_settings(
        &self,
        req: Request<GetSettingsRequest>,
    ) -> Result<Response<GetSettingsResponse>, Status> {
        self.authz(&req, "/itcmdr.Itcmdr/GetSettings", crate::auth::Role::Admin)
            .await?;
        let b = crate::settings::effective_backup(&self.pool, &self.cfg)
            .await
            .map_err(internal_err)?;
        Ok(Response::new(GetSettingsResponse {
            backup: Some(to_proto_backup(&b)),
        }))
    }

    async fn update_settings(
        &self,
        req: Request<UpdateSettingsRequest>,
    ) -> Result<Response<UpdateSettingsResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/UpdateSettings",
                crate::auth::Role::Admin,
            )
            .await?;
        let p = req
            .into_inner()
            .backup
            .ok_or_else(|| Status::invalid_argument("backup settings missing"))?;
        let b = crate::config::BackupConfig {
            dir: (!p.dir.trim().is_empty()).then(|| p.dir.trim().into()),
            nightly: p.nightly,
            hour: p.hour,
            keep_daily: p.keep_daily as usize,
            keep_weekly: p.keep_weekly as usize,
            keep_blocks: p.keep_blocks as usize,
        };
        crate::settings::set_backup(&self.pool, &b)
            .await
            .map_err(|e| Status::invalid_argument(format!("{e:#}")))?;
        self.audit(&actor, "settings-update", &serde_json::json!({"backup": b}))
            .await;
        Ok(Response::new(UpdateSettingsResponse {
            backup: Some(to_proto_backup(&b)),
        }))
    }

    async fn list_settings(
        &self,
        req: Request<ListSettingsRequest>,
    ) -> Result<Response<ListSettingsResponse>, Status> {
        self.authz(
            &req,
            "/itcmdr.Itcmdr/ListSettings",
            crate::auth::Role::Admin,
        )
        .await?;
        let m = req.into_inner().machine;
        let m = (!m.trim().is_empty()).then(|| m.trim().to_string());
        let all = crate::settings::resolve_all(&self.pool, Some(&self.cfg), m.as_deref())
            .await
            .map_err(internal_err)?;
        let store = crate::settings::load(&self.pool)
            .await
            .map_err(internal_err)?;
        Ok(Response::new(ListSettingsResponse {
            settings: all.iter().map(to_setting_item).collect(),
            machines_with_overrides: store.machines.keys().cloned().collect(),
        }))
    }

    async fn update_setting(
        &self,
        req: Request<UpdateSettingRequest>,
    ) -> Result<Response<UpdateSettingResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/UpdateSetting",
                crate::auth::Role::Admin,
            )
            .await?;
        let r = req.into_inner();
        let machine = (!r.machine.trim().is_empty()).then(|| r.machine.trim().to_string());
        let res = if r.unset {
            crate::settings::unset(&self.pool, &r.key, machine.as_deref()).await
        } else {
            crate::settings::set(&self.pool, &r.key, &r.value, machine.as_deref())
                .await
                .map(|_| ())
        };
        res.map_err(|e| Status::invalid_argument(format!("{e:#}")))?;
        self.audit(
            &actor,
            "settings-update",
            &serde_json::json!({"key": r.key, "value": if r.unset { serde_json::Value::Null } else { r.value.clone().into() }, "machine": machine, "unset": r.unset}),
        )
        .await;
        self.after_settings_change(&r.key).await;
        let all = crate::settings::resolve_all(&self.pool, Some(&self.cfg), machine.as_deref())
            .await
            .map_err(internal_err)?;
        let item = all.iter().find(|x| x.def.key == r.key).map(to_setting_item);
        Ok(Response::new(UpdateSettingResponse { setting: item }))
    }

    async fn run_plugin(
        &self,
        req: Request<RunPluginRequest>,
    ) -> Result<Response<RunPluginResponse>, Status> {
        let actor = self
            .authz(&req, "/itcmdr.Itcmdr/RunPlugin", crate::auth::Role::Admin)
            .await?;
        let r = req.into_inner();
        let spec = crate::runtime::RunSpec {
            plugin: r.plugin,
            machine: r.machine,
            report_only: r.report_only,
            args: r.args.into_iter().collect(),
            request_id: None,
        };
        let rep = crate::runtime::run_plugin(&self.pool, &self.cfg, spec, &actor)
            .await
            .map_err(|e| Status::failed_precondition(format!("{e:#}")))?;
        Ok(Response::new(RunPluginResponse {
            run_id: rep.run_id,
            status: rep.status,
            message: rep.message,
            stages: rep
                .stages
                .into_iter()
                .map(|s| PluginStage {
                    stage: s.stage,
                    ok: s.ok,
                    message: s.message,
                })
                .collect(),
            results: rep
                .results
                .into_iter()
                .map(|(c, s, m)| PluginCheckResult {
                    check_id: c,
                    status: s,
                    message: m,
                })
                .collect(),
        }))
    }

    async fn list_plugin_runs(
        &self,
        req: Request<ListPluginRunsRequest>,
    ) -> Result<Response<ListPluginRunsResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/ListPluginRuns",
            crate::auth::Role::User,
        )
        .await?;
        let r = req.into_inner();
        let p = (!r.plugin.is_empty()).then_some(r.plugin.as_str());
        let lim = if r.limit <= 0 { 100 } else { r.limit as i64 };
        let rows = crate::runtime::list_runs(&self.pool, p, lim)
            .await
            .map_err(internal_err)?;
        Ok(Response::new(ListPluginRunsResponse {
            runs: rows
                .into_iter()
                .map(|x| PluginRun {
                    id: x.id,
                    plugin: x.plugin,
                    version: x.version,
                    machine: x.machine,
                    started: x.started_ms,
                    finished: x.finished_ms,
                    status: x.status,
                    message: x.message,
                    request_id: x.request_id.unwrap_or_default(),
                })
                .collect(),
        }))
    }

    async fn get_plugin_metrics(
        &self,
        req: Request<GetPluginMetricsRequest>,
    ) -> Result<Response<GetPluginMetricsResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/GetPluginMetrics",
            crate::auth::Role::User,
        )
        .await?;
        let r = req.into_inner();
        let labels: Vec<(String, String)> = r.labels.into_iter().collect();
        let w = if r.window_days <= 0 {
            7
        } else {
            r.window_days as i64
        };
        let pts = crate::plugin_logs::query(&self.pool, &r.name, &labels, w, r.daily)
            .await
            .map_err(internal_err)?;
        let rej = crate::plugin_logs::rejects(&self.pool)
            .await
            .map_err(internal_err)?;
        Ok(Response::new(GetPluginMetricsResponse {
            points: pts
                .into_iter()
                .map(|p| PluginMetricPoint {
                    name: p.name,
                    labels: p.labels,
                    ts: p.ts_ms,
                    value: p.value,
                })
                .collect(),
            rejects: rej
                .into_iter()
                .map(|(file, count)| LogRejects { file, count })
                .collect(),
        }))
    }

    async fn create_schedule(
        &self,
        req: Request<SaveScheduleRequest>,
    ) -> Result<Response<ScheduleResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/CreateSchedule",
                crate::auth::Role::Admin,
            )
            .await?;
        let s = req.into_inner().schedule.unwrap_or_default();
        let (name, enabled) = (s.name.clone(), s.enabled);
        let created = crate::schedule::create(&self.pool, &name, from_proto_schedule(&s), enabled)
            .await
            .map_err(bad_arg)?;
        self.audit(
            &actor,
            "schedule-create",
            &serde_json::json!({"id": created.id, "name": created.name, "spec": created.spec}),
        )
        .await;
        Ok(Response::new(ScheduleResponse {
            schedule: Some(to_proto_schedule(&created)),
        }))
    }

    async fn update_schedule(
        &self,
        req: Request<SaveScheduleRequest>,
    ) -> Result<Response<ScheduleResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/UpdateSchedule",
                crate::auth::Role::Admin,
            )
            .await?;
        let s = req.into_inner().schedule.unwrap_or_default();
        let id = if s.id.is_empty() {
            s.name.clone()
        } else {
            s.id.clone()
        };
        let mut u =
            crate::schedule::update(&self.pool, &id, Some(&s.name), from_proto_schedule(&s))
                .await
                .map_err(bad_arg)?;
        if u.enabled != s.enabled {
            u = crate::schedule::set_enabled(&self.pool, &u.id, s.enabled)
                .await
                .map_err(bad_arg)?;
        }
        self.audit(
            &actor,
            "schedule-update",
            &serde_json::json!({"id": u.id, "name": u.name, "spec": u.spec, "enabled": u.enabled}),
        )
        .await;
        Ok(Response::new(ScheduleResponse {
            schedule: Some(to_proto_schedule(&u)),
        }))
    }

    async fn delete_schedule(
        &self,
        req: Request<ScheduleRef>,
    ) -> Result<Response<ScheduleResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/DeleteSchedule",
                crate::auth::Role::Admin,
            )
            .await?;
        let r = req.into_inner();
        let confirm = (!r.confirm.is_empty()).then_some(r.confirm.as_str());
        let d = crate::schedule::delete(&self.pool, &r.id, r.force, confirm)
            .await
            .map_err(bad_arg)?;
        self.audit(
            &actor,
            "schedule-delete",
            &serde_json::json!({"id": d.id, "name": d.name, "force": r.force}),
        )
        .await;
        Ok(Response::new(ScheduleResponse {
            schedule: Some(to_proto_schedule(&d)),
        }))
    }

    async fn reset_schedule_to_default(
        &self,
        req: Request<ScheduleRef>,
    ) -> Result<Response<ScheduleResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/ResetScheduleToDefault",
                crate::auth::Role::Admin,
            )
            .await?;
        let r = req.into_inner();
        let s = crate::cutover::reset_to_default(&self.pool, &self.cfg, &r.id, &actor)
            .await
            .map_err(bad_arg)?;
        Ok(Response::new(ScheduleResponse {
            schedule: Some(to_proto_schedule(&s)),
        }))
    }

    async fn list_schedules(
        &self,
        req: Request<ListSchedulesRequest>,
    ) -> Result<Response<ListSchedulesResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/ListSchedules",
            crate::auth::Role::User,
        )
        .await?;
        let l = crate::schedule::list(&self.pool)
            .await
            .map_err(internal_err)?;
        Ok(Response::new(ListSchedulesResponse {
            schedules: l.iter().map(to_proto_schedule).collect(),
        }))
    }

    async fn pause_schedule(
        &self,
        req: Request<ScheduleRef>,
    ) -> Result<Response<ScheduleResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/PauseSchedule",
                crate::auth::Role::Admin,
            )
            .await?;
        let s = crate::schedule::set_enabled(&self.pool, &req.into_inner().id, false)
            .await
            .map_err(bad_arg)?;
        self.audit(
            &actor,
            "schedule-pause",
            &serde_json::json!({"id": s.id, "name": s.name}),
        )
        .await;
        Ok(Response::new(ScheduleResponse {
            schedule: Some(to_proto_schedule(&s)),
        }))
    }

    async fn resume_schedule(
        &self,
        req: Request<ScheduleRef>,
    ) -> Result<Response<ScheduleResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/ResumeSchedule",
                crate::auth::Role::Admin,
            )
            .await?;
        let s = crate::schedule::set_enabled(&self.pool, &req.into_inner().id, true)
            .await
            .map_err(bad_arg)?;
        self.audit(
            &actor,
            "schedule-resume",
            &serde_json::json!({"id": s.id, "name": s.name}),
        )
        .await;
        Ok(Response::new(ScheduleResponse {
            schedule: Some(to_proto_schedule(&s)),
        }))
    }

    async fn run_schedule_now(
        &self,
        req: Request<ScheduleRef>,
    ) -> Result<Response<RunScheduleNowResponse>, Status> {
        let actor = self
            .authz(
                &req,
                "/itcmdr.Itcmdr/RunScheduleNow",
                crate::auth::Role::User,
            )
            .await?;
        let s = crate::schedule::get(&self.pool, &req.into_inner().id)
            .await
            .map_err(bad_arg)?;
        let (outcome, rid, note) = crate::schedule::fire(
            &self.pool,
            &self.cfg,
            &s,
            chrono::Utc::now().timestamp_millis(),
            &format!("run now by {actor}"),
            &actor,
            false,
        )
        .await
        .map_err(internal_err)?;
        self.audit(
            &actor,
            "schedule-run-now",
            &serde_json::json!({"id": s.id, "name": s.name, "outcome": outcome, "request_id": rid}),
        )
        .await;
        Ok(Response::new(RunScheduleNowResponse {
            outcome,
            request_id: rid.unwrap_or_default(),
            note,
        }))
    }

    async fn list_schedule_runs(
        &self,
        req: Request<ListScheduleRunsRequest>,
    ) -> Result<Response<ListScheduleRunsResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/ListScheduleRuns",
            crate::auth::Role::User,
        )
        .await?;
        let r = req.into_inner();
        let id = (!r.id.is_empty()).then_some(r.id.as_str());
        let lim = if r.limit <= 0 { 100 } else { r.limit as i64 };
        let rows = crate::schedule::history(&self.pool, id, lim)
            .await
            .map_err(bad_arg)?;
        Ok(Response::new(ListScheduleRunsResponse {
            runs: rows
                .into_iter()
                .map(|x| ScheduleRun {
                    schedule_id: x.schedule_id,
                    ts: x.ts_ms,
                    due: x.due_ms,
                    request_id: x.request_id.unwrap_or_default(),
                    outcome: x.outcome,
                    note: x.note,
                })
                .collect(),
        }))
    }

    async fn stage_restore(
        &self,
        req: Request<StageRestoreRequest>,
    ) -> Result<Response<StageRestoreResponse>, Status> {
        let p = self
            .signed_only(
                &req,
                "/itcmdr.Itcmdr/StageRestore",
                crate::auth::Role::Owner,
            )
            .await?;
        let actor = p.actor();
        let r = req.into_inner();
        let now = chrono::Utc::now().timestamp_millis();
        if !r.words.is_empty() {
            self.audit(
                &actor,
                "restore-refused",
                &serde_json::json!({"backup": r.backup, "reason": "plaintext words"}),
            )
            .await;
            return Err(Status::invalid_argument(
                "plaintext words are refused since v0.12.0: seal them for a GetRestoreKey key (sealed_words)",
            ));
        }
        let sw = r
            .sealed_words
            .ok_or_else(|| Status::invalid_argument("sealed_words is required"))?;
        let sealed = crate::restore_seal::Sealed {
            key_id: sw.key_id,
            eph: sw.eph,
            nonce: sw.nonce,
            ct: sw.ct,
        };
        let opened = self
            .restore_keys
            .open(&sealed, &p.fingerprint, &r.backup, now);
        let words = match opened {
            Ok(w) => w,
            Err(e) => {
                self.audit(&actor, "restore-refused", &serde_json::json!({"backup": r.backup, "dry_run": r.dry_run, "error": format!("{e:#}")}))
                    .await;
                return Err(Status::failed_precondition(format!("{e:#}")));
            }
        };
        if r.dry_run {
            let res = async {
                let archive =
                    crate::restore::archive_path(&self.pool, &self.cfg, &r.backup).await?;
                crate::backup::restore(&self.cfg, &archive, &words, true).await
            }
            .await;
            self.audit(
                &actor,
                "restore-dry-run",
                &serde_json::json!({"backup": r.backup, "ok": res.is_ok(), "error": res.as_ref().err().map(|e| format!("{e:#}"))}),
            )
            .await;
            res.map_err(|e| Status::failed_precondition(format!("{e:#}")))?;
            let id = self
                .restore_keys
                .record_dry_run(&p.fingerprint, &r.backup, now);
            return Ok(Response::new(StageRestoreResponse {
                status: None,
                dry_run_id: id,
                message: "dry run passed: the words open this backup and the archive is intact; nothing was changed".into(),
            }));
        }
        if let Err(e) =
            self.restore_keys
                .take_dry_run(&r.dry_run_id, &p.fingerprint, &r.backup, now)
        {
            self.audit(
                &actor,
                "restore-refused",
                &serde_json::json!({"backup": r.backup, "error": format!("{e:#}")}),
            )
            .await;
            return Err(Status::failed_precondition(format!("{e:#}")));
        }
        let res = crate::restore::stage(&self.pool, &self.cfg, &r.backup, &words, &actor).await;
        self.audit(
            &actor,
            "restore-stage",
            &serde_json::json!({"backup": r.backup, "ok": res.is_ok(), "error": res.as_ref().err().map(|e| format!("{e:#}"))}),
        )
        .await;
        let st = res.map_err(|e| Status::failed_precondition(format!("{e:#}")))?;
        Ok(Response::new(StageRestoreResponse {
            status: Some(to_proto_restore(&st)),
            dry_run_id: String::new(),
            message: String::new(),
        }))
    }

    async fn get_restore_key(
        &self,
        req: Request<GetRestoreKeyRequest>,
    ) -> Result<Response<RestoreKey>, Status> {
        let p = self
            .signed_only(
                &req,
                "/itcmdr.Itcmdr/GetRestoreKey",
                crate::auth::Role::Owner,
            )
            .await?;
        let (key_id, x25519_pub, expires) = self
            .restore_keys
            .issue(&p.fingerprint, chrono::Utc::now().timestamp_millis());
        Ok(Response::new(RestoreKey {
            key_id,
            x25519_pub,
            expires,
        }))
    }

    async fn who_am_i(
        &self,
        req: Request<WhoAmIRequest>,
    ) -> Result<Response<WhoAmIResponse>, Status> {
        let auth_mode = self.cfg.auth_mode.clone();
        if crate::auth::has_signature(req.metadata()) {
            let p = self
                .signed(&req, "/itcmdr.Itcmdr/WhoAmI", crate::auth::Role::Client)
                .await?;
            return Ok(Response::new(WhoAmIResponse {
                name: p.name,
                role: p.role.as_str().into(),
                kind: p.kind,
                fingerprint: p.fingerprint,
                auth_mode,
                signed: true,
            }));
        }
        if self.mode() == AuthMode::Signed {
            return Err(Status::unauthenticated(
                "this server only accepts signed requests",
            ));
        }
        // Token mode: the token is admin-equivalent for mutating calls (never owner).
        let role = if authorize(&self.cfg, req.metadata()).is_ok() {
            "admin"
        } else {
            "user"
        };
        Ok(Response::new(WhoAmIResponse {
            name: "token".into(),
            role: role.into(),
            kind: "token".into(),
            fingerprint: String::new(),
            auth_mode,
            signed: false,
        }))
    }

    async fn list_keys(
        &self,
        req: Request<ListKeysRequest>,
    ) -> Result<Response<ListKeysResponse>, Status> {
        self.signed_only(&req, "/itcmdr.Itcmdr/ListKeys", crate::auth::Role::Admin)
            .await?;
        let keys = crate::auth::list_keys(&self.pool)
            .await
            .map_err(internal_err)?;
        Ok(Response::new(ListKeysResponse {
            keys: keys.into_iter().map(key_info).collect(),
        }))
    }

    async fn register_key(
        &self,
        req: Request<RegisterKeyRequest>,
    ) -> Result<Response<RegisterKeyResponse>, Status> {
        let p = self
            .signed_only(&req, "/itcmdr.Itcmdr/RegisterKey", crate::auth::Role::Owner)
            .await?;
        let r = req.into_inner();
        #[derive(serde::Deserialize)]
        struct Bundle {
            kind: String,
            name: String,
            role: String,
            public_key: String,
            certificate: crate::keys::Certificate,
        }
        let b: Bundle = serde_json::from_str(&r.bundle_json)
            .map_err(|e| Status::invalid_argument(format!("bad key bundle: {e}")))?;
        let role = crate::auth::Role::parse(&b.role)
            .ok_or_else(|| Status::invalid_argument("unknown role in the bundle"))?;
        let opt = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
        let (email, phone) = (opt(&r.email), opt(&r.phone));
        let res = crate::auth::register(
            &self.pool,
            crate::auth::NewKey {
                kind: &b.kind,
                name: &b.name,
                role,
                email: email.as_deref(),
                phone: phone.as_deref(),
                public_key_hex: &b.public_key,
                certificate: &b.certificate,
            },
        )
        .await;
        self.audit(
            &p.actor(),
            "key-register",
            &serde_json::json!({"kind": b.kind, "name": b.name, "role": b.role, "fingerprint": b.certificate.fingerprint, "ok": res.is_ok(), "error": res.as_ref().err().map(|e| format!("{e:#}"))}),
        )
        .await;
        res.map_err(|e| Status::invalid_argument(format!("{e:#}")))?;
        let k = crate::auth::list_keys(&self.pool)
            .await
            .map_err(internal_err)?
            .into_iter()
            .find(|k| k.fingerprint == b.certificate.fingerprint)
            .map(key_info);
        Ok(Response::new(RegisterKeyResponse { key: k }))
    }

    async fn revoke_key(
        &self,
        req: Request<RevokeKeyRequest>,
    ) -> Result<Response<RevokeKeyResponse>, Status> {
        let p = self
            .signed_only(&req, "/itcmdr.Itcmdr/RevokeKey", crate::auth::Role::Owner)
            .await?;
        let fp = req.into_inner().fingerprint;
        if fp == p.fingerprint {
            return Err(Status::failed_precondition(
                "you cannot revoke the key you are signing with",
            ));
        }
        let revoked = crate::auth::revoke(&self.pool, &fp)
            .await
            .map_err(internal_err)?;
        self.audit(
            &p.actor(),
            "key-revoke",
            &serde_json::json!({"fingerprint": fp, "revoked": revoked}),
        )
        .await;
        Ok(Response::new(RevokeKeyResponse { revoked }))
    }

    async fn list_plugins(
        &self,
        req: Request<ListPluginsRequest>,
    ) -> Result<Response<ListPluginsResponse>, Status> {
        self.read_guard(&req, "/itcmdr.Itcmdr/ListPlugins", crate::auth::Role::User)
            .await?;
        let l = crate::packages::list(&self.pool, &self.cfg.layout())
            .await
            .map_err(internal_err)?;
        Ok(Response::new(ListPluginsResponse {
            plugins: l
                .into_iter()
                .map(|p| PluginInfo {
                    name: p.name,
                    version: p.version,
                    enabled: p.enabled,
                    installed_at: p.installed_at,
                    signer: p.signer,
                    package_sha256: p.package_sha256,
                })
                .collect(),
        }))
    }

    async fn install_plugin(
        &self,
        req: Request<InstallPluginRequest>,
    ) -> Result<Response<InstallPluginResponse>, Status> {
        let p = self
            .signed_only(
                &req,
                "/itcmdr.Itcmdr/InstallPlugin",
                crate::auth::Role::Admin,
            )
            .await?;
        let actor = p.actor();
        let r = req.into_inner();
        let layout = self.cfg.layout();
        let label = format!(
            "upload:{}",
            r.file_name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || "._-@".contains(*c))
                .take(80)
                .collect::<String>()
        );
        let src = match (r.url.trim().is_empty(), r.package.is_empty()) {
            (false, true) => {
                if !crate::packages::url_allowed(r.url.trim()) {
                    return Err(Status::invalid_argument(
                        "only https:// URLs are accepted (the server downloads the package)",
                    ));
                }
                crate::packages::Source::Url(r.url.trim())
            }
            (true, false) => crate::packages::Source::Bytes(&label, &r.package),
            _ => {
                return Err(Status::invalid_argument(
                    "send exactly one of url or package",
                ))
            }
        };
        let enable = if r.disabled { Some(false) } else { None };
        match crate::packages::install(&self.pool, &layout, src, enable, &actor).await {
            Ok(ip) => Ok(Response::new(InstallPluginResponse {
                message: format!(
                    "installed {} {} ({})",
                    ip.name,
                    ip.version,
                    if ip.enabled { "enabled" } else { "disabled" }
                ),
                plugin: Some(PluginInfo {
                    name: ip.name,
                    version: ip.version,
                    enabled: ip.enabled,
                    installed_at: ip.installed_at,
                    signer: ip.signer,
                    package_sha256: ip.package_sha256,
                }),
                quarantined: false,
            })),
            Err(crate::packages::InstallError::Quarantined { reason, file }) => {
                Ok(Response::new(InstallPluginResponse {
                    plugin: None,
                    quarantined: true,
                    message: format!("rejected and quarantined ({file}): {reason}"),
                }))
            }
            Err(e) => {
                self.audit(
                    &actor,
                    "plugin-install-failed",
                    &serde_json::json!({"error": format!("{e:#}")}),
                )
                .await;
                Err(Status::failed_precondition(format!("{e:#}")))
            }
        }
    }

    async fn enable_plugin(
        &self,
        req: Request<PluginRef>,
    ) -> Result<Response<PluginActionResponse>, Status> {
        let p = self
            .signed_only(
                &req,
                "/itcmdr.Itcmdr/EnablePlugin",
                crate::auth::Role::Admin,
            )
            .await?;
        let (name, version) = crate::packages::enable(&self.pool, &req.get_ref().spec, &p.actor())
            .await
            .map_err(|e| Status::failed_precondition(format!("{e:#}")))?;
        Ok(Response::new(PluginActionResponse {
            message: format!("enabled {name} {version}"),
            name,
            version,
        }))
    }

    async fn disable_plugin(
        &self,
        req: Request<PluginRef>,
    ) -> Result<Response<PluginActionResponse>, Status> {
        let p = self
            .signed_only(
                &req,
                "/itcmdr.Itcmdr/DisablePlugin",
                crate::auth::Role::Admin,
            )
            .await?;
        let name = req
            .get_ref()
            .spec
            .split('@')
            .next()
            .unwrap_or("")
            .to_string();
        crate::packages::disable(&self.pool, &name, &p.actor())
            .await
            .map_err(|e| Status::failed_precondition(format!("{e:#}")))?;
        Ok(Response::new(PluginActionResponse {
            message: format!("disabled {name}"),
            name,
            version: String::new(),
        }))
    }

    async fn remove_plugin(
        &self,
        req: Request<PluginRef>,
    ) -> Result<Response<PluginActionResponse>, Status> {
        let p = self
            .signed_only(
                &req,
                "/itcmdr.Itcmdr/RemovePlugin",
                crate::auth::Role::Admin,
            )
            .await?;
        let r = req.get_ref();
        let (name, version) =
            crate::packages::remove(&self.pool, &self.cfg.layout(), &r.spec, r.force, &p.actor())
                .await
                .map_err(|e| Status::failed_precondition(format!("{e:#}")))?;
        Ok(Response::new(PluginActionResponse {
            message: format!("removed {name} {version}"),
            name,
            version,
        }))
    }

    async fn list_quarantine(
        &self,
        req: Request<ListQuarantineRequest>,
    ) -> Result<Response<ListQuarantineResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/ListQuarantine",
            crate::auth::Role::Admin,
        )
        .await?;
        let l = crate::packages::list_quarantine(&self.cfg.layout()).map_err(internal_err)?;
        Ok(Response::new(ListQuarantineResponse {
            entries: l
                .into_iter()
                .map(|q| QuarantineEntry {
                    ts: q.ts,
                    file: q.file,
                    source: q.source,
                    sha256: q.sha256,
                    reason: q.reason,
                })
                .collect(),
        }))
    }

    async fn preview_schedule(
        &self,
        req: Request<SaveScheduleRequest>,
    ) -> Result<Response<PreviewScheduleResponse>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/PreviewSchedule",
            crate::auth::Role::User,
        )
        .await?;
        let s = req.into_inner().schedule.unwrap_or_default();
        let mut spec = from_proto_schedule(&s);
        let name = if s.name.trim().is_empty() {
            "preview".to_string()
        } else {
            s.name.clone()
        };
        let out = crate::schedule::validate(&name, &mut spec)
            .and_then(|_| crate::schedule::next_times(&spec, chrono::Utc::now(), 5));
        Ok(Response::new(match out {
            Ok(v) => PreviewScheduleResponse {
                next_runs: v.into_iter().map(|t| t.timestamp_millis()).collect(),
                error: String::new(),
            },
            Err(e) => PreviewScheduleResponse {
                next_runs: vec![],
                error: format!("{e:#}"),
            },
        }))
    }

    async fn get_restore_status(
        &self,
        req: Request<GetRestoreStatusRequest>,
    ) -> Result<Response<GetRestoreStatusResponse>, Status> {
        self.authz(
            &req,
            "/itcmdr.Itcmdr/GetRestoreStatus",
            crate::auth::Role::Admin,
        )
        .await?;
        Ok(Response::new(GetRestoreStatusResponse {
            status: crate::restore::read_status(&self.cfg)
                .as_ref()
                .map(to_proto_restore),
        }))
    }

    type WatchUpdatesStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<Update, Status>> + Send + 'static>>;
    async fn watch_updates(
        &self,
        req: Request<WatchUpdatesRequest>,
    ) -> Result<Response<Self::WatchUpdatesStream>, Status> {
        self.read_guard(
            &req,
            "/itcmdr.Itcmdr/WatchUpdates",
            crate::auth::Role::Client,
        )
        .await?;
        // Lagged receivers skip missed events instead of ending the stream.
        let stream =
            BroadcastStream::new(crate::events::subscribe()).filter_map(|item| item.ok().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }
}

fn internal_err<E: std::fmt::Display>(e: E) -> Status {
    Status::internal(e.to_string())
}

fn bad_arg(e: anyhow::Error) -> Status {
    let m = format!("{e:#}");
    if m.starts_with("no schedule ") {
        return Status::not_found(m);
    }
    Status::invalid_argument(m)
}

fn from_proto_schedule(s: &Schedule) -> crate::schedule::Spec {
    crate::schedule::Spec {
        all_machines: s.all_machines,
        machines: s.machines.clone(),
        groups: s.groups.clone(),
        categories: s.categories.clone(),
        plugins: s.plugins.clone(),
        repair: s.repair,
        when: s.when.clone(),
        timezone: s.timezone.clone(),
        guard: s.guard.clone(),
        missed: s.missed.clone(),
        builtin: false,
        ..Default::default()
    }
}

fn to_proto_schedule(s: &crate::schedule::Schedule) -> Schedule {
    let next = if s.enabled {
        crate::schedule::next_times(&s.spec, chrono::Utc::now(), 3)
            .unwrap_or_default()
            .into_iter()
            .map(|t| t.timestamp_millis())
            .collect()
    } else {
        vec![]
    };
    Schedule {
        id: s.id.clone(),
        name: s.name.clone(),
        enabled: s.enabled,
        all_machines: s.spec.all_machines,
        machines: s.spec.machines.clone(),
        groups: s.spec.groups.clone(),
        categories: s.spec.categories.clone(),
        plugins: s.spec.plugins.clone(),
        repair: s.spec.repair,
        when: s.spec.when.clone(),
        timezone: s.spec.timezone.clone(),
        guard: s.spec.guard.clone(),
        missed: s.spec.missed.clone(),
        builtin: s.spec.builtin,
        builtin_key: s.spec.builtin_key.clone(),
        action: s.spec.action.clone(),
        next_runs: next,
        last_fire: s.last_fire_ms,
    }
}

fn key_info(k: crate::auth::KeyInfo) -> KeyInfo {
    KeyInfo {
        kind: k.kind,
        name: k.name,
        role: k.role,
        fingerprint: k.fingerprint,
        created_at: k.created_at,
        revoked_at: k.revoked_at.unwrap_or_default(),
    }
}

fn to_proto_restore(s: &crate::restore::Status) -> RestoreStatus {
    RestoreStatus {
        state: s.state.clone(),
        message: s.message.clone(),
        backup: s.archive.clone(),
        actor: s.actor.clone(),
        started: s.started_ms,
        updated: s.updated_ms,
        safety_copy: s
            .safety_copy
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    }
}

fn value_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn to_setting_item(r: &crate::settings::Resolved) -> SettingItem {
    let (min, max) = match r.def.kind {
        crate::settings::Kind::Int { min, max } => (min, max),
        _ => (0, 0),
    };
    SettingItem {
        key: r.def.key.into(),
        scope: match r.def.scope {
            crate::settings::Scope::Server => "server".into(),
            crate::settings::Scope::Agent => "agent".into(),
        },
        r#type: r.def.kind.name().into(),
        value: value_text(&r.value),
        default_value: value_text(&r.def.default),
        source: r.source.clone(),
        description: r.def.description.into(),
        min,
        max,
        read_only: r.def.read_only,
        applies: r.def.applies.into(),
    }
}

fn to_proto_backup(b: &crate::config::BackupConfig) -> BackupSettings {
    BackupSettings {
        dir: b
            .dir
            .as_ref()
            .map(|d| d.display().to_string())
            .unwrap_or_default(),
        nightly: b.nightly,
        hour: b.hour,
        keep_daily: b.keep_daily as u32,
        keep_weekly: b.keep_weekly as u32,
        keep_blocks: b.keep_blocks as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::metadata::MetadataMap;

    fn cfg(hash: Option<&str>, insecure: bool) -> ServerConfig {
        ServerConfig {
            admin_token_hash: hash.map(|s| s.to_string()),
            allow_insecure_no_auth: insecure,
            ..ServerConfig::default()
        }
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
        assert!(authorize(&cfg(None, true), &meta(None))
            .unwrap()
            .contains("insecure"));
    }

    #[test]
    fn checks_token_hash() {
        let h = sha256_hex("s3cret").to_uppercase();
        let c = cfg(Some(&h), false);
        assert_eq!(authorize(&c, &meta(Some("s3cret"))).unwrap(), "admin");
        assert_eq!(
            authorize(&c, &meta(Some("nope"))).unwrap_err().code(),
            tonic::Code::Unauthenticated
        );
        assert_eq!(
            authorize(&c, &meta(None)).unwrap_err().code(),
            tonic::Code::Unauthenticated
        );
    }
}
