//! Plugin runtime (decision 6): run an installed, enabled service plugin as a child process over
//! gRPC `PluginService`. Stages: Describe, Verify (if the plugin has it), Execute. A failed Verify
//! skips Execute; Log is called whenever a stage fails. Every run gets an itcmdr.log/v1 file
//! (`ITCMDR_LOG_PATH`, `<data>/plugin-logs/<plugin>/<yyyy-mm-dd>.jsonl`), a `plugin_runs` row and
//! a `plugin-run` audit entry; the day's log is mirrored to `<logs_dir>/plugin-logs/` so the share
//! rotation archives it.
use crate::config::ServerConfig;
use crate::db::DbPool;
use crate::packages::{self, InstalledPlugin};
use anyhow::{anyhow, bail, Context, Result};
use itcmdr_plugin::log::Logger;
use itcmdr_plugin::proto::{DescribeRequest, LogRequest, RunContext, StageResponse};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct RunSpec {
    /// `name` or `name@version` (must be installed and enabled).
    pub plugin: String,
    pub machine: String,
    pub report_only: bool,
    pub args: BTreeMap<String, String>,
    /// On-demand request this run belongs to, if any.
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageOutcome {
    pub stage: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunReport {
    pub run_id: String,
    pub plugin: String,
    pub version: String,
    /// ok | verify-failed | failed | error
    pub status: String,
    pub message: String,
    pub stages: Vec<StageOutcome>,
    pub results: Vec<(String, String, String)>,
    pub log_path: PathBuf,
}

/// Resolve `name[@version]` to an installed, enabled plugin.
pub async fn find_enabled(
    pool: &DbPool,
    cfg: &ServerConfig,
    spec: &str,
) -> Result<InstalledPlugin> {
    let (name, ver) = match spec.split_once('@') {
        Some((n, v)) => (n, Some(v)),
        None => (spec, None),
    };
    let all = packages::list(pool, &cfg.layout()).await?;
    let p = all
        .into_iter()
        .find(|p| p.name == name && ver.is_none_or(|v| v == p.version) && p.enabled)
        .ok_or_else(|| anyhow!("plugin {spec} is not installed and enabled"))?;
    Ok(p)
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// The PowerShell host binary (ITCMDR_PLUGIN_PS_HOST, else next to this exe).
pub fn ps_host() -> PathBuf {
    std::env::var_os("ITCMDR_PLUGIN_PS_HOST")
        .map(PathBuf::from)
        .unwrap_or_else(|| exe_dir().join(exe("itcmdr-plugin-ps")))
}

/// Itcmdr.Plugin.psm1 (ITCMDR_PS_MODULE, else `<app>\agent\` next to the server folder, else next to this exe).
pub fn ps_module() -> PathBuf {
    if let Some(p) = std::env::var_os("ITCMDR_PS_MODULE") {
        return p.into();
    }
    let d = exe_dir();
    let up = d
        .parent()
        .map(|p| p.join("agent").join("Itcmdr.Plugin.psm1"));
    match up {
        Some(p) if p.exists() => p,
        _ => d.join("Itcmdr.Plugin.psm1"),
    }
}

/// Command that starts the plugin's main file.
pub async fn command_for(
    pool: &DbPool,
    p: &InstalledPlugin,
) -> Result<(tokio::process::Command, Vec<String>)> {
    let m = packages::parse_manifest(&std::fs::read(p.dir.join("package.toml"))?)
        .map_err(|r| anyhow!("{}: {}", p.name, r.0))?;
    let main = m
        .files
        .iter()
        .find(|f| f.main)
        .ok_or_else(|| anyhow!("{} has no main file", p.name))?;
    let path = p.dir.join(&main.name);
    let mut c = match main.kind.as_str() {
        "bin" => tokio::process::Command::new(&path),
        "powershell" => {
            let mut c = tokio::process::Command::new(ps_host());
            c.arg("--script").arg(&path);
            c.env("ITCMDR_PS_MODULE", ps_module());
            let shell = crate::settings::get(pool, None, "plugins.powershell")
                .await
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            if !shell.is_empty() {
                c.env("ITCMDR_POWERSHELL", shell);
            }
            c
        }
        other => bail!("{}: main file type {other} cannot be run", p.name),
    };
    c.current_dir(&p.dir).stderr(std::process::Stdio::null());
    Ok((c, m.features))
}

fn stage_err(s: &tonic::Status) -> String {
    if s.message().is_empty() {
        format!("{:?}", s.code())
    } else {
        s.message().to_string()
    }
}

/// Run one plugin through its stages and record the run.
pub async fn run_plugin(
    pool: &DbPool,
    cfg: &ServerConfig,
    spec: RunSpec,
    actor: &str,
) -> Result<RunReport> {
    let p = find_enabled(pool, cfg, &spec.plugin).await?;
    let run_id = uuid::Uuid::new_v4().to_string();
    let log_path = itcmdr_plugin::log::run_log_path(&cfg.layout().plugin_logs(), &p.name);
    let timeout = Duration::from_secs(
        crate::settings::get_i64(pool, None, "plugins.timeout_secs")
            .await
            .max(1) as u64,
    );
    let started = chrono::Utc::now().timestamp_millis();
    let ctx = RunContext {
        run_id: run_id.clone(),
        log_path: log_path.display().to_string(),
        machine: if spec.machine.is_empty() {
            hostname()
        } else {
            spec.machine.clone()
        },
        report_only: spec.report_only,
        args: spec.args.clone().into_iter().collect::<HashMap<_, _>>(),
        stage: String::new(),
    };
    let srvlog = |stage: &str| {
        let mut l = Logger::for_ctx(&p.name, &p.version, &ctx);
        l.stage = stage.into();
        l
    };
    let mut rep = RunReport {
        run_id: run_id.clone(),
        plugin: p.name.clone(),
        version: p.version.clone(),
        status: "error".into(),
        message: String::new(),
        stages: vec![],
        results: vec![],
        log_path: log_path.clone(),
    };
    let _ = srvlog("start").info("run-start", &format!("{} {} by {actor}", p.name, p.version));
    let res = run_stages(pool, &p, &ctx, timeout, &mut rep).await;
    if let Err(e) = res {
        rep.status = "error".into();
        rep.message = format!("{e:#}");
    }
    let _ = srvlog("end").write(&srvlog("end").line(
        if rep.status == "ok" { "info" } else { "error" },
        "run-end",
        &format!("{}: {}", rep.status, rep.message),
        serde_json::Map::new(),
    ));
    let finished = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO plugin_runs(id, plugin, version, machine, started_ms, finished_ms, status, message, log_path, actor, request_id) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&run_id)
    .bind(&p.name)
    .bind(&p.version)
    .bind(&ctx.machine)
    .bind(started)
    .bind(finished)
    .bind(&rep.status)
    .bind(&rep.message)
    .bind(log_path.display().to_string())
    .bind(actor)
    .bind(spec.request_id.as_deref())
    .execute(pool)
    .await?;
    let _ = crate::db::insert_audit(
        pool,
        actor,
        "plugin-run",
        &serde_json::json!({"run_id": run_id, "plugin": p.name, "version": p.version, "status": rep.status,
            "message": rep.message, "request_id": spec.request_id, "report_only": spec.report_only})
        .to_string(),
    )
    .await;
    mirror_to_share(cfg, &p.name, &log_path);
    Ok(rep)
}

async fn run_stages(
    pool: &DbPool,
    p: &InstalledPlugin,
    ctx: &RunContext,
    timeout: Duration,
    rep: &mut RunReport,
) -> Result<()> {
    let (cmd, features) = command_for(pool, p).await?;
    let mut cmd = cmd;
    cmd.env(itcmdr_plugin::ENV_LOG_PATH, &ctx.log_path)
        .env("ITCMDR_RUN_ID", &ctx.run_id)
        .env("ITCMDR_MACHINE", &ctx.machine);
    let launched = itcmdr_plugin::host::launch(cmd, timeout.min(Duration::from_secs(60)))
        .await
        .with_context(|| format!("starting {}", p.name))?;
    let mut c = launched.client.clone();
    let out = async {
        let d = tokio::time::timeout(timeout, c.describe(DescribeRequest {}))
            .await
            .map_err(|_| anyhow!("Describe timed out"))?
            .map_err(|s| anyhow!("Describe failed: {}", stage_err(&s)))?
            .into_inner();
        if d.name != p.name {
            bail!(
                "plugin describes itself as {:?}, installed as {:?}",
                d.name,
                p.name
            );
        }
        let has = |f: &str| d.features.iter().any(|x| x == f) || features.iter().any(|x| x == f);
        let call_log = |c: &mut itcmdr_plugin::host::Client, stage: &str, err: &str| {
            let mut c = c.clone();
            let req = LogRequest {
                ctx: Some(RunContext {
                    stage: "log".into(),
                    ..ctx.clone()
                }),
                failed_stage: stage.into(),
                error: err.into(),
            };
            async move {
                let _ = tokio::time::timeout(timeout, c.log(req)).await;
            }
        };
        for stage in ["verify", "execute"] {
            if stage == "verify" && !has("verify") {
                continue;
            }
            let sctx = RunContext {
                stage: stage.into(),
                ..ctx.clone()
            };
            let r = if stage == "verify" {
                tokio::time::timeout(timeout, c.verify(sctx)).await
            } else {
                tokio::time::timeout(timeout, c.execute(sctx)).await
            };
            let (ok, msg, resp): (bool, String, Option<StageResponse>) = match r {
                Err(_) => (false, format!("{stage} timed out after {timeout:?}"), None),
                Ok(Err(s)) => (false, stage_err(&s), None),
                Ok(Ok(r)) => {
                    let r = r.into_inner();
                    (r.ok, r.message.clone(), Some(r))
                }
            };
            if let Some(r) = &resp {
                rep.results.extend(
                    r.results
                        .iter()
                        .map(|x| (x.check_id.clone(), x.status.clone(), x.message.clone())),
                );
            }
            rep.stages.push(StageOutcome {
                stage: stage.into(),
                ok,
                message: msg.clone(),
            });
            if !ok {
                call_log(&mut c, stage, &msg).await;
                rep.status = if stage == "verify" {
                    "verify-failed"
                } else {
                    "failed"
                }
                .into();
                rep.message = format!("{stage}: {msg}");
                return Ok(());
            }
            rep.message = msg;
        }
        rep.status = "ok".into();
        Ok(())
    }
    .await;
    launched.shutdown().await;
    out
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "server".into())
}

/// Copy the run's day file to `<logs_dir>/plugin-logs/<plugin>/` (best effort).
pub fn mirror_to_share(cfg: &ServerConfig, plugin: &str, log: &Path) {
    if !log.exists() {
        return;
    }
    let dst = cfg.logs_dir.join("plugin-logs").join(plugin);
    if std::fs::create_dir_all(&dst).is_ok() {
        if let Some(n) = log.file_name() {
            if let Err(e) = std::fs::copy(log, dst.join(n)) {
                tracing::warn!(error=%e, "plugin log mirror failed");
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginRunRow {
    pub id: String,
    pub plugin: String,
    pub version: String,
    pub machine: String,
    pub started_ms: i64,
    pub finished_ms: i64,
    pub status: String,
    pub message: String,
    pub request_id: Option<String>,
}

pub async fn list_runs(
    pool: &DbPool,
    plugin: Option<&str>,
    limit: i64,
) -> Result<Vec<PluginRunRow>> {
    type Row = (
        String,
        String,
        String,
        String,
        i64,
        i64,
        String,
        String,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, plugin, version, machine, started_ms, finished_ms, status, message, request_id FROM plugin_runs \
         WHERE (? IS NULL OR plugin = ?) ORDER BY started_ms DESC LIMIT ?",
    )
    .bind(plugin)
    .bind(plugin)
    .bind(limit.clamp(1, 1000))
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| PluginRunRow {
            id: r.0,
            plugin: r.1,
            version: r.2,
            machine: r.3,
            started_ms: r.4,
            finished_ms: r.5,
            status: r.6,
            message: r.7,
            request_id: r.8,
        })
        .collect())
}
