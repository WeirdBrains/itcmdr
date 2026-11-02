//! v0.21.0: the site server upgrades itself. A server release is an owner-signed
//! `itcmdr-server-release` package (`release.toml`; files that replace those in `<app>\server`).
//! Started from the app/CLI or an approved release from adminsvr, it waits for the update window
//! (unless pushed), then the server:
//! 1. takes a safety copy of the database (`<data>\upgrade\db-before-<v>`),
//! 2. stages `<app>\server-staged\<v>` = the running server folder plus the release files,
//! 3. hands it to `itcmdr-server-watchdog` (`<data>\upgrade\request.json`), which stops the
//!    service, swaps the folders, starts the new version and waits for its health file; when it
//!    is not healthy in time it rolls back and runs `itcmdr-cli upgrade restore-db` (the old
//!    version's CLI) to put the safety copy back before the old server starts.
//!
//! The result is read on the next start: done, or failed with a Critical `server-upgrade` alert.
//! Agent releases may require `min_server`, so agents follow once the server runs the new version.
use crate::db::DbPool;
use crate::storage::Layout;
use crate::upgrades::{self, SERVER_RELEASE};
use anyhow::{bail, Context, Result};
use itcmdr_watchdog as wd;
use serde::Serialize;
use sqlx::Row;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS server_upgrades (
        id TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        state TEXT NOT NULL,
        push INTEGER NOT NULL,
        started_ms INTEGER NOT NULL,
        updated_ms INTEGER NOT NULL,
        reason TEXT NOT NULL,
        actor TEXT NOT NULL
    )
"#;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServerUpgrade {
    pub id: String,
    pub version: String,
    /// waiting | staged | done | failed | aborted
    pub state: String,
    pub push: bool,
    pub started_ms: i64,
    pub updated_ms: i64,
    pub reason: String,
    pub actor: String,
}

fn row(r: &sqlx::sqlite::SqliteRow) -> ServerUpgrade {
    ServerUpgrade {
        id: r.get("id"),
        version: r.get("version"),
        state: r.get("state"),
        push: r.get::<i64, _>("push") != 0,
        started_ms: r.get("started_ms"),
        updated_ms: r.get("updated_ms"),
        reason: r.get("reason"),
        actor: r.get("actor"),
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub async fn current(pool: &DbPool) -> Result<Option<ServerUpgrade>> {
    Ok(sqlx::query(
        "SELECT * FROM server_upgrades WHERE state IN ('waiting','staged') ORDER BY started_ms DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?
    .map(|r| row(&r)))
}

pub async fn history(pool: &DbPool, limit: i64) -> Result<Vec<ServerUpgrade>> {
    Ok(
        sqlx::query("SELECT * FROM server_upgrades ORDER BY started_ms DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await?
            .iter()
            .map(row)
            .collect(),
    )
}

async fn set(pool: &DbPool, id: &str, state: &str, reason: &str) -> Result<()> {
    sqlx::query("UPDATE server_upgrades SET state = ?, reason = ?, updated_ms = ? WHERE id = ?")
        .bind(state)
        .bind(reason.chars().take(1000).collect::<String>())
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Queue a server upgrade to an installed, owner-signed server release.
pub async fn start(
    pool: &DbPool,
    layout: &Layout,
    version: &str,
    actor: &str,
    push: bool,
) -> Result<ServerUpgrade> {
    let (_, rel, _, _) = upgrades::release_of(pool, layout, SERVER_RELEASE, version).await?;
    if itcmdr_agentlink::version_cmp(version, VERSION) != std::cmp::Ordering::Greater {
        bail!("the server already runs {VERSION} (not older than {version})");
    }
    if !rel.min_from.is_empty()
        && itcmdr_agentlink::version_cmp(VERSION, &rel.min_from) == std::cmp::Ordering::Less
    {
        bail!(
            "{version} upgrades from {} or later; this server runs {VERSION}",
            rel.min_from
        );
    }
    if let Some(c) = current(pool).await? {
        bail!(
            "a server upgrade to {} is {}: abort it first",
            c.version,
            c.state
        );
    }
    let id = uuid::Uuid::new_v4().to_string();
    let t = now();
    sqlx::query("INSERT INTO server_upgrades(id, version, state, push, started_ms, updated_ms, reason, actor) VALUES (?,?,'waiting',?,?,?,'',?)")
        .bind(&id)
        .bind(version)
        .bind(push as i64)
        .bind(t)
        .bind(t)
        .bind(actor)
        .execute(pool)
        .await?;
    crate::db::insert_audit(
        pool,
        actor,
        "server-upgrade-start",
        &serde_json::json!({"version": version, "id": id, "push": push}).to_string(),
    )
    .await?;
    current(pool).await?.context("server upgrade vanished")
}

pub async fn abort(pool: &DbPool, actor: &str) -> Result<ServerUpgrade> {
    let c = current(pool).await?.context("no server upgrade waiting")?;
    if c.state == "staged" {
        bail!("the watchdog is already installing {}", c.version);
    }
    set(pool, &c.id, "aborted", &format!("aborted by {actor}")).await?;
    crate::db::insert_audit(pool, actor, "server-upgrade-abort", &c.version).await?;
    Ok(ServerUpgrade {
        state: "aborted".into(),
        ..c
    })
}

/// The running server folder (`<app>\server`): this exe's folder.
pub fn server_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ITCMDR_SERVER_PROGRAM") {
        return Some(PathBuf::from(p));
    }
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let t = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &t)?;
        } else {
            std::fs::copy(e.path(), &t)?;
        }
    }
    Ok(())
}

