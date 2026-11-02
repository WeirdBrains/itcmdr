//! The watchdog program (v0.21.0), shared by `itcmdr-agent-watchdog` and `itcmdr-server-watchdog`:
//! carries out upgrades (swap, health check, rollback) and restarts the service when it stops or
//! its health file goes stale. Installed by the installers only (never by a release).
use crate as wd;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Which service a watchdog looks after.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// itcmdr-agent: `<app>\agent\current\`, data `%ProgramData%\itcmdr\agent`.
    Agent,
    /// itcmdr-server: `<app>\server` (the pre-v0.21 path), data `%ProgramData%\itcmdr`.
    Server,
}

impl Kind {
    pub fn watched(self) -> &'static str {
        match self {
            Kind::Agent => "itcmdr-agent",
            Kind::Server => "itcmdr-server",
        }
    }
    pub fn own_name(self) -> &'static str {
        match self {
            Kind::Agent => "itcmdr-agent-watchdog",
            Kind::Server => "itcmdr-server-watchdog",
        }
    }
}

#[derive(Parser, Clone)]
#[command(version, about = "itcmdr watchdog: upgrades, rollback, restarts")]
pub struct Cli {
    /// Program folder. Agent: the folder holding current\ (default: the folder above this exe's
    /// folder). Server: the server folder itself (default: <folder above this exe's>\server).
    #[arg(long)]
    root: Option<PathBuf>,
    /// Data folder (health.json, upgrade\). Default: %ProgramData%\itcmdr\agent (agent) or
    /// %ProgramData%\itcmdr (server).
    #[arg(long)]
    data: Option<PathBuf>,
    /// Service to watch (default: itcmdr-agent or itcmdr-server).
    #[arg(long)]
    service_name: Option<String>,
    /// Restart the service when its health file is older than this (seconds).
    #[arg(long, default_value_t = 300)]
    stale_secs: i64,
    /// Run one pass and exit (tests, manual use).
    #[arg(long)]
    once: bool,
    /// Run under the Windows Service Control Manager.
    #[arg(long, hide = true)]
    service: bool,
}

pub fn defaults(root: Option<PathBuf>, data: Option<PathBuf>, k: Kind) -> (wd::Dirs, PathBuf) {
    let above = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent()?.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let pd =
        PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into()))
            .join("itcmdr");
    match k {
        Kind::Agent => (
            wd::Dirs::under(&root.unwrap_or(above)),
            data.unwrap_or_else(|| {
                std::env::var("ITCMDR_AGENT_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| pd.join("agent"))
            }),
        ),
        Kind::Server => (
            wd::Dirs::beside(&root.unwrap_or_else(|| above.join("server"))),
            data.unwrap_or(pd),
        ),
    }
}

/// Healthy: the health file shows the new version and a link (agent: to its server; server:
/// gRPC answering with the database open) made after the swap.
pub fn healthy(data: &Path) -> impl Fn(&wd::Request, i64) -> bool + '_ {
    move |r, swapped| {
        wd::read_json::<wd::Health>(&wd::health_path(data))
            .is_some_and(|h| h.version == r.version && h.connected_ms >= swapped)
    }
}

/// One pass: an upgrade request first, else keep the service alive (at most once a minute).
fn pass(c: &Cli, k: Kind, svc: &dyn wd::Service, last_alive: &mut Instant) {
    let (dirs, data) = defaults(c.root.clone(), c.data.clone(), k);
    let check = healthy(&data);
    let r = wd::handle(&dirs, &data, svc, &check, &wd::Timing::default());
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

fn run(c: Cli, k: Kind, stop: impl Fn() -> bool) {
    let (_, data) = defaults(c.root.clone(), c.data.clone(), k);
    wd::log_to(data.join("logs").join(format!("{}.log", k.own_name())));
    let name = c.service_name.clone().unwrap_or_else(|| k.watched().into());
    wd::log(&format!(
        "{} {} watching {name}",
        k.own_name(),
        env!("CARGO_PKG_VERSION")
    ));
    let svc = wd::ScService(name);
    // Give the service a minute after boot before judging it.
    let mut last = Instant::now();
    loop {
        pass(&c, k, &svc, &mut last);
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

/// The whole program for one kind of watchdog.
pub fn main(k: Kind) {
    let c = Cli::parse_from(
        std::iter::once(std::ffi::OsString::from(k.own_name())).chain(std::env::args_os().skip(1)),
    );
    if c.service {
        if let Err(e) = service::start(c, k) {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }
    run(c, k, || false);
}

#[cfg(not(windows))]
mod service {
    pub fn start(_c: super::Cli, _k: super::Kind) -> anyhow::Result<()> {
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

    static CLI: OnceLock<(super::Cli, super::Kind)> = OnceLock::new();
    define_windows_service!(ffi_main, main);

    pub fn start(c: super::Cli, k: super::Kind) -> anyhow::Result<()> {
        let _ = CLI.set((c, k));
        service_dispatcher::start(k.own_name(), ffi_main)?;
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
        let Some((c, k)) = CLI.get().cloned() else {
            return;
        };
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = stop.clone();
        let Ok(h) = service_control_handler::register(k.own_name(), move |ctl| match ctl {
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
        super::run(c, k, || stop.load(Ordering::SeqCst));
        let _ = h.set_service_status(status(ServiceState::Stopped, ServiceControlAccept::empty()));
    }
}
