//! Runtime settings (decision 15): values set from the app or `itcmdr-cli config set` are stored in
//! the encrypted DB (`server_meta` key `settings.backup`) and override server.json. Unset keys fall
//! back to server.json, then the built-in defaults.
use crate::config::{BackupConfig, ServerConfig};
use crate::db::DbPool;
use anyhow::{anyhow, bail, ensure, Result};
use std::path::Path;

const KEY: &str = "settings.backup";

/// Settable keys and what they mean, for `config get` / errors.
pub const BACKUP_KEYS: [&str; 6] = [
    "backup.dir",
    "backup.nightly",
    "backup.hour",
    "backup.keep_daily",
    "backup.keep_weekly",
    "backup.keep_blocks",
];

async fn overlay(pool: &DbPool) -> Result<serde_json::Map<String, serde_json::Value>> {
    let v: Option<String> = sqlx::query_scalar("SELECT value FROM server_meta WHERE key = ?")
        .bind(KEY)
        .fetch_optional(pool)
        .await?;
    Ok(match v {
        Some(s) => serde_json::from_str(&s)?,
        None => Default::default(),
    })
}

async fn save_overlay(pool: &DbPool, m: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
    sqlx::query("INSERT INTO server_meta(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(KEY)
        .bind(serde_json::to_string(m)?)
        .execute(pool)
        .await?;
    Ok(())
}

/// server.json `backup` with the stored overrides applied.
pub async fn effective_backup(pool: &DbPool, cfg: &ServerConfig) -> Result<BackupConfig> {
    let mut base = serde_json::to_value(&cfg.backup)?;
    for (k, v) in overlay(pool).await? {
        base[k] = v;
    }
    Ok(serde_json::from_value(base)?)
}

/// Effective backup folder (overrides, then server.json, then `<data_dir>/backups`).
pub async fn backup_dir(pool: &DbPool, cfg: &ServerConfig) -> Result<std::path::PathBuf> {
    Ok(effective_backup(pool, cfg)
        .await?
        .dir
        .unwrap_or_else(|| cfg.layout().backups()))
}

/// The folder must exist and be writable (checked with a temp file).
pub fn check_writable_dir(p: &Path) -> Result<()> {
    ensure!(p.is_absolute(), "{} is not an absolute path", p.display());
    ensure!(
        p.is_dir(),
        "{} does not exist or is not a folder",
        p.display()
    );
    let probe = p.join(format!(".itcmdr-write-test-{}", uuid::Uuid::new_v4()));
    std::fs::write(&probe, b"ok").map_err(|e| anyhow!("{} is not writable: {e}", p.display()))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

fn validate(b: &BackupConfig) -> Result<()> {
    ensure!(b.hour <= 23, "backup.hour must be 0-23");
    ensure!(
        b.keep_daily <= 3650 && b.keep_weekly <= 520 && b.keep_blocks <= 1300,
        "retention value too large"
    );
    ensure!(
        b.keep_daily + b.keep_weekly + b.keep_blocks > 0,
        "retention would delete every nightly backup"
    );
    if let Some(d) = &b.dir {
        check_writable_dir(d)?;
    }
    Ok(())
}

/// Replace all backup settings (what the app sends). Validated before anything is stored.
pub async fn set_backup(pool: &DbPool, b: &BackupConfig) -> Result<()> {
    validate(b)?;
    let v = serde_json::to_value(b)?;
    let m = v.as_object().cloned().unwrap_or_default();
    save_overlay(pool, &m).await
}

/// Set one key from its text form (`itcmdr-cli config set backup.hour 3`).
pub async fn set_key(
    pool: &DbPool,
    cfg: &ServerConfig,
    key: &str,
    value: &str,
) -> Result<BackupConfig> {
    let field = key
        .strip_prefix("backup.")
        .filter(|_| BACKUP_KEYS.contains(&key))
        .ok_or_else(|| {
            anyhow!(
                "unknown setting {key:?}; settable: {}",
                BACKUP_KEYS.join(", ")
            )
        })?;
    let v: serde_json::Value = match field {
        "dir" => serde_json::Value::String(value.to_string()),
        "nightly" => serde_json::Value::Bool(match value {
            "true" | "on" | "yes" | "1" => true,
            "false" | "off" | "no" | "0" => false,
            _ => bail!("backup.nightly must be true or false"),
        }),
        _ => serde_json::Value::from(
            value
                .parse::<u64>()
                .map_err(|_| anyhow!("{key} must be a whole number"))?,
        ),
    };
    let mut m = overlay(pool).await?;
    m.insert(field.to_string(), v);
    let mut merged = serde_json::to_value(&cfg.backup)?;
    for (k, v) in &m {
        merged[k] = v.clone();
    }
    let b: BackupConfig = serde_json::from_value(merged)?;
    validate(&b)?;
    save_overlay(pool, &m).await?;
    Ok(b)
}

/// Drop an override (back to server.json / default).
pub async fn unset_key(pool: &DbPool, key: &str) -> Result<()> {
    let field = key
        .strip_prefix("backup.")
        .filter(|_| BACKUP_KEYS.contains(&key))
        .ok_or_else(|| anyhow!("unknown setting {key:?}"))?;
    let mut m = overlay(pool).await?;
    m.remove(field);
    save_overlay(pool, &m).await
}

/// Which keys are overridden in the DB (the rest come from server.json or defaults).
pub async fn overridden(pool: &DbPool) -> Result<Vec<String>> {
    Ok(overlay(pool)
        .await?
        .keys()
        .map(|k| format!("backup.{k}"))
        .collect())
}
