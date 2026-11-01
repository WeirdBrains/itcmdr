//! itcmdr-adminsvr: serve | init | recover-owner | audit-verify | service (Windows SCM)
//! Settings: flags, the environment, or an env file (--env-file / ITCMDR_ADMINSVR_ENV_FILE; on
//! Windows %ProgramData%\itcmdr-adminsvr\adminsvr.env by default). Values already in the
//! environment win over the file.
use clap::{Parser, Subcommand};
use itcmdr_adminsvr::config::Config;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "itcmdr-adminsvr",
    version,
    about = "itcmdr offsite admin server"
)]
struct Cli {
    /// KEY=VALUE settings file (read before the other flags; the environment wins)
    #[arg(long, env = "ITCMDR_ADMINSVR_ENV_FILE")]
    env_file: Option<PathBuf>,
    /// Database: postgres://user:pass@host/db, or sqlite://<path> (default: SQLite in the data dir)
    #[arg(long, env = "ITCMDR_ADMINSVR_DB_URL", default_value = "")]
    db_url: String,
    #[arg(long, env = "ITCMDR_ADMINSVR_DATA_DIR", default_value_os_t = default_data_dir())]
    data_dir: PathBuf,
    #[arg(long, env = "ITCMDR_ADMINSVR_API_ADDR", default_value = "0.0.0.0:8444")]
    api_addr: String,
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_SITE_ADDR",
        default_value = "0.0.0.0:8443"
    )]
    site_addr: String,
    /// host:port clients use to reach the API (put into enrollment codes)
    #[arg(long, env = "ITCMDR_ADMINSVR_PUBLIC_API", default_value = "")]
    public_api: String,
    /// host:port sites use to reach the SiteLink (put into site tokens)
    #[arg(long, env = "ITCMDR_ADMINSVR_PUBLIC_SITE", default_value = "")]
    public_site: String,
    /// extra TLS certificate names, comma separated
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_TLS_NAMES",
        default_value = "",
        value_delimiter = ','
    )]
    tls_names: Vec<String>,
    #[arg(long, env = "ITCMDR_ADMINSVR_HEARTBEAT_SECS", default_value_t = 60)]
    heartbeat_secs: i64,
    #[arg(long, env = "ITCMDR_ADMINSVR_SITE_MISSED", default_value_t = 5)]
    site_missed: i64,
    /// WebAuthn RP id for passkeys (default: the host of --public-api)
    #[arg(long, env = "ITCMDR_ADMINSVR_RP_ID", default_value = "")]
    rp_id: String,
    /// Extra WebAuthn origins (the web app), comma separated
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_WEB_ORIGINS",
        default_value = "",
        value_delimiter = ','
    )]
    web_origins: Vec<String>,
    /// Push (FCM): Firebase service account JSON file
    #[arg(long, env = "ITCMDR_ADMINSVR_FCM_SERVICE_ACCOUNT")]
    fcm_service_account: Option<PathBuf>,
    #[arg(long, env = "ITCMDR_ADMINSVR_FCM_URL", default_value = "")]
    fcm_url: String,
    /// Push (APNs): the .p8 auth key file
    #[arg(long, env = "ITCMDR_ADMINSVR_APNS_KEY")]
    apns_key: Option<PathBuf>,
    #[arg(long, env = "ITCMDR_ADMINSVR_APNS_KEY_ID", default_value = "")]
    apns_key_id: String,
    #[arg(long, env = "ITCMDR_ADMINSVR_APNS_TEAM_ID", default_value = "")]
    apns_team_id: String,
    /// The admin app's bundle id
    #[arg(long, env = "ITCMDR_ADMINSVR_APNS_TOPIC", default_value = "")]
    apns_topic: String,
    #[arg(long, env = "ITCMDR_ADMINSVR_APNS_URL", default_value = "")]
    apns_url: String,
    /// Lowest alert severity that is pushed: critical | cautious | informational
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_PUSH_MIN_SEVERITY",
        default_value = "cautious"
    )]
    push_min_severity: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the admin API and the SiteLink
    Serve,
    /// Console, once: create the first owner and print their enrollment code
    Init {
        #[arg(long)]
        email: String,
        #[arg(long)]
        name: String,
    },
    /// Console: fresh owner enrollment code when every owner device and the words are lost
    RecoverOwner {
        #[arg(long)]
        email: String,
    },
    /// Check the audit hash chain
    AuditVerify,
    /// Run as a Windows service (started by the service manager; installed by the Setup)
    #[command(hide = true)]
    Service,
}

