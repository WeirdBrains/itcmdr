//! itcmdr-server
//!
//! Usage:
//!   itcmdr-server                  run in the console (Ctrl+C stops)
//!   itcmdr-server --service        run under the Windows Service Control Manager
//!   itcmdr-server hash-token <t>   print the SHA-256 hex to put in admin_token_hash
//!   itcmdr-server init             first-run setup: prints the 24 recovery words ONCE
//!   itcmdr-server recover-db-key [--force]   read the words from stdin, restore keys/db.key from the escrow
//!   --data-dir <dir>               (any mode) data dir override; beats ITCMDR_DATA_DIR and server.json
use itcmdr_server::config::ServerConfig;

fn init_tracing(to_file: Option<std::path::PathBuf>) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact();
    if let Some(path) = to_file {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            builder
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(f))
                .init();
            return;
        }
    }
    builder.init();
}

static DATA_DIR_FLAG: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();

/// Load the config and apply `--data-dir`.
fn load_config() -> anyhow::Result<ServerConfig> {
    let mut cfg = ServerConfig::load()?;
    cfg.data_dir_flag = DATA_DIR_FLAG.get().cloned().flatten();
    Ok(cfg)
}

/// Remove `--data-dir <dir>` / `--data-dir=<dir>` from the arguments.
fn take_data_dir(args: &mut Vec<String>) -> anyhow::Result<Option<std::path::PathBuf>> {
    let mut out = None;
    let mut i = 0;
    while i < args.len() {
        if let Some(v) = args[i].strip_prefix("--data-dir=") {
            out = Some(v.into());
            args.remove(i);
        } else if args[i] == "--data-dir" {
            anyhow::ensure!(i + 1 < args.len(), "--data-dir needs a value");
            out = Some(args.remove(i + 1).into());
            args.remove(i);
        } else {
            i += 1;
        }
    }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let _ = DATA_DIR_FLAG.set(take_data_dir(&mut args)?);
    match args.first().map(String::as_str) {
        Some("hash-token") => {
            let t = args
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("usage: itcmdr-server hash-token <token>"))?;
            println!("{}", itcmdr_server::api::sha256_hex(t));
            Ok(())
        }
        Some("init") => {
            let cfg = load_config()?;
            tokio::runtime::Runtime::new()?.block_on(async {
                let pool = itcmdr_server::db::init_db(&cfg).await?;
                let out = itcmdr_server::auth::init(&pool, &cfg.layout()).await?;
                println!("itcmdr is initialized.");
                println!();
                println!("RECOVERY WORDS (shown once, never stored on this server). Write them down and keep them offline:");
                println!();
                for (i, w) in out.words.split(' ').enumerate() {
                    println!("  {:>2}. {w}", i + 1);
                }
                println!();
                println!("master key fingerprint: {}", out.master_fingerprint);
                println!("root client fingerprint: {}", out.root_fingerprint);
                Ok(())
            })
        }
        Some("recover-db-key") => {
            let force = args.iter().any(|a| a == "--force");
            let cfg = load_config()?;
            eprintln!("Enter the 24 recovery words, then press Enter:");
            let mut words = zeroize::Zeroizing::new(String::new());
            std::io::stdin().read_line(&mut words)?;
            itcmdr_server::auth::recover_db_key(&cfg.layout(), &words, force)?;
            println!("keys/db.key restored and protected for this machine.");
            Ok(())
        }
        Some("--service") => service::run(),
        Some("--console") | None => {
            init_tracing(None);
            let cfg = load_config()?;
            tokio::runtime::Runtime::new()?.block_on(itcmdr_server::run(cfg, async {
                let _ = tokio::signal::ctrl_c().await;
            }))
        }
        Some(other) => anyhow::bail!(
            "unknown argument {other:?}; use --console, --service, init, recover-db-key or hash-token <token>"
        ),
    }
}

#[cfg(not(windows))]
mod service {
    pub fn run() -> anyhow::Result<()> {
        anyhow::bail!(
            "--service is only supported on Windows; run without arguments for console mode"
        )
    }
}

#[cfg(windows)]
mod service {
    use std::ffi::OsString;
    use std::sync::Mutex;
    use std::time::Duration;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    pub const SERVICE_NAME: &str = "itcmdr-server";

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> anyhow::Result<()> {
        let log = std::env::var("ProgramData")
            .map(|p| {
                std::path::PathBuf::from(p)
                    .join("itcmdr")
                    .join("server-logs")
                    .join("itcmdr-server.txt")
            })
            .ok();
        super::init_tracing(log);
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
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let tx = Mutex::new(Some(tx));
        let handler = move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if let Some(t) = tx.lock().ok().and_then(|mut g| g.take()) {
                    let _ = t.send(());
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let handle = service_control_handler::register(SERVICE_NAME, handler)?;
        handle.set_service_status(status(
            ServiceState::StartPending,
            ServiceControlAccept::empty(),
            0,
        ))?;
        let result = super::load_config().and_then(|cfg| {
            let rt = tokio::runtime::Runtime::new()?;
            handle.set_service_status(status(
                ServiceState::Running,
                ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
                0,
            ))?;
            rt.block_on(itcmdr_server::run(cfg, async {
                let _ = rx.await;
            }))
        });
        if let Err(e) = &result {
            tracing::error!(error=%e, "server exited with error");
        }
        handle.set_service_status(status(
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            if result.is_ok() { 0 } else { 1 },
        ))?;
        result
    }
}
