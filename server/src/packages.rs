//! Service plugin packages (decisions 7 and 8).
//!
//! A package is a `.tar.gz` holding `package.toml`, `package.sig` and exactly the files listed in
//! `package.toml` (`[[files]]`, each with a sha256 `checksum`, exactly one `main = true`).
//! `package.sig` is JSON `{"v":1,"signer":"<fingerprint>","sig":"<hex>"}`: an Ed25519 signature by a
//! registered, active admin/owner key over `itcmdr-package/v1\n<sha256 hex of package.toml>`.
//! package.toml carries every file's checksum, so the signature covers the whole package.
//!
//! Anything wrong (bad gzip/tar/TOML, unsafe or non-regular entries, missing/extra files, checksum,
//! signature, unknown/revoked/low-role signer) moves the package to `quarantine/`, appends a line to
//! `quarantine/index.jsonl`, logs a warning, writes an audit entry, and denies the install.
use crate::db::DbPool;
use crate::storage::Layout;
use anyhow::{anyhow, bail, ensure, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// Default of the `plugins.max_package_mib` setting.
pub const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
pub const MANIFEST: &str = "package.toml";
pub const SIGNATURE: &str = "package.sig";
const SIG_DOMAIN: &str = "itcmdr-package/v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PackageFile {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub main: bool,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub files: Vec<PackageFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSig {
    pub v: u32,
    pub signer: String,
    pub sig: String,
}

/// Why a package was refused. Every variant quarantines the package.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Reject(pub String);

fn rej<T>(m: impl Into<String>) -> std::result::Result<T, Reject> {
    Err(Reject(m.into()))
}

