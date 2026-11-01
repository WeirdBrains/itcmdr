//! v0.18.0: plugins pushed by the server, kept in `plugins\<name>\` (one version each), and the
//! schedule set (`schedules.json`).
//!
//! A PluginUpdate is accepted only when the server root key's endorsement over
//! `plugin_bytes(name, version, sha256(package.toml))` verifies, package.toml names that plugin and
//! version, the file list is exactly package.toml + package.sig + the listed files (safe relative
//! names) and every listed file matches its checksum. It is written to a staging folder and swapped
//! in, so a bad update never replaces a good one.
use anyhow::{anyhow, bail, ensure, Context, Result};
use itcmdr_agentlink::pb;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "package.toml";
pub const SIGNATURE: &str = "package.sig";

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFile {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub main: bool,
    pub checksum: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    #[serde(default)]
    pub default_error_criticality: Option<String>,
}

pub fn plugins_dir(dir: &Path) -> PathBuf {
    dir.join("plugins")
}

pub fn plugin_dir(dir: &Path, name: &str) -> PathBuf {
    plugins_dir(dir).join(name)
}

fn safe_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 64
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        && !n.starts_with('.')
}

fn safe_rel(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.starts_with('\\')
        && !p.contains(':')
        && p.split(['/', '\\'])
            .all(|c| !c.is_empty() && c != "." && c != "..")
}

pub fn read_manifest(d: &Path) -> Result<Manifest> {
    let t = std::fs::read_to_string(d.join(MANIFEST)).context("package.toml")?;
    toml::from_str(&t).context("package.toml is not valid")
}

/// Installed plugins as Hello reports them: `name@version:sha256(package.toml)`.
pub fn installed(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(plugins_dir(dir))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| {
            let b = std::fs::read(e.path().join(MANIFEST)).ok()?;
            let m: Manifest = toml::from_str(std::str::from_utf8(&b).ok()?).ok()?;
            Some(format!(
                "{}@{}:{}",
                m.name,
                m.version,
                itcmdr_agentlink::sha256_hex(&b)
            ))
        })
        .collect();
    v.sort();
    v
}

/// Names of installed plugins.
pub fn installed_names(dir: &Path) -> Vec<String> {
    installed(dir)
        .into_iter()
        .filter_map(|s| s.split_once('@').map(|(n, _)| n.to_string()))
        .collect()
}

