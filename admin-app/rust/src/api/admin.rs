//! What itcmdr Admin (Flutter) calls. Everything goes through `itcmdr-admin-cli`'s client, so
//! the app signs every request with the device key, pins adminsvr's TLS key and answers passkey
//! step-ups exactly like the CLI. Functions block on a shared tokio runtime; flutter_rust_bridge
//! runs them off the UI thread (they are async in Dart).
use anyhow::{anyhow, bail, Result};
use flutter_rust_bridge::frb;
use itcmdr_admin::passkey::{Authenticator, Fido2Tools};
use itcmdr_admin::{proto as pb, Client, Identity};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

struct Session {
    dir: PathBuf,
    id: Identity,
    client: Arc<Client>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

fn block<F: std::future::Future>(f: F) -> F::Output {
    rt().block_on(f)
}

fn client() -> Result<Arc<Client>> {
    SESSION
        .lock()
        .as_ref()
        .map(|s| s.client.clone())
        .ok_or_else(|| anyhow!("not signed in"))
}

/// The security key driver on desktops (libfido2's tools), when they are installed.
fn authenticator() -> Option<Arc<dyn Authenticator>> {
    if cfg!(any(target_os = "android", target_os = "ios")) {
        return None;
    }
    which("fido2-token").map(|_| Arc::new(Fido2Tools::new()) as Arc<dyn Authenticator>)
}

fn which(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let names: Vec<String> = if cfg!(windows) {
        vec![format!("{exe}.exe")]
    } else {
        vec![exe.to_string()]
    };
    std::env::split_paths(&path)
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .find(|p| p.is_file())
}

fn platform() -> String {
    format!("app-{}", std::env::consts::OS)
}

// ---- DTOs (mirrored to Dart) ----

pub struct Me {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub server: String,
    pub passkeys: i32,
    pub locked: bool,
    /// A security key driver is available on this device
    pub authenticator: bool,
    pub key_backup: bool,
}

pub struct Enrolled {
    /// The 24 words: shown once, never stored by the app
    pub words: String,
    pub me: Me,
}

pub struct Device {
    pub fingerprint: String,
    pub platform: String,
    pub status: String,
    pub created_ms: i64,
    pub last_used_ms: i64,
}

pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub sites: Vec<String>,
    pub key_backup: bool,
    pub passkeys: i32,
    pub created_ms: i64,
    pub devices: Vec<Device>,
}

pub struct Machine {
    pub id: String,
    pub state: String,
    pub last_seen_ms: i64,
    pub version: String,
    pub display_name: String,
    /// The agent runs another version than the site server
    pub update_available: bool,
}

pub struct SiteStatus {
    pub version: String,
    pub uptime_secs: i64,
    pub auth_mode: String,
    pub cutover_state: String,
    pub backup_age_secs: i64,
    pub backup_size_bytes: i64,
    pub backup_last_result: String,
    pub agents_online: i32,
    pub agents_late: i32,
    pub agents_non_responsive: i32,
    pub agents_other: i32,
    pub alerts_critical: i64,
    pub alerts_cautious: i64,
    pub alerts_informational: i64,
    pub machines: Vec<Machine>,
    // v0.20.0: last 24 h summaries and disk space (no paths)
    pub disk_free_bytes: i64,
    pub disk_total_bytes: i64,
    pub runs_24h: i64,
    pub runs_failed_24h: i64,
    pub run_secs_avg_24h: i64,
    pub run_secs_max_24h: i64,
    pub plugin_errors_24h: i64,
    pub alerts_opened_24h: i64,
    /// v1.0.0: "last deep check: N days ago (ok)" or "last deep check: never"
    pub deep_check_text: String,
    /// v1.0.0: empty, or why the deep backup check needs attention (never, failed, too old)
    pub deep_check_warning: String,
}

pub struct Site {
    pub id: String,
    pub name: String,
    pub status: String,
    pub version: String,
    pub last_seen_ms: i64,
    pub share_display_names: bool,
    pub last_status: Option<SiteStatus>,
}

pub struct Sample {
    pub ts_ms: i64,
    pub agents_online: i32,
    pub agents_late: i32,
    pub agents_non_responsive: i32,
    pub agents_other: i32,
    pub alerts_critical: i64,
    pub alerts_cautious: i64,
    pub alerts_informational: i64,
    pub backup_age_secs: i64,
    pub uptime_secs: i64,
}

