//! Backup and restore (decision 13).
//!
//! Archive (`*.itcb`): `ITCMDRB1` | u32 LE header length | header JSON | ciphertext.
//! - The ciphertext is a zip of the server state (consistent SQLCipher snapshot, `keys/db.key.escrow`,
//!   `server.json`, installed `service_plugins/**`, `quarantine/index.jsonl`), encrypted with
//!   XChaCha20-Poly1305 under a random content key.
//! - The content key is sealed for the master X25519 key (only the 24 words open it).
//! - The header (manifest: versions, schema version, sha256 per file) is signed by the server's
//!   root key, whose master certificate is included. The signature covers the ciphertext hash.
//! - `keys/db.key` and `keys/root` are never archived (machine-bound); restore rebuilds db.key from
//!   the escrow with the words and issues a new root key.
use crate::auth;
use crate::config::ServerConfig;
use crate::db::{self, DbPool};
use crate::keys::{self, Certificate, Escrow, Master};
use anyhow::{anyhow, bail, ensure, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use chrono::{Datelike, TimeZone};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub const MAGIC: &[u8; 8] = b"ITCMDRB1";
pub const FORMAT: &str = "itcmdr-backup/v1";
pub const EXT: &str = "itcb";
const AAD: &[u8] = b"itcmdr-backup/v1";
const F_DB: &str = "itcmdr.database";
const F_ESCROW: &str = "keys/db.key.escrow";
const F_CONFIG: &str = "server.json";
const D_PLUGINS: &str = "service_plugins/";
const D_PLUGIN_LOGS: &str = "plugin-logs/";
const F_QINDEX: &str = "quarantine/index.jsonl";

/// Installed plugin files as (`service_plugins/<rel>`, bytes), plus the quarantine index.
fn plugin_files(layout: &crate::storage::Layout) -> Result<Files> {
    let mut out = Vec::new();
    let root = layout.service_plugins();
    if root.is_dir() {
        for e in walkdir::WalkDir::new(&root).sort_by_file_name() {
            let e = e?;
            let rel = e.path().strip_prefix(&root)?;
            if rel
                .components()
                .next()
                .is_some_and(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            {
                continue; // .installing-* temp dirs
            }
            if e.file_type().is_file() {
                let r = rel.to_string_lossy().replace('\\', "/");
                out.push((format!("{D_PLUGINS}{r}"), std::fs::read(e.path())?));
            }
        }
    }
    let q = crate::packages::quarantine_index(layout);
    if q.exists() {
        out.push((F_QINDEX.to_string(), std::fs::read(q)?));
    }
    // Plugin run logs (decision 11).
    let logs = layout.plugin_logs();
    if logs.is_dir() {
        for e in walkdir::WalkDir::new(&logs).sort_by_file_name() {
            let e = e?;
            if e.file_type().is_file() {
                let r = e
                    .path()
                    .strip_prefix(&logs)?
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((format!("{D_PLUGIN_LOGS}{r}"), std::fs::read(e.path())?));
            }
        }
    }
    Ok(out)
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(src) {
        let e = e?;
        let t = dst.join(e.path().strip_prefix(src)?);
        if e.file_type().is_dir() {
            std::fs::create_dir_all(&t)?;
        } else if e.file_type().is_file() {
            std::fs::copy(e.path(), &t)?;
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub format: String,
    pub itcmdr_version: String,
    pub schema_version: i64,
    pub created_ms: i64,
    /// "manual" | "nightly" | "pre-restore"
    pub kind: String,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Header {
    pub manifest: Manifest,
    pub master_ed25519_pub: String,
    pub master_x25519_pub: String,
    pub signer_public_key: String,
    pub signer_certificate: Certificate,
    pub content_key: Escrow,
    pub nonce: String,
    pub ciphertext_sha256: String,
    /// Ed25519 by the signer over `FORMAT\n` + this header as JSON with `sig` empty.
    pub sig: String,
}

impl Header {
    fn signed_message(&self) -> Result<Vec<u8>> {
        let mut h = self.clone();
        h.sig.clear();
        let mut m = format!("{FORMAT}\n").into_bytes();
        m.extend(serde_json::to_vec(&h)?);
        Ok(m)
    }
}

/// Archive contents: (path inside the archive, bytes).
pub type Files = Vec<(String, Vec<u8>)>;
type BucketKey = fn(chrono::DateTime<chrono::Local>) -> String;

/// What `list` shows.
#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub manifest: Manifest,
}

fn sha256_hex(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

/// `itcmdr-YYYYMMDD-HHMMSSmmm-<kind>.itcb` (UTC).
pub fn archive_name(created_ms: i64, kind: &str) -> String {
    let t = chrono::Utc
        .timestamp_millis_opt(created_ms)
        .single()
        .unwrap_or_default();
    format!("itcmdr-{}-{kind}.{EXT}", t.format("%Y%m%d-%H%M%S%3f"))
}

/// Creation time (unix ms) from an archive name.
pub fn parse_name_ms(name: &str) -> Option<i64> {
    parse_name(name).map(|(ms, _)| ms)
}

fn parse_name(name: &str) -> Option<(i64, String)> {
    let rest = name
        .strip_prefix("itcmdr-")?
        .strip_suffix(&format!(".{EXT}"))?;
    let (date, rest) = rest.split_at_checked(8)?;
    let rest = rest.strip_prefix('-')?;
    let (time, kind) = rest.split_at_checked(9)?;
    let kind = kind.strip_prefix('-')?;
    let dt =
        chrono::NaiveDateTime::parse_from_str(&format!("{date}{}", &time[..6]), "%Y%m%d%H%M%S")
            .ok()?;
    let ms: i64 = time[6..].parse().ok()?;
    Some((dt.and_utc().timestamp_millis() + ms, kind.to_string()))
}

/// Consistent snapshot of the live database (one read transaction via `sqlcipher_export`),
/// encrypted with the same db.key.
async fn snapshot_db(pool: &DbPool, key: &[u8; 32], dst: &Path) -> Result<()> {
    let _ = std::fs::remove_file(dst);
    let mut conn = pool.acquire().await?;
    let d = dst.display().to_string().replace('\'', "''");
    sqlx::query(&format!(
        "ATTACH DATABASE '{d}' AS bk KEY {}",
        crate::storage::key_pragma(key)
    ))
    .execute(&mut *conn)
    .await?;
    let r = async {
        sqlx::query("SELECT sqlcipher_export('bk')")
            .execute(&mut *conn)
            .await?;
        sqlx::query(&format!("PRAGMA bk.user_version = {}", db::SCHEMA_VERSION))
            .execute(&mut *conn)
            .await?;
        anyhow::Ok(())
    }
    .await;
    sqlx::query("DETACH DATABASE bk")
        .execute(&mut *conn)
        .await?;
    r
}

fn zip_files(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opt =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in files {
        w.start_file(name.as_str(), opt)?;
        w.write_all(data)?;
    }
    Ok(w.finish()?.into_inner())
}

fn unzip_files(data: &[u8]) -> Result<Files> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(data))?;
    let mut out = Vec::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i)?;
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        out.push((f.name().to_string(), b));
    }
    Ok(out)
}

/// Create an archive in `cfg.backup_dir()` (or `out_dir`). Needs `init` (master key, root key).
pub async fn create(
    pool: &DbPool,
    cfg: &ServerConfig,
    kind: &str,
    out_dir: Option<&Path>,
) -> Result<BackupInfo> {
    let layout = cfg.layout();
    let master = auth::master_public(pool)
        .await?
        .ok_or_else(|| anyhow!("server is not initialized; run `itcmdr-cli init` first"))?;
    let root = auth::load_root_key(&layout).context("loading keys/root")?;
    let root_fp = keys::fingerprint(&root.verifying_key());
    let cert = auth::certificate_of(pool, &root_fp)
        .await
        .context("root key is not registered (restored elsewhere? see restore)")?;
    if !auth::escrow_path(&layout).exists() {
        auth::escrow_db_key(&layout, &master)?;
    }
    let dir = match out_dir {
        Some(d) => d.to_path_buf(),
        None => crate::settings::backup_dir(pool, cfg).await?,
    };
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let created_ms = chrono::Utc::now().timestamp_millis();
    let name = archive_name(created_ms, kind);

    let key = Zeroizing::new(crate::storage::load_or_create_db_key(&layout)?);
    let snap = dir.join(format!(".{name}.snapshot"));
    snapshot_db(pool, &key, &snap).await?;
    let db_bytes = std::fs::read(&snap);
    let _ = std::fs::remove_file(&snap);
    let mut files = vec![
        (F_DB.to_string(), db_bytes?),
        (
            F_ESCROW.to_string(),
            std::fs::read(auth::escrow_path(&layout))?,
        ),
        (F_CONFIG.to_string(), serde_json::to_vec_pretty(cfg)?),
    ];
    files.extend(plugin_files(&layout)?);
    let manifest = Manifest {
        format: FORMAT.into(),
        itcmdr_version: env!("CARGO_PKG_VERSION").into(),
        schema_version: db::SCHEMA_VERSION,
        created_ms,
        kind: kind.into(),
        files: files
            .iter()
            .map(|(p, b)| FileEntry {
                path: p.clone(),
                size: b.len() as u64,
                sha256: sha256_hex(b),
            })
            .collect(),
    };
    let plain = Zeroizing::new(zip_files(&files)?);
    let mut ck = Zeroizing::new([0u8; 32]);
    rand::rngs::OsRng.fill_bytes(ck.as_mut());
    let mut nonce = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ct = XChaCha20Poly1305::new(ck.as_ref().into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plain,
                aad: AAD,
            },
        )
        .map_err(|_| anyhow!("backup encryption failed"))?;
    let mut h = Header {
        manifest: manifest.clone(),
        master_ed25519_pub: hex::encode(master.ed25519.as_bytes()),
        master_x25519_pub: hex::encode(master.x25519.as_bytes()),
        signer_public_key: hex::encode(root.verifying_key().as_bytes()),
        signer_certificate: cert,
        content_key: keys::seal(&master.x25519, ck.as_ref())?,
        nonce: hex::encode(nonce),
        ciphertext_sha256: sha256_hex(&ct),
        sig: String::new(),
    };
    h.sig = hex::encode(root.sign(&h.signed_message()?).to_bytes());
    let hj = serde_json::to_vec(&h)?;
    let path = dir.join(&name);
    let tmp = dir.join(format!(".{name}.tmp"));
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(MAGIC)?;
        f.write_all(&(hj.len() as u32).to_le_bytes())?;
        f.write_all(&hj)?;
        f.write_all(&ct)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &path)?;
    let size = std::fs::metadata(&path)?.len();
    Ok(BackupInfo {
        name,
        path,
        size,
        manifest,
    })
}

