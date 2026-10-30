//! App-triggered restore, server side (decision 13). A running server can't swap its own database,
//! so `StageRestore` verifies the archive with the words, writes `restore/job.json` and starts the
//! separate `itcmdr-restore` helper (the words go to it over stdin only; never to disk or logs).
//! The helper stops the service, restores (which takes the `pre-restore-<ts>` safety copy and leaves
//! the live server.json alone), starts the service and health-checks it: the DB opens with the
//! current schema and gRPC answers. If that fails it puts the safety copy back, restarts and reports
//! `rolled-back`. Progress is in `restore/status.json` (`GetRestoreStatus`).
use crate::config::ServerConfig;
use crate::db::DbPool;
use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const TERMINAL: [&str; 3] = ["done", "rolled-back", "failed"];
/// Test hook: make the helper's health check fail (forces a rollback).
pub const ENV_FORCE_HEALTH_FAIL: &str = "ITCMDR_RESTORE_FORCE_HEALTH_FAIL";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Job {
    pub archive: PathBuf,
    pub actor: String,
    pub staged_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Status {
    /// staged, stopping, restoring, starting, health-check, rolling-back, done, rolled-back, failed
    pub state: String,
    pub message: String,
    pub archive: String,
    pub actor: String,
    pub started_ms: i64,
    pub updated_ms: i64,
    pub safety_copy: Option<PathBuf>,
    pub steps: Vec<(i64, String, String)>,
}

pub fn dir(cfg: &ServerConfig) -> PathBuf {
    cfg.layout().root.join("restore")
}

pub fn read_status(cfg: &ServerConfig) -> Option<Status> {
    serde_json::from_slice(&std::fs::read(dir(cfg).join("status.json")).ok()?).ok()
}

fn write_json(p: &Path, v: &impl Serialize) -> Result<()> {
    std::fs::create_dir_all(p.parent().unwrap())?;
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    std::fs::rename(&tmp, p)?;
    Ok(())
}

fn step(cfg: &ServerConfig, st: &mut Status, state: &str, msg: &str) {
    let now = chrono::Utc::now().timestamp_millis();
    st.state = state.into();
    st.message = msg.into();
    st.updated_ms = now;
    st.steps.push((now, state.into(), msg.into()));
    if let Err(e) = write_json(&dir(cfg).join("status.json"), st) {
        eprintln!("cannot write restore status: {e:#}");
    }
}

/// A backup file name in the backup folder (no paths).
pub async fn archive_path(pool: &DbPool, cfg: &ServerConfig, name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty()
            && !name.contains(['/', '\\'])
            && !name.contains("..")
            && name.ends_with(".itcb"),
        "give a backup name from ListBackups (e.g. itcmdr-....itcb)"
    );
    let p = crate::settings::backup_dir(pool, cfg).await?.join(name);
    ensure!(p.is_file(), "no backup {name} in the backup folder");
    Ok(p)
}

fn helper_exe() -> PathBuf {
    if let Some(p) = std::env::var_os("ITCMDR_RESTORE_BIN") {
        return p.into();
    }
    let n = if cfg!(windows) {
        "itcmdr-restore.exe"
    } else {
        "itcmdr-restore"
    };
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(n)))
        .unwrap_or_else(|| n.into())
}

/// Verify, write the job and start the helper. Owner only (checked by the caller).
pub async fn stage(
    pool: &DbPool,
    cfg: &ServerConfig,
    name: &str,
    words: &str,
    actor: &str,
) -> Result<Status> {
    if let Some(s) = read_status(cfg) {
        let fresh = chrono::Utc::now().timestamp_millis() - s.updated_ms < 3_600_000;
        if !TERMINAL.contains(&s.state.as_str()) && fresh {
            bail!("a restore is already {} ({})", s.state, s.archive);
        }
    }
    let archive = archive_path(pool, cfg, name).await?;
    // Full verification and decryption with the words; nothing is changed.
    crate::backup::restore(cfg, &archive, words, true)
        .await
        .context("archive verification")?;
    let now = chrono::Utc::now().timestamp_millis();
    write_json(
        &dir(cfg).join("job.json"),
        &Job {
            archive: archive.clone(),
            actor: actor.into(),
            staged_ms: now,
        },
    )?;
    let mut st = Status {
        archive: name.into(),
        actor: actor.into(),
        started_ms: now,
        ..Default::default()
    };
    step(
        cfg,
        &mut st,
        "staged",
        "archive verified; starting the restore helper",
    );
    let mut cmd = std::process::Command::new(helper_exe());
    cmd.arg("--job").arg(dir(cfg).join("job.json"));
    if let Some(d) = &cfg.data_dir_flag {
        cmd.arg("--data-dir").arg(d);
    }
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP: survives the service stopping.
        cmd.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            step(
                cfg,
                &mut st,
                "failed",
                &format!("cannot start itcmdr-restore: {e}"),
            );
            bail!("cannot start itcmdr-restore: {e}");
        }
    };
    {
        let mut w = child.stdin.take().expect("piped");
        w.write_all(words.trim().as_bytes())?;
        w.write_all(b"\n")?;
    }
    // Reaped in the background; on Windows it outlives the service on purpose.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(st)
}

