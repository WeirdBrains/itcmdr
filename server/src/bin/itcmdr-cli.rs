//! itcmdr-cli: local administration of an itcmdr server (run it on the server, as an admin).
//!
//! Commands that need the 24 recovery words read them from stdin (one line), never from
//! arguments or the environment, so they don't end up in shell history.
use ::itcmdr_server as srv;
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use srv::auth::{NewKey, Role};
use srv::config::ServerConfig;
use srv::keys::{self, Certificate, Master};
use std::io::{BufRead, IsTerminal};
use std::path::PathBuf;
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(name = "itcmdr-cli", version, about = "itcmdr server administration")]
struct Cli {
    /// Data dir override (beats ITCMDR_DATA_DIR and server.json).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// Print JSON instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// First-run setup: prints the 24 recovery words ONCE. Do this in person at the server.
    Init,
    /// Restore keys/db.key from the escrow with the words (replacement machine).
    RecoverDbKey {
        #[arg(long)]
        force: bool,
    },
    /// Key management.
    #[command(subcommand)]
    Keys(KeysCmd),
    /// Register a service plugin client key (certified with the words).
    #[command(subcommand)]
    Service(ServiceCmd),
    /// Service plugin packages (also available as `service plugins ...`).
    #[command(subcommand)]
    Plugins(PluginsCmd),
    /// Users (people with app access).
    #[command(subcommand)]
    User(UserCmd),
    /// Backups (decision 13).
    #[command(subcommand)]
    Backup(BackupCmd),
    /// Runtime settings stored in the DB (override server.json): backup.dir, backup.hour, ...
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Scheduled runs (decision 14).
    #[command(subcommand)]
    Schedule(ScheduleCmd),
    /// Restore an archive into the data dir. Stop the server first.
    Restore {
        #[arg(long)]
        from: PathBuf,
        /// Verify and decrypt only, change nothing.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
struct Identity {
    #[arg(long, value_parser = ["client", "user"])]
    kind: String,
    #[arg(long)]
    name: String,
    #[arg(long, value_parser = ["owner", "admin", "user", "client"])]
    role: String,
}

#[derive(Subcommand)]
enum KeysCmd {
    /// OFFLINE: certify an existing public key (hex) with the words. Prints a key bundle (JSON).
    Certify {
        #[command(flatten)]
        id: Identity,
        #[arg(long)]
        public_key: String,
        /// Write the bundle here instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// OFFLINE: generate a keypair and certify it. Private key -> FILE, bundle -> FILE.pub.json.
    New {
        #[command(flatten)]
        id: Identity,
        #[arg(long)]
        out: PathBuf,
    },
    /// Register a certified key bundle on this server.
    Register {
        bundle: PathBuf,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        phone: Option<String>,
    },
    /// List registered keys.
    List,
    /// Revoke a key by fingerprint (takes effect immediately).
    Revoke { fingerprint: String },
    /// Replace a key: new keypair, same kind/name/role, certified with the words; old one revoked.
    Rotate {
        fingerprint: String,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum PluginsCmd {
    /// Validate and install a package. Valid packages are enabled (setting
    /// plugins.enable_on_install, default true) unless --disabled. Bad packages are quarantined
    /// and the install is denied.
    #[command(group(clap::ArgGroup::new("src").required(true).args(["url", "path"])))]
    Install {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        path: Option<PathBuf>,
        /// Install without enabling.
        #[arg(long, conflicts_with = "enable")]
        disabled: bool,
        /// Accepted for compatibility (enabling is the default).
        #[arg(long)]
        enable: bool,
    },
    /// Installed plugins.
    List,
    /// Enable NAME or NAME@VERSION (other versions of it are disabled).
    Enable { spec: String },
    /// Disable every version of NAME.
    Disable { name: String },
    /// Remove NAME or NAME@VERSION (files and record).
    Remove {
        spec: String,
        #[arg(long)]
        force: bool,
    },
    /// Packages refused so far (quarantine/index.jsonl).
    Quarantine,
    /// Run an enabled plugin now (Verify, then Execute; Log on failure).
    Run {
        /// NAME or NAME@VERSION
        spec: String,
        #[arg(long)]
        report_only: bool,
        /// Plugin argument KEY=VALUE (repeatable).
        #[arg(long = "arg")]
        args: Vec<String>,
    },
    /// Recent plugin runs.
    Runs {
        plugin: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    /// Build and sign a package from DIR/package.toml (checksums are filled in).
    Pack {
        #[arg(long)]
        dir: PathBuf,
        /// Private key file of an admin/owner key (from `keys new` / `user add --out`).
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum ServiceCmd {
    /// Same as `itcmdr-cli plugins ...`.
    #[command(subcommand)]
    Plugins(PluginsCmd),
    /// Generate, certify (words) and register a client key for a service plugin.
    Register {
        #[arg(long)]
        name: String,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum UserCmd {
    /// Certify (words) and register a user. Pass --public-key (preferred: the key never leaves
    /// the user's device) or --out to generate one here.
    Add {
        #[arg(long)]
        name: String,
        #[arg(long, value_parser = ["owner", "admin", "user"])]
        role: String,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        phone: Option<String>,
        #[arg(long)]
        public_key: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BackupCmd {
    /// Take a backup now.
    Create {
        /// Folder (default: server.json backup.dir, else <data_dir>/backups).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// List archives, newest first.
    List {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Check an archive's signature and hash; --deep also decrypts it with the words and checks
    /// every file against the manifest.
    Verify {
        archive: PathBuf,
        #[arg(long)]
        deep: bool,
    },
    /// Apply nightly retention now.
    Prune {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[derive(clap::Args, Clone, Default)]
struct ScheduleArgs {
    /// hourly | "every 6h" | "daily 01:30" | "weekdays 07:00" | "weekly mon 02:00" | "cron M H DOM MON DOW"
    #[arg(long)]
    when: Option<String>,
    /// IANA timezone (e.g. America/New_York) or "local".
    #[arg(long)]
    tz: Option<String>,
    /// Target every machine.
    #[arg(long)]
    all: bool,
    /// Target machines (comma-separated or repeated).
    #[arg(long, value_delimiter = ',')]
    machines: Vec<String>,
    /// Target groups = agent roles (Server, Workstation, RCU).
    #[arg(long, value_delimiter = ',')]
    groups: Vec<String>,
    /// Check categories (empty = all).
    #[arg(long, value_delimiter = ',')]
    categories: Vec<String>,
    /// Service plugins to run on the server.
    #[arg(long, value_delimiter = ',')]
    plugins: Vec<String>,
    /// Repair (default report-only).
    #[arg(long, conflicts_with = "report_only")]
    repair: bool,
    #[arg(long)]
    report_only: bool,
    /// report-only-in-hours (default) | skip-in-hours | none
    #[arg(long)]
    guard: Option<String>,
    /// skip (default) | catch-up
    #[arg(long)]
    missed: Option<String>,
}

#[derive(Subcommand)]
enum ScheduleCmd {
    /// Create a schedule.
    Add {
        #[arg(long)]
        name: String,
        #[command(flatten)]
        a: ScheduleArgs,
        /// Create paused.
        #[arg(long)]
        paused: bool,
    },
    /// All schedules with their next runs.
    List,
    /// Change a schedule (only the given options change).
    Edit {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        a: ScheduleArgs,
    },
    /// Delete a schedule (built-in ones need --force; pause them instead).
    Rm {
        id: String,
        #[arg(long)]
        force: bool,
    },
    Pause {
        id: String,
    },
    Resume {
        id: String,
    },
    /// Queue a run now (no office-hours guard; the agent's clinical guard still applies).
    RunNow {
        id: String,
    },
    /// Run history (all schedules, or one).
    History {
        id: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
}

fn apply_args(mut s: srv::schedule::Spec, a: &ScheduleArgs) -> srv::schedule::Spec {
    if let Some(w) = &a.when {
        s.when = w.clone();
    }
    if let Some(t) = &a.tz {
        s.timezone = t.clone();
    }
    if a.all || !a.machines.is_empty() || !a.groups.is_empty() {
        s.all_machines = a.all;
        s.machines = a.machines.clone();
        s.groups = a.groups.clone();
    }
    if !a.categories.is_empty() {
        s.categories = a.categories.clone();
    }
    if !a.plugins.is_empty() {
        s.plugins = a.plugins.clone();
    }
    if a.repair {
        s.repair = true;
    }
    if a.report_only {
        s.repair = false;
    }
    if let Some(g) = &a.guard {
        s.guard = g.clone();
    }
    if let Some(m) = &a.missed {
        s.missed = m.clone();
    }
    s
}

fn ms_local(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M %Z")
                .to_string()
        })
        .unwrap_or_default()
}

fn print_schedule(s: &srv::schedule::Schedule, json: bool) -> Result<()> {
    let next: Vec<String> = if s.enabled {
        srv::schedule::next_times(&s.spec, chrono::Utc::now(), 3)
            .unwrap_or_default()
            .into_iter()
            .map(|t| ms_local(t.timestamp_millis()))
            .collect()
    } else {
        vec![]
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"schedule": s, "next": next}))?
        );
        return Ok(());
    }
    let target = if s.spec.all_machines {
        "all machines".to_string()
    } else if !s.spec.groups.is_empty() {
        format!("groups {}", s.spec.groups.join(","))
    } else if !s.spec.machines.is_empty() {
        s.spec.machines.join(",")
    } else {
        "server".into()
    };
    let what = [
        (!s.spec.categories.is_empty()).then(|| format!("checks {}", s.spec.categories.join(","))),
        (!s.spec.plugins.is_empty()).then(|| format!("plugins {}", s.spec.plugins.join(","))),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" + ");
    println!(
        "{}  {}  [{}]  {} ({})  {} on {}  {}  guard={} missed={}{}  next: {}",
        &s.id[..8],
        s.name,
        if s.enabled { "on" } else { "paused" },
        s.spec.when,
        s.spec.timezone,
        if what.is_empty() {
            "all checks".into()
        } else {
            what
        },
        target,
        if s.spec.repair {
            "repair"
        } else {
            "report-only"
        },
        s.spec.guard,
        s.spec.missed,
        if s.spec.builtin { " built-in" } else { "" },
        if next.is_empty() {
            "-".into()
        } else {
            next.join(", ")
        }
    );
    Ok(())
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Every setting with its effective value and source (docs/SETTINGS.md).
    List {
        /// Resolve this PC's agent overrides.
        #[arg(long)]
        machine: Option<String>,
    },
    /// One setting (or all, like `list`).
    Get {
        key: Option<String>,
        #[arg(long)]
        machine: Option<String>,
    },
    /// Set one key, e.g. `config set backup.hour 3`. Validated; folders must exist and be writable.
    Set {
        key: String,
        value: String,
        /// Agent settings only: override for one PC.
        #[arg(long)]
        machine: Option<String>,
    },
    /// Remove an override (back to server.json / default).
    Unset {
        key: String,
        #[arg(long)]
        machine: Option<String>,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Bundle {
    kind: String,
    name: String,
    role: String,
    public_key: String,
    certificate: Certificate,
}

async fn print_settings(
    pool: &srv::db::DbPool,
    cfg: &srv::config::ServerConfig,
    key: Option<&str>,
    machine: Option<&str>,
    json: bool,
) -> Result<()> {
    let all = srv::settings::resolve_all(pool, Some(cfg), machine).await?;
    let rows: Vec<_> = all
        .iter()
        .filter(|r| key.is_none_or(|k| k == r.def.key))
        .collect();
    if json {
        let v: Vec<_> = rows
            .iter()
            .map(|r| serde_json::json!({"key": r.def.key, "value": r.value, "source": r.source, "default": r.def.default, "scope": format!("{:?}", r.def.scope).to_lowercase()}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        for r in rows {
            let v = match &r.value {
                serde_json::Value::String(t) => t.clone(),
                x => x.to_string(),
            };
            println!("{} = {}    [{}]", r.def.key, v, r.source);
        }
    }
    Ok(())
}

async fn export_if_agent(pool: &srv::db::DbPool, cfg: &srv::config::ServerConfig, key: &str) {
    if srv::settings::def(key).is_ok_and(|d| d.scope == srv::settings::Scope::Agent) {
        if let Err(e) = srv::settings::export_agent(pool, cfg).await {
            eprintln!("warning: agent-settings.json export failed: {e:#}");
        }
    }
}

fn read_words() -> Result<Zeroizing<String>> {
    if std::io::stdin().is_terminal() {
        eprintln!("Enter the 24 recovery words on one line, then press Enter:");
    }
    let mut w = Zeroizing::new(String::new());
    std::io::stdin().lock().read_line(&mut w)?;
    if w.trim().is_empty() {
        bail!("no recovery words given on stdin");
    }
    Ok(w)
}

fn master() -> Result<Master> {
    Master::from_words(&read_words()?)
}

fn role(s: &str) -> Role {
    Role::parse(s).expect("validated by clap")
}

fn bundle_for(m: &Master, id: &Identity, pk: &ed25519_dalek::VerifyingKey) -> Bundle {
    Bundle {
        kind: id.kind.clone(),
        name: id.name.clone(),
        role: id.role.clone(),
        public_key: hex::encode(pk.as_bytes()),
        certificate: keys::certify(&m.signing, &id.kind, &id.name, &id.role, pk),
    }
}

/// Private key file (DPAPI on Windows / 0600) + `<file>.pub.json` bundle.
fn write_new(m: &Master, id: &Identity, out: &std::path::Path) -> Result<Bundle> {
    if out.exists() {
        bail!("{} already exists", out.display());
    }
    let k = keys::new_keypair();
    let b = bundle_for(m, id, &k.verifying_key());
    srv::storage::write_secret(out, k.as_bytes())?;
    std::fs::write(pub_path(out), serde_json::to_vec_pretty(&b)?)?;
    Ok(b)
}

fn pub_path(out: &std::path::Path) -> PathBuf {
    PathBuf::from(format!("{}.pub.json", out.display()))
}

async fn register(
    pool: &srv::db::DbPool,
    b: &Bundle,
    email: Option<&str>,
    phone: Option<&str>,
) -> Result<()> {
    srv::auth::register(
        pool,
        NewKey {
            kind: &b.kind,
            name: &b.name,
            role: Role::parse(&b.role).context("bad role in bundle")?,
            email,
            phone,
            public_key_hex: &b.public_key,
            certificate: &b.certificate,
        },
    )
    .await?;
    println!(
        "registered {} {:?} ({}) fingerprint {}",
        b.kind, b.name, b.role, b.certificate.fingerprint
    );
    Ok(())
}

fn print_backup(json: bool, b: &srv::backup::BackupInfo) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string(b)?);
    } else {
        let t = chrono::DateTime::from_timestamp_millis(b.manifest.created_ms)
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M %Z")
                    .to_string()
            })
            .unwrap_or_default();
        println!(
            "{}  {:>10} bytes  {}  {}  v{} schema {}",
            b.name,
            b.size,
            t,
            b.manifest.kind,
            b.manifest.itcmdr_version,
            b.manifest.schema_version
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    srv::db::track_pools();
    let rt = tokio::runtime::Runtime::new()?;
    let r = run(&rt);
    rt.block_on(srv::db::close_tracked());
    r
}

fn run(rt: &tokio::runtime::Runtime) -> Result<()> {
    let cli = Cli::parse();
    let load = || -> Result<ServerConfig> {
        let mut c = ServerConfig::load()?;
        c.data_dir_flag = cli.data_dir.clone();
        Ok(c)
    };
    match &cli.cmd {
        Cmd::Init => {
            let cfg = load()?;
            rt.block_on(async {
                let pool = srv::db::init_db(&cfg).await?;
                let out = srv::auth::init(&pool, &cfg.layout()).await?;
                println!("itcmdr is initialized.\n");
                println!("RECOVERY WORDS (shown once, never stored on this server). Write them down and keep them offline:\n");
                for (i, w) in out.words.split(' ').enumerate() {
                    println!("  {:>2}. {w}", i + 1);
                }
                println!("\nmaster key fingerprint: {}", out.master_fingerprint);
                println!("root client fingerprint: {}", out.root_fingerprint);
                Ok(())
            })
        }
        Cmd::RecoverDbKey { force } => {
            let cfg = load()?;
            srv::auth::recover_db_key(&cfg.layout(), &read_words()?, *force)?;
            println!("keys/db.key restored and protected for this machine.");
            Ok(())
        }
        Cmd::Keys(k) => match k {
            KeysCmd::Certify {
                id,
                public_key,
                out,
            } => {
                let pk = keys::parse_public_key(public_key)?;
                let b = bundle_for(&master()?, id, &pk);
                let j = serde_json::to_string_pretty(&b)?;
                match out {
                    Some(p) => std::fs::write(p, j)?,
                    None => println!("{j}"),
                }
                Ok(())
            }
            KeysCmd::New { id, out } => {
                let b = write_new(&master()?, id, out)?;
                println!(
                    "private key: {}\nbundle: {}\nfingerprint: {}",
                    out.display(),
                    pub_path(out).display(),
                    b.certificate.fingerprint
                );
                Ok(())
            }
            KeysCmd::Register {
                bundle,
                email,
                phone,
            } => {
                let b: Bundle = serde_json::from_slice(&std::fs::read(bundle)?)?;
                let cfg = load()?;
                rt.block_on(async {
                    let pool = srv::db::init_db(&cfg).await?;
                    register(&pool, &b, email.as_deref(), phone.as_deref()).await
                })
            }
            KeysCmd::List => {
                let cfg = load()?;
                rt.block_on(async {
                    let pool = srv::db::init_db(&cfg).await?;
                    for k in srv::auth::list_keys(&pool).await? {
                        if cli.json {
                            println!("{}", serde_json::to_string(&k)?);
                        } else {
                            println!(
                                "{:<6} {:<24} {:<6} {}  {}",
                                k.kind,
                                k.name,
                                k.role,
                                k.fingerprint,
                                k.revoked_at
                                    .map(|r| format!("REVOKED {r}"))
                                    .unwrap_or_default()
                            );
                        }
                    }
                    Ok(())
                })
            }
            KeysCmd::Revoke { fingerprint } => {
                let cfg = load()?;
                rt.block_on(async {
                    let pool = srv::db::init_db(&cfg).await?;
                    if !srv::auth::revoke(&pool, fingerprint).await? {
                        bail!("no active key with that fingerprint");
                    }
                    println!("revoked {fingerprint}");
                    Ok(())
                })
            }
            KeysCmd::Rotate { fingerprint, out } => {
                let cfg = load()?;
                let m = master()?;
                rt.block_on(async {
                    let pool = srv::db::init_db(&cfg).await?;
                    let old = srv::auth::list_keys(&pool)
                        .await?
                        .into_iter()
                        .find(|k| &k.fingerprint == fingerprint && k.revoked_at.is_none())
                        .context("no active key with that fingerprint")?;
                    let id = Identity {
                        kind: old.kind,
                        name: old.name,
                        role: old.role,
                    };
                    let b = write_new(&m, &id, out)?;
                    register(&pool, &b, None, None).await?;
                    srv::auth::revoke(&pool, fingerprint).await?;
                    println!("revoked {fingerprint}; new private key: {}", out.display());
                    Ok(())
                })
            }
        },
        Cmd::Plugins(pc) | Cmd::Service(ServiceCmd::Plugins(pc)) => {
            let json = cli.json;
            if let PluginsCmd::Pack { dir, key, out } = pc {
                let b: [u8; 32] = srv::storage::read_secret(key)?.try_into().map_err(|_| {
                    anyhow::anyhow!("{} is not an itcmdr private key", key.display())
                })?;
                let k = ed25519_dalek::SigningKey::from_bytes(&b);
                let m = srv::packages::pack(dir, &k, out)?;
                println!(
                    "packed {} {} -> {} (signed by {})",
                    m.name,
                    m.version,
                    out.display(),
                    keys::fingerprint(&k.verifying_key())
                );
                return Ok(());
            }
            let cfg = load()?;
            let layout = cfg.layout();
            rt.block_on(async {
                let pool = srv::db::init_db(&cfg).await?;
                match pc {
                    PluginsCmd::Install {
                        url,
                        path,
                        disabled,
                        enable,
                    } => {
                        let src = match (url, path) {
                            (Some(u), None) => srv::packages::Source::Url(u),
                            (None, Some(p)) => srv::packages::Source::Path(p),
                            _ => bail!("pass exactly one of --url or --path"),
                        };
                        let p = srv::packages::install(
                            &pool,
                            &layout,
                            src,
                            if *disabled {
                                Some(false)
                            } else if *enable {
                                Some(true)
                            } else {
                                None
                            },
                            "itcmdr-cli",
                        )
                        .await?;
                        println!(
                            "installed {} {} into {} ({})",
                            p.name,
                            p.version,
                            p.dir.display(),
                            if p.enabled {
                                "enabled"
                            } else {
                                "disabled: run `plugins enable` to use it"
                            }
                        );
                    }
                    PluginsCmd::List => {
                        for p in srv::packages::list(&pool, &layout).await? {
                            if json {
                                println!("{}", serde_json::to_string(&p)?);
                            } else {
                                println!(
                                    "{:<24} {:<12} {:<8} signed by {}",
                                    p.name,
                                    p.version,
                                    if p.enabled { "enabled" } else { "disabled" },
                                    p.signer
                                );
                            }
                        }
                    }
                    PluginsCmd::Enable { spec } => {
                        let (n, v) = srv::packages::enable(&pool, spec, "itcmdr-cli").await?;
                        println!("enabled {n} {v}");
                    }
                    PluginsCmd::Disable { name } => {
                        srv::packages::disable(&pool, name, "itcmdr-cli").await?;
                        println!("disabled {name}");
                    }
                    PluginsCmd::Remove { spec, force } => {
                        let (n, v) =
                            srv::packages::remove(&pool, &layout, spec, *force, "itcmdr-cli")
                                .await?;
                        println!("removed {n} {v}");
                    }
                    PluginsCmd::Quarantine => {
                        for q in srv::packages::list_quarantine(&layout)? {
                            if json {
                                println!("{}", serde_json::to_string(&q)?);
                            } else {
                                println!("{}  {}  {}", q.ts, q.file, q.reason);
                            }
                        }
                    }
                    PluginsCmd::Run {
                        spec,
                        report_only,
                        args,
                    } => {
                        let mut a = std::collections::BTreeMap::new();
                        for kv in args {
                            let (k, v) = kv.split_once('=').ok_or_else(|| {
                                anyhow::anyhow!("--arg wants KEY=VALUE, got {kv}")
                            })?;
                            a.insert(k.to_string(), v.to_string());
                        }
                        let rep = srv::runtime::run_plugin(
                            &pool,
                            &cfg,
                            srv::runtime::RunSpec {
                                plugin: spec.clone(),
                                report_only: *report_only,
                                args: a,
                                ..Default::default()
                            },
                            "itcmdr-cli",
                        )
                        .await?;
                        if json {
                            println!("{}", serde_json::to_string_pretty(&rep)?);
                        } else {
                            for st in &rep.stages {
                                println!(
                                    "{:8} {}  {}",
                                    st.stage,
                                    if st.ok { "ok" } else { "FAILED" },
                                    st.message
                                );
                            }
                            for (c, s, m) in &rep.results {
                                println!("  {c} {s} {m}");
                            }
                            println!(
                                "{} {}: {}  (log {})",
                                rep.plugin,
                                rep.status,
                                rep.message,
                                rep.log_path.display()
                            );
                        }
                        if rep.status != "ok" {
                            bail!("plugin run {}", rep.status);
                        }
                    }
                    PluginsCmd::Runs { plugin, limit } => {
                        for r in srv::runtime::list_runs(&pool, plugin.as_deref(), *limit).await? {
                            if json {
                                println!("{}", serde_json::to_string(&r)?);
                            } else {
                                println!(
                                    "{}  {} {}  {}  {}  {}",
                                    ms_local(r.started_ms),
                                    r.plugin,
                                    r.version,
                                    r.machine,
                                    r.status,
                                    r.message
                                );
                            }
                        }
                    }
                    PluginsCmd::Pack { .. } => unreachable!(),
                }
                Ok(())
            })
        }
        Cmd::Service(ServiceCmd::Register { name, out }) => {
            let cfg = load()?;
            let m = master()?;
            rt.block_on(async {
                let pool = srv::db::init_db(&cfg).await?;
                let id = Identity {
                    kind: "client".into(),
                    name: name.clone(),
                    role: "client".into(),
                };
                let b = write_new(&m, &id, out)?;
                register(&pool, &b, None, None).await?;
                println!("service private key: {}", out.display());
                Ok(())
            })
        }
        Cmd::User(UserCmd::Add {
            name,
            role: r,
            email,
            phone,
            public_key,
            out,
        }) => {
            let cfg = load()?;
            let m = master()?;
            let id = Identity {
                kind: "user".into(),
                name: name.clone(),
                role: r.clone(),
            };
            let _ = role(r);
            rt.block_on(async {
                let pool = srv::db::init_db(&cfg).await?;
                let b = match (public_key, out) {
                    (Some(pk), None) => bundle_for(&m, &id, &keys::parse_public_key(pk)?),
                    (None, Some(o)) => write_new(&m, &id, o)?,
                    _ => bail!("pass exactly one of --public-key or --out"),
                };
                register(&pool, &b, email.as_deref(), phone.as_deref()).await
            })
        }
        Cmd::Backup(b) => {
            let cfg = load()?;
            match b {
                BackupCmd::Create { out } => rt.block_on(async {
                    let pool = srv::db::init_db(&cfg).await?;
                    let b = srv::backup::create(&pool, &cfg, "manual", out.as_deref()).await?;
                    srv::db::insert_audit(
                        &pool,
                        "itcmdr-cli",
                        "backup-create",
                        &serde_json::json!({"name": b.name}).to_string(),
                    )
                    .await?;
                    print_backup(cli.json, &b)
                }),
                BackupCmd::List { dir } => {
                    let d = match dir {
                        Some(d) => d.clone(),
                        None => rt.block_on(async {
                            srv::settings::backup_dir(&srv::db::init_db(&cfg).await?, &cfg).await
                        })?,
                    };
                    for b in srv::backup::list(&d)? {
                        print_backup(cli.json, &b)?;
                    }
                    Ok(())
                }
                BackupCmd::Verify { archive, deep } => {
                    if *deep {
                        let (h, files) = srv::backup::open(archive, &master()?)?;
                        println!("OK (deep): signature valid, decrypted, {} files match the manifest (v{}, schema {})", files.len(), h.manifest.itcmdr_version, h.manifest.schema_version);
                    } else {
                        // Pin to this server's master key when it has one.
                        let expected = rt.block_on(async {
                            match srv::db::init_db(&cfg).await {
                                Ok(pool) => srv::auth::master_public(&pool).await.ok().flatten(),
                                Err(_) => None,
                            }
                        });
                        let h =
                            srv::backup::verify(archive, expected.as_ref().map(|m| &m.ed25519))?;
                        println!(
                            "OK: signature and contents hash valid{} (v{}, schema {}). Use --deep with the words to decrypt and check every file.",
                            if expected.is_some() { ", made by this server" } else { " (no local master key to compare)" },
                            h.manifest.itcmdr_version,
                            h.manifest.schema_version
                        );
                    }
                    Ok(())
                }
                BackupCmd::Prune { dir } => {
                    let (d, bc) = rt.block_on(async {
                        let pool = srv::db::init_db(&cfg).await?;
                        let bc = srv::settings::effective_backup(&pool, &cfg).await?;
                        let d = bc.dir.clone().unwrap_or_else(|| cfg.layout().backups());
                        anyhow::Ok((dir.clone().unwrap_or(d), bc))
                    })?;
                    for n in srv::backup::prune(&d, &bc)? {
                        println!("deleted {n}");
                    }
                    Ok(())
                }
            }
        }
        Cmd::Config(c) => {
            let cfg = load()?;
            rt.block_on(async {
                let pool = srv::db::init_db(&cfg).await?;
                match c {
                    ConfigCmd::List { machine } | ConfigCmd::Get { key: None, machine } => {
                        print_settings(&pool, &cfg, None, machine.as_deref(), cli.json).await
                    }
                    ConfigCmd::Get {
                        key: Some(k),
                        machine,
                    } => {
                        srv::settings::def(k)?;
                        print_settings(&pool, &cfg, Some(k), machine.as_deref(), cli.json).await
                    }
                    ConfigCmd::Set {
                        key,
                        value,
                        machine,
                    } => {
                        let v = srv::settings::set(&pool, key, value, machine.as_deref()).await?;
                        srv::db::insert_audit(
                            &pool,
                            "itcmdr-cli",
                            "settings-update",
                            &serde_json::json!({"key": key, "value": value, "machine": machine})
                                .to_string(),
                        )
                        .await?;
                        export_if_agent(&pool, &cfg, key).await;
                        println!(
                            "{key} = {v}{}",
                            machine
                                .as_deref()
                                .map(|m| format!("  [{m}]"))
                                .unwrap_or_default()
                        );
                        Ok(())
                    }
                    ConfigCmd::Unset { key, machine } => {
                        srv::settings::unset(&pool, key, machine.as_deref()).await?;
                        srv::db::insert_audit(
                            &pool,
                            "itcmdr-cli",
                            "settings-update",
                            &serde_json::json!({"key": key, "unset": true, "machine": machine})
                                .to_string(),
                        )
                        .await?;
                        export_if_agent(&pool, &cfg, key).await;
                        println!("{key} reverted to server.json/default");
                        Ok(())
                    }
                }
            })
        }
        Cmd::Schedule(sc) => {
            let cfg = load()?;
            let json = cli.json;
            rt.block_on(async {
                let pool = srv::db::init_db(&cfg).await?;
                let audit = |action: &'static str, d: serde_json::Value| {
                    let pool = pool.clone();
                    async move { srv::db::insert_audit(&pool, "itcmdr-cli", action, &d.to_string()).await }
                };
                match sc {
                    ScheduleCmd::Add { name, a, paused } => {
                        let s = srv::schedule::create(&pool, name, apply_args(Default::default(), a), !paused).await?;
                        audit("schedule-create", serde_json::json!({"id": s.id, "name": s.name, "spec": s.spec})).await?;
                        print_schedule(&s, json)?;
                    }
                    ScheduleCmd::List => {
                        for s in srv::schedule::list(&pool).await? {
                            print_schedule(&s, json)?;
                        }
                    }
                    ScheduleCmd::Edit { id, name, a } => {
                        let cur = srv::schedule::get(&pool, id).await?;
                        let s = srv::schedule::update(&pool, &cur.id, name.as_deref(), apply_args(cur.spec.clone(), a)).await?;
                        audit("schedule-update", serde_json::json!({"id": s.id, "name": s.name, "spec": s.spec})).await?;
                        print_schedule(&s, json)?;
                    }
                    ScheduleCmd::Rm { id, force } => {
                        let s = srv::schedule::delete(&pool, id, *force).await?;
                        audit("schedule-delete", serde_json::json!({"id": s.id, "name": s.name})).await?;
                        println!("deleted {}", s.name);
                    }
                    ScheduleCmd::Pause { id } | ScheduleCmd::Resume { id } => {
                        let on = matches!(sc, ScheduleCmd::Resume { .. });
                        let s = srv::schedule::set_enabled(&pool, id, on).await?;
                        audit(if on { "schedule-resume" } else { "schedule-pause" }, serde_json::json!({"id": s.id, "name": s.name})).await?;
                        print_schedule(&s, json)?;
                    }
                    ScheduleCmd::RunNow { id } => {
                        let s = srv::schedule::get(&pool, id).await?;
                        let now = chrono::Utc::now().timestamp_millis();
                        let (o, rid, note) = srv::schedule::fire(&pool, &cfg, &s, now, "run now by itcmdr-cli", "itcmdr-cli", false).await?;
                        audit("schedule-run-now", serde_json::json!({"id": s.id, "outcome": o, "request_id": rid})).await?;
                        println!("{o} {} {note}", rid.unwrap_or_default());
                        // Plugin parts run in this process: wait for them before exiting.
                        if !s.spec.plugins.is_empty() && o == "queued" {
                            for _ in 0..1200 {
                                let n: i64 = sqlx::query_scalar("SELECT count(*) FROM requests r JOIN schedule_runs s ON s.request_id = r.request_id WHERE s.schedule_id = ? AND r.status IN ('queued','running')")
                                    .bind(&s.id).fetch_one(&pool).await?;
                                if n == 0 || !s.spec.categories.is_empty() {
                                    break;
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                        }
                    }
                    ScheduleCmd::History { id, limit } => {
                        let names: std::collections::HashMap<String, String> =
                            srv::schedule::list(&pool).await?.into_iter().map(|s| (s.id, s.name)).collect();
                        for r in srv::schedule::history(&pool, id.as_deref(), *limit).await? {
                            if json {
                                println!("{}", serde_json::to_string(&r)?);
                            } else {
                                println!(
                                    "{}  {}  {}  {}  {}",
                                    ms_local(r.ts_ms),
                                    names.get(&r.schedule_id).cloned().unwrap_or_else(|| r.schedule_id.clone()),
                                    r.outcome,
                                    r.request_id.unwrap_or_default(),
                                    r.note
                                );
                            }
                        }
                    }
                }
                Ok(())
            })
        }
        Cmd::Restore { from, dry_run } => {
            let cfg = load()?;
            let words = read_words()?;
            let r = rt.block_on(srv::backup::restore(&cfg, from, &words, *dry_run))?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else if r.dry_run {
                let m = r.manifest.unwrap();
                println!("DRY RUN OK: archive verified and decrypted; {} files, v{}, schema {}. Nothing was changed.", m.files.len(), m.itcmdr_version, m.schema_version);
            } else {
                if let Some(p) = &r.safety_copy {
                    println!("previous state copied to {}", p.display());
                }
                if let Some(p) = &r.restored_config {
                    println!("archived server.json written to {} (review; the active config was not replaced)", p.display());
                }
                println!(
                    "restored. new root key fingerprint: {}",
                    r.new_root_fingerprint.unwrap_or_default()
                );
                for (t, n) in r.table_counts {
                    println!("  {t}: {n}");
                }
            }
            Ok(())
        }
    }
}
