//! The one queue path for runs (decision 14): on-demand triggers, schedules and run-now all go
//! through `queue_run`. Agent checks become a request file for the pollers (as before); service
//! plugins run on the server through the plugin runtime and fold into the same request status.
use crate::config::ServerConfig;
use crate::db::DbPool;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RunRequest {
    /// Empty = all machines.
    pub machines: Vec<String>,
    pub categories: Vec<String>,
    /// Service plugins to run on the server (`name` or `name@version`).
    pub plugins: Vec<String>,
    pub report_only: bool,
    pub repair: bool,
    pub force: bool,
    /// Schedule that queued it, if any.
    pub schedule_id: Option<String>,
}

/// Queue a run and return its request id.
pub async fn queue_run(
    pool: &DbPool,
    cfg: &ServerConfig,
    r: &RunRequest,
    actor: &str,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let agent_part = !r.categories.is_empty() || r.plugins.is_empty();
    let options = serde_json::json!({
        "requestId": id,
        "machines": r.machines,
        "reportOnly": r.report_only,
        "repair": r.repair,
        "force": r.force,
        "categories": r.categories,
        "plugins": r.plugins,
        "scheduleId": r.schedule_id,
    });
    crate::db::insert_request(pool, &id, &options.to_string()).await?;
    if agent_part {
        let dir = cfg.requests_dir();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{id}.json")), options.to_string())?;
    }
    if !r.plugins.is_empty() {
        let (pool, cfg, r, actor, id2) = (
            pool.clone(),
            cfg.clone(),
            r.clone(),
            actor.to_string(),
            id.clone(),
        );
        let fut = async move {
            let mut msgs = Vec::new();
            let mut failed = false;
            for p in &r.plugins {
                let spec = crate::runtime::RunSpec {
                    plugin: p.clone(),
                    report_only: r.report_only || !r.repair,
                    request_id: Some(id2.clone()),
                    ..Default::default()
                };
                match crate::runtime::run_plugin(&pool, &cfg, spec, &actor).await {
                    Ok(rep) => {
                        failed |= rep.status != "ok";
                        msgs.push(format!("{}={}", rep.plugin, rep.status));
                    }
                    Err(e) => {
                        failed = true;
                        msgs.push(format!("{p}=error ({e:#})"));
                    }
                }
            }
            // Plugin-only requests are finished here; mixed ones keep folding agent responses.
            if r.categories.is_empty() {
                let st = if failed { "failed" } else { "done" };
                let _ = crate::db::set_request_status(&pool, &id2, st, &msgs.join(", ")).await;
            }
            crate::events::notify("request", "request", &id2);
        };
        tokio::spawn(fut);
    }
    Ok(id)
}
