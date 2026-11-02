//! v0.21.0: agent self-upgrade (SERVER-AGENT.md section 8).
//!
//! An [`pb::UpgradeOffer`] is checked here (root endorsement that the signer is an owner, the
//! owner's own package signature, every checksum, release.toml and min_from), written to
//! `<program>\staged\<version>\` and handed to `itcmdr-agent-watchdog` with
//! `<data>\upgrade\request.json`. The watchdog swaps it in, waits for this process's health file
//! to show the new version linked to the server, and rolls back otherwise. Its result is
//! reported to the server (Applied "upgrade") after the next connect.
use crate::plugins::{safe_rel, Manifest};
use anyhow::{anyhow, ensure, Context, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use itcmdr_agentlink::pb;
use itcmdr_watchdog as wd;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};

pub const RELEASE: &str = "itcmdr-agent-release";
pub const EXE: &str = if cfg!(windows) {
    "itcmdr-agent.exe"
} else {
    "itcmdr-agent"
};
const PKG_DOMAIN: &str = "itcmdr-package/v1";

static CONNECTED_MS: AtomicI64 = AtomicI64::new(0);

/// Called on every Welcome.
pub fn mark_connected() {
    CONNECTED_MS.store(chrono::Utc::now().timestamp_millis(), Ordering::SeqCst);
}

/// Write `<data>\health.json` (the watchdog reads it).
pub fn write_health(dir: &Path) {
    let h = wd::Health {
        version: crate::link::VERSION.into(),
        pid: std::process::id(),
        ts_ms: chrono::Utc::now().timestamp_millis(),
        connected_ms: CONNECTED_MS.load(Ordering::SeqCst),
    };
    if let Err(e) = wd::write_health(dir, &h) {
        tracing::warn!(error=%format!("{e:#}"), "health file");
    }
}

/// The program folder (`<root>` of `<root>\current\itcmdr-agent.exe`), if installed that way.
pub fn program_root() -> Option<PathBuf> {
    std::env::var_os("ITCMDR_AGENT_PROGRAM")
        .map(PathBuf::from)
        .or_else(|| wd::layout_root(&std::env::current_exe().ok()?))
}

fn key(hex_s: &str, what: &str) -> Result<VerifyingKey> {
    let b: [u8; 32] = hex::decode(hex_s.trim())
        .with_context(|| format!("{what} is not hex"))?
        .try_into()
        .map_err(|_| anyhow!("{what} has the wrong length"))?;
    Ok(VerifyingKey::from_bytes(&b)?)
}

fn sig(hex_s: &str, what: &str) -> Result<Signature> {
    let b: [u8; 64] = hex::decode(hex_s.trim())
        .with_context(|| format!("{what} is not hex"))?
        .try_into()
        .map_err(|_| anyhow!("{what} has the wrong length"))?;
    Ok(Signature::from_bytes(&b))
}

#[derive(serde::Deserialize)]
struct PackageSig {
    signer: String,
    sig: String,
}

#[derive(serde::Deserialize)]
struct ReleaseToml {
    version: String,
    #[serde(default)]
    min_from: String,
}