fn run_cmd(argv: &[String]) -> Result<()> {
    if argv.is_empty() {
        return Ok(());
    }
    let out = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .with_context(|| format!("running {}", argv.join(" ")))?;
    ensure!(
        out.status.success(),
        "{} exited with {:?}",
        argv.join(" "),
        out.status.code()
    );
    Ok(())
}

fn probe_addr(cfg: &ServerConfig) -> String {
    let a = cfg
        .bind_addr
        .replace("0.0.0.0", "127.0.0.1")
        .replace("[::]", "[::1]");
    format!("http://{a}")
}

/// gRPC answers at all (any status other than a transport failure).
async fn grpc_answers(cfg: &ServerConfig) -> bool {
    use crate::generated::itcmdr::itcmdr_client::ItcmdrClient;
    let Ok(ep) = tonic::transport::Endpoint::from_shared(probe_addr(cfg)) else {
        return false;
    };
    let Ok(ch) = ep.connect_timeout(Duration::from_secs(3)).connect().await else {
        return false;
    };
    let mut c = ItcmdrClient::new(ch);
    match tokio::time::timeout(
        Duration::from_secs(5),
        c.list_machines(crate::generated::itcmdr::ListMachinesRequest {}),
    )
    .await
    {
        Ok(Ok(_)) => true,
        Ok(Err(s)) => !matches!(s.code(), tonic::Code::Unavailable | tonic::Code::Unknown),
        Err(_) => false,
    }
}

