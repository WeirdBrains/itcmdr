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
    #[arg(long, env = "ITCMDR_ADMINSVR_ENV_FILE", default_value = "")]
    env_file: String,
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
    #[arg(long, env = "ITCMDR_ADMINSVR_FCM_SERVICE_ACCOUNT", default_value = "")]
    fcm_service_account: String,
    #[arg(long, env = "ITCMDR_ADMINSVR_FCM_URL", default_value = "")]
    fcm_url: String,
    /// Push (APNs): the .p8 auth key file
    #[arg(long, env = "ITCMDR_ADMINSVR_APNS_KEY", default_value = "")]
    apns_key: String,
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
    /// v1.1.0 ACME (Let's Encrypt) for the admin API: certificate names, comma separated (empty = off)
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_ACME_DOMAINS",
        default_value = "",
        value_delimiter = ','
    )]
    acme_domains: Vec<String>,
    /// Contact e-mail(s) for the ACME account, comma separated
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_ACME_CONTACT",
        default_value = "",
        value_delimiter = ','
    )]
    acme_contact: Vec<String>,
    /// production | staging | an ACME directory URL
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_ACME_DIRECTORY",
        default_value = "production"
    )]
    acme_directory: String,
    /// PEM root for the ACME directory's own HTTPS (only for test CAs such as Pebble)
    #[arg(long, env = "ITCMDR_ADMINSVR_ACME_DIRECTORY_ROOT", default_value = "")]
    acme_directory_root: String,
    /// tls-alpn-01 (on the admin API port; forward 443 to it) | http-01 (on --acme-http-addr)
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_ACME_CHALLENGE",
        default_value = "tls-alpn-01"
    )]
    acme_challenge: String,
    #[arg(
        long,
        env = "ITCMDR_ADMINSVR_ACME_HTTP_ADDR",
        default_value = "0.0.0.0:80"
    )]
    acme_http_addr: String,
    /// Account key and certificate (default <data dir>/acme, 0700)
    #[arg(long, env = "ITCMDR_ADMINSVR_ACME_CACHE_DIR", default_value = "")]
    acme_cache_dir: String,
    #[arg(long, env = "ITCMDR_ADMINSVR_ACME_RENEW_DAYS", default_value_t = 30)]
    acme_renew_days: i64,
    #[arg(long, env = "ITCMDR_ADMINSVR_ACME_RETRY_SECS", default_value_t = 3600)]
    acme_retry_secs: u64,
    /// v1.2.0 adminsvr's own backups: archive folder (default <data dir>/backups)
    #[arg(long, env = "ITCMDR_ADMINSVR_BACKUP_DIR", default_value = "")]
    backup_dir: String,
    /// Nightly backup hour (0-23, local time) while serving, or `off`
    #[arg(long, env = "ITCMDR_ADMINSVR_BACKUP_HOUR", default_value = "3")]
    backup_hour: String,
    /// Nightly archives kept (manual archives are never pruned)
    #[arg(long, env = "ITCMDR_ADMINSVR_BACKUP_KEEP", default_value_t = 14)]
    backup_keep: usize,
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
    /// v1.1.0: the admin API certificate (mode, expiry, last renewal, last error)
    TlsStatus,
    /// v1.2.0: adminsvr's own backups (its database, keys, TLS, ACME and settings; never site backups)
    #[command(subcommand)]
    Backup(BackupCmd),
    /// v1.2.0: restore adminsvr from one of its own backups (recovery words on stdin; stop the service first)
    Restore {
        /// The archive (.itcab)
        #[arg(long)]
        from: PathBuf,
        /// Open and check everything, show what would be written, change nothing
        #[arg(long)]
        dry_run: bool,
    },
    /// Run as a Windows service (started by the service manager; installed by the Setup)
    #[command(hide = true)]
    Service,
}