/// Check an offer; the files to stage on success.
pub fn verify(
    root_key: &str,
    o: &pb::UpgradeOffer,
    running: &str,
) -> Result<BTreeMap<String, Vec<u8>>> {
    ensure!(
        !root_key.is_empty(),
        "no server root key known: enroll again"
    );
    ensure!(wd::safe_version(&o.version), "bad version {:?}", o.version);
    let files: BTreeMap<String, Vec<u8>> = o
        .files
        .iter()
        .map(|f| (f.name.clone(), f.data.clone()))
        .collect();
    ensure!(files.len() == o.files.len(), "duplicate file in the offer");
    let m = files
        .get("package.toml")
        .ok_or_else(|| anyhow!("no package.toml"))?;
    let sha = itcmdr_agentlink::sha256_hex(m);
    key(root_key, "root key")?
        .verify_strict(
            &itcmdr_agentlink::upgrade_bytes(&o.version, &sha, &o.signer),
            &sig(&o.endorsement, "endorsement")?,
        )
        .map_err(|_| anyhow!("the release is not endorsed by the server root key"))?;
    let pk = key(&o.signer_public_key, "signer key")?;
    ensure!(
        itcmdr_agentlink::sha256_hex(pk.as_bytes()) == o.signer,
        "the signer key does not match the signer fingerprint"
    );
    let ps: PackageSig = serde_json::from_slice(
        files
            .get("package.sig")
            .ok_or_else(|| anyhow!("no package.sig"))?,
    )
    .context("package.sig")?;
    ensure!(
        ps.signer == o.signer,
        "package.sig is signed by {}, not {}",
        ps.signer,
        o.signer
    );
    pk.verify_strict(
        format!("{PKG_DOMAIN}\n{sha}").as_bytes(),
        &sig(&ps.sig, "package signature")?,
    )
    .map_err(|_| anyhow!("the owner signature on the package does not verify"))?;
    let man: Manifest = toml::from_str(std::str::from_utf8(m)?).context("package.toml")?;
    ensure!(
        man.name == RELEASE,
        "package is {}, not {RELEASE}",
        man.name
    );
    ensure!(
        man.version == o.version,
        "package is {}, the offer says {}",
        man.version,
        o.version
    );
    for f in &man.files {
        ensure!(safe_rel(&f.name), "unsafe file name {:?}", f.name);
        let d = files
            .get(&f.name)
            .ok_or_else(|| anyhow!("{} is missing", f.name))?;
        ensure!(
            itcmdr_agentlink::sha256_hex(d).eq_ignore_ascii_case(&f.checksum),
            "checksum mismatch for {}",
            f.name
        );
    }
    for n in files.keys() {
        ensure!(
            n == "package.toml" || n == "package.sig" || man.files.iter().any(|f| &f.name == n),
            "unexpected file {n:?}"
        );
    }
    let rel: ReleaseToml = toml::from_str(std::str::from_utf8(
        files
            .get("release.toml")
            .filter(|_| man.files.iter().any(|f| f.name == "release.toml"))
            .ok_or_else(|| anyhow!("no release.toml"))?,
    )?)
    .context("release.toml")?;
    ensure!(
        rel.version == o.version,
        "release.toml says {}",
        rel.version
    );
    ensure!(
        rel.min_from.is_empty()
            || itcmdr_agentlink::version_cmp(running, &rel.min_from) != std::cmp::Ordering::Less,
        "{} upgrades from {} or later; this agent is {running}",
        o.version,
        rel.min_from
    );
    ensure!(
        itcmdr_agentlink::version_cmp(running, &o.version) == std::cmp::Ordering::Less,
        "this agent already runs {running}"
    );
    ensure!(
        files.contains_key(EXE) && man.files.iter().any(|f| f.name == EXE),
        "the release has no {EXE}"
    );
    Ok(files)
}

/// Verify, stage and hand over. `program` is the program folder (see [`program_root`]).
pub fn apply(
    dir: &Path,
    program: Option<&Path>,
    root_key: &str,
    o: &pb::UpgradeOffer,
) -> Result<()> {
    let files = verify(root_key, o, crate::link::VERSION)?;
    let program = program.ok_or_else(|| {
        anyhow!("this agent is not installed in the current\\ layout (reinstall with the v0.21 installer)")
    })?;
    ensure!(
        !wd::request_path(dir).exists(),
        "an upgrade is already waiting for the watchdog"
    );
    let stage = program.join(wd::STAGED).join(&o.version);
    let _ = std::fs::remove_dir_all(&stage);
    // The release replaces the whole of current\ except what it does not ship: keep the
    // helpers next to the exe (itcmdr-plugin-ps) unless the release has its own.
    if let Ok(rd) = std::fs::read_dir(program.join(wd::CURRENT)) {
        std::fs::create_dir_all(&stage)?;
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if e.path().is_file() && !files.contains_key(&n) {
                std::fs::copy(e.path(), stage.join(&n))?;
            }
        }
    }
    for (n, d) in &files {
        let p = stage.join(n);
        if let Some(par) = p.parent() {
            std::fs::create_dir_all(par)?;
        }
        std::fs::write(&p, d)?;
    }
    wd::write_json(
        &wd::request_path(dir),
        &wd::Request {
            version: o.version.clone(),
            from: crate::link::VERSION.into(),
            health_secs: o.health_secs.clamp(30, 3600) as u64,
            ts_ms: chrono::Utc::now().timestamp_millis(),
        },
    )?;
    Ok(())
}

/// The watchdog's result of the last upgrade, not yet reported to the server.
pub fn pending_result(dir: &Path) -> Option<wd::Outcome> {
    wd::read_json(&wd::result_path(dir))
}

pub fn mark_reported(dir: &Path) {
    let p = wd::result_path(dir);
    let _ = std::fs::rename(&p, p.with_extension("reported.json"));
}