/// Verify and install a PluginUpdate (`root`: the server root public key, hex).
pub fn apply(dir: &Path, root: &str, u: &pb::PluginUpdate) -> Result<()> {
    ensure!(safe_name(&u.name), "unsafe plugin name {:?}", u.name);
    ensure!(!root.is_empty(), "no server root key known: enroll again");
    let files: BTreeMap<&str, &[u8]> = u
        .files
        .iter()
        .map(|f| (f.name.as_str(), f.data.as_slice()))
        .collect();
    ensure!(files.len() == u.files.len(), "duplicate file in the update");
    let mbytes = files
        .get(MANIFEST)
        .ok_or_else(|| anyhow!("no package.toml in the update"))?;
    let sha = itcmdr_agentlink::sha256_hex(mbytes);
    let pk: [u8; 32] = hex::decode(root)?
        .try_into()
        .map_err(|_| anyhow!("bad root key"))?;
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk)?;
    let sig: [u8; 64] = hex::decode(&u.endorsement)
        .context("endorsement is not hex")?
        .try_into()
        .map_err(|_| anyhow!("endorsement has the wrong length"))?;
    vk.verify_strict(
        &itcmdr_agentlink::plugin_bytes(&u.name, &u.version, &sha),
        &ed25519_dalek::Signature::from_bytes(&sig),
    )
    .map_err(|_| anyhow!("the plugin is not endorsed by the server root key"))?;
    let m: Manifest = toml::from_str(std::str::from_utf8(mbytes)?).context("package.toml")?;
    ensure!(
        m.name == u.name && m.version == u.version,
        "package.toml is {} {}, the update says {} {}",
        m.name,
        m.version,
        u.name,
        u.version
    );
    ensure!(
        m.files.iter().filter(|f| f.main).count() == 1,
        "package needs exactly one main file"
    );
    for f in &m.files {
        ensure!(safe_rel(&f.name), "unsafe file name {:?}", f.name);
        let data = files
            .get(f.name.as_str())
            .ok_or_else(|| anyhow!("{} is missing", f.name))?;
        ensure!(
            itcmdr_agentlink::sha256_hex(data).eq_ignore_ascii_case(&f.checksum),
            "checksum mismatch for {}",
            f.name
        );
    }
    for n in files.keys() {
        ensure!(
            *n == MANIFEST || *n == SIGNATURE || m.files.iter().any(|f| f.name == *n),
            "unexpected file {n:?}"
        );
    }
    let base = plugins_dir(dir);
    std::fs::create_dir_all(&base)?;
    let stage = base.join(format!(".stage-{}", u.name));
    let _ = std::fs::remove_dir_all(&stage);
    for (n, data) in &files {
        let p = stage.join(n);
        if let Some(par) = p.parent() {
            std::fs::create_dir_all(par)?;
        }
        std::fs::write(&p, data)?;
    }
    let dst = plugin_dir(dir, &u.name);
    let old = base.join(format!(".old-{}", u.name));
    let _ = std::fs::remove_dir_all(&old);
    if dst.exists() {
        std::fs::rename(&dst, &old)?;
    }
    if let Err(e) = std::fs::rename(&stage, &dst) {
        let _ = std::fs::rename(&old, &dst);
        bail!("installing {}: {e}", u.name);
    }
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