pub fn sha256_hex(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

fn sig_message(manifest_bytes: &[u8]) -> Vec<u8> {
    format!("{SIG_DOMAIN}\n{}", sha256_hex(manifest_bytes)).into_bytes()
}

const FEATURES: [&str; 3] = ["verify", "execute", "log"];
const KINDS: [&str; 4] = ["bin", "powershell", "data", "config"];

/// Safe relative path: no absolute/root/prefix, no `..`, no backslashes, no empty segments.
/// Returns the normalized `a/b/c` form.
pub fn safe_rel_path(p: &str) -> std::result::Result<String, Reject> {
    if p.is_empty() || p.contains('\\') || p.contains('\0') {
        return rej(format!("unsafe path {p:?}"));
    }
    let mut parts = Vec::new();
    for c in Path::new(p).components() {
        match c {
            Component::Normal(s) => {
                let s = s
                    .to_str()
                    .ok_or_else(|| Reject(format!("non-UTF8 path {p:?}")))?;
                if s.contains(':') {
                    return rej(format!("unsafe path {p:?}"));
                }
                parts.push(s.to_string());
            }
            Component::CurDir => {}
            _ => return rej(format!("unsafe path {p:?} (absolute or ..)")),
        }
    }
    if parts.is_empty() {
        return rej(format!("unsafe path {p:?}"));
    }
    Ok(parts.join("/"))
}

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

fn valid_version(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars().next().is_some_and(|c| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
}

pub fn parse_manifest(bytes: &[u8]) -> std::result::Result<Manifest, Reject> {
    let text = std::str::from_utf8(bytes).map_err(|_| Reject("package.toml is not UTF8".into()))?;
    let m: Manifest =
        toml::from_str(text).map_err(|e| Reject(format!("package.toml is invalid: {e}")))?;
    if !valid_name(&m.name) {
        return rej(format!(
            "invalid package name {:?} (a-z, 0-9, - and _, max 64)",
            m.name
        ));
    }
    if !valid_version(&m.version) {
        return rej(format!("invalid version {:?}", m.version));
    }
    for f in &m.features {
        if !FEATURES.contains(&f.as_str()) {
            return rej(format!("unknown feature {f:?}"));
        }
    }
    if m.files.is_empty() {
        return rej("package.toml lists no files");
    }
    let mut seen = std::collections::HashSet::new();
    for f in &m.files {
        let n = safe_rel_path(&f.name)?;
        if n != f.name {
            return rej(format!("file name {:?} must be written as {n:?}", f.name));
        }
        if n == MANIFEST || n == SIGNATURE {
            return rej(format!("{n} cannot be listed as a file"));
        }
        if !seen.insert(n.to_ascii_lowercase()) {
            return rej(format!("{n} is listed twice"));
        }
        if !KINDS.contains(&f.kind.as_str()) {
            return rej(format!(
                "{n}: unknown type {:?} (bin, powershell, data, config)",
                f.kind
            ));
        }
        if f.checksum.len() != 64 || !f.checksum.chars().all(|c| c.is_ascii_hexdigit()) {
            return rej(format!("{n}: checksum must be a sha256 hex string"));
        }
    }
    let mains = m.files.iter().filter(|f| f.main).count();
    if mains != 1 {
        return rej(format!(
            "exactly one file must have main = true (found {mains})"
        ));
    }
    Ok(m)
}

/// Unpack a `.tar.gz` into memory. Only regular files and directories are allowed.
pub fn read_tar_gz(bytes: &[u8]) -> std::result::Result<BTreeMap<String, Vec<u8>>, Reject> {
    read_tar_gz_max(bytes, MAX_PACKAGE_BYTES)
}

pub fn read_tar_gz_max(
    bytes: &[u8],
    max: u64,
) -> std::result::Result<BTreeMap<String, Vec<u8>>, Reject> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return rej("not a .tar.gz package (no gzip header)");
    }
    let gz = flate2::read::GzDecoder::new(bytes).take(max + 1);
    let mut ar = tar::Archive::new(gz);
    let mut out = BTreeMap::new();
    let mut total = 0u64;
    let entries = ar
        .entries()
        .map_err(|e| Reject(format!("not a .tar.gz package: {e}")))?;
    for e in entries {
        let mut e = e.map_err(|e| Reject(format!("damaged package: {e}")))?;
        let raw = String::from_utf8(e.path_bytes().to_vec())
            .map_err(|_| Reject("non-UTF8 path in package".into()))?;
        let t = e.header().entry_type();
        if t.is_dir() {
            safe_rel_path(raw.trim_end_matches('/')).or_else(|r| {
                if raw == "./" {
                    Ok(String::new())
                } else {
                    Err(r)
                }
            })?;
            continue;
        }
        if !t.is_file() {
            return rej(format!(
                "{raw}: only regular files are allowed (found {t:?})"
            ));
        }
        let p = safe_rel_path(&raw)?;
        let mut b = Vec::new();
        e.read_to_end(&mut b)
            .map_err(|e| Reject(format!("damaged package: {e}")))?;
        total += b.len() as u64;
        if total > max {
            return rej("package is too large");
        }
        if out.insert(p.clone(), b).is_some() {
            return rej(format!("{p} appears twice"));
        }
    }
    Ok(out)
}

/// An admin/owner key that may sign packages.
#[derive(Clone, Debug)]
pub struct PackageSigner {
    pub fingerprint: String,
    pub public_key: ed25519_dalek::VerifyingKey,
    pub name: String,
}

async fn signer_key(pool: &DbPool, fp: &str) -> Result<Option<PackageSigner>> {
    for t in ["users", "clients"] {
        let row: Option<(String, String, String)> = sqlx::query_as(&format!(
            "SELECT k.public_key, k.name, r.name FROM {t} k JOIN roles r ON r.id = k.role_id \
             WHERE k.fingerprint = ? AND k.revoked_at IS NULL"
        ))
        .bind(fp)
        .fetch_optional(pool)
        .await?;
        if let Some((pk, name, role)) = row {
            if role != "owner" && role != "admin" {
                return Ok(None);
            }
            return Ok(Some(PackageSigner {
                fingerprint: fp.into(),
                public_key: crate::keys::parse_public_key(&pk)?,
                name,
            }));
        }
    }
    Ok(None)
}

/// A package that passed every check.
#[derive(Debug)]
pub struct Validated {
    pub manifest: Manifest,
    pub files: BTreeMap<String, Vec<u8>>,
    pub signer: String,
    pub package_sha256: String,
}

