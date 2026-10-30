//! itcmdr-plugin-ps: serves `PluginService` for a PowerShell plugin (decision 6).
//!
//! `itcmdr-plugin-ps --script <main.ps1>`: for every call it runs Windows PowerShell 5.1
//! (`powershell.exe`; `pwsh` elsewhere, override with ITCMDR_POWERSHELL) on the script with a JSON
//! request on stdin, and reads one `ITCMDR-RESULT|{json}` line back. Scripts use the
//! Itcmdr.Plugin module (`Invoke-ItcmdrPlugin`, `Write-ItcmdrLog`, `Write-ItcmdrMetric`), found via
//! ITCMDR_PS_MODULE. The JSON stays between this host and the script; the server only speaks gRPC.
use anyhow::{anyhow, bail, Context, Result};
use itcmdr_plugin::proto::*;
use itcmdr_plugin::Plugin;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const RESULT: &str = "ITCMDR-RESULT|";

struct PsPlugin {
    script: PathBuf,
    shell: String,
    info: DescribeResponse,
}

fn default_shell() -> String {
    std::env::var("ITCMDR_POWERSHELL").unwrap_or_else(|_| {
        if cfg!(windows) {
            "powershell.exe".into()
        } else {
            "pwsh".into()
        }
    })
}

fn call(shell: &str, script: &PathBuf, req: &Value, log_path: &str) -> Result<Value> {
    let mut c = Command::new(shell);
    c.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ])
    .arg(script)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    if !log_path.is_empty() {
        c.env(itcmdr_plugin::ENV_LOG_PATH, log_path);
    }
    if let Some(dir) = script.parent() {
        c.current_dir(dir);
    }
    let mut child = c.spawn().with_context(|| format!("starting {shell}"))?;
    child
        .stdin
        .take()
        .expect("piped")
        .write_all(req.to_string().as_bytes())?;
    let out = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .rev()
        .find_map(|l| l.trim().strip_prefix(RESULT))
        .ok_or_else(|| {
            anyhow!(
                "script printed no {RESULT} line (exit {:?}): {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            )
        })?;
    let v: Value = serde_json::from_str(line).context("result JSON")?;
    if let Some(e) = v.get("error").and_then(Value::as_str) {
        bail!("{e}");
    }
    Ok(v)
}

fn ctx_json(c: &RunContext) -> Value {
    json!({"run_id": c.run_id, "log_path": c.log_path, "machine": c.machine,
           "report_only": c.report_only, "args": c.args, "stage": c.stage})
}

fn to_stage(v: Value) -> StageResponse {
    StageResponse {
        ok: v.get("ok").and_then(Value::as_bool).unwrap_or(false),
        message: v
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        results: match v.get("results") {
            // PowerShell turns a one-element array into the element itself.
            Some(Value::Array(a)) => a.clone(),
            Some(o @ Value::Object(_)) => vec![o.clone()],
            _ => vec![],
        }
        .iter()
        .map(|r| CheckResult {
            check_id: r
                .get("check_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            status: r
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            message: r
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        })
        .collect(),
    }
}

impl PsPlugin {
    fn stage(&self, stage: &str, ctx: &RunContext) -> Result<StageResponse, String> {
        let mut c = ctx.clone();
        c.stage = stage.into();
        call(
            &self.shell,
            &self.script,
            &json!({"stage": stage, "ctx": ctx_json(&c)}),
            &c.log_path,
        )
        .map(to_stage)
        .map_err(|e| format!("{e:#}"))
    }
}

impl Plugin for PsPlugin {
    fn describe(&self) -> DescribeResponse {
        self.info.clone()
    }
    fn verify(&self, ctx: &RunContext) -> Result<StageResponse, String> {
        if !self.info.features.iter().any(|f| f == "verify") {
            return Ok(itcmdr_plugin::ok("no verify stage"));
        }
        self.stage("verify", ctx)
    }
    fn execute(&self, ctx: &RunContext) -> Result<StageResponse, String> {
        self.stage("execute", ctx)
    }
    fn log(&self, ctx: &RunContext, failed_stage: &str, error: &str) {
        if !self.info.features.iter().any(|f| f == "log") {
            return;
        }
        let mut c = ctx.clone();
        c.stage = "log".into();
        let _ = call(
            &self.shell,
            &self.script,
            &json!({"stage": "log", "ctx": ctx_json(&c), "failed_stage": failed_stage, "error": error}),
            &c.log_path,
        );
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut script = std::env::var("ITCMDR_PLUGIN_SCRIPT")
        .ok()
        .map(PathBuf::from);
    while let Some(a) = args.next() {
        if a == "--script" {
            script = args.next().map(PathBuf::from);
        } else {
            bail!("usage: itcmdr-plugin-ps --script <main.ps1>");
        }
    }
    let script = script.ok_or_else(|| anyhow!("usage: itcmdr-plugin-ps --script <main.ps1>"))?;
    let script = script
        .canonicalize()
        .with_context(|| format!("{}", script.display()))?;
    let shell = default_shell();
    let d = call(&shell, &script, &json!({"stage": "describe"}), "")?;
    let s = |k: &str| {
        d.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let info = DescribeResponse {
        name: s("name"),
        version: s("version"),
        description: s("description"),
        features: d
            .get("features")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    };
    itcmdr_plugin::serve(PsPlugin {
        script,
        shell,
        info,
    })
}