#[derive(Subcommand)]
enum BackupCmd {
    /// Set up backups: print 24 new recovery words ONCE (write them down; they are never stored)
    Init {
        /// Issue new words (archives made before still need the old words)
        #[arg(long)]
        replace: bool,
    },
    /// Make a backup now
    Create {
        /// Write it here instead of the backup folder
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// List the backups
    List {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Check a backup: signature and hash; --deep (words on stdin) also decrypts and checks every file
    Verify {
        archive: PathBuf,
        #[arg(long)]
        deep: bool,
    },
    /// Restore drill: check the newest backup without touching the live install;
    /// --deep (words on stdin) also decrypts it and restores it into a throwaway database
    Drill {
        #[arg(long)]
        deep: bool,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Delete old nightly backups (keeps --backup-keep)
    Prune {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Recovery key, last backup and last drill
    Status,
}

/// The recovery words from stdin (typed or piped; never from a flag).
fn read_words() -> anyhow::Result<String> {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        eprintln!("Type the 24 recovery words, then Enter:");
    }
    let mut w = String::new();
    let mut line = String::new();
    while w.split_whitespace().count() < 24 {
        line.clear();
        if std::io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        w.push(' ');
        w.push_str(&line);
    }
    Ok(w)
}

fn fmt_ms(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M %Z")
                .to_string()
        })
        .unwrap_or_else(|| "never".into())
}

async fn backup_cmd(cfg: Config, cmd: BackupCmd) -> anyhow::Result<()> {
    use itcmdr_adminsvr::backup as b;
    match cmd {
        BackupCmd::Init { replace } => {
            let (words, r) = b::init(&cfg, replace)?;
            println!("Recovery words for adminsvr's backups (shown once; write them down and keep them offline, apart from the backups):\n\n{words}\n");
            println!("Recovery key {}. Only these words open adminsvr's backups; adminsvr keeps just the public key.", r.fingerprint());
            Ok(())
        }
        BackupCmd::Create { out } => {
            let pool = itcmdr_adminsvr::store::connect(&cfg.db_url).await?;
            itcmdr_adminsvr::store::migrate(&pool).await?;
            let i = b::create(&cfg, &pool, "manual", out.as_deref()).await?;
            println!(
                "{} ({} bytes, {} files)",
                i.path.display(),
                i.size,
                i.manifest.files.len()
            );
            Ok(())
        }
        BackupCmd::List { dir } => {
            for i in b::list(&b::backup_dir(&cfg, dir.as_deref()))? {
                println!(
                    "{}  {} bytes  {}  {}",
                    i.name,
                    i.size,
                    i.manifest.kind,
                    if i.manifest.format == "damaged" {
                        "DAMAGED".to_string()
                    } else {
                        format!(
                            "v{} schema {}",
                            i.manifest.version, i.manifest.schema_version
                        )
                    }
                );
            }
            Ok(())
        }
        BackupCmd::Verify { archive, deep } => {
            let pinned = b::load_recovery(&cfg.data_dir)?;
            if deep {
                let (_, f) = b::open(&archive, &read_words()?, pinned.as_ref())?;
                println!("ok: signature, hash, decryption and {} files", f.len());
            } else {
                let h = b::verify(&archive, pinned.as_ref())?;
                println!(
                    "ok: signature and hash ({} files, recovery key {}{})",
                    h.manifest.files.len(),
                    h.recovery.fingerprint(),
                    if pinned.is_none() {
                        "; not pinned: this install has no recovery key"
                    } else {
                        ""
                    }
                );
            }
            Ok(())
        }
        BackupCmd::Drill { deep, dir } => {
            let pool = itcmdr_adminsvr::store::connect(&cfg.db_url).await.ok();
            let w = if deep { Some(read_words()?) } else { None };
            let r = b::drill(&cfg, pool.as_ref(), dir.as_deref(), w.as_deref()).await?;
            if r.ok {
                println!(
                    "drill ok{}: {} ({} files{})",
                    if r.deep { " (deep)" } else { "" },
                    r.archive,
                    r.files,
                    if r.deep {
                        format!(", {} users", r.users)
                    } else {
                        String::new()
                    }
                );
                Ok(())
            } else {
                anyhow::bail!("drill FAILED: {} {}", r.archive, r.error)
            }
        }
        BackupCmd::Prune { dir } => {
            for g in b::prune(&b::backup_dir(&cfg, dir.as_deref()), cfg.backup.keep)? {
                println!("deleted {g}");
            }
            Ok(())
        }
        BackupCmd::Status => {
            match b::load_recovery(&cfg.data_dir)? {
                None => println!("backups: not set up (run `itcmdr-adminsvr backup init`)"),
                Some(r) => println!(
                    "recovery key {} (since {})",
                    r.fingerprint(),
                    fmt_ms(r.created_ms)
                ),
            }
            println!("folder: {}", cfg.backup.dir.display());
            println!(
                "nightly: {}",
                cfg.backup
                    .hour
                    .map(|h| format!("{h}:00 local, keep {}", cfg.backup.keep))
                    .unwrap_or("off".into())
            );
            let all = b::list(&cfg.backup.dir)?;
            println!(
                "newest: {}",
                all.last().map(|i| i.name.clone()).unwrap_or("none".into())
            );
            match b::last_drill(&cfg.data_dir) {
                None => println!("last drill: never"),
                Some(d) => println!(
                    "last drill: {} {} {}{}",
                    fmt_ms(d.ms),
                    if d.deep { "deep" } else { "quick" },
                    if d.ok { "ok" } else { "FAILED" },
                    if d.ok {
                        String::new()
                    } else {
                        format!(" ({})", d.error)
                    }
                ),
            }
            Ok(())
        }
    }
}

/// Ctrl-C, or SIGTERM (what systemd, SysV init, OpenRC, launchd and podman send to stop).
async fn stop_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut t) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = t.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    tracing::info!("stopping");
}