/// Every check of decisions 7 and 8. `Err(Reject)` = quarantine; other errors are internal.
pub async fn validate(
    pool: &DbPool,
    bytes: &[u8],
    max: u64,
) -> Result<std::result::Result<Validated, Reject>> {
    let files = match read_tar_gz_max(bytes, max) {
        Ok(f) => f,
        Err(r) => return Ok(Err(r)),
    };
    let Some(mbytes) = files.get(MANIFEST) else {
        return Ok(rej("package.toml is missing"));
    };
    let manifest = match parse_manifest(mbytes) {
        Ok(m) => m,
        Err(r) => return Ok(Err(r)),
    };
    let Some(sbytes) = files.get(SIGNATURE) else {
        return Ok(rej(
            "package.sig is missing (packages must be signed by an admin or owner key)",
        ));
    };
    let listed: std::collections::BTreeSet<&str> =
        manifest.files.iter().map(|f| f.name.as_str()).collect();
    for name in files.keys() {
        if name != MANIFEST && name != SIGNATURE && !listed.contains(name.as_str()) {
            return Ok(rej(format!(
                "{name} is in the package but not listed in package.toml"
            )));
        }
    }
    for f in &manifest.files {
        let Some(b) = files.get(&f.name) else {
            return Ok(rej(format!(
                "{} is listed but missing from the package",
                f.name
            )));
        };
        if !sha256_hex(b).eq_ignore_ascii_case(&f.checksum) {
            return Ok(rej(format!(
                "{}: sha256 does not match package.toml",
                f.name
            )));
        }
    }
    let sig: PackageSig = match serde_json::from_slice(sbytes) {
        Ok(s) => s,
        Err(e) => return Ok(rej(format!("package.sig is invalid: {e}"))),
    };
    if sig.v != 1 {
        return Ok(rej(format!("unsupported package.sig version {}", sig.v)));
    }
    let Some(signer) = signer_key(pool, &sig.signer).await? else {
        return Ok(rej(format!(
            "signer {} is not an active admin or owner key on this server",
            &sig.signer[..sig.signer.len().min(16)]
        )));
    };
    let sb: [u8; 64] = match hex::decode(&sig.sig).ok().and_then(|v| v.try_into().ok()) {
        Some(b) => b,
        None => return Ok(rej("package.sig signature is malformed")),
    };
    if signer
        .public_key
        .verify(&sig_message(mbytes), &Signature::from_bytes(&sb))
        .is_err()
    {
        return Ok(rej("package signature is invalid"));
    }
    Ok(Ok(Validated {
        manifest,
        files,
        signer: format!("{} ({})", signer.name, &signer.fingerprint[..12]),
        package_sha256: sha256_hex(bytes),
    }))
}

/// One line of `quarantine/index.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub ts: String,
    pub file: String,
    pub source: String,
    pub sha256: String,
    pub reason: String,
}

pub fn quarantine_index(layout: &Layout) -> PathBuf {
    layout.quarantine().join("index.jsonl")
}

fn quarantine(
    layout: &Layout,
    bytes: &[u8],
    source: &str,
    reason: &str,
) -> Result<QuarantineEntry> {
    use std::io::Write;
    layout.ensure()?;
    let sha = sha256_hex(bytes);
    let base: String = source
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("package")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    let file = format!(
        "{}-{}-{}",
        chrono::Utc::now().format("%Y%m%d-%H%M%S"),
        &sha[..12],
        base
    );
    std::fs::write(layout.quarantine().join(&file), bytes)?;
    let e = QuarantineEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        file,
        source: source.into(),
        sha256: sha,
        reason: reason.into(),
    };
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(quarantine_index(layout))?;
    writeln!(f, "{}", serde_json::to_string(&e)?)?;
    Ok(e)
}