pub fn remove(dir: &Path, name: &str) -> Result<()> {
    ensure!(safe_name(name), "unsafe plugin name");
    let d = plugin_dir(dir, name);
    if d.exists() {
        std::fs::remove_dir_all(&d)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sched {
    pub id: String,
    pub name: String,
    pub cron: String,
    pub timezone: String,
    pub plugins: Vec<String>,
    pub catch_up: bool,
    #[serde(default)]
    pub args: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schedules {
    pub version: String,
    pub schedules: Vec<Sched>,
    /// Last fire time (unix ms) per schedule id.
    #[serde(default)]
    pub last_fire: BTreeMap<String, i64>,
}

fn sched_path(dir: &Path) -> PathBuf {
    dir.join("schedules.json")
}

pub fn load_schedules(dir: &Path) -> Schedules {
    std::fs::read(sched_path(dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_schedules(dir: &Path, s: &Schedules) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let p = sched_path(dir);
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(&tmp, &p)?;
    Ok(())
}

/// Replace the set (keeps last-fire times of schedules that stay; new ones start now).
pub fn set_schedules(dir: &Path, set: &pb::ScheduleSet, now_ms: i64) -> Result<()> {
    let mut cur = load_schedules(dir);
    for s in &set.schedules {
        cron::Schedule::from_str_checked(&s.cron)?;
    }
    let last: BTreeMap<String, i64> = set
        .schedules
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                cur.last_fire.get(&s.id).copied().unwrap_or(now_ms),
            )
        })
        .collect();
    cur = Schedules {
        version: set.version.clone(),
        schedules: set
            .schedules
            .iter()
            .map(|s| Sched {
                id: s.id.clone(),
                name: s.name.clone(),
                cron: s.cron.clone(),
                timezone: s.timezone.clone(),
                plugins: s.plugins.clone(),
                catch_up: s.catch_up,
                args: s.args.clone().into_iter().collect(),
            })
            .collect(),
        last_fire: last,
    };
    save_schedules(dir, &cur)
}

trait FromStrChecked {
    fn from_str_checked(s: &str) -> Result<cron::Schedule>;
}
impl FromStrChecked for cron::Schedule {
    fn from_str_checked(s: &str) -> Result<cron::Schedule> {
        use std::str::FromStr;
        cron::Schedule::from_str(s).map_err(|e| anyhow!("invalid cron {s:?}: {e}"))
    }
}

/// First fire time of `s` strictly after `after_ms` (None if never).
pub fn next_fire(s: &Sched, after_ms: i64) -> Option<i64> {
    let c = cron::Schedule::from_str_checked(&s.cron).ok()?;
    let after = chrono::DateTime::from_timestamp_millis(after_ms)?;
    let tz = s.timezone.trim();
    let zone: Option<chrono_tz::Tz> = if tz.is_empty() || tz.eq_ignore_ascii_case("local") {
        iana_time_zone::get_timezone()
            .ok()
            .and_then(|n| n.parse().ok())
    } else {
        tz.parse().ok()
    };
    match zone {
        Some(z) => c
            .after(&after.with_timezone(&z))
            .next()
            .map(|t| t.timestamp_millis()),
        None => c
            .after(&after.with_timezone(&chrono::Local))
            .next()
            .map(|t| t.timestamp_millis()),
    }
}

/// Schedules due at `now_ms`: (schedule, catch-up?) — a schedule whose next fire after its last
/// fire is in the past is due once (missed fires collapse into one, and only with catch_up when
/// more than 5 minutes late). Updates last-fire times.
pub fn due(dir: &Path, now_ms: i64) -> Result<Vec<Sched>> {
    let mut s = load_schedules(dir);
    let mut out = vec![];
    for sc in &s.schedules {
        let last = s.last_fire.get(&sc.id).copied().unwrap_or(now_ms);
        let Some(n) = next_fire(sc, last) else {
            continue;
        };
        if n <= now_ms && (now_ms - n <= 5 * 60_000 || sc.catch_up) {
            out.push(sc.clone());
        }
    }
    for sc in &s.schedules {
        let last = s.last_fire.get(&sc.id).copied().unwrap_or(now_ms);
        if next_fire(sc, last).is_some_and(|n| n <= now_ms) {
            s.last_fire.insert(sc.id.clone(), now_ms);
        }
    }
    save_schedules(dir, &s)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_and_missed() {
        let d = std::env::temp_dir().join(format!("itp-{}", rand::random::<u32>()));
        let set = pb::ScheduleSet {
            version: "v1".into(),
            schedules: vec![
                pb::AgentSchedule {
                    id: "a".into(),
                    cron: "0 * * * * *".into(),
                    timezone: "UTC".into(),
                    plugins: vec!["*".into()],
                    ..Default::default()
                },
                pb::AgentSchedule {
                    id: "b".into(),
                    cron: "0 0 3 * * *".into(),
                    timezone: "UTC".into(),
                    plugins: vec!["x".into()],
                    catch_up: true,
                    ..Default::default()
                },
            ],
        };
        let t0 = 1_800_000_000_000i64; // some time
        set_schedules(&d, &set, t0).unwrap();
        assert!(due(&d, t0).unwrap().is_empty());
        // 61 s later: "a" (every minute) is due, "b" (03:00) maybe not.
        let v = due(&d, t0 + 61_000).unwrap();
        assert!(v.iter().any(|s| s.id == "a"));
        // Offline 2 days: "a" missed many times, no catch-up and more than 5 min late -> skipped;
        // "b" has catch-up -> due once.
        let v = due(&d, t0 + 2 * 86_400_000).unwrap();
        assert!(!v.iter().any(|s| s.id == "a"));
        assert_eq!(v.iter().filter(|s| s.id == "b").count(), 1);
        assert!(due(&d, t0 + 2 * 86_400_000 + 1000).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn safe_names() {
        assert!(safe_rel("a/b.ps1") && !safe_rel("../x") && !safe_rel("/etc") && !safe_rel("C:x"));
        assert!(safe_name("disk-free") && !safe_name("../x") && !safe_name(".hidden"));
    }
}
