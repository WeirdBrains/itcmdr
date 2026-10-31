//! First-run setup and install checks (v0.14.0), used by `itcmdr-cli setup` and the installer.
//!
//! * [`apply`] writes site values into server.json. With `keep_existing` it only fills values that
//!   are missing or empty, so an upgrade never changes what an admin set.
//! * [`check`] reports whether the config, data dir, log share and backup folder exist and are
//!   writable, whether the server is initialized, and whether an app key can sign.
//! * [`first_run`] creates the master key (24 words) and registers the first admin app key,
//!   certified with the words while they are only in memory, so signed mode works from day one.
//!   The caller shows the words; they are never written anywhere.
use crate::auth::{self, NewKey, Role};
use crate::config::ServerConfig;
use crate::db::DbPool;
use crate::keys;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// Site values for server.json. `None` = leave alone.
#[derive(Debug, Default, Clone)]
pub struct Values {
    pub logs_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub bind_addr: Option<String>,
}

/// Server.json written when none exists (same as `installer/server.windows.json`, without site paths).
pub const TEMPLATE: &str = r#"{
  "bind_addr": "0.0.0.0:50051",
  "logs_dir": "",
  "admin_token_hash": "",
  "allow_insecure_no_auth": false,
  "retention_days": 400,
  "ingest_interval_secs": 30,
  "request_timeout_secs": 10800,
  "backup": { "dir": null, "nightly": true, "hour": 2, "keep_daily": 7, "keep_weekly": 4, "keep_blocks": 13 }
}"#;

fn is_unset(v: Option<&serde_json::Value>) -> bool {
    match v {
        None | Some(serde_json::Value::Null) => true,
        Some(serde_json::Value::String(s)) => s.trim().is_empty(),
        _ => false,
    }
}

/// Merge `v` into the server.json at `path` (created from [`TEMPLATE`] when missing). Returns the
/// keys that changed. With `keep_existing`, keys that already have a value are not touched.
pub fn apply(path: &Path, v: &Values, keep_existing: bool) -> Result<Vec<String>> {
    let text = if path.exists() {
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?
    } else {
        TEMPLATE.to_string()
    };
    let text = text.trim_start_matches('\u{feff}');
    let mut j: serde_json::Value =
        serde_json::from_str(text).with_context(|| format!("parsing {}", path.display()))?;
    if !j.is_object() {
        bail!("{} is not a JSON object", path.display());
    }
    let mut changed = Vec::new();
    let path_s = |p: &PathBuf| serde_json::Value::String(p.display().to_string());
    let mut set = |j: &mut serde_json::Value, dotted: &str, val: serde_json::Value| {
        let (obj, key) = match dotted.split_once('.') {
            Some((a, b)) => {
                if !j[a].is_object() {
                    j[a] = serde_json::json!({});
                }
                (&mut j[a], b)
            }
            None => (&mut *j, dotted),
        };
        if keep_existing && !is_unset(obj.get(key)) {
            return;
        }
        if obj.get(key) != Some(&val) {
            obj[key] = val;
            changed.push(dotted.to_string());
        }
    };
    if let Some(p) = &v.logs_dir {
        set(&mut j, "logs_dir", path_s(p));
    }
    if let Some(p) = &v.data_dir {
        set(&mut j, "data_dir", path_s(p));
    }
    if let Some(p) = &v.backup_dir {
        set(&mut j, "backup.dir", path_s(p));
    }
    if let Some(b) = &v.bind_addr {
        b.parse::<std::net::SocketAddr>()
            .with_context(|| format!("bind address {b:?} is not host:port"))?;
        set(&mut j, "bind_addr", serde_json::Value::String(b.clone()));
    }
    if !path.exists() || !changed.is_empty() {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&j)?)?;
        std::fs::rename(&tmp, path)?;
    }
    Ok(changed)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub level: Level,
    pub detail: String,
}

fn c(name: &str, level: Level, detail: impl Into<String>) -> Check {
    Check {
        name: name.into(),
        level,
        detail: detail.into(),
    }
}