pub fn list_quarantine(layout: &Layout) -> Result<Vec<QuarantineEntry>> {
    let p = quarantine_index(layout);
    if !p.exists() {
        return Ok(vec![]);
    }
    Ok(std::fs::read_to_string(p)?
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}

/// Where a package comes from.
pub enum Source<'a> {
    Path(&'a Path),
    Url(&'a str),
}

impl Source<'_> {
    fn label(&self) -> String {
        match self {
            Source::Path(p) => p.display().to_string(),
            Source::Url(u) => u.to_string(),
        }
    }
    fn fetch(&self, max: u64) -> Result<Vec<u8>> {
        let mut b = Vec::new();
        match self {
            Source::Path(p) => {
                std::fs::File::open(p)
                    .with_context(|| format!("opening {}", p.display()))?
                    .take(max + 1)
                    .read_to_end(&mut b)?;
            }
            Source::Url(u) => {
                ensure!(
                    u.starts_with("https://") || u.starts_with("http://"),
                    "only http(s) URLs are supported"
                );
                let r = ureq::get(u)
                    .timeout(std::time::Duration::from_secs(120))
                    .call()
                    .map_err(|e| anyhow!("download failed: {e}"))?;
                r.into_reader().take(max + 1).read_to_end(&mut b)?;
            }
        }
        ensure!(
            b.len() as u64 <= max,
            "package is larger than {max} bytes (setting plugins.max_package_mib)"
        );
        Ok(b)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub installed_at: String,
    pub signer: String,
    pub package_sha256: String,
    pub dir: PathBuf,
}

pub fn plugin_dir(layout: &Layout, name: &str, version: &str) -> PathBuf {
    layout.service_plugins().join(format!("{name}-{version}"))
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("package rejected and quarantined ({file}): {reason}")]
    Quarantined { reason: String, file: String },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Fetch, validate, and install into `service_plugins/{name}-{version}`. `enable`: None = the
/// `plugins.enable_on_install` setting (default true); enabling disables other versions.
pub async fn install(
    pool: &DbPool,
    layout: &Layout,
    src: Source<'_>,
    enable: Option<bool>,
    actor: &str,
) -> std::result::Result<InstalledPlugin, InstallError> {
    let enable = match enable {
        Some(e) => e,
        None => crate::settings::get_bool(pool, None, "plugins.enable_on_install").await,
    };
    let max =
        crate::settings::get_i64(pool, None, "plugins.max_package_mib").await as u64 * 1024 * 1024;
    let label = src.label();
    // Downloads and file reads block: keep them off the async runtime.
    let bytes = match &src {
        Source::Path(p) => {
            let p = p.to_path_buf();
            tokio::task::spawn_blocking(move || Source::Path(&p).fetch(max))
        }
        Source::Url(u) => {
            let u = u.to_string();
            tokio::task::spawn_blocking(move || Source::Url(&u).fetch(max))
        }
    }
    .await
    .map_err(|e| anyhow!("fetch task failed: {e}"))??;
    let v = match validate(pool, &bytes, max).await? {
        Ok(v) => v,
        Err(Reject(reason)) => {
            let q = quarantine(layout, &bytes, &label, &reason)?;
            tracing::warn!(source=%label, %reason, file=%q.file, "plugin package quarantined");
            crate::db::insert_audit(
                pool,
                actor,
                "plugin-quarantine",
                &serde_json::json!({"source": label, "reason": reason, "file": q.file, "sha256": q.sha256}).to_string(),
            )
            .await?;
            return Err(InstallError::Quarantined {
                reason,
                file: q.file,
            });
        }
    };
    let (name, version) = (v.manifest.name.clone(), v.manifest.version.clone());
    let dst = plugin_dir(layout, &name, &version);
    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM plugins WHERE name = ? AND version = ?")
            .bind(&name)
            .bind(&version)
            .fetch_optional(pool)
            .await
            .map_err(anyhow::Error::from)?;
    if exists.is_some() || dst.exists() {
        return Err(anyhow!("{name} {version} is already installed (remove it first)").into());
    }
    layout.ensure()?;
    let tmp = layout
        .service_plugins()
        .join(format!(".installing-{}", uuid::Uuid::new_v4()));
    let write = || -> Result<()> {
        for (rel, b) in &v.files {
            let p = tmp.join(rel);
            std::fs::create_dir_all(p.parent().unwrap())?;
            std::fs::write(&p, b)?;
        }
        std::fs::rename(&tmp, &dst)?;
        Ok(())
    };
    if let Err(e) = write() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e.into());
    }
    let now = chrono::Utc::now().to_rfc3339();
    let r = async {
        let mut tx = pool.begin().await?;
        if enable {
            sqlx::query("UPDATE plugins SET enabled = 0 WHERE name = ?").bind(&name).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO plugins(id,name,version,enabled,installed_at,signer,package_sha256) VALUES (?,?,?,?,?,?,?)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&name)
            .bind(&version)
            .bind(enable as i64)
            .bind(&now)
            .bind(&v.signer)
            .bind(&v.package_sha256)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        anyhow::Ok(())
    }
    .await;
    if let Err(e) = r {
        let _ = std::fs::remove_dir_all(&dst);
        return Err(e.into());
    }
    crate::db::insert_audit(
        pool,
        actor,
        "plugin-install",
        &serde_json::json!({"name": name, "version": version, "signer": v.signer, "sha256": v.package_sha256, "enabled": enable}).to_string(),
    )
    .await?;
    tracing::info!(%name, %version, "plugin installed");
    Ok(InstalledPlugin {
        dir: dst,
        name,
        version,
        enabled: enable,
        installed_at: now,
        signer: v.signer,
        package_sha256: v.package_sha256,
    })
}

