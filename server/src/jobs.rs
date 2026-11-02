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
    // v0.22.0: with the share retired (agent state), agent checks go to the Rust agents as RunNow
    // ("*" = every plugin the agent holds), never to requests/ on the share.
    if agent_part && crate::cutover::share_retired(pool).await {
        let mut args = std::collections::BTreeMap::new();
        args.insert("requestId".to_string(), id.clone());
        args.insert("reportOnly".to_string(), r.report_only.to_string());
        args.insert("repair".to_string(), r.repair.to_string());
        args.insert("force".to_string(), r.force.to_string());
        if !r.categories.is_empty() {
            args.insert("categories".to_string(), r.categories.join(","));
        }
        let mut sent = vec![];
        for a in crate::agents::list(pool, false).await? {
            if r.machines.is_empty()
                || r.machines
                    .iter()
                    .any(|m| m.eq_ignore_ascii_case(&a.machine))
            {
                // Agents before 0.22.0 do not know "*": send them one request per plugin they
                // reported as installed instead.
                let names: Vec<String> = if supports_run_all(&a.version) {
                    vec!["*".to_string()]
                } else {
                    crate::agent_plugins::list_plugins(pool, Some(&a.machine))
                        .await?
                        .iter()
                        .filter(|p| p["ok"].as_bool().unwrap_or(false))
                        .filter_map(|p| p["name"].as_str().map(str::to_string))
                        .collect()
                };
                for n in &names {
                    crate::agent_plugins::request_run(pool, &a.machine, n, &args, actor).await?;
                }
                if !names.is_empty() {
                    sent.push(a.machine);
                }
            }
        }
        if r.plugins.is_empty() {
            let msg = if sent.is_empty() {
                "no enrolled agent matched".to_string()
            } else {
                format!(
                    "sent to {} agent(s) over the link; results arrive as agent runs",
                    sent.len()
                )
            };
            let st = if sent.is_empty() { "failed" } else { "done" };
            crate::db::set_request_status(pool, &id, st, &msg).await?;
        }
    } else if agent_part {
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

/// v0.22.0: agents from 0.22.0 on expand RunNow "*" to every installed plugin.
pub fn supports_run_all(agent_version: &str) -> bool {
    itcmdr_agentlink::version_cmp(agent_version, "0.22.0") != std::cmp::Ordering::Less
}