fn path_opt(s: &str) -> Option<PathBuf> {
    let s = s.trim();
    (!s.is_empty()).then(|| PathBuf::from(s))
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
    let text =
        std::fs::read_to_string(p).map_err(|e| anyhow::anyhow!("env file {}: {e}", p.display()))?;
    // Later lines win inside the file (like a shell); the real environment wins over the file.
    let mut m = std::collections::BTreeMap::new();
    for (k, v) in parse_env(&text) {
        m.insert(k, v);
    }
    for (k, v) in m {
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
    itcmdr_help::keep_stamp();
    pre_env()?;
    let mut c = Cli::parse();
    if matches!(c.cmd, Cmd::Service) {
        return service::run(config(c));
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        // Plain text in log files (SysV, launchd, containers); colors only on a terminal.
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_writer(std::io::stderr)
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
    let domains: Vec<String> = c
        .acme_domains
        .iter()
        .map(|d| d.trim().to_ascii_lowercase())
        .filter(|d| !d.is_empty())
        .collect();
    let acme = (!domains.is_empty()).then(|| itcmdr_adminsvr::config::AcmeConfig {
        domains,
        contact: c
            .acme_contact
            .iter()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        directory: c.acme_directory.clone(),
        directory_root: path_opt(&c.acme_directory_root),
        challenge: c.acme_challenge.parse().unwrap_or_else(|e: String| {
            eprintln!("{e}");
            std::process::exit(2)
        }),
        http_addr: c.acme_http_addr.clone(),
        cache_dir: path_opt(&c.acme_cache_dir).unwrap_or_else(|| c.data_dir.join("acme")),
        renew_days: c.acme_renew_days,
        retry_secs: c.acme_retry_secs.max(10),
    });
    let backup = itcmdr_adminsvr::config::BackupConfig {
        dir: path_opt(&c.backup_dir).unwrap_or_else(|| c.data_dir.join("backups")),
        hour: match c.backup_hour.trim() {
            "" | "off" | "none" => None,
            h => Some(
                h.parse::<u32>()
                    .ok()
                    .filter(|h| *h < 24)
                    .unwrap_or_else(|| {
                        eprintln!("--backup-hour must be 0-23 or off");
                        std::process::exit(2)
                    }),
            ),
        },
        keep: c.backup_keep,
    };
    Config {
        backup,
        acme,
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
            fcm_service_account: path_opt(&c.fcm_service_account),
            fcm_url: c.fcm_url,
            apns_key: path_opt(&c.apns_key),
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
        Cmd::Serve => itcmdr_adminsvr::run(cfg, stop_signal()).await,
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
        Cmd::Backup(b) => backup_cmd(cfg, b).await,
        Cmd::Restore { from, dry_run } => {
            let w = read_words()?;
            let r = itcmdr_adminsvr::backup::restore(&cfg, &from, &w, dry_run).await?;
            let verb = if dry_run { "would write" } else { "wrote" };
            for x in &r.written {
                println!("{verb} {x}");
            }
            for x in &r.review {
                println!("{verb} for review (not active): {}", x.display());
            }
            if let Some(s) = &r.safety_copy {
                println!("the state before the restore is in {}", s.display());
            }
            if !dry_run {
                println!("restored. Review the settings in {}/restored/ (the active config was not changed), then start the service.", cfg.data_dir.display());
            }
            Ok(())
        }
        Cmd::TlsStatus => {
            let pool = itcmdr_adminsvr::store::connect(&cfg.db_url).await?;
            let m = itcmdr_adminsvr::material::load(&cfg)?;
            println!(
                "internal certificate (pinned by enrolled devices and sites): {}",
                m.fingerprint
            );
            match &cfg.acme {
                None => println!("ACME: off (only the internal certificate is served)"),
                Some(a) => {
                    let st = itcmdr_adminsvr::acme::load_status(&pool).await;
                    let d = |ms: i64| {
                        if ms <= 0 {
                            "never".to_string()
                        } else {
                            let days = (ms - itcmdr_adminsvr::now_ms()) / 86_400_000;
                            format!("{} ms since epoch ({days:+} days)", ms)
                        }
                    };
                    println!(
                        "ACME: on, {:?}, directory {}",
                        a.challenge,
                        itcmdr_adminsvr::acme::directory_url(&a.directory)
                    );
                    println!("  domains:      {}", a.domains.join(", "));
                    println!("  expires:      {}", d(st.expires_ms));
                    println!("  last renewal: {}", d(st.renewed_ms));
                    println!("  issuer:       {}", st.issuer);
                    println!("  fingerprint:  {}", st.fingerprint);
                    if st.last_error_ms > st.renewed_ms {
                        println!(
                            "  LAST ATTEMPT FAILED ({}): {}",
                            d(st.last_error_ms),
                            st.last_error
                        );
                    }
                }
            }
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
        let cfg = CFG
            .get()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no config"))?;
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