pub async fn list(pool: &DbPool, layout: &Layout) -> Result<Vec<InstalledPlugin>> {
    let rows: Vec<(String, String, i64, String, String, String)> = sqlx::query_as(
        "SELECT name, version, enabled, installed_at, signer, package_sha256 FROM plugins ORDER BY name, installed_at",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(name, version, en, installed_at, signer, package_sha256)| InstalledPlugin {
                dir: plugin_dir(layout, &name, &version),
                name,
                version,
                enabled: en != 0,
                installed_at,
                signer,
                package_sha256,
            },
        )
        .collect())
}

/// `name` or `name@version`; without a version there must be exactly one installed version.
async fn resolve(pool: &DbPool, spec: &str) -> Result<(String, String)> {
    let (name, ver) = match spec.split_once('@') {
        Some((n, v)) => (n, Some(v)),
        None => (spec, None),
    };
    let vs: Vec<String> =
        sqlx::query_scalar("SELECT version FROM plugins WHERE name = ? ORDER BY installed_at")
            .bind(name)
            .fetch_all(pool)
            .await?;
    match ver {
        Some(v) if vs.iter().any(|x| x == v) => Ok((name.into(), v.into())),
        Some(v) => bail!("{name} {v} is not installed"),
        None if vs.len() == 1 => Ok((name.into(), vs[0].clone())),
        None if vs.is_empty() => bail!("{name} is not installed"),
        None => bail!(
            "{name} has several versions installed ({}); use {name}@<version>",
            vs.join(", ")
        ),
    }
}

/// Enable one version (other versions of the same plugin are disabled).
pub async fn enable(pool: &DbPool, spec: &str, actor: &str) -> Result<(String, String)> {
    let (n, v) = resolve(pool, spec).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE plugins SET enabled = (version = ?) WHERE name = ?")
        .bind(&v)
        .bind(&n)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    crate::db::insert_audit(
        pool,
        actor,
        "plugin-enable",
        &serde_json::json!({"name": n, "version": v}).to_string(),
    )
    .await?;
    Ok((n, v))
}

/// Disable every version of a plugin.
pub async fn disable(pool: &DbPool, name: &str, actor: &str) -> Result<()> {
    let n = sqlx::query("UPDATE plugins SET enabled = 0 WHERE name = ?")
        .bind(name)
        .execute(pool)
        .await?
        .rows_affected();
    ensure!(n > 0, "{name} is not installed");
    crate::db::insert_audit(
        pool,
        actor,
        "plugin-disable",
        &serde_json::json!({"name": name}).to_string(),
    )
    .await?;
    Ok(())
}