/// Health check after the restart: DB opens with this schema and passes integrity_check, and
/// gRPC answers, within `timeout`.
pub async fn health(cfg: &ServerConfig, timeout: Duration) -> Result<()> {
    if std::env::var_os(ENV_FORCE_HEALTH_FAIL).is_some() {
        bail!("health check forced to fail ({ENV_FORCE_HEALTH_FAIL})");
    }
    let pool = crate::db::init_db(cfg)
        .await
        .context("database does not open")?;
    let v: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    let ok: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    ensure!(
        v == crate::db::SCHEMA_VERSION,
        "schema {v}, expected {}",
        crate::db::SCHEMA_VERSION
    );
    ensure!(ok == "ok", "integrity_check: {ok}");
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if grpc_answers(cfg).await {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            bail!(
                "gRPC did not answer at {} within {timeout:?}",
                probe_addr(cfg)
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Put the pre-restore safety copy back.
pub fn rollback(cfg: &ServerConfig, safety: &Path) -> Result<()> {
    let layout = cfg.layout();
    let dbp = cfg.database_path();
    let dbname = dbp.file_name().unwrap().to_string_lossy().to_string();
    let escrow = crate::auth::escrow_path(&layout);
    let escrow_name = escrow.file_name().unwrap().to_string_lossy().to_string();
    for suf in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suf}", dbp.display()));
    }
    for e in std::fs::read_dir(safety)? {
        let e = e?;
        let n = e.file_name().to_string_lossy().to_string();
        if e.file_type()?.is_dir() {
            continue;
        }
        let dst = if n == dbname || n == format!("{dbname}-wal") || n == format!("{dbname}-shm") {
            dbp.parent().unwrap().join(&n)
        } else if n == escrow_name {
            escrow.clone()
        } else if n == "db.key" || n == "root" {
            layout.keys().join(&n)
        } else if n == "quarantine-index.jsonl" {
            crate::packages::quarantine_index(&layout)
        } else {
            continue;
        };
        let tmp = dst.with_extension("rollback");
        std::fs::copy(e.path(), &tmp)?;
        std::fs::rename(&tmp, &dst)?;
    }
    let sp = safety.join("service_plugins");
    let live = layout.service_plugins();
    if live.exists() {
        std::fs::remove_dir_all(&live)?;
    }
    std::fs::create_dir_all(&live)?;
    if sp.is_dir() {
        for e in walkdir::WalkDir::new(&sp)
            .into_iter()
            .filter_map(Result::ok)
        {
            let rel = e.path().strip_prefix(&sp)?;
            let d = live.join(rel);
            if e.file_type().is_dir() {
                std::fs::create_dir_all(&d)?;
            } else {
                std::fs::copy(e.path(), &d)?;
            }
        }
    }
    Ok(())
}

async fn audit(cfg: &ServerConfig, actor: &str, action: &str, detail: serde_json::Value) {
    if let Ok(pool) = crate::db::init_db(cfg).await {
        let _ = crate::db::insert_audit(&pool, actor, action, &detail.to_string()).await;
        pool.close().await;
    }
}

/// The helper's work (itcmdr-restore). Status goes to restore/status.json throughout.
pub async fn run_helper(cfg: &ServerConfig, job_path: &Path, words: &str) -> Result<Status> {
    let job: Job = serde_json::from_slice(&std::fs::read(job_path)?).context("restore job")?;
    let mut st = read_status(cfg).unwrap_or_default();
    st.archive = job
        .archive
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    st.actor = job.actor.clone();
    let timeout = {
        let pool = crate::db::init_db(cfg).await?;
        let t = crate::settings::get_i64(&pool, Some(cfg), "restore.health_timeout_secs").await;
        pool.close().await;
        Duration::from_secs(t.max(5) as u64)
    };
    step(
        cfg,
        &mut st,
        "stopping",
        &format!(
            "stopping the service ({})",
            cfg.restore.stop_command.join(" ")
        ),
    );
    if let Err(e) = run_cmd(&cfg.restore.stop_command) {
        step(
            cfg,
            &mut st,
            "failed",
            &format!("could not stop the service: {e:#}; nothing was changed"),
        );
        return Ok(st);
    }
    if !cfg.restore.stop_command.is_empty() {
        // Wait until the old process stopped answering.
        for _ in 0..60 {
            if !grpc_answers(cfg).await {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    step(
        cfg,
        &mut st,
        "restoring",
        "taking the safety copy and restoring",
    );
    let rep = crate::backup::restore(cfg, &job.archive, words, false).await;
    let failure = match rep {
        Ok(r) => {
            st.safety_copy = r.safety_copy.clone();
            step(
                cfg,
                &mut st,
                "starting",
                &format!(
                    "restored; starting the service ({})",
                    cfg.restore.start_command.join(" ")
                ),
            );
            match run_cmd(&cfg.restore.start_command) {
                Err(e) => Some(format!("could not start the service: {e:#}")),
                Ok(()) => {
                    step(
                        cfg,
                        &mut st,
                        "health-check",
                        "waiting for the restarted server",
                    );
                    health(cfg, timeout)
                        .await
                        .err()
                        .map(|e| format!("health check failed: {e:#}"))
                }
            }
        }
        Err(e) => Some(format!("restore failed: {e:#}")),
    };
    let Some(why) = failure else {
        step(cfg, &mut st, "done", "restored and healthy");
        audit(
            cfg,
            &job.actor,
            "restore-done",
            serde_json::json!({"archive": st.archive}),
        )
        .await;
        let _ = std::fs::remove_file(job_path);
        return Ok(st);
    };
    let Some(safety) = st.safety_copy.clone() else {
        step(
            cfg,
            &mut st,
            "failed",
            &format!("{why}; there was no safety copy to roll back to"),
        );
        let _ = run_cmd(&cfg.restore.start_command);
        return Ok(st);
    };
    step(
        cfg,
        &mut st,
        "rolling-back",
        &format!("{why}; putting {} back", safety.display()),
    );
    let _ = run_cmd(&cfg.restore.stop_command);
    let rb = rollback(cfg, &safety);
    let _ = run_cmd(&cfg.restore.start_command);
    match rb {
        Ok(()) => {
            step(
                cfg,
                &mut st,
                "rolled-back",
                &format!("{why}; the previous state was restored"),
            );
            audit(
                cfg,
                &job.actor,
                "restore-rolled-back",
                serde_json::json!({"archive": st.archive, "reason": why}),
            )
            .await;
        }
        Err(e) => step(
            cfg,
            &mut st,
            "failed",
            &format!(
                "{why}; ROLLBACK FAILED: {e:#} (safety copy at {})",
                safety.display()
            ),
        ),
    }
    let _ = std::fs::remove_file(job_path);
    Ok(st)
}

/// Parse `--job <path> [--data-dir <dir>]`.
pub fn parse_args(args: &[String]) -> Result<(PathBuf, Option<PathBuf>)> {
    let (mut job, mut dd) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--job" => job = it.next().map(PathBuf::from),
            "--data-dir" => dd = it.next().map(PathBuf::from),
            other => bail!("unknown argument {other}"),
        }
    }
    Ok((
        job.ok_or_else(|| {
            anyhow!(
                "usage: itcmdr-restore --job <restore/job.json> [--data-dir DIR] (words on stdin)"
            )
        })?,
        dd,
    ))
}