fn read_archive(path: &Path) -> Result<(Header, Vec<u8>)> {
    let b = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    ensure!(
        b.len() > 12 && &b[..8] == MAGIC,
        "not an itcmdr backup archive"
    );
    let n = u32::from_le_bytes(b[8..12].try_into()?) as usize;
    ensure!(b.len() >= 12 + n, "archive is truncated");
    let h: Header = serde_json::from_slice(&b[12..12 + n]).context("archive header is damaged")?;
    ensure!(
        h.manifest.format == FORMAT,
        "unsupported archive format {}",
        h.manifest.format
    );
    Ok((h, b[12 + n..].to_vec()))
}

/// Read the manifest without verifying anything (for listing).
pub fn read_manifest(path: &Path) -> Result<Manifest> {
    Ok(read_archive(path)?.0.manifest)
}

/// Check the signature chain and the ciphertext hash. `expected_master`: this server's master
/// public key (from the DB) or the one rebuilt from the words; None checks self-consistency only.
pub fn verify(path: &Path, expected_master: Option<&VerifyingKey>) -> Result<Header> {
    let (h, ct) = read_archive(path)?;
    verify_parsed(&h, &ct, expected_master)?;
    Ok(h)
}

fn verify_parsed(h: &Header, ct: &[u8], expected_master: Option<&VerifyingKey>) -> Result<()> {
    let master = keys::parse_public_key(&h.master_ed25519_pub)?;
    if let Some(m) = expected_master {
        ensure!(
            m == &master,
            "archive belongs to a different itcmdr server (master key mismatch)"
        );
    }
    keys::verify_certificate(&master, &h.signer_certificate)?;
    let signer = keys::parse_public_key(&h.signer_public_key)?;
    let c = &h.signer_certificate;
    ensure!(
        c.fingerprint == keys::fingerprint(&signer) && c.kind == "client" && c.role == "owner",
        "archive signer is not this server's root key"
    );
    let sig: [u8; 64] = hex::decode(&h.sig)?
        .try_into()
        .map_err(|_| anyhow!("bad archive signature"))?;
    signer
        .verify(&h.signed_message()?, &Signature::from_bytes(&sig))
        .map_err(|_| anyhow!("archive signature is invalid (header was changed)"))?;
    ensure!(
        sha256_hex(ct) == h.ciphertext_sha256,
        "archive contents were changed or truncated (hash mismatch)"
    );
    Ok(())
}