/// Exists (or can be created with `create`) and a file can be written and removed in it.
pub fn writable_dir(p: &Path, create: bool) -> std::result::Result<String, String> {
    if p.as_os_str().is_empty() {
        return Err("not set".into());
    }
    if !p.exists() {
        if !create {
            return Err(format!("{} does not exist", p.display()));
        }
        std::fs::create_dir_all(p).map_err(|e| format!("cannot create {}: {e}", p.display()))?;
    }
    if !p.is_dir() {
        return Err(format!("{} is not a folder", p.display()));
    }
    let probe = p.join(format!(".itcmdr-write-test-{}", std::process::id()));
    std::fs::write(&probe, b"x").map_err(|e| format!("cannot write in {}: {e}", p.display()))?;
    let _ = std::fs::remove_file(&probe);
    Ok(format!("{} (writable)", p.display()))
}

/// Install checks that need no database (safe before the server ever ran).
pub fn check_paths(cfg: &ServerConfig) -> Vec<Check> {
    let mut out = Vec::new();
    match &cfg.source_path {
        Some(p) => out.push(c("config", Level::Ok, p.display().to_string())),
        None => out.push(c(
            "config",
            Level::Fail,
            "no server.json found (run `itcmdr-cli setup apply`)",
        )),
    }
    let dir = |name: &str, p: &Path, create: bool, fail: Level, out: &mut Vec<Check>| {
        out.push(match writable_dir(p, create) {
            Ok(d) => c(name, Level::Ok, d),
            Err(e) => c(name, fail, e),
        })
    };
    dir("data_dir", &cfg.layout().root, true, Level::Fail, &mut out);
    dir("logs_dir", &cfg.logs_dir, false, Level::Fail, &mut out);
    match &cfg.backup.dir {
        Some(b) => dir("backup.dir", b, false, Level::Fail, &mut out),
        None => out.push(c(
            "backup.dir",
            Level::Warn,
            format!(
                "not set; backups go to {} on this disk. Set a share or second disk with `itcmdr-cli setup apply --backup-dir`",
                cfg.layout().backups().display()
            ),
        )),
    }
    if cfg.bind_addr.parse::<std::net::SocketAddr>().is_err() {
        out.push(c(
            "bind_addr",
            Level::Fail,
            format!("{:?} is not host:port", cfg.bind_addr),
        ));
    } else {
        out.push(c("bind_addr", Level::Ok, cfg.bind_addr.clone()));
    }
    out
}

