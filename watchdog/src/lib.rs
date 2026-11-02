//! itcmdr watchdogs (v0.21.0, SERVER-AGENT.md section 8 and ADMINSVR.md "Site-server self-upgrade").
//!
//! The program folder of a service (`root`) holds `current\` (what the service runs),
//! `previous\` (the version before), `staged\<version>\` (a verified release the service handed
//! over) and `failed\<version>\` (a release that was rolled back). The service asks for an upgrade
//! by writing `<data>\upgrade\request.json`; the watchdog stops the service, swaps
//! `staged\<v>` -> `current` (keeping the old one as `previous`), starts it and waits up to
//! `health_secs` for its health check. If that fails it rolls back (`current` -> `failed\<v>`,
//! `previous` -> `current`), starts the old version and writes `<data>\upgrade\result.json`
//! (the service reports it to its server on the next connect).
//!
//! Between upgrades the watchdog restarts the service when it is stopped or when its health file
//! (`<data>\health.json`, written every 30 s) is stale. The watchdog itself lives outside
//! `current\` and is only replaced by the installer, so a bad release cannot remove the rollback.
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CURRENT: &str = "current";
pub const PREVIOUS: &str = "previous";
pub const STAGED: &str = "staged";
pub const FAILED: &str = "failed";

/// `<data>\upgrade\request.json`, written by the service after it verified and staged a release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Request {
    pub version: String,
    /// The version running now (what a rollback returns to).
    pub from: String,
    pub health_secs: u64,
    pub ts_ms: i64,
}

/// `<data>\upgrade\result.json`, written by the watchdog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Outcome {
    pub version: String,
    pub from: String,
    pub ok: bool,
    pub rolled_back: bool,
    pub error: String,
    pub ts_ms: i64,
}

/// `<data>\health.json`, written by the service every 30 s.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Health {
    pub version: String,
    pub pid: u32,
    pub ts_ms: i64,
    /// Last time the service linked to its server (agent: Welcome; server: setup check + gRPC up).
    pub connected_ms: i64,
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn upgrade_dir(data: &Path) -> PathBuf {
    data.join("upgrade")
}
pub fn request_path(data: &Path) -> PathBuf {
    upgrade_dir(data).join("request.json")
}
pub fn result_path(data: &Path) -> PathBuf {
    upgrade_dir(data).join("result.json")
}
pub fn health_path(data: &Path) -> PathBuf {
    data.join("health.json")
}

/// Write JSON atomically (temp file + rename).
pub fn write_json<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    std::fs::rename(&tmp, p)?;
    Ok(())
}

pub fn read_json<T: for<'a> Deserialize<'a>>(p: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(p).ok()?).ok()
}

pub fn write_health(data: &Path, h: &Health) -> Result<()> {
    write_json(&health_path(data), h)
}

/// Is this binary running from `<root>\current\`? Then `<root>` is returned.
pub fn layout_root(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    (dir.file_name()? == CURRENT).then(|| dir.parent().map(Path::to_path_buf))?
}

/// Valid version text for a folder name.
pub fn safe_version(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 40
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
        && !v.starts_with('.')
}