/// Full check with the words: signature, decryption, every file's size and sha256.
pub fn open(path: &Path, master: &Master) -> Result<(Header, Files)> {
    let (h, ct) = read_archive(path)?;
    let mp = master.public();
    verify_parsed(&h, &ct, Some(&mp.ed25519))?;
    ensure!(
        h.master_x25519_pub == hex::encode(mp.x25519.as_bytes()),
        "archive encryption key does not match these words"
    );
    let ck = keys::unseal(master, &h.content_key)?;
    ensure!(ck.len() == 32, "bad content key");
    let nonce: [u8; 24] = hex::decode(&h.nonce)?
        .try_into()
        .map_err(|_| anyhow!("bad nonce"))?;
    let plain = Zeroizing::new(
        XChaCha20Poly1305::new(ck.as_slice().into())
            .decrypt(XNonce::from_slice(&nonce), Payload { msg: &ct, aad: AAD })
            .map_err(|_| anyhow!("archive does not decrypt"))?,
    );
    let files = unzip_files(&plain)?;
    let mut names: Vec<_> = files.iter().map(|(n, _)| n.clone()).collect();
    let mut want: Vec<_> = h.manifest.files.iter().map(|f| f.path.clone()).collect();
    names.sort();
    want.sort();
    ensure!(names == want, "archive files do not match the manifest");
    for (n, b) in &files {
        let e = h.manifest.files.iter().find(|f| &f.path == n).unwrap();
        ensure!(
            e.size == b.len() as u64 && e.sha256 == sha256_hex(b),
            "{n}: sha256 does not match the manifest"
        );
    }
    Ok((h, files))
}

