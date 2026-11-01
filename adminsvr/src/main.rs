//! itcmdr-adminsvr: serve | init | recover-owner | audit-verify
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
    /// Database: postgres://user:pass@host/db, or sqlite://<path> (default: SQLite in the data dir)
    #[arg(long, env = "ITCMDR_ADMINSVR_DB_URL", default_value = "")]
    db_url: String,
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_DATA_DIR",
        default_value = "/var/lib/itcmdr-adminsvr"
    )]
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
    #[arg(long, env = "ITCMDR_ADMINSVR_PUSH_MIN_SEVERITY", default_value = "cautious")]
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let c = Cli::parse();
    let host = |a: &str, p: &str| {
        if p.is_empty() {
            a.replace("0.0.0.0", "127.0.0.1")
        } else {
            p.to_string()
        }
    };
    let cfg = Config {
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
    };
    match c.cmd {
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
