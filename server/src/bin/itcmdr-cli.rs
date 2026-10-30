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
    /// Users (people with app access).
    #[command(subcommand)]
    User(UserCmd),
    /// Backups (decision 13).
    #[command(subcommand)]
    Backup(BackupCmd),
    /// Runtime settings stored in the DB (override server.json): backup.dir, backup.hour, ...
    #[command(subcommand)]
    Config(ConfigCmd),
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
enum ServiceCmd {
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

#[derive(Subcommand)]
enum ConfigCmd {
    /// Show the effective settings (and which come from the DB).
    Get { key: Option<String> },
    /// Set one key, e.g. `config set backup.hour 3`. Folders must exist and be writable.
    Set { key: String, value: String },
    /// Remove an override (back to server.json / default).
    Unset { key: String },
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Bundle {
    kind: String,
    name: String,
    role: String,
    public_key: String,
    certificate: Certificate,
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
    let cli = Cli::parse();
    let load = || -> Result<ServerConfig> {
        let mut c = ServerConfig::load()?;
        c.data_dir_flag = cli.data_dir.clone();
        Ok(c)
    };
    let rt = tokio::runtime::Runtime::new()?;
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
                    ConfigCmd::Get { key } => {
                        let b = srv::settings::effective_backup(&pool, &cfg).await?;
                        let over = srv::settings::overridden(&pool).await?;
                        let v = serde_json::to_value(&b)?;
                        for k in srv::settings::BACKUP_KEYS {
                            if key.as_deref().is_some_and(|q| q != k) {
                                continue;
                            }
                            let field = &k["backup.".len()..];
                            let val = match &v[field] {
                                serde_json::Value::Null if field == "dir" => {
                                    format!("(default) {}", cfg.layout().backups().display())
                                }
                                x => x.to_string().trim_matches('"').to_string(),
                            };
                            let src = if over.iter().any(|o| o == k) {
                                "db"
                            } else {
                                "server.json/default"
                            };
                            println!("{k} = {val}    [{src}]");
                        }
                        Ok(())
                    }
                    ConfigCmd::Set { key, value } => {
                        srv::settings::set_key(&pool, &cfg, key, value).await?;
                        srv::db::insert_audit(
                            &pool,
                            "itcmdr-cli",
                            "settings-update",
                            &serde_json::json!({"key": key, "value": value}).to_string(),
                        )
                        .await?;
                        println!("{key} = {value}");
                        Ok(())
                    }
                    ConfigCmd::Unset { key } => {
                        srv::settings::unset_key(&pool, key).await?;
                        srv::db::insert_audit(
                            &pool,
                            "itcmdr-cli",
                            "settings-update",
                            &serde_json::json!({"key": key, "unset": true}).to_string(),
                        )
                        .await?;
                        println!("{key} reverted to server.json/default");
                        Ok(())
                    }
                }
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