/// Archives in `dir`, newest first. Unreadable files are skipped.
pub fn list(dir: &Path) -> Result<Vec<BackupInfo>> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(out);
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if parse_name(&name).is_none() {
            continue;
        }
        if let Ok(m) = read_manifest(&e.path()) {
            out.push(BackupInfo {
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
                path: e.path(),
                name,
                manifest: m,
            });
        }
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.manifest.created_ms));
    Ok(out)
}

/// 4-week block of a date: ISO weeks 4N-3..4N, 13 per ISO year, week 53 in block 13.
pub fn block_key(d: chrono::NaiveDate) -> String {
    let w = d.iso_week();
    format!(
        "{}-B{:02}",
        w.year(),
        crate::metrics::block_of_week(w.week())
    )
}

/// Which nightly archives to delete: keep the newest per day for `daily` days, per ISO week for
/// `weekly` weeks and per 4-week block for `blocks` blocks (local time). Other kinds are never pruned.
pub fn prune_plan(names: &[String], daily: usize, weekly: usize, blocks: usize) -> Vec<String> {
    let mut nightly: Vec<(i64, &String)> = names
        .iter()
        .filter_map(|n| match parse_name(n) {
            Some((ms, k)) if k == "nightly" => Some((ms, n)),
            _ => None,
        })
        .collect();
    nightly.sort_by_key(|a| std::cmp::Reverse(a.0));
    let mut keep = std::collections::HashSet::new();
    let buckets: [(usize, BucketKey); 3] = [
        (daily, |t| t.format("%Y-%m-%d").to_string()),
        (weekly, |t| {
            let w = t.iso_week();
            format!("{}-W{}", w.year(), w.week())
        }),
        (blocks, |t| block_key(t.date_naive())),
    ];
    for (n, key) in buckets {
        let mut seen = Vec::new();
        for (ms, name) in &nightly {
            let t = chrono::Local
                .timestamp_millis_opt(*ms)
                .single()
                .unwrap_or_default();
            let k = key(t);
            if !seen.contains(&k) {
                if seen.len() >= n {
                    break;
                }
                seen.push(k);
                keep.insert((*name).clone());
            }
        }
    }
    nightly
        .into_iter()
        .filter(|(_, n)| !keep.contains(*n))
        .map(|(_, n)| n.clone())
        .collect()
}

