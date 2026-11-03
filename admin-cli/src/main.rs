//! itcmdr-admin: every admin app action as a command (ADMINSVR.md decision 4, parity).
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use itcmdr_admin::{proto as pb, Client, Identity};
use std::io::{BufRead, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "itcmdr-admin",
    version,
    about = "itcmdr adminsvr CLI (same signed API as the admin app)"
)]
struct Cli {
    /// Where this device's identity lives (default: the user config dir; env ITCMDR_ADMIN_HOME)
    #[arg(long, global = true)]
    home: Option<PathBuf>,
    /// v1.1.0: trust adminsvr's certificate when it is not trusted by this system (self-signed or
    /// private CA): only this exact SHA-256 is accepted, and it is remembered for this server
    #[arg(long, global = true)]
    trust_fingerprint: Option<String>,
    /// Also accept that certificate when it is expired or does not match the name (explicit override)
    #[arg(long, global = true)]
    trust_anyway: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

/// v1.1.0 certificate trust for this device's adminsvr.
#[derive(Subcommand)]
enum TrustCmd {
    /// How this device trusts adminsvr now
    Show,
    /// Connect and show the certificate adminsvr presents, compared with the trusted one
    Review,
    /// Trust exactly this certificate (it must be the one adminsvr presents now)
    Set {
        sha256: String,
        /// Replace a different, previously trusted certificate
        #[arg(long)]
        replace: bool,
    },
    /// Forget the trusted certificate (validate against the system trust store again)
    Remove,
}

#[derive(Subcommand)]
enum Cmd {
    /// adminsvr version and root key (unsigned)
    Info {
        addr: String,
        /// The internal certificate's SHA-256 (empty = the system trust store)
        #[arg(long, default_value = "")]
        pin: String,
    },
    /// v1.1.0: the certificate adminsvr presents and how this device trusts it
    #[command(subcommand)]
    Trust(TrustCmd),
    /// Enroll this device with a one-time code: shows your 24 words ONCE, then asks three back
    Enroll {
        code: String,
        /// Don't store the encrypted key backup on adminsvr (it is on by default)
        #[arg(long)]
        no_key_backup: bool,
        /// Write the words to this file (0600) instead of showing them (automation)
        #[arg(long)]
        words_file: Option<PathBuf>,
    },
    /// New device from your 24 words (read from stdin): restore the backed-up key or certify a new one
    Recover {
        addr: String,
        #[arg(long, default_value = "")]
        pin: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        from_backup: bool,
    },
    Whoami,
    #[command(subcommand)]
    User(UserCmd),
    #[command(subcommand)]
    Device(DeviceCmd),
    #[command(subcommand, name = "key-backup")]
    KeyBackup(KbCmd),
    #[command(subcommand)]
    Site(SiteCmd),
    #[command(subcommand)]
    Alert(AlertCmd),
    #[command(subcommand)]
    Backup(BackupCmd),
    #[command(subcommand)]
    Command(CommandCmd),
    #[command(subcommand)]
    Audit(AuditCmd),
    /// Passkeys / security keys: the optional second factor (needs libfido2's tools)
    #[command(subcommand)]
    Passkey(PasskeyCmd),
    /// Which actions need a passkey touch, and for which roles a passkey is required (owners set it)
    #[command(subcommand)]
    Policy(PolicyCmd),
    /// Push notifications for alerts on this device (FCM / APNs token; the admin app does this itself)
    #[command(subcommand)]
    Push(PushCmd),
    /// Org-wide settings: whether every approval needs a second owner/admin (owners set it)
    #[command(subcommand)]
    Org(OrgCmd),
    /// Server/agent releases: approve one once, then roll it out to sites (v0.21.0)
    #[command(subcommand)]
    Release(ReleaseCmd),
    /// A site's settings and schedules as adminsvr sees them; set a setting (v0.20.0)
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Run, pause or resume a site's schedule (by the id from `config show`)
    #[command(subcommand)]
    Schedule(ScheduleCmd),
    /// Plugin catalog, a site's plugins, enable/disable/push (two-person)
    #[command(subcommand)]
    Plugin(PluginCmd),
    /// Global freeze of every command (owners)
    #[command(subcommand)]
    Freeze(FreezeCmd),
}

#[derive(Subcommand)]
enum ConfigCmd {
    Show {
        site: String,
        /// also settings whose values stay on the site
        #[arg(long)]
        all: bool,
    },
    /// Set a remote-settable setting (numbers, switches, times)
    Set {
        site: String,
        key: String,
        value: String,
        /// preview only: the site checks and shows before/after, changes nothing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum ScheduleCmd {
    Run {
        site: String,
        id: String,
        #[arg(long)]
        dry_run: bool,
    },
    Pause {
        site: String,
        id: String,
        #[arg(long)]
        dry_run: bool,
    },
    Resume {
        site: String,
        id: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum OrgCmd {
    /// Show the org policy; `--second-approver on|off` changes it (owner only, audited)
    Policy {
        #[arg(long)]
        second_approver: Option<String>,
        /// Warn when a site's last deep backup check is older than this many days (default 90).
        #[arg(long)]
        deep_check_days: Option<i32>,
    },
}

#[derive(Subcommand)]
enum ReleaseCmd {
    /// Server and agent releases in the catalog, and who approved each
    List,
    /// Sign a one-time approval of a release (owners/admins)
    Approve { sha256: String },
    /// Roll an approved release out to a site, in its update window; `--now` ignores the window
    Rollout {
        site: String,
        sha256: String,
        #[arg(long)]
        now: bool,
    },
}

#[derive(Subcommand)]
enum PluginCmd {
    /// Packages in adminsvr's catalog
    Catalog,
    /// Add a signed package (.tar.gz) to the catalog (owners/admins; adminsvr can't sign)
    Upload { file: PathBuf },
    /// Installed plugins and quarantine of a site
    Status { site: String },
    /// Push a catalog package to a site (two-person: a second owner/admin approves)
    Push {
        site: String,
        sha256: String,
        #[arg(long)]
        dry_run: bool,
    },
    Enable {
        site: String,
        name: String,
        #[arg(long, default_value = "")]
        version: String,
        #[arg(long)]
        dry_run: bool,
    },
    Disable {
        site: String,
        name: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum FreezeCmd {
    Status,
    On,
    Off,
}

#[derive(Subcommand)]
enum PushCmd {
    Register {
        /// fcm | apns
        #[arg(long)]
        provider: String,
        #[arg(long)]
        token: String,
        #[arg(long, default_value = "")]
        platform: String,
    },
    /// Remove a token (no token: every token of this device)
    Unregister {
        #[arg(long, default_value = "")]
        token: String,
    },
    /// Send a test push to your own devices
    Test,
}

#[derive(Subcommand)]
enum PasskeyCmd {
    /// Register the security key that is plugged in
    Add {
        #[arg(long)]
        label: String,
        /// Also lock this device's key with it (hmac-secret): every use then needs a touch
        #[arg(long)]
        unlock: bool,
    },
    List {
        #[arg(default_value = "")]
        user: String,
    },
    /// Remove one of your passkeys
    Remove { credential_id: String },
    /// Owner/admin: remove another user's lost passkey (they then add a new one)
    Reset { user: String, credential_id: String },
    /// Lock this device's key with a registered passkey (hmac-secret)
    Lock {
        #[arg(long)]
        credential_id: Option<String>,
    },
    /// Store this device's key unlocked again (one touch)
    Unlock,
}

#[derive(Subcommand)]
enum PolicyCmd {
    Show,
    /// Replace the policy (owners). Lists are comma separated; empty = none.
    Set {
        /// AdminApi methods that need a touch, e.g. SetRole,RevokeDevice
        #[arg(long)]
        stepup: String,
        /// Roles that must have a passkey, e.g. owner
        #[arg(long, default_value = "")]
        required_roles: String,
        /// Require user verification (PIN or biometric) on every touch
        #[arg(long)]
        uv: bool,
    },
}

#[derive(Subcommand)]
enum UserCmd {
    List,
    /// Create a user (role sre by default); prints their one-time enrollment code
    Create {
        email: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "sre")]
        role: String,
    },
    Role {
        user: String,
        role: String,
    },
    Disable {
        user: String,
    },
    Enable {
        user: String,
    },
    /// Assign sites to an SRE (replaces the list)
    Scopes {
        user: String,
        sites: Vec<String>,
    },
    /// Approve a pending device after checking its fingerprint with the user
    Approve {
        user: String,
        fingerprint: String,
    },
    /// A new one-time enrollment code
    Code {
        user: String,
    },
}

#[derive(Subcommand)]
enum DeviceCmd {
    List {
        #[arg(default_value = "")]
        user: String,
    },
    Revoke {
        fingerprint: String,
        #[arg(long, default_value = "")]
        user: String,
    },
}

#[derive(Subcommand)]
enum KbCmd {
    /// Store the encrypted key backup again (needs your 24 words on stdin)
    On,
    /// Opt out: delete the backup from adminsvr
    Off,
}

#[derive(Subcommand)]
enum SiteCmd {
    /// Create a site; prints its one-time site token for `itcmdr-cli adminsvr enroll` on the site
    Create {
        name: String,
    },
    List,
    Revoke {
        site: String,
    },
    /// Status history (numbers only), oldest first
    Metrics {
        site: String,
        /// hours back
        #[arg(long, default_value_t = 24)]
        hours: i64,
    },
}

#[derive(Subcommand)]
enum AlertCmd {
    List {
        #[arg(long, default_value = "")]
        site: String,
        #[arg(long, default_value = "open")]
        state: String,
        #[arg(long, default_value_t = 200)]
        limit: i32,
    },
    Ack {
        site: String,
        alert: String,
    },
    Resolve {
        site: String,
        alert: String,
    },
}

#[derive(Subcommand)]
enum BackupCmd {
    /// Ask a site to start a local backup (the archive stays on the site)
    Start { site: String },
    /// Ask a site to run its restore drill: it checks its newest backup on-site and reports only
    /// ok/failed (adminsvr never sees the archive)
    Drill { site: String },
}

#[derive(Subcommand)]
enum CommandCmd {
    List {
        #[arg(default_value = "")]
        site: String,
    },
    /// State, result and preview of one command
    Show {
        id: String,
    },
    /// Be the second person for a two-person command (owners/admins, not the requester)
    Approve {
        id: String,
    },
    Reject {
        id: String,
    },
}

#[derive(Subcommand)]
enum AuditCmd {
    List {
        #[arg(long, default_value = "")]
        user: String,
        #[arg(long, default_value_t = 50)]
        limit: i32,
    },
    Verify,
}

fn ts(ms: i64) -> String {
    if ms <= 0 {
        return "-".into();
    }
    format!("{}s ago", (itcmdr_keys::now_ms() - ms) / 1000)
}

fn user(u: &pb::UserInfo) {
    println!(
        "{}  {}  {}  {}  {}  sites=[{}]  key_backup={}{}",
        u.id,
        u.email,
        u.name,
        u.role,
        u.status,
        u.sites.join(","),
        u.key_backup,
        if u.key_backup_opt_out {
            " (opted out)"
        } else {
            ""
        }
    );
    for d in &u.devices {
        println!("    device {}  {}  {}", d.fingerprint, d.status, d.platform);
    }
}

fn print_tls(i: &pb::InfoResponse) {
    if i.tls_mode == "acme" {
        let days = (i.tls_expires_ms - itcmdr_keys::now_ms()) / 86_400_000;
        println!(
            "TLS: ACME for {}  expires in {days} days  issuer {}{}",
            i.tls_domains.join(", "),
            i.tls_issuer,
            if i.tls_renewal_failing {
                "  WARNING: the last renewal failed (itcmdr-adminsvr tls-status on the server)"
            } else {
                ""
            }
        );
    } else if !i.tls_mode.is_empty() {
        println!("TLS: internal certificate {}", i.internal_fingerprint);
    }
}

fn trust_text(id: &Identity) -> String {
    if id.pin.is_empty() {
        format!("{}: validated against the system trust store", id.addr)
    } else if id.pin_tofu {
        format!(
            "{}: trusted certificate (chosen on this device) {}",
            id.addr, id.pin
        )
    } else {
        format!(
            "{}: pinned certificate from the enrollment code {}",
            id.addr, id.pin
        )
    }
}

/// A refused certificate: never connect silently. `--trust-fingerprint` (or a typed answer on a
/// terminal) trusts exactly the certificate shown; a changed trusted certificate is refused.
fn decide_trust(
    id: &mut Identity,
    r: &itcmdr_admin::CertReport,
    flag: Option<&str>,
    anyway: bool,
) -> Result<()> {
    use std::io::IsTerminal;
    if r.problem == "pin-changed" {
        bail!(
            "{}\nreview it with `itcmdr-admin trust review`; if you are sure, `itcmdr-admin trust set <sha256> --replace`",
            itcmdr_admin::CertProblem(r.clone())
        );
    }
    let fp = match flag {
        Some(f) => itcmdr_admin::check_choice(r, f)?,
        None if std::io::stdin().is_terminal() => {
            eprintln!("{} is not trusted by this system.\n{}", id.addr, r.describe());
            let want = if r.needs_override() { "trust anyway" } else { "trust" };
            eprint!("Type '{want}' to trust exactly this certificate for {} (anything else cancels): ", id.addr);
            let mut s = String::new();
            std::io::stdin().lock().read_line(&mut s)?;
            if s.trim() != want {
                bail!("not trusted; nothing was changed");
            }
            r.fingerprint.clone()
        }
        None => bail!(
            "{} is not trusted by this system.\n{}\nto trust exactly this certificate, run again with --trust-fingerprint {}",
            id.addr,
            r.describe(),
            r.fingerprint
        ),
    };
    if flag.is_some() && r.needs_override() && !anyway {
        bail!(
            "{}\nthis certificate is {}: add --trust-anyway to accept it explicitly",
            r.describe(),
            r.problem
        );
    }
    id.trust_cert(&fp);
    eprintln!("trusted {fp} for {}", id.addr);
    Ok(())
}

fn read_words() -> Result<String> {
    eprint!("24 words: ");
    let mut s = String::new();
    std::io::stdin().lock().read_line(&mut s)?;
    Ok(s.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = cli.home.clone().unwrap_or_else(itcmdr_admin::default_dir);
    let platform = format!("cli-{}", std::env::consts::OS);
    match cli.cmd {
        Cmd::Info { addr, pin } => {
            let mut api = pb::admin_api_client::AdminApiClient::new(
                itcmdr_admin::channel(&addr, &pin).await?,
            );
            let i = api.info(pb::Empty {}).await?.into_inner();
            println!(
                "adminsvr {}  initialized={}  root={}",
                i.version, i.initialized, i.root_public_key
            );
            print_tls(&i);
        }
        Cmd::Enroll {
            code,
            no_key_backup,
            words_file,
        } => {
            if Identity::path(&home).exists() {
                bail!(
                    "this device is already enrolled ({}); remove it first",
                    Identity::path(&home).display()
                );
            }
            let e = itcmdr_admin::enroll(&code, &platform, no_key_backup, None).await?;
            e.identity.save(&home)?;
            match words_file {
                Some(p) => {
                    std::fs::write(&p, e.words.as_bytes())?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
                    }
                }
                None => {
                    let w: Vec<&str> = e.words.split_whitespace().collect();
                    println!("Your 24 words (write them down on paper now; they are shown once):");
                    for (i, c) in w.chunks(6).enumerate() {
                        println!("  {:>2}-{:>2}: {}", i * 6 + 1, i * 6 + 6, c.join(" "));
                    }
                    for i in [3usize, 11, 20] {
                        loop {
                            print!("word {}: ", i + 1);
                            std::io::stdout().flush()?;
                            let mut s = String::new();
                            std::io::stdin().lock().read_line(&mut s)?;
                            if s.trim().eq_ignore_ascii_case(w[i]) {
                                break;
                            }
                            println!("  not that one, try again");
                        }
                    }
                    print!("\x1b[2J\x1b[H");
                }
            }
            let d = e
                .user
                .devices
                .last()
                .map(|d| (d.fingerprint.clone(), d.status.clone()))
                .unwrap_or_default();
            println!(
                "Enrolled {} ({}); device {} is {}.",
                e.user.email, e.user.role, d.0, d.1
            );
            if d.1 == "pending" {
                println!("Read this fingerprint to the owner/admin who created you; they run: itcmdr-admin user approve {} {}", e.user.email, d.0);
            }
            println!(
                "Key backup on adminsvr: {}",
                if no_key_backup {
                    "off (opted out)"
                } else {
                    "on (encrypted with your words)"
                }
            );
        }
        Cmd::Recover {
            addr,
            pin,
            email,
            from_backup,
        } => {
            let words = read_words()?;
            let id = itcmdr_admin::recover(
                &addr,
                &pin,
                &email,
                &words,
                from_backup,
                &platform,
                Some(fido()),
            )
            .await?;
            id.save(&home)?;
            println!(
                "device key {} on this device",
                if from_backup {
                    "restored from the backup"
                } else {
                    "certified"
                }
            );
        }
        Cmd::Trust(t) => {
            let mut id = Identity::load(&home)?;
            match t {
                TrustCmd::Show => println!("{}", trust_text(&id)),
                TrustCmd::Review => {
                    println!("{}", trust_text(&id));
                    let r = itcmdr_admin::inspect(&id.addr, &id.pin, id.pin_tofu).await?;
                    println!("{} presents:\n{}", id.addr, r.describe());
                    if r.problem == "ok" {
                        println!("result: trusted");
                    }
                }
                TrustCmd::Set { sha256, replace } => {
                    let r = itcmdr_admin::inspect(&id.addr, "", false).await?;
                    if !id.pin.is_empty() && id.pin != r.fingerprint && !replace {
                        bail!("a different certificate ({}) is trusted for {}; add --replace after reviewing (itcmdr-admin trust review)", id.pin, id.addr);
                    }
                    let fp = itcmdr_admin::check_choice(&r, &sha256)?;
                    if r.needs_override() && !cli.trust_anyway {
                        bail!("{}\nthis certificate is {}: add --trust-anyway to accept it explicitly", r.describe(), r.problem);
                    }
                    id.trust_cert(&fp);
                    id.save(&home)?;
                    println!("trusted {fp} for {}", id.addr);
                }
                TrustCmd::Remove => {
                    id.forget_cert();
                    id.save(&home)?;
                    println!(
                        "no trusted certificate for {}: the system trust store decides again",
                        id.addr
                    );
                }
            }
        }
        cmd => {
            let mut id = Identity::load(&home)?;
            let c = match Client::connect_with(&id, Some(fido())).await {
                Ok(c) => c,
                Err(e) => match itcmdr_admin::cert_problem(&e) {
                    Some(r) => {
                        decide_trust(
                            &mut id,
                            &r,
                            cli.trust_fingerprint.as_deref(),
                            cli.trust_anyway,
                        )?;
                        id.save(&home)?;
                        Client::connect_with(&id, Some(fido())).await?
                    }
                    None => return Err(e),
                },
            };
            match cmd {
                Cmd::Passkey(pc) => {
                    match pc {
                        PasskeyCmd::Add { label, unlock } => {
                            let (lp, list) = c.add_passkey(&label, unlock).await?;
                            println!(
                                "passkey {} ({}) registered; you have {}",
                                lp.label,
                                lp.credential_id,
                                list.passkeys.len()
                            );
                            id.passkeys.push(lp.clone());
                            if unlock {
                                let seed = id
                                    .key_with(Some(fido().as_ref()))
                                    .map(|k| hex::encode(k.to_bytes()))?;
                                id.locked =
                                    Some(itcmdr_admin::passkey::lock(fido().as_ref(), &lp, &seed)?);
                                id.device_seed.clear();
                                println!("this device's key is now locked with it (a touch for every use)");
                            }
                            id.save(&home)?;
                        }
                        PasskeyCmd::List { user: u } => {
                            for p in c.list_passkeys(&u).await?.passkeys {
                                println!(
                                    "{}  {}  alg {}  {}  count {}  hmac {}  last used {}",
                                    p.credential_id,
                                    p.label,
                                    p.alg,
                                    p.attestation,
                                    p.sign_count,
                                    p.hmac_secret,
                                    ts(p.last_used_ms)
                                );
                            }
                        }
                        PasskeyCmd::Remove { credential_id } => {
                            let l = c.remove_passkey("", &credential_id).await?;
                            id.passkeys.retain(|p| p.credential_id != credential_id);
                            id.save(&home)?;
                            println!("removed; {} left", l.passkeys.len());
                        }
                        PasskeyCmd::Reset {
                            user: u,
                            credential_id,
                        } => {
                            let l = c.remove_passkey(&u, &credential_id).await?;
                            println!("reset; {u} has {} left", l.passkeys.len());
                        }
                        PasskeyCmd::Lock { credential_id } => {
                            if id.locked.is_some() {
                                bail!("already locked");
                            }
                            let lp = id
                            .passkeys
                            .iter()
                            .find(|p| p.hmac_secret && credential_id.as_ref().is_none_or(|c| *c == p.credential_id))
                            .cloned()
                            .ok_or_else(|| anyhow::anyhow!("no passkey with hmac-secret on this device (passkey add --unlock)"))?;
                            id.locked = Some(itcmdr_admin::passkey::lock(
                                fido().as_ref(),
                                &lp,
                                &id.device_seed,
                            )?);
                            id.device_seed.clear();
                            id.save(&home)?;
                            println!("locked with {}", lp.label);
                        }
                        PasskeyCmd::Unlock => {
                            let k = id.key_with(Some(fido().as_ref()))?;
                            id.device_seed = hex::encode(k.to_bytes());
                            id.locked = None;
                            id.save(&home)?;
                            println!("unlocked (stored without the passkey again)");
                        }
                    }
                }
                Cmd::Org(OrgCmd::Policy {
                    second_approver,
                    deep_check_days,
                }) => {
                    if let Some(d) = deep_check_days {
                        anyhow::ensure!((1..=3650).contains(&d), "--deep-check-days 1..3650");
                        c.set_deep_check_days(d).await?;
                    }
                    let p = match second_approver.as_deref() {
                        None => c.get_org_policy().await?,
                        Some("on" | "true") => c.set_org_policy(true).await?,
                        Some("off" | "false") => c.set_org_policy(false).await?,
                        Some(x) => anyhow::bail!("--second-approver on|off, not {x}"),
                    };
                    println!(
                        "second approver: {}  deep check warning after {} days{}",
                        if p.require_second_approver {
                            "required"
                        } else {
                            "off"
                        },
                        p.deep_check_max_days,
                        if p.updated_ms > 0 {
                            format!("  (changed {} by {})", ts(p.updated_ms), p.updated_by)
                        } else {
                            String::new()
                        }
                    );
                }
                Cmd::Release(rc) => match rc {
                    ReleaseCmd::List => {
                        for r in c.list_releases().await?.releases {
                            println!(
                                "{}  {} {}  {}  approved by [{}] ({} needed)",
                                &r.sha256[..r.sha256.len().min(16)],
                                r.name,
                                r.version,
                                if r.approved {
                                    "approved"
                                } else {
                                    "not approved"
                                },
                                r.approved_by.join(", "),
                                r.needed
                            );
                        }
                    }
                    ReleaseCmd::Approve { sha256 } => {
                        let r = c.approve_release(&sha256).await?;
                        println!(
                            "{} {}: {} of {} approvals{}",
                            r.name,
                            r.version,
                            r.approved_by.len(),
                            r.needed,
                            if r.approved {
                                ", ready to roll out"
                            } else {
                                ""
                            }
                        );
                    }
                    ReleaseCmd::Rollout { site, sha256, now } => {
                        let x = c.rollout_release(&site, &sha256, now).await?;
                        show_command(&x);
                        if x.state == "awaiting-approval" {
                            println!(
                                "needs a second owner or admin: itcmdr-admin command approve {}",
                                x.id
                            );
                        }
                    }
                },
                Cmd::Push(pc) => match pc {
                    PushCmd::Register {
                        provider,
                        token,
                        platform,
                    } => {
                        c.register_push(&provider, &token, &platform).await?;
                        println!("registered");
                    }
                    PushCmd::Unregister { token } => {
                        c.unregister_push(&token).await?;
                        println!("unregistered");
                    }
                    PushCmd::Test => {
                        let r = c.test_push().await?;
                        println!(
                            "devices={} sent={} failed={} providers={}",
                            r.devices,
                            r.sent,
                            r.failed,
                            r.providers.join(",")
                        );
                    }
                },
                Cmd::Policy(pc) => {
                    let p = match pc {
                        PolicyCmd::Show => c.get_auth_policy().await?,
                        PolicyCmd::Set {
                            stepup,
                            required_roles,
                            uv,
                        } => {
                            let l = |s: &str| {
                                s.split(',')
                                    .map(str::trim)
                                    .filter(|x| !x.is_empty())
                                    .map(String::from)
                                    .collect()
                            };
                            c.set_auth_policy(pb::AuthPolicy {
                                stepup_actions: l(&stepup),
                                passkey_required_roles: l(&required_roles),
                                require_user_verification: uv,
                            })
                            .await?
                        }
                    };
                    println!("step-up actions: {}", p.stepup_actions.join(", "));
                    println!(
                        "passkey required for roles: {}",
                        if p.passkey_required_roles.is_empty() {
                            "(none)".into()
                        } else {
                            p.passkey_required_roles.join(", ")
                        }
                    );
                    println!(
                        "user verification required: {}",
                        p.require_user_verification
                    );
                }
                Cmd::Whoami => user(&c.whoami().await?),
                Cmd::User(u) => match u {
                    UserCmd::List => {
                        for x in c.list_users().await?.users {
                            user(&x)
                        }
                    }
                    UserCmd::Create { email, name, role } => {
                        let r = c.create_user(&email, &name, &role).await?;
                        user(r.user.as_ref().unwrap());
                        println!("enrollment code (24 h, single use):\n{}", r.enroll_code);
                    }
                    UserCmd::Role { user: u, role } => user(&c.set_role(&u, &role).await?),
                    UserCmd::Disable { user: u } => user(&c.set_disabled(&u, true).await?),
                    UserCmd::Enable { user: u } => user(&c.set_disabled(&u, false).await?),
                    UserCmd::Scopes { user: u, sites } => user(&c.set_scopes(&u, sites).await?),
                    UserCmd::Approve {
                        user: u,
                        fingerprint,
                    } => user(&c.approve_device(&u, &fingerprint).await?),
                    UserCmd::Code { user: u } => println!("{}", c.new_code(&u).await?.enroll_code),
                },
                Cmd::Device(d) => {
                    let l = match d {
                        DeviceCmd::List { user: u } => c.list_devices(&u).await?,
                        DeviceCmd::Revoke {
                            fingerprint,
                            user: u,
                        } => c.revoke_device(&u, &fingerprint).await?,
                    };
                    for d in l.devices {
                        println!(
                            "{}  {}  {}  used {}",
                            d.fingerprint,
                            d.status,
                            d.platform,
                            ts(d.last_used_ms)
                        );
                    }
                }
                Cmd::KeyBackup(k) => match k {
                    KbCmd::On => {
                        let words = read_words()?;
                        let seed: [u8; 32] = hex::decode(&id.device_seed)?
                            .try_into()
                            .map_err(|_| anyhow::anyhow!("bad seed"))?;
                        let w = itcmdr_keys::wrap::wrap(&words, &id.email.to_lowercase(), &seed)?;
                        c.put_key_backup(serde_json::to_string(&w)?).await?;
                        println!("key backup stored (encrypted with your words)");
                    }
                    KbCmd::Off => user(&c.key_backup_opt_out(true).await?),
                },
                Cmd::Site(s) => match s {
                    SiteCmd::Create { name } => {
                        let r = c.create_site(&name).await?;
                        println!("site {} created. Site token (24 h, single use), on the site server run:\n  itcmdr-cli adminsvr enroll {}", r.site.unwrap().id, r.token);
                    }
                    SiteCmd::List => {
                        let max = c.get_org_policy().await?.deep_check_max_days;
                        let now = itcmdr_keys::now_ms();
                        for s in c.list_sites().await?.sites {
                            let st = s.last_status.unwrap_or_default();
                            println!("{}  {}  {}  v{}  seen {}  backup_age={}s ({})  {}  agents online/late/nr={}/{}/{}  names_shared={}", s.id, s.name, s.status, s.version, ts(s.last_seen_ms), st.backup_age_secs, st.backup_last_result, itcmdr_adminlink::deep_check_text(&st, now), st.agents_online, st.agents_late, st.agents_non_responsive, s.share_display_names);
                            if let (_, Some(w)) = itcmdr_adminlink::deep_check(&st, max, now) {
                                println!("  WARNING: {w}");
                            }
                        }
                    }
                    SiteCmd::Revoke { site } => {
                        println!("{} {}", site, c.revoke_site(&site).await?.status)
                    }
                    SiteCmd::Metrics { site, hours } => {
                        let since = itcmdr_keys::now_ms() - hours * 3600 * 1000;
                        for m in c.site_metrics(&site, since, 0).await?.samples {
                            let st = m.status.unwrap_or_default();
                            let oa = |i: usize| st.open_alerts.get(i).copied().unwrap_or(0);
                            println!("{}  agents online/late/nr/other={}/{}/{}/{}  alerts c/w/i={}/{}/{}  backup_age={}s ({})  uptime={}s  v{}", ts(m.ts_ms), st.agents_online, st.agents_late, st.agents_non_responsive, st.agents_other, oa(0), oa(1), oa(2), st.backup_age_secs, st.backup_last_result, st.uptime_secs, st.version);
                        }
                    }
                },
                Cmd::Alert(a) => match a {
                    AlertCmd::List { site, state, limit } => {
                        for a in c.list_alerts(&site, &state, limit).await?.alerts {
                            let x = a.alert.unwrap_or_default();
                            println!(
                                "{}  {}  {}  {}  {}  machine={}{}  plugin={}  x{}  last {}",
                                a.site,
                                x.id,
                                x.severity,
                                x.state,
                                x.code,
                                x.machine,
                                if x.machine_display_name.is_empty() {
                                    String::new()
                                } else {
                                    format!(" ({})", x.machine_display_name)
                                },
                                x.plugin,
                                x.count,
                                ts(x.last_seen_ms)
                            );
                        }
                    }
                    AlertCmd::Ack { site, alert } => println!(
                        "{:?}",
                        c.command(&site, "alert.ack", &[("alert_id", &alert)])
                            .await?
                    ),
                    AlertCmd::Resolve { site, alert } => println!(
                        "{:?}",
                        c.command(&site, "alert.resolve", &[("alert_id", &alert)])
                            .await?
                    ),
                },
                Cmd::Backup(BackupCmd::Start { site }) => {
                    let r = c.command(&site, "backup.start", &[]).await?;
                    println!("queued {} (the backup runs and stays on the site)", r.id);
                }
                Cmd::Backup(BackupCmd::Drill { site }) => {
                    let r = c.command(&site, "backup.drill", &[]).await?;
                    show_command(
                        &c.wait_command(&r.id, std::time::Duration::from_secs(60))
                            .await?,
                    );
                }
                Cmd::Command(CommandCmd::Show { id }) => {
                    show_command(&c.get_command(&id).await?.info.unwrap_or_default());
                }
                Cmd::Command(CommandCmd::Approve { id }) => {
                    let x = c.approve_command(&id).await?;
                    show_command(
                        &c.wait_command(&x.id, std::time::Duration::from_secs(20))
                            .await?,
                    );
                }
                Cmd::Command(CommandCmd::Reject { id }) => {
                    show_command(&c.reject_command(&id).await?)
                }
                Cmd::Config(cc) => match cc {
                    ConfigCmd::Show { site, all } => {
                        let i = c.site_config(&site).await?;
                        let Some(d) = i.config else {
                            println!("{site} has not sent its configuration yet");
                            return Ok(());
                        };
                        println!(
                            "version {} (received {}), two-person {}, allowed: {}",
                            &d.version[..d.version.len().min(12)],
                            ts(i.received_ms),
                            d.require_two_person,
                            d.allowed.join(" ")
                        );
                        for x in d.settings.iter().filter(|x| all || !x.hidden) {
                            println!(
                                "  {:<36} {:<5} {:<24} {}{}",
                                x.key,
                                x.kind,
                                if x.hidden {
                                    "(on the site)".into()
                                } else {
                                    x.value.clone()
                                },
                                x.source,
                                if x.remote { "" } else { "  read-only here" }
                            );
                        }
                        println!("schedules:");
                        for x in &d.schedules {
                            println!(
                                "  {}  {:<8} {}  {} {}  {}",
                                x.id,
                                if x.enabled { "enabled" } else { "paused" },
                                x.action,
                                x.when,
                                x.timezone,
                                if x.name.is_empty() {
                                    x.builtin.clone()
                                } else {
                                    x.name.clone()
                                }
                            );
                        }
                    }
                    ConfigCmd::Set {
                        site,
                        key,
                        value,
                        dry_run,
                    } => {
                        run_cmd(
                            &c,
                            &site,
                            "setting.set",
                            &[("key", &key), ("value", &value)],
                            dry_run,
                        )
                        .await?
                    }
                },
                Cmd::Schedule(sc) => {
                    let (k, site, id, dry) = match sc {
                        ScheduleCmd::Run { site, id, dry_run } => {
                            ("schedule.run", site, id, dry_run)
                        }
                        ScheduleCmd::Pause { site, id, dry_run } => {
                            ("schedule.pause", site, id, dry_run)
                        }
                        ScheduleCmd::Resume { site, id, dry_run } => {
                            ("schedule.resume", site, id, dry_run)
                        }
                    };
                    run_cmd(&c, &site, k, &[("schedule", &id)], dry).await?
                }
                Cmd::Plugin(pc) => match pc {
                    PluginCmd::Catalog => {
                        for p in c.catalog().await?.packages {
                            println!(
                                "{}  {} {}  {} bytes  by {}  {}",
                                p.sha256,
                                p.name,
                                p.version,
                                p.size,
                                p.uploaded_by,
                                ts(p.uploaded_ms)
                            );
                        }
                    }
                    PluginCmd::Upload { file } => {
                        let e = c.upload_plugin(std::fs::read(&file)?).await?;
                        println!("{} {} added: {}", e.name, e.version, e.sha256);
                    }
                    PluginCmd::Status { site } => {
                        let i = c.site_plugins(&site).await?;
                        let d = i.plugins.unwrap_or_default();
                        for p in d.installed {
                            println!(
                                "  {} {}  {}  {}",
                                p.name,
                                p.version,
                                if p.enabled { "enabled" } else { "disabled" },
                                &p.sha256[..p.sha256.len().min(16)]
                            );
                        }
                        if !d.quarantine.is_empty() {
                            println!("quarantine:");
                            for q in d.quarantine {
                                println!(
                                    "  {}  {}  {}",
                                    &q.sha256[..q.sha256.len().min(16)],
                                    q.reason,
                                    ts(q.ts_ms)
                                );
                            }
                        }
                    }
                    PluginCmd::Push {
                        site,
                        sha256,
                        dry_run,
                    } => {
                        run_cmd(&c, &site, "plugin.install", &[("sha256", &sha256)], dry_run)
                            .await?
                    }
                    PluginCmd::Enable {
                        site,
                        name,
                        version,
                        dry_run,
                    } => {
                        let mut a = vec![("name", name.as_str())];
                        if !version.is_empty() {
                            a.push(("version", version.as_str()));
                        }
                        run_cmd(&c, &site, "plugin.enable", &a, dry_run).await?
                    }
                    PluginCmd::Disable {
                        site,
                        name,
                        dry_run,
                    } => run_cmd(&c, &site, "plugin.disable", &[("name", &name)], dry_run).await?,
                },
                Cmd::Freeze(f) => {
                    let st = match f {
                        FreezeCmd::Status => c.freeze().await?,
                        FreezeCmd::On => c.set_freeze(true).await?,
                        FreezeCmd::Off => c.set_freeze(false).await?,
                    };
                    if st.frozen {
                        println!(
                            "FROZEN by {} since {}: every command is refused",
                            st.by,
                            ts(st.since_ms)
                        );
                    } else {
                        println!("not frozen");
                    }
                }
                Cmd::Command(CommandCmd::List { site }) => {
                    for x in c.list_commands(&site).await?.commands {
                        println!(
                            "{}  {}  {}  {}  {}  by {}  {}",
                            x.id,
                            x.site,
                            x.kind,
                            x.state,
                            x.result_code,
                            x.requested_by,
                            ts(x.created_ms)
                        );
                    }
                }
                Cmd::Audit(a) => match a {
                    AuditCmd::List { user: u, limit } => {
                        for e in c.list_audit(&u, limit).await?.entries {
                            println!(
                                "{:>6}  {}  {}  {}  {}  {}",
                                e.seq,
                                ts(e.ts_ms),
                                e.actor,
                                e.action,
                                e.target,
                                &e.hash[..16]
                            );
                        }
                    }
                    AuditCmd::Verify => {
                        let v = c.verify_audit().await?;
                        if v.ok {
                            println!("audit chain OK ({} entries)", v.entries)
                        } else {
                            bail!(
                                "audit chain BROKEN at seq {}: {}",
                                v.first_bad_seq,
                                v.problem
                            )
                        }
                    }
                },
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}

fn fido() -> std::sync::Arc<dyn itcmdr_admin::passkey::Authenticator> {
    std::sync::Arc::new(itcmdr_admin::passkey::Fido2Tools::new())
}

fn show_command(x: &pb::CommandInfo) {
    let mut args: Vec<_> = x.args.iter().map(|(k, v)| format!("{k}={v}")).collect();
    args.sort();
    println!(
        "{}  {}{}  {}  {}  {}  by {}{}",
        x.id,
        x.kind,
        if x.dry_run { " (dry run)" } else { "" },
        args.join(" "),
        x.state,
        x.result_code,
        x.requested_by,
        if x.approved_by.is_empty() {
            String::new()
        } else {
            format!(", approved by {}", x.approved_by)
        }
    );
    for p in &x.preview {
        println!("  {}: {} -> {}", p.target, p.before, p.after);
    }
}

async fn run_cmd(
    c: &itcmdr_admin::Client,
    site: &str,
    kind: &str,
    args: &[(&str, &str)],
    dry_run: bool,
) -> anyhow::Result<()> {
    let x = c.command_ex(site, kind, args, dry_run).await?;
    if x.state == "awaiting-approval" {
        show_command(&x);
        println!(
            "needs a second owner or admin: itcmdr-admin command approve {}",
            x.id
        );
        return Ok(());
    }
    show_command(
        &c.wait_command(&x.id, std::time::Duration::from_secs(20))
            .await?,
    );
    Ok(())
}
