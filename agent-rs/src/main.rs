//! itcmdr-agent: enroll | run | status | --service (Windows SCM).
use anyhow::Result;
use clap::{Parser, Subcommand};
use itcmdr_agent::{link, state};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "itcmdr-agent",
    version,
    about = "itcmdr agent: heartbeat link to the site server (pinned TLS + client certificate)"
)]
struct Cli {
    /// Data folder (default: %ProgramData%\itcmdr\agent on Windows, /var/lib/itcmdr-agent elsewhere, or ITCMDR_AGENT_DIR).
    #[arg(long, global = true)]
    dir: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Run under the Windows Service Control Manager.
    #[arg(long, hide = true)]
    service: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Enroll with a one-time token from `itcmdr-cli agent enroll-token` (or the app).
    Enroll {
        token: String,
        /// Machine name to enroll as (default: this computer's name).
        #[arg(long)]
        machine: Option<String>,
        /// workstation | server (the office server's own agent).
        #[arg(long, default_value = "workstation", value_parser = ["workstation", "server"])]
        role: String,
    },
    /// Run in the foreground (Ctrl+C stops).
    Run {
        /// Token file used when not enrolled yet (default: ..\agent-enroll.token next to the data folder).
        #[arg(long)]
        token_file: Option<PathBuf>,
    },
    /// Show the enrollment.
    Status,
    /// Queue an alert for the server; delivered by the service, also after an outage (v0.17.1).
    Alert {
        /// Alert key (raised on the server as agent:<machine>:<key>; same key = same alert).
        key: String,
        title: String,
        #[arg(long, default_value = "cautious", value_parser = ["critical", "cautious", "informational"])]
        level: String,
        #[arg(long, default_value = "")]
        detail: String,
        /// Resolve the open alert with this key instead of raising one.
        #[arg(long)]
        resolve: bool,
    },
}

fn init_tracing(file: Option<PathBuf>) {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    match file.and_then(|p| {
        std::fs::create_dir_all(p.parent()?).ok()?;
        if std::fs::metadata(&p)
            .map(|m| m.len() > 10 << 20)
            .unwrap_or(false)
        {
            let _ = std::fs::rename(&p, p.with_extension("log.1"));
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
            .ok()
    }) {
        Some(f) => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(f))
            .init(),
        None => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }
}

fn main() {
    if let Err(e) = real_main() {
        // One line, so it survives PowerShell's stderr handling.
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    let dir = cli.dir.clone().unwrap_or_else(state::default_dir);
    if cli.service {
        return service::run(dir);
    }
    let rt = tokio::runtime::Runtime::new()?;
    match cli.cmd {
        Some(Cmd::Enroll {
            token,
            machine,
            role,
        }) => {
            let st = rt.block_on(link::enroll(&dir, &token, machine.as_deref(), &role))?;
            println!(
                "enrolled {} as agent {} with {} (certificate until {})",
                st.machine,
                st.agent_id,
                st.address,
                ms(st.cert_expires_ms)
            );
            Ok(())
        }
        Some(Cmd::Status) => {
            match state::load(&dir)? {
                Some(s) => println!(
                    "{}",
                    serde_json::json!({"agent_id": s.agent_id, "machine": s.machine, "role": s.role, "address": s.address, "server_fingerprint": s.pin, "cert_expires": ms(s.cert_expires_ms), "dir": dir})
                ),
                None => println!("not enrolled (data folder {})", dir.display()),
            }
            Ok(())
        }
        Some(Cmd::Alert {
            key,
            title,
            level,
            detail,
            resolve,
        }) => {
            let p = itcmdr_agent::queue::spool(
                &dir,
                &itcmdr_agent::queue::Item {
                    seq: 0,
                    key,
                    level,
                    title,
                    detail,
                    ts_ms: chrono::Utc::now().timestamp_millis(),
                    resolve,
                    report: None,
                },
            )?;
            println!("queued {}", p.display());
            Ok(())
        }
        Some(Cmd::Run { token_file }) => {
            init_tracing(None);
            let tf = token_file.unwrap_or_else(|| state::default_token_file(&dir));
            let (tx, rx) = tokio::sync::watch::channel(false);
            rt.block_on(async {
                tokio::spawn(async move {
                    let _ = tokio::signal::ctrl_c().await;
                    let _ = tx.send(true);
                });
                link::run(&dir, &tf, rx).await
            })
        }
        None => {
            eprintln!("use enroll, run, status or alert (--help)");
            std::process::exit(2);
        }
    }
}

fn ms(v: i64) -> String {
    chrono::DateTime::from_timestamp_millis(v)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}

#[cfg(not(windows))]
mod service {
    pub fn run(_dir: std::path::PathBuf) -> anyhow::Result<()> {
        anyhow::bail!("--service is only supported on Windows; use `itcmdr-agent run`")
    }
}

#[cfg(windows)]
mod service {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    pub const SERVICE_NAME: &str = "itcmdr-agent";
    static DIR: OnceLock<PathBuf> = OnceLock::new();

    define_windows_service!(ffi_service_main, service_main);

    pub fn run(dir: PathBuf) -> anyhow::Result<()> {
        super::init_tracing(Some(dir.join("logs").join("agent.log")));
        let _ = DIR.set(dir);
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        Ok(())
    }

    fn service_main(_args: Vec<OsString>) {
        if let Err(e) = run_service() {
            tracing::error!(error=%e, "service failed");
        }
    }

    fn status(state: ServiceState, accept: ServiceControlAccept, code: u32) -> ServiceStatus {
        ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(code),
            checkpoint: 0,
            wait_hint: Duration::from_secs(15),
            process_id: None,
        }
    }

    fn run_service() -> anyhow::Result<()> {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let tx = Mutex::new(tx);
        let handler = move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if let Ok(t) = tx.lock() {
                    let _ = t.send(true);
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let handle = service_control_handler::register(SERVICE_NAME, handler)?;
        let dir = DIR
            .get()
            .cloned()
            .unwrap_or_else(itcmdr_agent::state::default_dir);
        let tf = itcmdr_agent::state::default_token_file(&dir);
        handle.set_service_status(status(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            0,
        ))?;
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(itcmdr_agent::link::run(&dir, &tf, rx));
        if let Err(e) = &result {
            tracing::error!(error=%e, "agent exited with error");
        }
        handle.set_service_status(status(
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            if result.is_ok() { 0 } else { 1 },
        ))?;
        result
    }
}