/// Apply `prune_plan` in `dir`. Returns the deleted names.
pub fn prune(dir: &Path, cfg: &crate::config::BackupConfig) -> Result<Vec<String>> {
    let names: Vec<String> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let del = prune_plan(&names, cfg.keep_daily, cfg.keep_weekly, cfg.keep_blocks);
    for n in &del {
        std::fs::remove_file(dir.join(n))?;
    }
    Ok(del)
}

/// Background loop hook: take tonight's backup when due, then prune. Errors are logged.
pub async fn nightly(pool: &DbPool, cfg: &ServerConfig) {
    let Ok(bc) = crate::settings::effective_backup(pool, cfg).await else {
        return;
    };
    if !bc.nightly {
        return;
    }
    let now = chrono::Local::now();
    if now.hour_() < bc.hour {
        return;
    }
    let dir = bc.dir.clone().unwrap_or_else(|| cfg.layout().backups());
    let today = now.format("%Y-%m-%d").to_string();
    let done = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten().any(|e| {
                matches!(parse_name(&e.file_name().to_string_lossy()), Some((ms, k))
                    if k == "nightly" && chrono::Local.timestamp_millis_opt(ms).single()
                        .map(|t| t.format("%Y-%m-%d").to_string()) == Some(today.clone()))
            })
        })
        .unwrap_or(false);
    if done {
        return;
    }
    match auth::master_public(pool).await {
        Ok(Some(_)) => {}
        _ => return, // not initialized yet: nothing can be encrypted
    }
    match create(pool, cfg, "nightly", None).await {
        Ok(b) => {
            tracing::info!(archive=%b.path.display(), size=b.size, "nightly backup written");
            match prune(&dir, &bc) {
                Ok(d) if !d.is_empty() => tracing::info!(deleted=?d, "old backups pruned"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error=%e, "backup retention failed"),
            }
        }
        Err(e) => tracing::warn!(error=%e, "nightly backup failed"),
    }
}