/// Remove an installed version (files and record). An enabled version needs `force`.
pub async fn remove(
    pool: &DbPool,
    layout: &Layout,
    spec: &str,
    force: bool,
    actor: &str,
) -> Result<(String, String)> {
    let (n, v) = resolve(pool, spec).await?;
    let en: i64 = sqlx::query_scalar("SELECT enabled FROM plugins WHERE name = ? AND version = ?")
        .bind(&n)
        .bind(&v)
        .fetch_one(pool)
        .await?;
    ensure!(
        en == 0 || force,
        "{n} {v} is enabled; disable it first or pass --force"
    );
    let dir = plugin_dir(layout, &n, &v);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    sqlx::query("DELETE FROM plugins WHERE name = ? AND version = ?")
        .bind(&n)
        .bind(&v)
        .execute(pool)
        .await?;
    crate::db::insert_audit(
        pool,
        actor,
        "plugin-remove",
        &serde_json::json!({"name": n, "version": v}).to_string(),
    )
    .await?;
    Ok((n, v))
}

/// Build and sign a package from `dir/package.toml` (checksums are filled in) with `key`.
pub fn pack(dir: &Path, key: &SigningKey, out: &Path) -> Result<Manifest> {
    let text = std::fs::read_to_string(dir.join(MANIFEST))
        .with_context(|| format!("reading {}", dir.join(MANIFEST).display()))?;
    let mut v: toml::Value = toml::from_str(&text)?;
    let files = v
        .get_mut("files")
        .and_then(|f| f.as_array_mut())
        .ok_or_else(|| anyhow!("package.toml needs [[files]] entries"))?;
    for f in files.iter_mut() {
        let t = f
            .as_table_mut()
            .ok_or_else(|| anyhow!("bad [[files]] entry"))?;
        let name = t
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("[[files]] entry without name"))?
            .to_string();
        let b = std::fs::read(dir.join(&name)).with_context(|| format!("reading {name}"))?;
        t.insert("checksum".into(), toml::Value::String(sha256_hex(&b)));
    }
    let mtext = toml::to_string(&v)?;
    let m = parse_manifest(mtext.as_bytes()).map_err(|r| anyhow!("{r}"))?;
    let sig = PackageSig {
        v: 1,
        signer: crate::keys::fingerprint(&key.verifying_key()),
        sig: hex::encode(key.sign(&sig_message(mtext.as_bytes())).to_bytes()),
    };
    let mut entries: Vec<(String, Vec<u8>)> = vec![
        (MANIFEST.into(), mtext.into_bytes()),
        (SIGNATURE.into(), serde_json::to_vec_pretty(&sig)?),
    ];
    for f in &m.files {
        entries.push((f.name.clone(), std::fs::read(dir.join(&f.name))?));
    }
    write_tar_gz(out, &entries)?;
    Ok(m)
}

/// Write regular-file entries (mode 0644, mtime 0) as `.tar.gz`.
pub fn write_tar_gz(out: &Path, entries: &[(String, Vec<u8>)]) -> Result<()> {
    let f = std::fs::File::create(out)?;
    let gz = flate2::write::GzEncoder::new(f, flate2::Compression::default());
    let mut t = tar::Builder::new(gz);
    for (name, b) in entries {
        let mut h = tar::Header::new_gnu();
        h.set_size(b.len() as u64);
        h.set_mode(0o644);
        h.set_mtime(0);
        h.set_entry_type(tar::EntryType::Regular);
        t.append_data(&mut h, name, &b[..])?;
    }
    t.into_inner()?.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_paths() {
        assert_eq!(safe_rel_path("a/b.ps1").unwrap(), "a/b.ps1");
        assert_eq!(safe_rel_path("./a").unwrap(), "a");
        for bad in ["", "/etc/x", "../x", "a/../../x", "a\\b", "C:/x", "c:x"] {
            assert!(safe_rel_path(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn names_and_versions() {
        assert!(
            valid_name("xray-watch")
                && !valid_name("Xray")
                && !valid_name("-x")
                && !valid_name("a b")
        );
        assert!(
            valid_version("1.0.6")
                && valid_version("2.0.0-rc.1+b")
                && !valid_version("v1")
                && !valid_version("1/2")
        );
    }
}