pub struct Alert {
    pub site: String,
    pub id: String,
    pub code: String,
    pub severity: String,
    pub state: String,
    pub source: String,
    pub plugin: String,
    pub machine: String,
    pub machine_display_name: String,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
    pub count: i64,
    pub origin: String,
}

pub struct CommandRow {
    pub id: String,
    pub site: String,
    pub kind: String,
    pub state: String,
    pub result_code: String,
    pub requested_by: String,
    pub created_ms: i64,
    // v0.20.0
    pub dry_run: bool,
    pub preview: Vec<Change>,
    pub approved_by: String,
    /// "key=value" pairs, sorted
    pub args: Vec<String>,
}

/// One before/after line of a dry run or an applied command.
pub struct Change {
    pub target: String,
    pub before: String,
    pub after: String,
}

pub struct AuditRow {
    pub seq: i64,
    pub ts_ms: i64,
    pub actor: String,
    pub action: String,
    pub target: String,
}

pub struct Policy {
    pub stepup_actions: Vec<String>,
    pub passkey_required_roles: Vec<String>,
    pub require_user_verification: bool,
}

pub struct Passkey {
    pub credential_id: String,
    pub label: String,
    pub sign_count: i64,
    pub hmac_secret: bool,
    pub created_ms: i64,
    pub last_used_ms: i64,
}

pub struct PushResult {
    pub devices: i32,
    pub sent: i32,
    pub failed: i32,
    pub providers: Vec<String>,
}

fn me_of(u: pb::UserInfo, id: &Identity) -> Me {
    Me {
        id: u.id,
        email: u.email,
        name: u.name,
        role: u.role,
        server: id.addr.clone(),
        passkeys: u.passkeys,
        locked: id.locked.is_some(),
        authenticator: authenticator().is_some(),
        key_backup: u.key_backup,
    }
}

fn user_of(u: pb::UserInfo) -> User {
    User {
        id: u.id,
        email: u.email,
        name: u.name,
        role: u.role,
        status: u.status,
        sites: u.sites,
        key_backup: u.key_backup,
        passkeys: u.passkeys,
        created_ms: u.created_ms,
        devices: u.devices.into_iter().map(device_of).collect(),
    }
}

fn device_of(d: pb::DeviceInfo) -> Device {
    Device {
        fingerprint: d.fingerprint,
        platform: d.platform,
        status: d.status,
        created_ms: d.created_ms,
        last_used_ms: d.last_used_ms,
    }
}

fn alerts3(v: &[i64]) -> (i64, i64, i64) {
    let g = |i: usize| v.get(i).copied().unwrap_or(0);
    (g(0), g(1), g(2))
}

/// The org's deep-check limit, refreshed by `sites()` (v1.0.0).
static DEEP_MAX: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

fn status_of(s: pb::SiteStatus) -> SiteStatus {
    let (c, w, i) = alerts3(&s.open_alerts);
    let now = itcmdr_keys::now_ms();
    let deep_check_text = itcmdr_adminlink::deep_check_text(&s, now);
    let deep_check_warning =
        itcmdr_adminlink::deep_check(&s, DEEP_MAX.load(std::sync::atomic::Ordering::Relaxed), now)
            .1
            .unwrap_or_default();
    let v = s.version.clone();
    SiteStatus {
        machines: s
            .machines
            .into_iter()
            .map(|m| Machine {
                update_available: !m.version.is_empty() && !v.is_empty() && m.version != v,
                id: m.id,
                state: m.state,
                last_seen_ms: m.last_seen_ms,
                version: m.version,
                display_name: m.display_name,
            })
            .collect(),
        version: s.version,
        uptime_secs: s.uptime_secs,
        auth_mode: s.auth_mode,
        cutover_state: s.cutover_state,
        backup_age_secs: s.backup_age_secs,
        backup_size_bytes: s.backup_size_bytes,
        backup_last_result: s.backup_last_result,
        agents_online: s.agents_online,
        agents_late: s.agents_late,
        agents_non_responsive: s.agents_non_responsive,
        agents_other: s.agents_other,
        alerts_critical: c,
        alerts_cautious: w,
        alerts_informational: i,
        disk_free_bytes: s.disk_free_bytes,
        disk_total_bytes: s.disk_total_bytes,
        runs_24h: s.runs_24h as i64,
        runs_failed_24h: s.runs_failed_24h as i64,
        run_secs_avg_24h: s.run_secs_avg_24h,
        run_secs_max_24h: s.run_secs_max_24h,
        plugin_errors_24h: s.plugin_errors_24h as i64,
        alerts_opened_24h: s.alerts_opened_24h as i64,
        deep_check_text,
        deep_check_warning,
    }
}