trait HourExt {
    fn hour_(&self) -> u32;
}
impl HourExt for chrono::DateTime<chrono::Local> {
    fn hour_(&self) -> u32 {
        chrono::Timelike::hour(self)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct RestoreReport {
    pub dry_run: bool,
    pub manifest: Option<Manifest>,
    /// Where the previous state was copied (None on a fresh data dir or dry run).
    pub safety_copy: Option<PathBuf>,
    /// The archived server.json, written for review (the active config is not replaced).
    pub restored_config: Option<PathBuf>,
    pub new_root_fingerprint: Option<String>,
    pub table_counts: Vec<(String, i64)>,
}

/// Restore an archive into `cfg`'s data dir. The server must be stopped. Steps: verify + decrypt
/// with the words, check the schema version, copy the current state aside, write the database and
/// escrow, rebuild `keys/db.key` from the escrow, open (migrates older schemas), then issue a new
/// root key. `dry_run` stops after the checks.
pub async fn restore(
    cfg: &ServerConfig,
    archive: &Path,
    words: &str,
    dry_run: bool,
) -> Result<RestoreReport> {
    let master = Master::from_words(words)?;
    let (h, files) = open(archive, &master)?;
    let m = &h.manifest;
    if m.schema_version > db::SCHEMA_VERSION {
        bail!(
            "archive schema {} is newer than this itcmdr ({}, schema {}); upgrade itcmdr first",
            m.schema_version,
            env!("CARGO_PKG_VERSION"),
            db::SCHEMA_VERSION
        );
    }
    let mut rep = RestoreReport {
        dry_run,
        manifest: Some(m.clone()),
        ..Default::default()
    };
    if dry_run {
        return Ok(rep);
    }
    let layout = cfg.layout();
    layout.ensure()?;
    let dbp = cfg.database_path();
    let side = |ext: &str| PathBuf::from(format!("{}{ext}", dbp.display()));
    let keyf = layout.keys().join("db.key");
    let escrow = auth::escrow_path(&layout);
    let current: Vec<PathBuf> = [
        dbp.clone(),
        side("-wal"),
        side("-shm"),
        escrow.clone(),
        keyf.clone(),
        layout.keys().join("root"),
    ]
    .into_iter()
    .filter(|p| p.exists())
    .collect();
    // Archived plugin paths must be safe before anything is touched.
    let mut plugin_out = Vec::new();
    let mut log_out = Vec::new();
    for (n, b) in &files {
        if let Some(rel) = n.strip_prefix(D_PLUGINS) {
            let rel = crate::packages::safe_rel_path(rel).map_err(|r| anyhow!("archive: {r}"))?;
            plugin_out.push((layout.service_plugins().join(rel), b));
        }
        if let Some(rel) = n.strip_prefix(D_PLUGIN_LOGS) {
            let rel = crate::packages::safe_rel_path(rel).map_err(|r| anyhow!("archive: {r}"))?;
            log_out.push((layout.plugin_logs().join(rel), b));
        }
    }
    let plugins_dir = layout.service_plugins();
    let qindex = crate::packages::quarantine_index(&layout);
    let has_plugins = plugins_dir.is_dir() && std::fs::read_dir(&plugins_dir)?.next().is_some();
    if !current.is_empty() || has_plugins || qindex.exists() {
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let dst = layout.root.join(format!("pre-restore-{ts}"));
        std::fs::create_dir_all(&dst)?;
        for p in &current {
            std::fs::copy(p, dst.join(p.file_name().unwrap()))
                .with_context(|| format!("safety copy of {}", p.display()))?;
        }
        if has_plugins {
            copy_tree(&plugins_dir, &dst.join("service_plugins"))?;
        }
        if qindex.exists() {
            std::fs::copy(&qindex, dst.join("quarantine-index.jsonl"))?;
        }
        rep.safety_copy = Some(dst);
    }
    let get = |n: &str| {
        files
            .iter()
            .find(|(f, _)| f == n)
            .map(|(_, b)| b.as_slice())
    };
    let tmp = dbp.with_extension("restoring");
    std::fs::write(
        &tmp,
        get(F_DB).ok_or_else(|| anyhow!("archive has no database"))?,
    )?;
    for p in [side("-wal"), side("-shm")] {
        let _ = std::fs::remove_file(p);
    }
    std::fs::rename(&tmp, &dbp)?;
    std::fs::write(
        &escrow,
        get(F_ESCROW).ok_or_else(|| anyhow!("archive has no db.key escrow"))?,
    )?;
    auth::recover_db_key(&layout, words, true)?;
    if plugins_dir.exists() {
        std::fs::remove_dir_all(&plugins_dir)?;
    }
    std::fs::create_dir_all(&plugins_dir)?;
    // Plugin logs are merged: files that exist locally (newer lines) are kept.
    for (p, b) in log_out {
        if !p.exists() {
            std::fs::create_dir_all(p.parent().unwrap())?;
            std::fs::write(p, b)?;
        }
    }
    for (p, b) in plugin_out {
        std::fs::create_dir_all(p.parent().unwrap())?;
        std::fs::write(p, b)?;
    }
    if let Some(q) = get(F_QINDEX) {
        std::fs::write(&qindex, q)?;
    }
    if let Some(c) = get(F_CONFIG) {
        let p = layout.root.join("restored-server.json");
        std::fs::write(&p, c)?;
        rep.restored_config = Some(p);
    }
    let pool = db::init_db(cfg).await?;
    let ok: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await?;
    ensure!(ok == "ok", "restored database failed integrity_check: {ok}");
    rep.new_root_fingerprint = Some(auth::rotate_root(&pool, &layout, &master).await?);
    rep.table_counts = db::table_counts_of(&pool).await?;
    pool.close().await;
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        let ms = 1_790_000_000_123;
        let n = archive_name(ms, "nightly");
        assert_eq!(parse_name(&n), Some((ms, "nightly".into())));
        assert_eq!(parse_name("itcmdr-junk.itcb"), None);
    }
}
