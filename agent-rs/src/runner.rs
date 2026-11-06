//! v0.18.0: run one installed plugin through its stages (Describe, Verify if it has it, Execute;
//! Log on failure), always report-only, and build the RunReport. Same contract as the server's
//! runtime (server/src/runtime.rs): plugins are child processes over gRPC PluginService.
use crate::plugins;
use anyhow::{anyhow, bail, Context, Result};
use itcmdr_agentlink::pb;
use itcmdr_plugin::proto::{DescribeRequest, LogRequest, RunContext, StageResponse};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Longest a stage may take.
pub const STAGE_TIMEOUT: Duration = Duration::from_secs(600);

/// How long a plugin may take to print its handshake: 60s by default, or
/// `ITCMDR_PLUGIN_START_SECS` (a PowerShell plugin's cold start can take longer on a slow or
/// overloaded machine).
pub fn start_timeout() -> Duration {
    let secs = std::env::var("ITCMDR_PLUGIN_START_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(60);
    Duration::from_secs(secs)
}
/// Caps on what a report carries.
const MAX_LOG_BYTES: usize = 256 * 1024;
const MAX_ARTIFACT_BYTES: usize = 1024 * 1024;
const MAX_ARTIFACTS_TOTAL: usize = 4 * 1024 * 1024;

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

pub fn ps_host() -> PathBuf {
    std::env::var_os("ITCMDR_PLUGIN_PS_HOST")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            exe_dir().join(if cfg!(windows) {
                "itcmdr-plugin-ps.exe"
            } else {
                "itcmdr-plugin-ps"
            })
        })
}

pub fn ps_module() -> PathBuf {
    std::env::var_os("ITCMDR_PS_MODULE")
        .map(PathBuf::from)
        .unwrap_or_else(|| exe_dir().join("Itcmdr.Plugin.psm1"))
}