fn default_data_dir() -> PathBuf {
    if cfg!(windows) {
        program_data().join("data")
    } else {
        PathBuf::from("/var/lib/itcmdr-adminsvr")
    }
}

/// %ProgramData%\itcmdr-adminsvr (Windows)
fn program_data() -> PathBuf {
    PathBuf::from(std::env::var_os("ProgramData").unwrap_or_else(|| "C:\\ProgramData".into()))
        .join("itcmdr-adminsvr")
}

/// Load KEY=VALUE lines (# comments, optional quotes) into the environment, without overriding.
fn load_env_file(p: &std::path::Path) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(p)
        .map_err(|e| anyhow::anyhow!("env file {}: {e}", p.display()))?;
    for (k, v) in parse_env(&text) {
        if std::env::var_os(&k).is_none() {
            std::env::set_var(k, v);
        }
    }
    Ok(())
}

fn parse_env(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                return None;
            }
            let l = l.strip_prefix("export ").unwrap_or(l);
            let (k, v) = l.split_once('=')?;
            let k = k.trim();
            if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return None;
            }
            let v = v.trim();
            let v = if v.len() >= 2
                && ((v.starts_with('"') && v.ends_with('"'))
                    || (v.starts_with('\'') && v.ends_with('\'')))
            {
                &v[1..v.len() - 1]
            } else {
                v
            };
            Some((k.to_string(), v.to_string()))
        })
        .collect()
}

