//! v0.21.0: agent self-upgrade (SERVER-AGENT.md section 8).
//!
//! An agent release is an ordinary service-plugin package named [`AGENT_RELEASE`] (validated and
//! quarantined like any package) that must be signed by an **owner** key and must hold a
//! `release.toml` (`version`, `min_from`). It is never pushed as a plugin. A rollout walks the rings
//! (`agent.ring`: 0 = test VM/PC, 1 = one PC, 2 = everything): every agent of the current ring gets
//! an [`pb::UpgradeOffer`]; when all of them run the new version with fresh heartbeats, the ring
//! soaks for `agent.upgradeSoakMins` and the next ring starts. A failed apply (or watchdog
//! rollback) or a new Critical alert on a ring machine pauses the rollout and raises a Critical
//! alert. `agent.upgrades = false` (per machine or global) holds a machine back.
use crate::db::DbPool;
use crate::packages::{self, InstalledPlugin};
use crate::storage::Layout;
use anyhow::{bail, ensure, Context, Result};
use itcmdr_agentlink::pb;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::cmp::Ordering;

pub const AGENT_RELEASE: &str = "itcmdr-agent-release";
pub const SERVER_RELEASE: &str = "itcmdr-server-release";
pub const RELEASE_TOML: &str = "release.toml";
pub const TOP_RING: i64 = 2;
/// A ring member counts as healthy when seen within this many minutes.
const SEEN_MIN: i64 = 10;

pub const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS agent_rollouts (
        id TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        ring INTEGER NOT NULL,
        state TEXT NOT NULL,
        started_ms INTEGER NOT NULL,
        ring_started_ms INTEGER NOT NULL,
        ring_ok_ms INTEGER NOT NULL,
        reason TEXT NOT NULL,
        actor TEXT NOT NULL,
        updated_ms INTEGER NOT NULL,
        push INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS agent_upgrades (
        machine TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        ok INTEGER NOT NULL,
        error TEXT NOT NULL,
        ts_ms INTEGER NOT NULL
    )
"#;

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// `release.toml` in an agent (or server) release package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseToml {
    pub version: String,
    /// Oldest version this release upgrades from ("" = any).
    #[serde(default)]
    pub min_from: String,
    /// Agent releases: the server must run at least this version before agents get the offer
    /// (the server updates first). "" = no requirement (v0.21.0).
    #[serde(default)]
    pub min_server: String,
}

pub fn parse_release(b: &[u8]) -> Result<ReleaseToml> {
    let r: ReleaseToml =
        toml::from_str(std::str::from_utf8(b).context("release.toml is not valid text")?)
            .context("release.toml")?;
    ensure!(
        !r.version.is_empty()
            && r.version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c)),
        "release.toml: bad version {:?}",
        r.version
    );
    Ok(r)
}