/// [`check_paths`] plus initialization and signing state from the database.
pub async fn check(pool: &DbPool, cfg: &ServerConfig) -> Result<Vec<Check>> {
    let mut out = check_paths(cfg);
    let mut cfg = cfg.clone();
    auth::settle_auth_mode(pool, &mut cfg).await?;
    let init = auth::master_public(pool).await?.is_some();
    out.push(if init {
        c("initialized", Level::Ok, "master key present")
    } else {
        c(
            "initialized",
            Level::Warn,
            "not yet: run `itcmdr-cli setup first-run` in person (shows the 24 words once)",
        )
    });
    let signers: Vec<_> = auth::list_keys(pool)
        .await?
        .into_iter()
        .filter(|k| {
            k.revoked_at.is_none() && k.name != "root" && (k.role == "owner" || k.role == "admin")
        })
        .collect();
    out.push(if !signers.is_empty() {
        c(
            "app_keys",
            Level::Ok,
            format!(
                "{} admin/owner key(s): {}",
                signers.len(),
                signers
                    .iter()
                    .map(|k| k.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    } else {
        c(
            "app_keys",
            if init { Level::Warn } else { Level::Ok },
            if init {
                "no admin/owner app key registered: certify the app's key with the words (`keys certify` + `keys register`)"
            } else {
                "no admin/owner app key yet; `setup first-run` registers the first one"
            },
        )
    });
    let mode = cfg.auth_mode.as_str();
    out.push(match (mode, cfg.token_hash().is_some()) {
        ("signed", _) => c("auth_mode", Level::Ok, "signed"),
        (m, true) => c(
            "auth_mode",
            Level::Warn,
            format!("{m}: the shared token still works; switch with `itcmdr-cli auth set-mode signed` (docs/INSTALL.md)"),
        ),
        (m, false) => c("auth_mode", Level::Ok, format!("{m} (no shared token configured)")),
    });
    Ok(out)
}

pub fn worst(checks: &[Check]) -> Level {
    if checks.iter().any(|c| c.level == Level::Fail) {
        Level::Fail
    } else if checks.iter().any(|c| c.level == Level::Warn) {
        Level::Warn
    } else {
        Level::Ok
    }
}

/// The first admin key for [`first_run`].
pub struct FirstAdmin {
    pub name: String,
    pub public_key: ed25519_dalek::VerifyingKey,
}

pub struct FirstRun {
    /// Shown once by the caller, never stored.
    pub words: Zeroizing<String>,
    pub master_fingerprint: String,
    pub root_fingerprint: String,
    /// Fingerprint of the registered admin key.
    pub admin_fingerprint: Option<String>,
}

/// Initialize the server and register the first admin app key (role owner, kind user).
/// Refuses when the server is already initialized (use the words and `keys certify` then).
pub async fn first_run(
    pool: &DbPool,
    cfg: &ServerConfig,
    admin: Option<FirstAdmin>,
) -> Result<FirstRun> {
    if let Some(a) = &admin {
        if a.name.trim().is_empty() {
            bail!("the admin name is empty");
        }
    }
    let out = auth::init(pool, &cfg.layout()).await?;
    let mut admin_fp = None;
    if let Some(a) = admin {
        let master = keys::Master::from_words(&out.words)?;
        let cert = keys::certify(
            &master.signing,
            "user",
            a.name.trim(),
            "owner",
            &a.public_key,
        );
        let pk_hex = hex::encode(a.public_key.as_bytes());
        auth::register(
            pool,
            NewKey {
                kind: "user",
                name: a.name.trim(),
                role: Role::Owner,
                email: None,
                phone: None,
                public_key_hex: &pk_hex,
                certificate: &cert,
            },
        )
        .await
        .context("registering the first admin key")?;
        admin_fp = Some(cert.fingerprint.clone());
    }
    Ok(FirstRun {
        words: out.words,
        master_fingerprint: out.master_fingerprint,
        root_fingerprint: out.root_fingerprint,
        admin_fingerprint: admin_fp,
    })
}

/// 3 distinct word positions (1-based) to ask back after showing the words.
pub fn confirm_positions() -> [usize; 3] {
    use rand::seq::index::sample;
    let mut v: Vec<usize> = sample(&mut rand::rngs::OsRng, 24, 3)
        .into_iter()
        .map(|i| i + 1)
        .collect();
    v.sort_unstable();
    [v[0], v[1], v[2]]
}

/// Does `answer` hold the words at `positions` (case and spacing ignored)?
pub fn confirm_ok(words: &str, positions: &[usize], answer: &str) -> bool {
    let w: Vec<&str> = words.split_whitespace().collect();
    let a: Vec<String> = answer
        .split_whitespace()
        .map(|s| s.to_lowercase())
        .collect();
    a.len() == positions.len()
        && positions
            .iter()
            .zip(&a)
            .all(|(p, x)| w.get(p - 1).is_some_and(|y| y.eq_ignore_ascii_case(x)))
}

/// The app's key file on Windows (`%APPDATA%\itcmdr\app-key.dpapi`), as in `app/lib/key_store.dart`.
pub fn app_key_file() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("itcmdr").join("app-key.dpapi"))
}

/// Windows: the public key of the app key in this user's DPAPI store, creating the key when there
/// is none (the app then finds it on first start). Elsewhere this is not available.
pub fn app_store_public_key() -> Result<(ed25519_dalek::VerifyingKey, bool)> {
    #[cfg(windows)]
    {
        let f = app_key_file().context("APPDATA is not set")?;
        if f.exists() {
            let hexs = Zeroizing::new(crate::storage::unprotect_for_app(&std::fs::read(&f)?)?);
            let seed: Zeroizing<Vec<u8>> = Zeroizing::new(
                hex::decode(String::from_utf8_lossy(&hexs).trim()).context("app key is not hex")?,
            );
            let seed: [u8; 32] = seed
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("app key has the wrong length"))?;
            return Ok((
                ed25519_dalek::SigningKey::from_bytes(&seed).verifying_key(),
                false,
            ));
        }
        let k = keys::new_keypair();
        let hexs = Zeroizing::new(hex::encode(k.as_bytes()));
        std::fs::create_dir_all(f.parent().unwrap())?;
        let tmp = f.with_extension("dpapi.tmp");
        std::fs::write(&tmp, crate::storage::protect_for_app(hexs.as_bytes())?)?;
        std::fs::rename(&tmp, &f)?;
        Ok((k.verifying_key(), true))
    }
    #[cfg(not(windows))]
    {
        bail!("--app-key-store is Windows only; pass --admin-public-key (the app's Keys screen shows it)")
    }
}