/// The env file named on the command line or in the environment (or the Windows default).
fn pre_env() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut f = std::env::var_os("ITCMDR_ADMINSVR_ENV_FILE").map(PathBuf::from);
    for (i, a) in args.iter().enumerate() {
        if a == "--env-file" {
            f = args.get(i + 1).map(PathBuf::from);
        } else if let Some(v) = a.strip_prefix("--env-file=") {
            f = Some(PathBuf::from(v));
        }
    }
    match f {
        Some(p) if !p.as_os_str().is_empty() => load_env_file(&p),
        _ => {
            let d = program_data().join("adminsvr.env");
            if cfg!(windows) && d.is_file() {
                load_env_file(&d)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn env_file_lines() {
        let v = super::parse_env("# c\n\nA=1\nexport B = \"two words\"\nC='x=y'\nbad line\n-X=1\n");
        assert_eq!(
            v,
            vec![
                ("A".into(), "1".into()),
                ("B".into(), "two words".into()),
                ("C".into(), "x=y".into())
            ]
        );
    }
}

fn main() -> anyhow::Result<()> {
    pre_env()?;
    let mut c = Cli::parse();
    if matches!(c.cmd, Cmd::Service) {
        return service::run(config(c));
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let cmd = std::mem::replace(&mut c.cmd, Cmd::Service);
    let cfg = config(c);
    tokio::runtime::Runtime::new()?.block_on(dispatch(cfg, cmd))
}

fn config(c: Cli) -> Config {
    let host = |a: &str, p: &str| {
        if p.is_empty() {
            a.replace("0.0.0.0", "127.0.0.1")
        } else {
            p.to_string()
        }
    };
    Config {
        db_url: if c.db_url.trim().is_empty() {
            format!("sqlite://{}", c.data_dir.join("adminsvr.db").display())
        } else {
            c.db_url
        },
        data_dir: c.data_dir,
        public_api: host(&c.api_addr, &c.public_api),
        public_site: host(&c.site_addr, &c.public_site),
        api_addr: c.api_addr,
        site_addr: c.site_addr,
        tls_names: c.tls_names.into_iter().filter(|s| !s.is_empty()).collect(),
        heartbeat_secs: c.heartbeat_secs,
        site_missed: c.site_missed,
        max_skew_secs: 300,
        rp_id: c.rp_id,
        web_origins: c
            .web_origins
            .into_iter()
            .filter(|o| !o.trim().is_empty())
            .collect(),
        push: itcmdr_adminsvr::config::PushConfig {
            fcm_service_account: c.fcm_service_account.filter(|p| !p.as_os_str().is_empty()),
            fcm_url: c.fcm_url,
            apns_key: c.apns_key.filter(|p| !p.as_os_str().is_empty()),
            apns_key_id: c.apns_key_id,
            apns_team_id: c.apns_team_id,
            apns_topic: c.apns_topic,
            apns_url: c.apns_url,
            min_severity: c.push_min_severity,
        },
    }
}

async fn dispatch(cfg: Config, cmd: Cmd) -> anyhow::Result<()> {
    match cmd {
        Cmd::Service => unreachable!(),
        Cmd::Serve => {
            itcmdr_adminsvr::run(cfg, async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
        }
        Cmd::Init { email, name } => {
            let code = itcmdr_adminsvr::api::init(&cfg, &email, &name).await?;
            println!("First owner created. Enrollment code (24 h, single use; run `itcmdr-admin enroll <code>` on the owner's device):\n{code}");
            Ok(())
        }
        Cmd::RecoverOwner { email } => {
            let code = itcmdr_adminsvr::api::recover_owner(&cfg, &email).await?;
            println!("Owner recovery code (24 h, single use; audited):\n{code}");
            Ok(())
        }
        Cmd::AuditVerify => {
            let pool = itcmdr_adminsvr::store::connect(&cfg.db_url).await?;
            let v = itcmdr_adminsvr::audit::verify(&pool).await?;
            if v.ok {
                println!("audit chain OK ({} entries)", v.entries);
                Ok(())
            } else {
                anyhow::bail!(
                    "audit chain BROKEN at seq {}: {}",
                    v.first_bad_seq,
                    v.problem
                )
            }
        }
    }
}

#[cfg(not(windows))]
mod service {
    pub fn run(_cfg: super::Config) -> anyhow::Result<()> {
        anyhow::bail!("`service` is for Windows; use systemd, SysV init, OpenRC or launchd (see packaging/adminsvr)")
    }
}

#[cfg(windows)]
mod service {
    use super::Config;
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    pub const SERVICE_NAME: &str = "itcmdr-adminsvr";
    static CFG: OnceLock<Config> = OnceLock::new();

    define_windows_service!(ffi_service_main, service_main);

    pub fn run(cfg: Config) -> anyhow::Result<()> {
        let logs = super::program_data().join("logs");
        let _ = std::fs::create_dir_all(&logs);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs.join("itcmdr-adminsvr.log"))?;
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .init();
        let _ = CFG.set(cfg);
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
        let cfg = CFG.get().cloned().ok_or_else(|| anyhow::anyhow!("no config"))?;
        let rt = tokio::runtime::Runtime::new()?;
        handle.set_service_status(status(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            0,
        ))?;
        let result = rt.block_on(itcmdr_adminsvr::run(cfg, async {
            let _ = rx.await;
        }));
        if let Err(e) = &result {
            tracing::error!(error=%e, "adminsvr exited with error");
        }
        // A non-zero exit makes the service manager apply the recovery actions (restart).
        handle.set_service_status(status(
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            if result.is_ok() { 0 } else { 1 },
        ))?;
        result
    }
}