/// Watchdog of the watchdog: on Windows, start `itcmdr-agent-watchdog` when it is not running.
/// Returns Some(true) when it runs, Some(false) when it is missing/stopped (and was started),
/// None when there is nothing to watch (not installed as a service).
pub fn check_watchdog() -> Option<bool> {
    if !cfg!(windows) || program_root().is_none() {
        return None;
    }
    use wd::Service;
    let s = wd::ScService("itcmdr-agent-watchdog".into());
    if s.running() {
        return Some(true);
    }
    let _ = s.start();
    Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    pub(crate) fn offer(
        root: &SigningKey,
        owner: &SigningKey,
        version: &str,
        min_from: &str,
        exe: &[u8],
    ) -> pb::UpgradeOffer {
        let rel = format!("version = \"{version}\"\nmin_from = \"{min_from}\"\n");
        let man = format!(
            "name = \"{RELEASE}\"\nversion = \"{version}\"\n\n[[files]]\nname = \"{EXE}\"\ntype = \"bin\"\nmain = true\nchecksum = \"{}\"\n\n[[files]]\nname = \"release.toml\"\ntype = \"config\"\nchecksum = \"{}\"\n",
            itcmdr_agentlink::sha256_hex(exe),
            itcmdr_agentlink::sha256_hex(rel.as_bytes())
        );
        let sha = itcmdr_agentlink::sha256_hex(man.as_bytes());
        let fp = itcmdr_agentlink::sha256_hex(owner.verifying_key().as_bytes());
        let psig = serde_json::json!({"v": 1, "signer": fp, "sig": hex::encode(owner.sign(format!("{PKG_DOMAIN}\n{sha}").as_bytes()).to_bytes())});
        pb::UpgradeOffer {
            version: version.into(),
            files: vec![
                pb::PluginFile {
                    name: "package.toml".into(),
                    data: man.into_bytes(),
                },
                pb::PluginFile {
                    name: "package.sig".into(),
                    data: psig.to_string().into_bytes(),
                },
                pb::PluginFile {
                    name: "release.toml".into(),
                    data: rel.into_bytes(),
                },
                pb::PluginFile {
                    name: EXE.into(),
                    data: exe.to_vec(),
                },
            ],
            signer: fp.clone(),
            signer_public_key: hex::encode(owner.verifying_key().as_bytes()),
            endorsement: hex::encode(
                root.sign(&itcmdr_agentlink::upgrade_bytes(version, &sha, &fp))
                    .to_bytes(),
            ),
            health_secs: 300,
        }
    }

    fn keys() -> (SigningKey, SigningKey, String) {
        let r = SigningKey::from_bytes(&[7; 32]);
        let o = SigningKey::from_bytes(&[9; 32]);
        let rh = hex::encode(r.verifying_key().as_bytes());
        (r, o, rh)
    }

    #[test]
    fn good_offer_stages_and_requests() {
        let (r, o, rh) = keys();
        let t = tempfile::tempdir().unwrap();
        let (data, prog) = (t.path().join("data"), t.path().join("prog"));
        std::fs::create_dir_all(prog.join("current")).unwrap();
        std::fs::write(prog.join("current").join("itcmdr-plugin-ps.exe"), b"ps").unwrap();
        std::fs::write(prog.join("current").join(EXE), b"old").unwrap();
        apply(
            &data,
            Some(&prog),
            &rh,
            &offer(&r, &o, "99.0.0", "0.1.0", b"new"),
        )
        .unwrap();
        let st = prog.join("staged").join("99.0.0");
        assert_eq!(std::fs::read(st.join(EXE)).unwrap(), b"new");
        assert_eq!(
            std::fs::read(st.join("itcmdr-plugin-ps.exe")).unwrap(),
            b"ps"
        );
        let rq: wd::Request = wd::read_json(&wd::request_path(&data)).unwrap();
        assert_eq!(
            (rq.version.as_str(), rq.from.as_str(), rq.health_secs),
            ("99.0.0", crate::link::VERSION, 300)
        );
        // A second offer while the first waits is refused.
        assert!(apply(
            &data,
            Some(&prog),
            &rh,
            &offer(&r, &o, "99.0.0", "", b"new")
        )
        .is_err());
    }

    #[test]
    fn tampered_or_wrongly_signed_offers_are_refused() {
        let (r, o, rh) = keys();
        let run = crate::link::VERSION;
        let mut x = offer(&r, &o, "99.0.0", "", b"new");
        x.files[3].data = b"evil".to_vec();
        assert!(format!("{:#}", verify(&rh, &x, run).unwrap_err()).contains("checksum"));
        // Root endorsement by another key.
        let other = SigningKey::from_bytes(&[3; 32]);
        let x = offer(&other, &o, "99.0.0", "", b"new");
        assert!(format!("{:#}", verify(&rh, &x, run).unwrap_err()).contains("root key"));
        // Endorsed for a different signer than package.sig.
        let mut x = offer(&r, &o, "99.0.0", "", b"new");
        let k2 = SigningKey::from_bytes(&[4; 32]);
        x.signer_public_key = hex::encode(k2.verifying_key().as_bytes());
        assert!(verify(&rh, &x, run).is_err());
        // min_from above this version, and a downgrade.
        assert!(format!(
            "{:#}",
            verify(&rh, &offer(&r, &o, "99.0.0", "98.0.0", b"n"), run).unwrap_err()
        )
        .contains("upgrades from"));
        assert!(format!(
            "{:#}",
            verify(&rh, &offer(&r, &o, "0.0.1", "", b"n"), run).unwrap_err()
        )
        .contains("already runs"));
        // Not installed in the current\ layout.
        let t = tempfile::tempdir().unwrap();
        assert!(format!(
            "{:#}",
            apply(t.path(), None, &rh, &offer(&r, &o, "99.0.0", "", b"n")).unwrap_err()
        )
        .contains("layout"));
    }
}