/// Stage and hand over (no window check). Returns the request written.
pub async fn stage(
    pool: &DbPool,
    cfg: &crate::config::ServerConfig,
    u: &ServerUpgrade,
    program: &Path,
) -> Result<wd::Request> {
    let layout = cfg.layout();
    let data = layout.root.clone();
    if wd::request_path(&data).exists() || wd::busy_path(&data).exists() {
        bail!("the server watchdog is busy with an upgrade");
    }
    let (p, _rel, _, _) = upgrades::release_of(pool, &layout, SERVER_RELEASE, &u.version).await?;
    let dirs = wd::Dirs::beside(program);
    let st = dirs.staged.join(&u.version);
    let _ = std::fs::remove_dir_all(&st);
    copy_dir(program, &st).context("copying the running server")?;
    for f in std::fs::read_dir(&p.dir)? {
        let f = f?;
        if f.file_type()?.is_file() {
            std::fs::copy(f.path(), st.join(f.file_name()))?;
        }
    }
    // Safety copy of the database for a rollback.
    let up = wd::upgrade_dir(&data);
    std::fs::create_dir_all(&up)?;
    let safety = up.join(format!("db-before-{}", u.version));
    let _ = std::fs::remove_file(&safety);
    sqlx::query("VACUUM INTO ?")
        .bind(safety.to_string_lossy().to_string())
        .execute(pool)
        .await
        .context("database safety copy")?;
    let cli = program.join(if cfg!(windows) {
        "itcmdr-cli.exe"
    } else {
        "itcmdr-cli"
    });
    let req = wd::Request {
        version: u.version.clone(),
        from: VERSION.into(),
        health_secs: crate::settings::get_i64(pool, None, "agent.upgradeHealthSecs").await as u64,
        ts_ms: now(),
        rollback_cmd: vec![
            cli.to_string_lossy().to_string(),
            "upgrade".into(),
            "restore-db".into(),
            "--from".into(),
            safety.to_string_lossy().to_string(),
            "--to".into(),
            cfg.database_path().to_string_lossy().to_string(),
        ],
    };
    wd::write_json(&wd::request_path(&data), &req)?;
    Ok(req)
}

/// Put a safety copy back (run by the watchdog after a rollback, with the server stopped).
pub fn restore_db(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        bail!("no safety copy at {}", from.display());
    }
    for ext in ["-wal", "-shm"] {
        let mut p = to.as_os_str().to_owned();
        p.push(ext);
        let _ = std::fs::remove_file(PathBuf::from(p));
    }
    std::fs::copy(from, to).context("restoring the safety copy")?;
    println!("database restored from {}", from.display());
    Ok(())
}

/// One pass: read a watchdog result, finish an upgrade the server now runs, stage a waiting one
/// when the gate opens.
pub async fn tick(
    pool: &DbPool,
    cfg: &crate::config::ServerConfig,
) -> Result<Option<ServerUpgrade>> {
    let data = cfg.layout().root;
    if let Some(out) = wd::read_json::<wd::Outcome>(&wd::result_path(&data)) {
        let _ = std::fs::rename(
            wd::result_path(&data),
            wd::upgrade_dir(&data).join("result.reported.json"),
        );
        // After a rollback the database is the safety copy, taken before the row was marked
        // staged, so the row reads `waiting` again: match it too, or the server would stage the
        // same broken release again and loop (found on the VM in v0.22.0).
        let row: Option<ServerUpgrade> = sqlx::query("SELECT * FROM server_upgrades WHERE version = ? AND state IN ('staged', 'waiting') ORDER BY started_ms DESC LIMIT 1")
            .bind(&out.version)
            .fetch_optional(pool)
            .await?
            .map(|r| row(&r));
        if let Some(r) = row {
            if out.ok && VERSION == out.version {
                set(pool, &r.id, "done", "").await?;
                let _ = crate::alerts::resolve_key(pool, "server-upgrade").await;
            } else {
                set(pool, &r.id, "failed", &out.error).await?;
                crate::alerts::raise(
                    pool,
                    &crate::alerts::Raise {
                        key: "server-upgrade".into(),
                        level: "critical".into(),
                        source: "server".into(),
                        machine: String::new(),
                        title: format!(
                            "Server upgrade to {} failed and was rolled back",
                            out.version
                        ),
                        detail: out.error.clone(),
                        run_id: None,
                    },
                )
                .await?;
            }
        }
    }
    let Some(c) = current(pool).await? else {
        return Ok(None);
    };
    if c.state == "staged" {
        if VERSION == c.version {
            set(pool, &c.id, "done", "").await?;
        }
        return current(pool).await;
    }
    if let Some(why) = upgrades::gate(pool, c.push, chrono::Utc::now()).await {
        if c.reason != why {
            sqlx::query("UPDATE server_upgrades SET reason = ? WHERE id = ?")
                .bind(&why)
                .bind(&c.id)
                .execute(pool)
                .await?;
        }
        return current(pool).await;
    }
    let Some(program) = server_dir() else {
        bail!("cannot find the server folder");
    };
    match stage(pool, cfg, &c, &program).await {
        Ok(_) => {
            set(pool, &c.id, "staged", "handed to itcmdr-server-watchdog").await?;
            tracing::info!(version=%c.version, "server release staged; the watchdog swaps it in");
        }
        Err(e) => {
            set(pool, &c.id, "failed", &format!("{e:#}")).await?;
            tracing::error!(error=%format!("{e:#}"), "server upgrade staging failed");
        }
    }
    current(pool).await
}