/// A release package: never sent to agents as a plugin.
pub fn is_release(name: &str) -> bool {
    name == AGENT_RELEASE || name == SERVER_RELEASE
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Rollout {
    pub id: String,
    pub version: String,
    pub ring: i64,
    pub state: String, // active | paused | done | aborted
    pub started_ms: i64,
    pub ring_started_ms: i64,
    pub ring_ok_ms: i64,
    pub reason: String,
    pub actor: String,
    pub updated_ms: i64,
    /// Pushed: installs at once, outside the update window and with automatic updates off.
    pub push: bool,
}

fn row(r: &sqlx::sqlite::SqliteRow) -> Rollout {
    Rollout {
        id: r.get("id"),
        version: r.get("version"),
        ring: r.get("ring"),
        state: r.get("state"),
        started_ms: r.get("started_ms"),
        ring_started_ms: r.get("ring_started_ms"),
        ring_ok_ms: r.get("ring_ok_ms"),
        reason: r.get("reason"),
        actor: r.get("actor"),
        updated_ms: r.get("updated_ms"),
        push: r.get::<i64, _>("push") != 0,
    }
}

/// May upgrades start now? Pushed: always. Otherwise automatic updates must be on and the update
/// window open. Returns the reason when not.
pub async fn gate(pool: &DbPool, push: bool, at: chrono::DateTime<chrono::Utc>) -> Option<String> {
    if push {
        return None;
    }
    if !crate::update_window::auto_on(pool).await {
        return Some("automatic updates are off for this site (push to install)".into());
    }
    if !crate::update_window::site_open(pool, at).await {
        let w = crate::settings::get_str(pool, None, "updates.window").await;
        return Some(format!("waiting for the update window ({w})"));
    }
    None
}

fn at_ms(ms: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp_millis(ms).unwrap_or_else(chrono::Utc::now)
}

/// The newest rollout that is not finished (active or paused).
pub async fn current(pool: &DbPool) -> Result<Option<Rollout>> {
    Ok(sqlx::query(
        "SELECT * FROM agent_rollouts WHERE state IN ('active','paused') ORDER BY started_ms DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?
    .map(|r| row(&r)))
}

pub async fn history(pool: &DbPool, limit: i64) -> Result<Vec<Rollout>> {
    Ok(
        sqlx::query("SELECT * FROM agent_rollouts ORDER BY started_ms DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await?
            .iter()
            .map(row)
            .collect(),
    )
}

async fn signer_role(pool: &DbPool, fp: &str) -> Result<Option<(String, String)>> {
    for t in ["users", "clients"] {
        let r: Option<(String, String)> = sqlx::query_as(&format!(
            "SELECT k.public_key, r.name FROM {t} k JOIN roles r ON r.id = k.role_id \
             WHERE k.fingerprint = ? AND k.revoked_at IS NULL"
        ))
        .bind(fp)
        .fetch_optional(pool)
        .await?;
        if r.is_some() {
            return Ok(r);
        }
    }
    Ok(None)
}

/// An installed, enabled agent release of this version, its release.toml, and the owner key
/// that signed it. Refuses a release not signed by an owner.
pub async fn release(
    pool: &DbPool,
    layout: &Layout,
    version: &str,
) -> Result<(InstalledPlugin, ReleaseToml, String, String)> {
    release_of(pool, layout, AGENT_RELEASE, version).await
}

/// [`release`] for either kind of release package (`AGENT_RELEASE` or `SERVER_RELEASE`).
pub async fn release_of(
    pool: &DbPool,
    layout: &Layout,
    name: &str,
    version: &str,
) -> Result<(InstalledPlugin, ReleaseToml, String, String)> {
    let p = packages::list(pool, layout)
        .await?
        .into_iter()
        .find(|p| p.name == name && p.version == version)
        .with_context(|| format!("{name} {version} is not installed"))?;
    let rel = parse_release(
        &std::fs::read(p.dir.join(RELEASE_TOML))
            .with_context(|| format!("{name} {version} has no {RELEASE_TOML}"))?,
    )?;
    ensure!(
        rel.version == p.version,
        "release.toml says {} but the package is {}",
        rel.version,
        p.version
    );
    let ps: packages::PackageSig = serde_json::from_slice(
        &std::fs::read(p.dir.join(packages::SIGNATURE)).context("package.sig")?,
    )
    .context("package.sig")?;
    let fp = ps.signer;
    let Some((pk, role)) = signer_role(pool, &fp).await? else {
        bail!(
            "{name} {version}: signer {} is unknown or revoked",
            p.signer
        );
    };
    ensure!(
        role == "owner",
        "{name} {version} is signed by a {role} key; releases need an owner signature"
    );
    Ok((p, rel, pk, fp))
}

/// Start a rollout at ring 0. Refused while another is active or paused.
pub async fn start(
    pool: &DbPool,
    layout: &Layout,
    version: &str,
    actor: &str,
    push: bool,
) -> Result<Rollout> {
    release(pool, layout, version).await?;
    if let Some(c) = current(pool).await? {
        bail!(
            "rollout of {} is {} at ring {}: resume, finish or abort it first",
            c.version,
            c.state,
            c.ring
        );
    }
    let t = now();
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO agent_rollouts(id, version, ring, state, started_ms, ring_started_ms, ring_ok_ms, reason, actor, updated_ms, push) VALUES (?,?,0,'active',?,?,0,'',?,?,?)")
        .bind(&id)
        .bind(version)
        .bind(t)
        .bind(t)
        .bind(actor)
        .bind(t)
        .bind(push as i64)
        .execute(pool)
        .await?;
    crate::db::insert_audit(
        pool,
        actor,
        "agent-upgrade-start",
        &serde_json::json!({"version": version, "id": id, "push": push}).to_string(),
    )
    .await?;
    tracing::info!(%version, "agent rollout started at ring 0");
    current(pool).await?.context("rollout vanished")
}

async fn set_state(pool: &DbPool, id: &str, state: &str, reason: &str) -> Result<()> {
    sqlx::query("UPDATE agent_rollouts SET state = ?, reason = ?, updated_ms = ? WHERE id = ?")
        .bind(state)
        .bind(reason.chars().take(1000).collect::<String>())
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn pause(pool: &DbPool, actor: &str, reason: &str) -> Result<Rollout> {
    let c = current(pool).await?.context("no rollout in progress")?;
    set_state(pool, &c.id, "paused", &format!("{reason} ({actor})")).await?;
    crate::db::insert_audit(
        pool,
        actor,
        "agent-upgrade-pause",
        &serde_json::json!({"version": c.version, "reason": reason}).to_string(),
    )
    .await?;
    current(pool).await?.context("rollout vanished")
}

/// Resume a paused rollout (the ring restarts its soak). Clears failed records for this version
/// so those machines are offered it again.
pub async fn resume(pool: &DbPool, actor: &str) -> Result<Rollout> {
    let c = current(pool).await?.context("no rollout in progress")?;
    ensure!(
        c.state == "paused",
        "the rollout is {}, not paused",
        c.state
    );
    sqlx::query("DELETE FROM agent_upgrades WHERE version = ? AND ok = 0")
        .bind(&c.version)
        .execute(pool)
        .await?;
    sqlx::query("UPDATE agent_rollouts SET state = 'active', reason = '', ring_started_ms = ?, ring_ok_ms = 0, updated_ms = ? WHERE id = ?")
        .bind(now())
        .bind(now())
        .bind(&c.id)
        .execute(pool)
        .await?;
    crate::alerts::resolve_prefix(pool, "agent-upgrade:").await?;
    crate::db::insert_audit(
        pool,
        actor,
        "agent-upgrade-resume",
        &serde_json::json!({"version": c.version}).to_string(),
    )
    .await?;
    current(pool).await?.context("rollout vanished")
}

pub async fn abort(pool: &DbPool, actor: &str) -> Result<Rollout> {
    let c = current(pool).await?.context("no rollout in progress")?;
    set_state(pool, &c.id, "aborted", &format!("aborted by {actor}")).await?;
    crate::db::insert_audit(
        pool,
        actor,
        "agent-upgrade-abort",
        &serde_json::json!({"version": c.version}).to_string(),
    )
    .await?;
    Ok(Rollout {
        state: "aborted".into(),
        ..c
    })
}

async fn ring_of(pool: &DbPool, machine: &str) -> i64 {
    crate::agents::machine_i64(pool, "agent.ring", machine)
        .await
        .clamp(0, TOP_RING)
}

async fn enabled_for(pool: &DbPool, machine: &str) -> bool {
    crate::settings::resolve_all(pool, None, Some(machine))
        .await
        .ok()
        .and_then(|v| v.into_iter().find(|r| r.def.key == "agent.upgrades"))
        .and_then(|r| r.value.as_bool())
        .unwrap_or(true)
}

/// The server runs at least the release's `min_server`.
pub fn server_ok(rel: &ReleaseToml) -> bool {
    rel.min_server.is_empty()
        || itcmdr_agentlink::version_cmp(env!("CARGO_PKG_VERSION"), &rel.min_server)
            != Ordering::Less
}

/// The offer for this machine, if the active rollout covers it and it runs an older version
/// (and did not already fail this version).
pub async fn offer_for(
    pool: &DbPool,
    layout: &Layout,
    machine: &str,
    running: &str,
) -> Result<Option<pb::UpgradeOffer>> {
    let Some(c) = current(pool).await? else {
        return Ok(None);
    };
    if c.state != "active"
        || itcmdr_agentlink::version_cmp(running, &c.version) != Ordering::Less
        || ring_of(pool, machine).await > c.ring
        || !enabled_for(pool, machine).await
    {
        return Ok(None);
    }
    let failed: Option<i64> = sqlx::query_scalar(
        "SELECT ok FROM agent_upgrades WHERE machine = ? COLLATE NOCASE AND version = ?",
    )
    .bind(machine)
    .bind(&c.version)
    .fetch_optional(pool)
    .await?;
    if failed == Some(0) {
        return Ok(None);
    }
    if gate(pool, c.push, chrono::Utc::now()).await.is_some() {
        return Ok(None);
    }
    let (p, rel, pk, fp) = release(pool, layout, &c.version).await?;
    if !server_ok(&rel) {
        return Ok(None);
    }
    if !rel.min_from.is_empty()
        && itcmdr_agentlink::version_cmp(running, &rel.min_from) == Ordering::Less
    {
        return Ok(None);
    }
    let mbytes = std::fs::read(p.dir.join(packages::MANIFEST))?;
    let m = packages::parse_manifest(&mbytes).map_err(|r| anyhow::anyhow!("{}", r.0))?;
    let sha = itcmdr_agentlink::sha256_hex(&mbytes);
    let mut files = vec![pb::PluginFile {
        name: packages::MANIFEST.into(),
        data: mbytes,
    }];
    files.push(pb::PluginFile {
        name: packages::SIGNATURE.into(),
        data: std::fs::read(p.dir.join(packages::SIGNATURE))?,
    });
    for f in &m.files {
        files.push(pb::PluginFile {
            name: f.name.clone(),
            data: std::fs::read(p.dir.join(&f.name))?,
        });
    }
    let root = crate::auth::load_root_key(layout)?;
    use ed25519_dalek::Signer;
    let sig = root.sign(&itcmdr_agentlink::upgrade_bytes(&c.version, &sha, &fp));
    Ok(Some(pb::UpgradeOffer {
        version: c.version,
        files,
        signer: fp,
        signer_public_key: pk,
        endorsement: hex::encode(sig.to_bytes()),
        health_secs: crate::agents::machine_i64(pool, "agent.upgradeHealthSecs", machine).await
            as i32,
    }))
}

/// Applied(kind = "upgrade") from an agent: ok = staged and handed to the watchdog; not ok = the
/// agent refused it, or the watchdog rolled back (the agent reports that after the restart).
pub async fn applied(pool: &DbPool, machine: &str, ap: &pb::Applied) -> Result<()> {
    sqlx::query("INSERT INTO agent_upgrades(machine, version, ok, error, ts_ms) VALUES (?,?,?,?,?) ON CONFLICT(machine) DO UPDATE SET version = excluded.version, ok = excluded.ok, error = excluded.error, ts_ms = excluded.ts_ms")
        .bind(machine)
        .bind(&ap.version)
        .bind(ap.ok)
        .bind(ap.error.chars().take(2000).collect::<String>())
        .bind(now())
        .execute(pool)
        .await?;
    let key = format!("agent-upgrade:{machine}");
    if ap.ok {
        return Ok(());
    }
    if let Some(c) = current(pool).await? {
        if c.version == ap.version && c.state == "active" {
            set_state(pool, &c.id, "paused", &format!("{machine}: {}", ap.error)).await?;
        }
    }
    crate::alerts::raise(
        pool,
        &crate::alerts::Raise {
            key,
            level: "critical".into(),
            source: "agent".into(),
            machine: machine.into(),
            title: format!(
                "{machine}: agent upgrade to {} failed; the rollout is paused",
                ap.version
            ),
            detail: ap.error.chars().take(2000).collect(),
            run_id: None,
        },
    )
    .await?;
    Ok(())
}

/// Ring members: non-revoked agents in exactly this ring that take upgrades.
async fn members(pool: &DbPool, ring: i64) -> Result<Vec<crate::agents::Agent>> {
    let mut out = vec![];
    for a in crate::agents::list(pool, false).await? {
        if ring_of(pool, &a.machine).await == ring && enabled_for(pool, &a.machine).await {
            out.push(a);
        }
    }
    Ok(out)
}

/// What still holds the current ring back (empty = the ring is healthy on the new version).
pub async fn ring_problems(pool: &DbPool, r: &Rollout, now_ms: i64) -> Result<Vec<String>> {
    let mut p = vec![];
    for a in members(pool, r.ring).await? {
        if itcmdr_agentlink::version_cmp(&a.version, &r.version) == Ordering::Less {
            p.push(format!(
                "{}: runs {}, not {}",
                a.machine, a.version, r.version
            ));
        } else if now_ms - a.last_seen_ms > SEEN_MIN * 60_000 {
            p.push(format!(
                "{}: not seen in the last {SEEN_MIN} min",
                a.machine
            ));
        }
    }
    Ok(p)
}

/// New Critical alerts on this ring's machines since the ring started.
async fn ring_criticals(pool: &DbPool, r: &Rollout) -> Result<Vec<String>> {
    let mut out = vec![];
    for a in members(pool, r.ring).await? {
        let t: Vec<String> = sqlx::query_scalar(
            "SELECT title FROM alerts WHERE level = 'critical' AND state = 'open' AND machine = ? COLLATE NOCASE AND first_seen_ms >= ? AND key NOT LIKE 'agent-upgrade:%'",
        )
        .bind(&a.machine)
        .bind(r.ring_started_ms)
        .fetch_all(pool)
        .await?;
        out.extend(t);
    }
    Ok(out)
}

/// One rollout pass: auto-pause on new Critical alerts, mark the ring healthy, advance after the soak.
pub async fn tick(pool: &DbPool, now_ms: i64) -> Result<Option<Rollout>> {
    let Some(r) = current(pool).await? else {
        return Ok(None);
    };
    if r.state != "active" {
        return Ok(Some(r));
    }
    let crit = ring_criticals(pool, &r).await?;
    if !crit.is_empty() {
        let why = format!("new critical alert in ring {}: {}", r.ring, crit.join("; "));
        set_state(pool, &r.id, "paused", &why).await?;
        crate::alerts::raise(
            pool,
            &crate::alerts::Raise {
                key: "agent-upgrade:rollout".into(),
                level: "critical".into(),
                source: "server".into(),
                machine: String::new(),
                title: format!("Agent rollout of {} paused at ring {}", r.version, r.ring),
                detail: why,
                run_id: None,
            },
        )
        .await?;
        return current(pool).await;
    }
    let problems = ring_problems(pool, &r, now_ms).await?;
    let empty = members(pool, r.ring).await?.is_empty();
    let soak = crate::settings::get_i64(pool, None, "agent.upgradeSoakMins").await * 60_000;
    if !problems.is_empty() {
        if r.ring_ok_ms != 0 {
            sqlx::query("UPDATE agent_rollouts SET ring_ok_ms = 0, updated_ms = ? WHERE id = ?")
                .bind(now_ms)
                .bind(&r.id)
                .execute(pool)
                .await?;
        }
        return current(pool).await;
    }
    let ok_since = if r.ring_ok_ms == 0 {
        now_ms
    } else {
        r.ring_ok_ms
    };
    let advance = empty || now_ms - ok_since >= soak;
    if advance && r.ring < TOP_RING && gate(pool, r.push, at_ms(now_ms)).await.is_some() {
        // Soaked, but the next ring only starts inside the update window (unless pushed).
        if r.ring_ok_ms == 0 {
            sqlx::query("UPDATE agent_rollouts SET ring_ok_ms = ?, updated_ms = ? WHERE id = ?")
                .bind(ok_since)
                .bind(now_ms)
                .bind(&r.id)
                .execute(pool)
                .await?;
        }
        return current(pool).await;
    }
    if advance {
        if r.ring >= TOP_RING {
            sqlx::query("UPDATE agent_rollouts SET state = 'done', ring_ok_ms = ?, updated_ms = ? WHERE id = ?")
                .bind(ok_since)
                .bind(now_ms)
                .bind(&r.id)
                .execute(pool)
                .await?;
            tracing::info!(version=%r.version, "agent rollout done");
        } else {
            sqlx::query("UPDATE agent_rollouts SET ring = ring + 1, ring_started_ms = ?, ring_ok_ms = 0, updated_ms = ? WHERE id = ?")
                .bind(now_ms)
                .bind(now_ms)
                .bind(&r.id)
                .execute(pool)
                .await?;
            tracing::info!(version=%r.version, ring = r.ring + 1, "agent rollout: next ring");
        }
    } else if r.ring_ok_ms == 0 {
        sqlx::query("UPDATE agent_rollouts SET ring_ok_ms = ?, updated_ms = ? WHERE id = ?")
            .bind(now_ms)
            .bind(now_ms)
            .bind(&r.id)
            .execute(pool)
            .await?;
    }
    Ok(match current(pool).await? {
        Some(c) => Some(c),
        None => history(pool, 1).await?.into_iter().next(),
    })
}

/// An installed agent release and whether it can be rolled out.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub min_from: String,
    pub enabled: bool,
    pub signer: String,
    pub owner_signed: bool,
    pub problem: String,
}

pub async fn releases(pool: &DbPool, layout: &Layout) -> Result<Vec<ReleaseInfo>> {
    let mut out = vec![];
    for p in packages::list(pool, layout).await? {
        if p.name != AGENT_RELEASE {
            continue;
        }
        let (min_from, owner_signed, problem) = match release(pool, layout, &p.version).await {
            Ok((_, r, _, _)) => (r.min_from, true, String::new()),
            Err(e) => {
                let e = format!("{e:#}");
                (String::new(), !e.contains("owner signature"), e)
            }
        };
        out.push(ReleaseInfo {
            version: p.version,
            min_from,
            enabled: p.enabled,
            signer: p.signer,
            owner_signed,
            problem,
        });
    }
    out.sort_by(|a, b| itcmdr_agentlink::version_cmp(&b.version, &a.version));
    Ok(out)
}

/// Every agent's ring, version and last upgrade result.
#[derive(Debug, Clone, Serialize)]
pub struct MachineInfo {
    pub machine: String,
    pub ring: i64,
    pub version: String,
    pub state: String,
    pub upgrades: bool,
    pub last_version: String,
    pub last_ok: bool,
    pub last_error: String,
    pub last_ms: i64,
}

pub async fn machines(pool: &DbPool) -> Result<Vec<MachineInfo>> {
    let mut out = vec![];
    for a in crate::agents::list(pool, false).await? {
        let last: Option<(String, i64, String, i64)> = sqlx::query_as(
            "SELECT version, ok, error, ts_ms FROM agent_upgrades WHERE machine = ? COLLATE NOCASE",
        )
        .bind(&a.machine)
        .fetch_optional(pool)
        .await?;
        let (lv, lok, le, lms) = last.unwrap_or_default();
        out.push(MachineInfo {
            ring: ring_of(pool, &a.machine).await,
            upgrades: enabled_for(pool, &a.machine).await,
            machine: a.machine,
            version: a.version,
            state: a.state,
            last_version: lv,
            last_ok: lok == 1,
            last_error: le,
            last_ms: lms,
        });
    }
    out.sort_by(|a, b| (a.ring, &a.machine).cmp(&(b.ring, &b.machine)));
    Ok(out)
}
