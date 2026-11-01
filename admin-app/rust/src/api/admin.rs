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

fn status_of(s: pb::SiteStatus) -> SiteStatus {
    let (c, w, i) = alerts3(&s.open_alerts);
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