fn site_of(s: pb::SiteInfo) -> Site {
    Site {
        id: s.id,
        name: s.name,
        status: s.status,
        version: s.version,
        last_seen_ms: s.last_seen_ms,
        share_display_names: s.share_display_names,
        last_status: s.last_status.map(status_of),
    }
}

fn command_of(c: pb::CommandInfo) -> CommandRow {
    CommandRow {
        id: c.id,
        site: c.site,
        kind: c.kind,
        state: c.state,
        result_code: c.result_code,
        requested_by: c.requested_by,
        created_ms: c.created_ms,
        dry_run: c.dry_run,
        preview: c.preview.into_iter().map(change_of).collect(),
        approved_by: c.approved_by,
        args: c
            .args
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect(),
    }
}

fn change_of(c: pb::ConfigChange) -> Change {
    Change {
        target: c.target,
        before: c.before,
        after: c.after,
    }
}

// ---- Session ----

fn connect(dir: &Path, id: Identity) -> Result<Me> {
    let client = Arc::new(block(Client::connect_with(&id, authenticator()))?);
    let u = block(client.whoami())?;
    let me = me_of(u, &id);
    *SESSION.lock() = Some(Session {
        dir: dir.to_path_buf(),
        id,
        client,
    });
    Ok(me)
}

/// Sign in with the identity stored in `dir` (None: this device is not enrolled yet).
pub fn open(dir: String) -> Result<Option<Me>> {
    let d = PathBuf::from(&dir);
    if !Identity::path(&d).exists() {
        return Ok(None);
    }
    let id = Identity::load(&d)?;
    connect(&d, id).map(Some)
}

/// Enroll this device with a one-time code from an owner/admin: new 24 words (returned once),
/// a device key certified by them, the encrypted key backup unless `key_backup` is false.
pub fn enroll(dir: String, code: String, key_backup: bool) -> Result<Enrolled> {
    let d = PathBuf::from(&dir);
    if Identity::path(&d).exists() {
        bail!("this device is already enrolled; sign out (forget this device) first");
    }
    let e = block(itcmdr_admin::enroll(
        code.trim(),
        &platform(),
        !key_backup,
        None,
    ))?;
    std::fs::create_dir_all(&d)?;
    e.identity.save(&d)?;
    let words = e.words.to_string();
    let me = match connect(&d, e.identity) {
        Ok(m) => m,
        // A new SRE/admin device waits for approval: still enrolled.
        Err(err) if err.to_string().contains("waiting for approval") => Me {
            id: e.user.id,
            email: e.user.email,
            name: e.user.name,
            role: e.user.role,
            server: String::new(),
            passkeys: 0,
            locked: false,
            authenticator: authenticator().is_some(),
            key_backup,
        },
        Err(err) => return Err(err),
    };
    Ok(Enrolled { words, me })
}