fn stage_err(s: &tonic::Status) -> String {
    if s.message().is_empty() {
        format!("{:?}", s.code())
    } else {
        s.message().to_string()
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Run `name` on this machine. Never fails: problems become a report with status `error`.
pub async fn run(
    dir: &Path,
    machine: &str,
    name: &str,
    args: &BTreeMap<String, String>,
    trigger: &str,
) -> pb::RunReport {
    let run_id = uuid::Uuid::new_v4().to_string();
    let mut rep = pb::RunReport {
        run_id: run_id.clone(),
        plugin: name.into(),
        trigger: trigger.into(),
        started_ms: now(),
        status: "error".into(),
        report_only: crate::config::report_only(dir),
        ..Default::default()
    };
    let pdir = plugins::plugin_dir(dir, name);
    let log_path = itcmdr_plugin::log::run_log_path(&dir.join("plugin-logs"), name);
    let art_dir = dir.join("artifacts").join(&run_id);
    let mut errors: Vec<itcmdr_common::PluginErrorRecord> = vec![];
    let r = async {
        let m = plugins::read_manifest(&pdir)
            .with_context(|| format!("plugin {name} is not installed"))?;
        rep.version = m.version.clone();
        let ctx = RunContext {
            run_id: run_id.clone(),
            log_path: log_path.display().to_string(),
            machine: machine.into(),
            report_only: rep.report_only,
            args: args.clone().into_iter().collect::<HashMap<_, _>>(),
            stage: String::new(),
        };
        stages(&pdir, &m, &ctx, &art_dir, &mut rep, &mut errors).await
    }
    .await;
    if let Err(e) = r {
        rep.status = "error".into();
        rep.message = format!("{e:#}");
    }
    rep.finished_ms = now();
    rep.errors_json = serde_json::to_string(&errors).unwrap_or_else(|_| "[]".into());
    // This run's log lines.
    if let Ok(t) = std::fs::read_to_string(&log_path) {
        let mut s = String::new();
        for l in t.lines().filter(|l| l.contains(&run_id)) {
            if s.len() + l.len() + 1 > MAX_LOG_BYTES {
                break;
            }
            s.push_str(l);
            s.push('\n');
        }
        rep.log_lines = s;
    }
    // Artifacts the plugin left in ITCMDR_ARTIFACT_DIR.
    let mut total = 0usize;
    if let Ok(rd) = std::fs::read_dir(&art_dir) {
        for e in rd.flatten() {
            let Ok(b) = std::fs::read(e.path()) else {
                continue;
            };
            if !e.path().is_file()
                || b.len() > MAX_ARTIFACT_BYTES
                || total + b.len() > MAX_ARTIFACTS_TOTAL
            {
                continue;
            }
            total += b.len();
            rep.artifacts.push(pb::Artifact {
                name: e.file_name().to_string_lossy().to_string(),
                sha256: itcmdr_agentlink::sha256_hex(&b),
                data: b,
            });
        }
    }
    let _ = std::fs::remove_dir_all(&art_dir);
    rep
}

async fn stages(
    pdir: &Path,
    m: &plugins::Manifest,
    ctx: &RunContext,
    art_dir: &Path,
    rep: &mut pb::RunReport,
    errors: &mut Vec<itcmdr_common::PluginErrorRecord>,
) -> Result<()> {
    // Pre-spawn platform gate: when supported_platforms is set, refuse to launch on others.
    let cur = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    if !m.supported_platforms.is_empty()
        && !m
            .supported_platforms
            .iter()
            .any(|p| p.eq_ignore_ascii_case(cur))
    {
        bail!(
            "unsupported platform: {} (supported: {})",
            cur,
            m.supported_platforms.join(", ")
        );
    }
    let main = m
        .files
        .iter()
        .find(|f| f.main)
        .ok_or_else(|| anyhow!("{} has no main file", m.name))?;
    let path = pdir.join(&main.name);
    let mut cmd = match main.kind.as_str() {
        "bin" => tokio::process::Command::new(&path),
        "powershell" => {
            let mut c = tokio::process::Command::new(ps_host());
            c.arg("--script").arg(&path);
            c.env("ITCMDR_PS_MODULE", ps_module());
            c
        }
        other => bail!("{}: main file type {other} cannot be run", m.name),
    };
    let default_crit = m
        .default_error_criticality
        .as_deref()
        .and_then(itcmdr_common::ErrorCriticality::parse)
        .unwrap_or_default();
    std::fs::create_dir_all(art_dir)?;
    cmd.current_dir(pdir)
        .stderr(std::process::Stdio::null())
        .env(itcmdr_plugin::ENV_LOG_PATH, &ctx.log_path)
        .env("ITCMDR_RUN_ID", &ctx.run_id)
        .env("ITCMDR_MACHINE", &ctx.machine)
        .env("ITCMDR_REPORT_ONLY", "1")
        .env("ITCMDR_ARTIFACT_DIR", art_dir);
    let launched = itcmdr_plugin::host::launch(cmd, start_timeout())
        .await
        .with_context(|| format!("starting {}", m.name))?;
    let mut c = launched.client.clone();
    let t = STAGE_TIMEOUT;
    let out = async {
        let d = tokio::time::timeout(t, c.describe(DescribeRequest {}))
            .await
            .map_err(|_| anyhow!("Describe timed out"))?
            .map_err(|s| anyhow!("Describe failed: {}", stage_err(&s)))?
            .into_inner();
        if d.name != m.name {
            bail!(
                "plugin describes itself as {:?}, installed as {:?}",
                d.name,
                m.name
            );
        }
        let has = |f: &str| d.features.iter().any(|x| x == f) || m.features.iter().any(|x| x == f);
        for stage in ["verify", "execute"] {
            if stage == "verify" && !has("verify") {
                continue;
            }
            let sctx = RunContext {
                stage: stage.into(),
                ..ctx.clone()
            };
            let r = if stage == "verify" {
                tokio::time::timeout(t, c.verify(sctx)).await
            } else {
                tokio::time::timeout(t, c.execute(sctx)).await
            };
            let (ok, msg, resp): (bool, String, Option<StageResponse>) = match r {
                Err(_) => (false, format!("{stage} timed out after {t:?}"), None),
                Ok(Err(s)) => (false, stage_err(&s), None),
                Ok(Ok(r)) => {
                    let r = r.into_inner();
                    (r.ok, r.message.clone(), Some(r))
                }
            };
            match resp.as_ref().and_then(|r| r.error.as_ref()) {
                Some(e) => {
                    let mut rec = e.to_record(default_crit);
                    if rec.service_plugin.is_empty() {
                        rec.service_plugin = m.name.clone();
                    }
                    if rec.error.is_empty() {
                        rec.error = msg.clone();
                    }
                    errors.push(rec);
                }
                None if !ok => {
                    let mut rec =
                        itcmdr_common::PluginErrorRecord::new(&m.name, default_crit, &msg);
                    rec.kind = if msg.contains("timed out") {
                        "timeout"
                    } else {
                        "stage-failed"
                    }
                    .into();
                    rec.context.insert("stage".into(), stage.into());
                    errors.push(rec);
                }
                None => {}
            }
            if let Some(r) = &resp {
                rep.results
                    .extend(r.results.iter().map(|x| pb::CheckResult {
                        check_id: x.check_id.clone(),
                        status: x.status.clone(),
                        message: x.message.clone(),
                    }));
            }
            rep.stages.push(pb::StageResult {
                stage: stage.into(),
                ok,
                message: msg.clone(),
            });
            if !ok {
                let req = LogRequest {
                    ctx: Some(RunContext {
                        stage: "log".into(),
                        ..ctx.clone()
                    }),
                    failed_stage: stage.into(),
                    error: msg.clone(),
                };
                let _ = tokio::time::timeout(t, c.log(req)).await;
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
