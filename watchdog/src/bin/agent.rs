//! itcmdr-agent-watchdog (v0.21.0): carries out agent upgrades (swap, health check, rollback) and
//! restarts the itcmdr-agent service when it stops or its health file goes stale. Installed by
//! the agent installer only (never by a release), next to `current\`.
use clap::Parser;
use itcmdr_watchdog as wd;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Parser, Clone)]
#[command(
    name = "itcmdr-agent-watchdog",
    version,
    about = "itcmdr agent watchdog: upgrades, rollback, restarts"
)]
struct Cli {
    /// Agent program folder (holds current\, previous\, staged\). Default: the folder above this exe.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Agent data folder (health.json, upgrade\). Default: %ProgramData%\itcmdr\agent.
    #[arg(long)]
    data: Option<PathBuf>,
    #[arg(long, default_value = "itcmdr-agent")]
    service_name: String,
    /// Restart the agent when its health file is older than this (seconds).
    #[arg(long, default_value_t = 300)]
    stale_secs: i64,
    /// Run one pass and exit (tests, manual use).
    #[arg(long)]
    once: bool,
    /// Run under the Windows Service Control Manager.
    #[arg(long, hide = true)]
    service: bool,
}

fn defaults(c: &Cli) -> (PathBuf, PathBuf) {
    let root = c.root.clone().unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent()?.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let data = c.data.clone().unwrap_or_else(|| {
        std::env::var("ITCMDR_AGENT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
                PathBuf::from(pd).join("itcmdr").join("agent")
            })
    });
    (root, data)
}

/// The upgraded agent is healthy when its health file shows the new version and a server link
/// made after the swap.
fn agent_healthy(data: &std::path::Path) -> impl Fn(&wd::Request, i64) -> bool + '_ {
    move |r, swapped| {
        wd::read_json::<wd::Health>(&wd::health_path(data))
            .is_some_and(|h| h.version == r.version && h.connected_ms >= swapped)
    }
}

/// One pass: an upgrade request first, else keep the agent alive (at most once a minute).
fn pass(c: &Cli, svc: &dyn wd::Service, last_alive: &mut Instant) {
    let (root, data) = defaults(c);
    let check = agent_healthy(&data);
    let r = wd::handle(
        &wd::Dirs::under(&root),
        &data,
        svc,
        &check,
        &wd::Timing::default(),
    );
    match r {
        Ok(Some(o)) => wd::log(&format!("upgrade result: {o:?}")),
        Ok(None) => {
            if last_alive.elapsed() >= Duration::from_secs(60) || c.once {
                *last_alive = Instant::now();
                if let Some(did) = wd::keep_alive(&data, svc, c.stale_secs, wd::now_ms()) {
                    wd::log(&did);
                }
            }
        }
        Err(e) => wd::log(&format!("upgrade failed: {e:#}")),
    }
}

fn run(c: Cli, stop: impl Fn() -> bool) {
    let (_, data) = defaults(&c);
    wd::log_to(data.join("logs").join("watchdog.log"));
    wd::log(&format!(
        "itcmdr-agent-watchdog {} watching {}",
        env!("CARGO_PKG_VERSION"),
        c.service_name
    ));
    let svc = wd::ScService(c.service_name.clone());
    // Give the agent a minute after boot before judging it.
    let mut last = Instant::now();
    loop {
        pass(&c, &svc, &mut last);
        if c.once {
            return;
        }
        for _ in 0..5 {
            if stop() {
                return;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

fn main() {
    let c = Cli::parse();
    if c.service {
        if let Err(e) = service::start(c) {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }
    run(c, || false);
}

#[cfg(not(windows))]
mod service {
    pub fn start(_c: super::Cli) -> anyhow::Result<()> {
        anyhow::bail!("--service is only supported on Windows")
    }
}

#[cfg(windows)]
mod service {
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, OnceLock};
    use std::time::Duration;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    const NAME: &str = "itcmdr-agent-watchdog";
    static CLI: OnceLock<super::Cli> = OnceLock::new();
    define_windows_service!(ffi_main, main);

    pub fn start(c: super::Cli) -> anyhow::Result<()> {
        let _ = CLI.set(c);
        service_dispatcher::start(NAME, ffi_main)?;
        Ok(())
    }

    fn status(state: ServiceState, accept: ServiceControlAccept) -> ServiceStatus {
        ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        }
    }

    fn main(_a: Vec<OsString>) {
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = stop.clone();
        let Ok(h) = service_control_handler::register(NAME, move |c| match c {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                s2.store(true, Ordering::SeqCst);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }) else {
            return;
        };
        let _ = h.set_service_status(status(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        ));
        let c = CLI.get().cloned().expect("cli");
        super::run(c, || stop.load(Ordering::SeqCst));
        let _ = h.set_service_status(status(ServiceState::Stopped, ServiceControlAccept::empty()));
    }
}