/// The service the watchdog looks after.
pub trait Service {
    fn stop(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    fn running(&self) -> bool;
}

/// Swap `staged\<v>` in as `current` (the old `current` becomes `previous`).
pub fn swap_in(root: &Path, version: &str) -> Result<()> {
    ensure!(safe_version(version), "bad version {version:?}");
    let staged = root.join(STAGED).join(version);
    ensure!(staged.is_dir(), "{} is missing", staged.display());
    let cur = root.join(CURRENT);
    let prev = root.join(PREVIOUS);
    if prev.exists() {
        std::fs::remove_dir_all(&prev).with_context(|| format!("removing {}", prev.display()))?;
    }
    if cur.exists() {
        std::fs::rename(&cur, &prev).context("current -> previous")?;
    }
    if let Err(e) = std::fs::rename(&staged, &cur) {
        // Put the old version back before reporting.
        let _ = std::fs::rename(&prev, &cur);
        bail!("staged -> current: {e}");
    }
    Ok(())
}

/// Undo a swap: `current` -> `failed\<v>`, `previous` -> `current`.
pub fn roll_back(root: &Path, version: &str) -> Result<()> {
    let cur = root.join(CURRENT);
    let prev = root.join(PREVIOUS);
    ensure!(prev.is_dir(), "no previous version to roll back to");
    let failed = root.join(FAILED).join(version);
    if failed.exists() {
        std::fs::remove_dir_all(&failed)?;
    }
    std::fs::create_dir_all(root.join(FAILED))?;
    if cur.exists() {
        std::fs::rename(&cur, &failed).context("current -> failed")?;
    }
    std::fs::rename(&prev, &cur).context("previous -> current")?;
    Ok(())
}

/// Health check of an upgraded service: it runs the new version and linked after the swap.
pub type Check<'a> = &'a dyn Fn(&Request, i64) -> bool;

/// Settings of [`handle`] (short in tests).
pub struct Timing {
    pub poll: Duration,
    pub stop_wait: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            poll: Duration::from_secs(5),
            stop_wait: Duration::from_secs(30),
        }
    }
}

fn wait_stopped(svc: &dyn Service, t: &Timing) -> Result<()> {
    let end = std::time::Instant::now() + t.stop_wait;
    while svc.running() {
        if std::time::Instant::now() > end {
            bail!("the service did not stop within {:?}", t.stop_wait);
        }
        std::thread::sleep(t.poll.min(Duration::from_millis(500)));
    }
    Ok(())
}

/// Carry out one upgrade request. Always leaves a service running (new or old) when it can, and
/// returns the outcome it wrote to result.json. The request file is removed first, so a crash
/// mid-way never repeats an upgrade in a loop.
pub fn handle(
    root: &Path,
    data: &Path,
    svc: &dyn Service,
    healthy: Check,
    t: &Timing,
) -> Result<Option<Outcome>> {
    let rp = request_path(data);
    let Some(req) = read_json::<Request>(&rp) else {
        if rp.exists() {
            let _ = std::fs::remove_file(&rp);
            log("ignored an unreadable upgrade request");
        }
        return Ok(None);
    };
    std::fs::remove_file(&rp)?;
    let mut out = Outcome {
        version: req.version.clone(),
        from: req.from.clone(),
        ok: false,
        rolled_back: false,
        error: String::new(),
        ts_ms: now_ms(),
    };
    log(&format!(
        "upgrade {} -> {}: stopping the service",
        req.from, req.version
    ));
    let r = (|| -> Result<()> {
        svc.stop()?;
        wait_stopped(svc, t)?;
        swap_in(root, &req.version)
    })();
    if let Err(e) = r {
        out.error = format!("swap failed: {e:#}");
        log(&out.error);
        let _ = svc.start();
        write_json(&result_path(data), &out)?;
        return Ok(Some(out));
    }
    let swapped = now_ms();
    let started = svc.start();
    let end = std::time::Instant::now() + Duration::from_secs(req.health_secs.max(1));
    let mut ok = false;
    if started.is_ok() {
        loop {
            if healthy(&req, swapped) {
                ok = true;
                break;
            }
            if std::time::Instant::now() >= end {
                break;
            }
            std::thread::sleep(t.poll);
        }
    }
    if ok {
        out.ok = true;
        log(&format!("upgrade to {} is healthy", req.version));
    } else {
        out.error = match started {
            Err(e) => format!(
                "{} did not start: {e:#}; rolled back to {}",
                req.version, req.from
            ),
            Ok(()) => format!(
                "{} was not healthy within {} s; rolled back to {}",
                req.version, req.health_secs, req.from
            ),
        };
        log(&out.error);
        let _ = svc.stop();
        let _ = wait_stopped(svc, t);
        match roll_back(root, &req.version) {
            Ok(()) => out.rolled_back = true,
            Err(e) => out.error.push_str(&format!(" (ROLLBACK FAILED: {e:#})")),
        }
        let _ = svc.start();
    }
    out.ts_ms = now_ms();
    write_json(&result_path(data), &out)?;
    Ok(Some(out))
}