/// Forget this device locally (the server keeps it until an admin revokes it).
pub fn forget(dir: String) -> Result<()> {
    *SESSION.lock() = None;
    let p = Identity::path(Path::new(&dir));
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

pub fn me() -> Result<Me> {
    let (c, id) = {
        let g = SESSION.lock();
        let s = g.as_ref().ok_or_else(|| anyhow!("not signed in"))?;
        (s.client.clone(), s.id.clone())
    };
    Ok(me_of(block(c.whoami())?, &id))
}

// ---- Users ----

pub fn users() -> Result<Vec<User>> {
    Ok(block(client()?.list_users())?
        .users
        .into_iter()
        .map(user_of)
        .collect())
}

/// New user (owner/admin); returns their one-time enrollment code.
pub fn create_user(email: String, name: String, role: String) -> Result<String> {
    Ok(block(client()?.create_user(&email, &name, &role))?.enroll_code)
}

pub fn set_role(user: String, role: String) -> Result<User> {
    Ok(user_of(block(client()?.set_role(&user, &role))?))
}

pub fn set_disabled(user: String, disabled: bool) -> Result<User> {
    Ok(user_of(block(client()?.set_disabled(&user, disabled))?))
}

pub fn set_scopes(user: String, sites: Vec<String>) -> Result<User> {
    Ok(user_of(block(client()?.set_scopes(&user, sites))?))
}

pub fn approve_device(user: String, fingerprint: String) -> Result<User> {
    Ok(user_of(block(
        client()?.approve_device(&user, &fingerprint),
    )?))
}

pub fn revoke_device(user: String, fingerprint: String) -> Result<Vec<Device>> {
    Ok(block(client()?.revoke_device(&user, &fingerprint))?
        .devices
        .into_iter()
        .map(device_of)
        .collect())
}

pub fn new_enroll_code(user: String) -> Result<String> {
    Ok(block(client()?.new_code(&user))?.enroll_code)
}

// ---- Servers ----

pub fn sites() -> Result<Vec<Site>> {
    if let Ok(p) = block(client()?.get_org_policy()) {
        DEEP_MAX.store(p.deep_check_max_days, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(block(client()?.list_sites())?
        .sites
        .into_iter()
        .map(site_of)
        .collect())
}

/// New site (admin+); returns the one-time site token for `itcmdr-cli adminsvr enroll`.
pub fn create_site(name: String) -> Result<String> {
    Ok(block(client()?.create_site(&name))?.token)
}

pub fn revoke_site(site: String) -> Result<()> {
    block(client()?.revoke_site(&site))?;
    Ok(())
}

/// Status history of a site, oldest first.
pub fn metrics(site: String, hours: i64) -> Result<Vec<Sample>> {
    let since = itcmdr_keys::now_ms() - hours.max(1) * 3600 * 1000;
    Ok(block(client()?.site_metrics(&site, since, 0))?
        .samples
        .into_iter()
        .map(|m| {
            let s = m.status.unwrap_or_default();
            let (c, w, i) = alerts3(&s.open_alerts);
            Sample {
                ts_ms: m.ts_ms,
                agents_online: s.agents_online,
                agents_late: s.agents_late,
                agents_non_responsive: s.agents_non_responsive,
                agents_other: s.agents_other,
                alerts_critical: c,
                alerts_cautious: w,
                alerts_informational: i,
                backup_age_secs: s.backup_age_secs,
                uptime_secs: s.uptime_secs,
            }
        })
        .collect())
}

pub fn commands(site: String) -> Result<Vec<CommandRow>> {
    Ok(block(client()?.list_commands(&site))?
        .commands
        .into_iter()
        .map(command_of)
        .collect())
}

pub fn start_backup(site: String) -> Result<CommandRow> {
    Ok(command_of(block(client()?.command(
        &site,
        "backup.start",
        &[],
    ))?))
}

// ---- Alerts ----

/// state: open (open/acked/silenced) | resolved | all; site "" = every site you may see.
pub fn alerts(site: String, state: String) -> Result<Vec<Alert>> {
    Ok(block(client()?.list_alerts(&site, &state, 500))?
        .alerts
        .into_iter()
        .map(|a| {
            let x = a.alert.unwrap_or_default();
            Alert {
                site: a.site,
                id: x.id,
                code: x.code,
                severity: x.severity,
                state: x.state,
                source: x.source,
                plugin: x.plugin,
                machine: x.machine,
                machine_display_name: x.machine_display_name,
                first_seen_ms: x.first_seen_ms,
                last_seen_ms: x.last_seen_ms,
                count: x.count,
                origin: a.origin,
            }
        })
        .collect())
}

pub fn ack_alert(site: String, alert: String) -> Result<CommandRow> {
    Ok(command_of(block(client()?.command(
        &site,
        "alert.ack",
        &[("alert_id", &alert)],
    ))?))
}

pub fn resolve_alert(site: String, alert: String) -> Result<CommandRow> {
    Ok(command_of(block(client()?.command(
        &site,
        "alert.resolve",
        &[("alert_id", &alert)],
    ))?))
}

// ---- Audit ----

pub fn audit(limit: i32) -> Result<Vec<AuditRow>> {
    Ok(block(client()?.list_audit("", limit))?
        .entries
        .into_iter()
        .map(|e| AuditRow {
            seq: e.seq,
            ts_ms: e.ts_ms,
            actor: e.actor,
            action: e.action,
            target: e.target,
        })
        .collect())
}

/// "ok (N entries)" or what is broken.
pub fn verify_audit() -> Result<String> {
    let v = block(client()?.verify_audit())?;
    Ok(if v.ok {
        format!("ok ({} entries)", v.entries)
    } else {
        format!("BROKEN at seq {}: {}", v.first_bad_seq, v.problem)
    })
}

// ---- Passkeys and policy ----

pub fn policy() -> Result<Policy> {
    let p = block(client()?.get_auth_policy())?;
    Ok(Policy {
        stepup_actions: p.stepup_actions,
        passkey_required_roles: p.passkey_required_roles,
        require_user_verification: p.require_user_verification,
    })
}

pub fn set_policy(p: Policy) -> Result<Policy> {
    let r = block(client()?.set_auth_policy(pb::AuthPolicy {
        stepup_actions: p.stepup_actions,
        passkey_required_roles: p.passkey_required_roles,
        require_user_verification: p.require_user_verification,
    }))?;
    Ok(Policy {
        stepup_actions: r.stepup_actions,
        passkey_required_roles: r.passkey_required_roles,
        require_user_verification: r.require_user_verification,
    })
}

pub fn passkeys(user: String) -> Result<Vec<Passkey>> {
    Ok(block(client()?.list_passkeys(&user))?
        .passkeys
        .into_iter()
        .map(|p| Passkey {
            credential_id: p.credential_id,
            label: p.label,
            sign_count: p.sign_count,
            hmac_secret: p.hmac_secret,
            created_ms: p.created_ms,
            last_used_ms: p.last_used_ms,
        })
        .collect())
}

/// Register the plugged-in security key (two touches when you already have one: step-up, then
/// the new key).
pub fn add_passkey(label: String) -> Result<i32> {
    let (c, dir) = {
        let g = SESSION.lock();
        let s = g.as_ref().ok_or_else(|| anyhow!("not signed in"))?;
        (s.client.clone(), s.dir.clone())
    };
    if authenticator().is_none() {
        bail!("no security key driver: install libfido2 (fido2-token, fido2-cred, fido2-assert)");
    }
    let (lp, list) = block(c.add_passkey(&label, true))?;
    let mut g = SESSION.lock();
    if let Some(s) = g.as_mut() {
        s.id.passkeys.push(lp);
        s.id.save(&dir)?;
    }
    Ok(list.passkeys.len() as i32)
}

/// Remove a passkey ("" user = yours; another user's: owner/admin reset).
pub fn remove_passkey(user: String, credential_id: String) -> Result<i32> {
    let n = block(client()?.remove_passkey(&user, &credential_id))?
        .passkeys
        .len() as i32;
    let mut g = SESSION.lock();
    if let Some(s) = g.as_mut() {
        if user.is_empty() {
            s.id.passkeys.retain(|p| p.credential_id != credential_id);
            let d = s.dir.clone();
            s.id.save(&d)?;
        }
    }
    Ok(n)
}

// ---- Push ----

/// This device's push token (provider fcm | apns), from the platform's push service.
pub fn register_push(provider: String, token: String, platform: String) -> Result<()> {
    block(client()?.register_push(&provider, &token, &platform))?;
    Ok(())
}

pub fn unregister_push() -> Result<()> {
    block(client()?.unregister_push(""))?;
    Ok(())
}

pub fn test_push() -> Result<PushResult> {
    let r = block(client()?.test_push())?;
    Ok(PushResult {
        devices: r.devices,
        sent: r.sent,
        failed: r.failed,
        providers: r.providers,
    })
}

// ---- v0.20.0: configs, schedules, plugins, approvals, freeze ----

pub struct Setting {
    pub key: String,
    /// int | bool | time | path | text
    pub kind: String,
    /// Empty when the site hides it (paths and text never leave the site)
    pub value: String,
    pub hidden: bool,
    /// May be set from here (setting.set)
    pub remote: bool,
    pub source: String,
    pub scope: String,
    pub min: i64,
    pub max: i64,
}

pub struct Schedule {
    /// Opaque id (the site maps it back)
    pub id: String,
    /// Only with the site's display-name opt-in
    pub name: String,
    pub enabled: bool,
    pub when: String,
    pub timezone: String,
    pub action: String,
    pub builtin: String,
    pub all_machines: bool,
    pub machines: i32,
    pub last_fire_ms: i64,
}

pub struct SiteConfig {
    /// 0: the site has not sent its document yet
    pub received_ms: i64,
    pub settings: Vec<Setting>,
    pub schedules: Vec<Schedule>,
    pub require_two_person: bool,
    pub allowed: Vec<String>,
}

pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub sha256: String,
}

pub struct Quarantined {
    pub sha256: String,
    /// A reason code (no file names or messages leave the site)
    pub reason: String,
    pub ts_ms: i64,
}

pub struct SitePlugins {
    pub received_ms: i64,
    pub installed: Vec<InstalledPlugin>,
    pub quarantine: Vec<Quarantined>,
}

pub struct CatalogEntry {
    pub sha256: String,
    pub name: String,
    pub version: String,
    pub size: i64,
    pub uploaded_by: String,
    pub uploaded_ms: i64,
    pub description: String,
}

pub struct Freeze {
    pub frozen: bool,
    pub by: String,
    pub since_ms: i64,
}

pub fn site_config(site: String) -> Result<SiteConfig> {
    let i = block(client()?.site_config(&site))?;
    let c = i.config.unwrap_or_default();
    Ok(SiteConfig {
        received_ms: i.received_ms,
        settings: c
            .settings
            .into_iter()
            .map(|s| Setting {
                key: s.key,
                kind: s.kind,
                value: s.value,
                hidden: s.hidden,
                remote: s.remote,
                source: s.source,
                scope: s.scope,
                min: s.min,
                max: s.max,
            })
            .collect(),
        schedules: c
            .schedules
            .into_iter()
            .map(|s| Schedule {
                id: s.id,
                name: s.name,
                enabled: s.enabled,
                when: s.when,
                timezone: s.timezone,
                action: s.action,
                builtin: s.builtin,
                all_machines: s.all_machines,
                machines: s.machines,
                last_fire_ms: s.last_fire_ms,
            })
            .collect(),
        require_two_person: c.require_two_person,
        allowed: c.allowed,
    })
}

pub fn site_plugins(site: String) -> Result<SitePlugins> {
    let i = block(client()?.site_plugins(&site))?;
    let p = i.plugins.unwrap_or_default();
    Ok(SitePlugins {
        received_ms: i.received_ms,
        installed: p
            .installed
            .into_iter()
            .map(|p| InstalledPlugin {
                name: p.name,
                version: p.version,
                enabled: p.enabled,
                sha256: p.sha256,
            })
            .collect(),
        quarantine: p
            .quarantine
            .into_iter()
            .map(|q| Quarantined {
                sha256: q.sha256,
                reason: q.reason,
                ts_ms: q.ts_ms,
            })
            .collect(),
    })
}

/// Any allowlisted kind (setting.set, schedule.run|pause|resume, plugin.install|enable|disable,
/// ...). `args` are "key=value". `dry_run`: the site previews and changes nothing. Plugin kinds
/// come back `awaiting-approval` until a second owner/admin approves.
pub fn send_command(
    site: String,
    kind: String,
    args: Vec<String>,
    dry_run: bool,
) -> Result<CommandRow> {
    let pairs: Vec<(String, String)> = args
        .iter()
        .map(|a| {
            a.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| anyhow!("argument {a:?} is not key=value"))
        })
        .collect::<Result<_>>()?;
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    Ok(command_of(block(
        client()?.command_ex(&site, &kind, &refs, dry_run),
    )?))
}

/// The command's current row (state, result, preview).
pub fn get_command(id: String) -> Result<CommandRow> {
    let d = block(client()?.get_command(&id))?;
    d.info
        .map(command_of)
        .ok_or_else(|| anyhow!("no such command"))
}

pub fn approve_command(id: String) -> Result<CommandRow> {
    Ok(command_of(block(client()?.approve_command(&id))?))
}

pub fn reject_command(id: String) -> Result<CommandRow> {
    Ok(command_of(block(client()?.reject_command(&id))?))
}

/// Commands waiting for a second person, across the sites this user can see.
pub fn pending_approvals() -> Result<Vec<CommandRow>> {
    let c = client()?;
    let mut out = Vec::new();
    for s in block(c.list_sites())?.sites {
        for r in block(c.list_commands(&s.id))?.commands {
            if r.state == "awaiting-approval" {
                out.push(command_of(r));
            }
        }
    }
    out.sort_by_key(|r| -r.created_ms);
    Ok(out)
}

pub fn catalog() -> Result<Vec<CatalogEntry>> {
    Ok(block(client()?.catalog())?
        .packages
        .into_iter()
        .map(|e| CatalogEntry {
            sha256: e.sha256,
            name: e.name,
            version: e.version,
            size: e.size,
            uploaded_by: e.uploaded_by,
            uploaded_ms: e.uploaded_ms,
            description: e.description,
        })
        .collect())
}

/// Owners/admins: add a signed plugin package (.tar.gz) file to the catalog. adminsvr cannot
/// sign packages; each site checks the signature and the files itself on install.
pub fn upload_plugin(path: String) -> Result<CatalogEntry> {
    let bytes = std::fs::read(&path)?;
    let e = block(client()?.upload_plugin(bytes))?;
    Ok(CatalogEntry {
        sha256: e.sha256,
        name: e.name,
        version: e.version,
        size: e.size,
        uploaded_by: e.uploaded_by,
        uploaded_ms: e.uploaded_ms,
        description: e.description,
    })
}

pub fn freeze() -> Result<Freeze> {
    let f = block(client()?.freeze())?;
    Ok(Freeze {
        frozen: f.frozen,
        by: f.by,
        since_ms: f.since_ms,
    })
}

/// Owners only: stop every command and approval (global freeze), or lift it.
pub fn set_freeze(frozen: bool) -> Result<Freeze> {
    let f = block(client()?.set_freeze(frozen))?;
    Ok(Freeze {
        frozen: f.frozen,
        by: f.by,
        since_ms: f.since_ms,
    })
}

// ---- v0.21.0: org second-approver setting and release approvals ----

pub struct OrgPolicy {
    pub require_second_approver: bool,
    pub updated_by: String,
    pub updated_ms: i64,
    /// v1.0.0: warn when a site's last deep backup check is older than this (days)
    pub deep_check_max_days: i32,
}

fn org_of(p: pb::OrgPolicy) -> OrgPolicy {
    OrgPolicy {
        deep_check_max_days: p.deep_check_max_days,
        require_second_approver: p.require_second_approver,
        updated_by: p.updated_by,
        updated_ms: p.updated_ms,
    }
}

pub fn org_policy() -> Result<OrgPolicy> {
    Ok(org_of(block(client()?.get_org_policy())?))
}

/// Owners only; audited by adminsvr.
pub fn set_org_policy(require_second_approver: bool) -> Result<OrgPolicy> {
    Ok(org_of(block(
        client()?.set_org_policy(require_second_approver),
    )?))
}

/// Owners only; audited (v1.0.0): days before a site's deep backup check is flagged.
pub fn set_deep_check_days(days: i32) -> Result<OrgPolicy> {
    Ok(org_of(block(client()?.set_deep_check_days(days))?))
}

pub struct Release {
    pub sha256: String,
    pub name: String,
    pub version: String,
    pub approved_by: Vec<String>,
    pub needed: i32,
    pub approved: bool,
}

fn release_of(r: pb::ReleaseState) -> Release {
    Release {
        sha256: r.sha256,
        name: r.name,
        version: r.version,
        approved_by: r.approved_by,
        needed: r.needed,
        approved: r.approved,
    }
}

pub fn releases() -> Result<Vec<Release>> {
    Ok(block(client()?.list_releases())?
        .releases
        .into_iter()
        .map(release_of)
        .collect())
}

/// Sign a one-time approval of exactly this release.
pub fn approve_release(sha256: String) -> Result<Release> {
    Ok(release_of(block(client()?.approve_release(&sha256))?))
}

/// Roll an approved release out to a site; `push` ignores the site's update window.
pub fn rollout_release(site: String, sha256: String, push: bool) -> Result<CommandRow> {
    Ok(command_of(block(
        client()?.rollout_release(&site, &sha256, push),
    )?))
}