/// Between upgrades: start a stopped service, restart one whose health file is older than
/// `stale_secs`. Returns what it did.
pub fn keep_alive(data: &Path, svc: &dyn Service, stale_secs: i64, now: i64) -> Option<String> {
    if !svc.running() {
        return Some(match svc.start() {
            Ok(()) => "the service was stopped: started it".into(),
            Err(e) => format!("the service was stopped and did not start: {e:#}"),
        });
    }
    let h: Health = read_json(&health_path(data))?;
    if now - h.ts_ms > stale_secs * 1000 {
        let _ = svc.stop();
        std::thread::sleep(Duration::from_secs(2));
        return Some(match svc.start() {
            Ok(()) => format!(
                "health file {} s old: restarted the service",
                (now - h.ts_ms) / 1000
            ),
            Err(e) => format!("health file stale; restart failed: {e:#}"),
        });
    }
    None
}

static LOG: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Log to this file as well as stderr (a service has no stderr).
pub fn log_to(p: PathBuf) {
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if std::fs::metadata(&p)
        .map(|m| m.len() > 5 << 20)
        .unwrap_or(false)
    {
        let _ = std::fs::rename(&p, p.with_extension("log.1"));
    }
    let _ = LOG.set(p);
}

pub fn log(msg: &str) {
    let line = format!(
        "{} {msg}",
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z")
    );
    eprintln!("{line}");
    if let Some(p) = LOG.get() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// A Windows service controlled with sc.exe (no extra rights beyond the watchdog's LocalSystem).
pub struct ScService(pub String);

impl Service for ScService {
    fn stop(&self) -> Result<()> {
        let _ = sc(&["stop", &self.0]);
        Ok(())
    }
    fn start(&self) -> Result<()> {
        let o = sc(&["start", &self.0])?;
        // 1056 = already running.
        ensure!(
            o.0 || o.1.contains("1056"),
            "sc start {}: {}",
            self.0,
            o.1.trim()
        );
        Ok(())
    }
    fn running(&self) -> bool {
        sc(&["query", &self.0])
            .map(|(_, out)| {
                out.contains("RUNNING")
                    || out.contains("START_PENDING")
                    || out.contains("STOP_PENDING")
            })
            .unwrap_or(false)
    }
}

fn sc(args: &[&str]) -> Result<(bool, String)> {
    let o = std::process::Command::new("sc.exe").args(args).output()?;
    let mut s = String::from_utf8_lossy(&o.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&o.stderr));
    Ok((o.status.success(), s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct Fake {
        root: PathBuf,
        up: Cell<bool>,
        /// Version the "service" reports once started (read from current\version.txt).
        ran: RefCell<Vec<String>>,
        fail_start: Cell<bool>,
    }
    impl Service for Fake {
        fn stop(&self) -> Result<()> {
            self.up.set(false);
            Ok(())
        }
        fn start(&self) -> Result<()> {
            if self.fail_start.get() {
                bail!("boom");
            }
            let v = std::fs::read_to_string(self.root.join(CURRENT).join("version.txt"))?;
            self.ran.borrow_mut().push(v);
            self.up.set(true);
            Ok(())
        }
        fn running(&self) -> bool {
            self.up.get()
        }
    }

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, Fake) {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("prog");
        let data = t.path().join("data");
        for (d, v) in [
            (CURRENT.to_string(), "1.0"),
            (format!("{STAGED}/2.0"), "2.0"),
        ] {
            std::fs::create_dir_all(root.join(&d)).unwrap();
            std::fs::write(root.join(&d).join("version.txt"), v).unwrap();
        }
        write_json(
            &request_path(&data),
            &Request {
                version: "2.0".into(),
                from: "1.0".into(),
                health_secs: 1,
                ts_ms: 0,
            },
        )
        .unwrap();
        let f = Fake {
            root: root.clone(),
            up: Cell::new(true),
            ran: RefCell::new(vec![]),
            fail_start: Cell::new(false),
        };
        (t, root, data, f)
    }
    fn fast() -> Timing {
        Timing {
            poll: Duration::from_millis(20),
            stop_wait: Duration::from_secs(1),
        }
    }
    fn cur(root: &Path) -> String {
        std::fs::read_to_string(root.join(CURRENT).join("version.txt")).unwrap()
    }

    #[test]
    fn healthy_upgrade_keeps_the_new_version() {
        let (_t, root, data, f) = setup();
        let o = handle(
            &root,
            &data,
            &f,
            &|r: &Request, _| cur(&f.root) == r.version,
            &fast(),
        )
        .unwrap()
        .unwrap();
        assert!(o.ok && !o.rolled_back, "{o:?}");
        assert_eq!(cur(&root), "2.0");
        assert_eq!(
            std::fs::read_to_string(root.join(PREVIOUS).join("version.txt")).unwrap(),
            "1.0"
        );
        assert!(!request_path(&data).exists());
        assert_eq!(read_json::<Outcome>(&result_path(&data)).unwrap(), o);
        assert!(f.running());
    }

    #[test]
    fn unhealthy_upgrade_rolls_back() {
        let (_t, root, data, f) = setup();
        let o = handle(&root, &data, &f, &|_, _| false, &fast())
            .unwrap()
            .unwrap();
        assert!(!o.ok && o.rolled_back, "{o:?}");
        assert!(o.error.contains("not healthy"), "{}", o.error);
        assert_eq!(cur(&root), "1.0");
        assert_eq!(
            std::fs::read_to_string(root.join(FAILED).join("2.0").join("version.txt")).unwrap(),
            "2.0"
        );
        assert_eq!(*f.ran.borrow(), vec!["2.0".to_string(), "1.0".to_string()]);
        assert!(f.running());
    }

    #[test]
    fn missing_stage_keeps_the_old_version_running() {
        let (_t, root, data, f) = setup();
        std::fs::remove_dir_all(root.join(STAGED).join("2.0")).unwrap();
        let o = handle(&root, &data, &f, &|_, _| true, &fast())
            .unwrap()
            .unwrap();
        assert!(
            !o.ok && !o.rolled_back && o.error.contains("swap failed"),
            "{o:?}"
        );
        assert_eq!(cur(&root), "1.0");
        assert!(f.running());
    }

    #[test]
    fn a_new_version_that_cannot_start_rolls_back() {
        let (_t, root, data, f) = setup();
        f.fail_start.set(true);
        let o = handle(&root, &data, &f, &|_, _| true, &fast())
            .unwrap()
            .unwrap();
        assert!(
            !o.ok && o.rolled_back && o.error.contains("did not start"),
            "{o:?}"
        );
        assert_eq!(cur(&root), "1.0");
    }

    #[test]
    fn bad_request_is_dropped_not_repeated() {
        let (_t, root, data, f) = setup();
        std::fs::write(request_path(&data), b"{nope").unwrap();
        assert!(handle(&root, &data, &f, &|_, _| true, &fast())
            .unwrap()
            .is_none());
        assert!(!request_path(&data).exists());
        assert_eq!(cur(&root), "1.0");
    }

    #[test]
    fn keep_alive_restarts_stopped_or_stale() {
        let (_t, _root, data, f) = setup();
        f.up.set(false);
        assert!(keep_alive(&data, &f, 180, 1_000_000)
            .unwrap()
            .contains("started"));
        write_health(
            &data,
            &Health {
                version: "1.0".into(),
                pid: 1,
                ts_ms: 1_000_000,
                connected_ms: 0,
            },
        )
        .unwrap();
        assert!(keep_alive(&data, &f, 180, 1_000_000 + 60_000).is_none());
        assert!(keep_alive(&data, &f, 180, 1_000_000 + 200_000)
            .unwrap()
            .contains("restarted"));
    }

    #[test]
    fn layout_root_needs_current() {
        assert_eq!(
            layout_root(Path::new("/p/agent/current/x.exe")),
            Some(PathBuf::from("/p/agent"))
        );
        assert_eq!(layout_root(Path::new("/p/agent/x.exe")), None);
        assert!(!safe_version("../x") && safe_version("0.21.0-pre1"));
    }
}
