//! v1.2.0: adminsvr's own backup (docs/BACKUP-ADMINSVR.md).
//!
//! Scope is adminsvr's OWN state only: its database, keys/root.key, tls/ (CA and server
//! certificate), the ACME account and certificate, the backup recovery public keys, its config
//! (ITCMDR_ADMINSVR_* settings) and the push key files it was configured with. adminsvr never
//! holds, fetches, decrypts or inspects a site's backups; nothing here reads anything a site sent
//! beyond the (already redacted) rows in adminsvr's own database. `ALLOWED` is the complete list
//! of what an archive may contain; `open` refuses anything else.
//!
//! Same design as the site server's backups (docs/BACKUP.md): the archive is encrypted with a
//! random content key that is sealed for a recovery key rebuilt from 24 words. The words are
//! shown once by `backup init` and never stored; adminsvr keeps only the public keys, so it can
//! make backups but cannot open them. The header (manifest with sha256 per file) is signed by
//! adminsvr's root key, and the root key is certified by the recovery key at `backup init`.
use crate::config::{BackupConfig, Config};
use crate::store::{self, Db, Pool};
use anyhow::{anyhow, bail, ensure, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub const FORMAT: &str = "itcmdr-adminsvr-backup/v1";
const MAGIC: &[u8; 8] = b"ITCADB01";
const AAD: &[u8] = b"itcmdr-adminsvr-backup/v1";
const EXT: &str = ".itcab";
const ROOT_CERT_MSG: &str = "itcmdr-adminsvr-backup/root/v1\n";

pub const F_SQLITE: &str = "db/adminsvr.sqlite";
pub const F_PG: &str = "db/postgres.json";
pub const F_ROOT: &str = "keys/root.key";
pub const F_RECOVERY: &str = "backup/recovery.json";
pub const F_CONFIG: &str = "config/adminsvr.env";
const TLS_FILES: [&str; 4] = [
    "tls/ca.pem",
    "tls/ca.key",
    "tls/server.pem",
    "tls/server.key",
];
const D_ACME: &str = "acme/";
const D_PUSH: &str = "push/";

/// Everything an archive may hold (exact names, or a folder prefix).
pub const ALLOWED: [&str; 11] = [
    F_SQLITE,
    F_PG,
    F_ROOT,
    F_RECOVERY,
    F_CONFIG,
    "tls/ca.pem",
    "tls/ca.key",
    "tls/server.pem",
    "tls/server.key",
    D_ACME,
    D_PUSH,
];

pub fn allowed(path: &str) -> bool {
    !path.contains("..")
        && !path.starts_with('/')
        && !path.contains('\\')
        && ALLOWED.iter().any(|a| {
            if a.ends_with('/') {
                path.starts_with(a) && path.len() > a.len()
            } else {
                path == *a
            }
        })
}

type Files = BTreeMap<String, Vec<u8>>;

fn sha256_hex(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

// ---------------------------------------------------------------- recovery key

/// `<data_dir>/backup/recovery.json`: the recovery PUBLIC keys and the root certificate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Recovery {
    pub v: u32,
    pub ed25519: String,
    pub x25519: String,
    /// The root key this recovery key certified (hex) and the signature (hex)
    pub root: String,
    pub root_sig: String,
    pub created_ms: i64,
}

impl Recovery {
    pub fn fingerprint(&self) -> String {
        sha256_hex(self.ed25519.as_bytes())[..16].to_string()
    }
    fn ed(&self) -> Result<VerifyingKey> {
        itcmdr_keys::parse_public_key(&self.ed25519)
    }
    fn x(&self) -> Result<x25519_dalek::PublicKey> {
        let b: [u8; 32] = hex::decode(&self.x25519)?
            .try_into()
            .map_err(|_| anyhow!("bad recovery x25519 key"))?;
        Ok(x25519_dalek::PublicKey::from(b))
    }
    /// The root key is certified by the recovery key.
    pub fn check(&self) -> Result<()> {
        let sig: [u8; 64] = hex::decode(&self.root_sig)?
            .try_into()
            .map_err(|_| anyhow!("bad root certificate"))?;
        self.ed()?
            .verify(
                format!("{ROOT_CERT_MSG}{}", self.root).as_bytes(),
                &Signature::from_bytes(&sig),
            )
            .map_err(|_| anyhow!("the root key is not certified by this recovery key"))
    }
}

pub fn recovery_path(data_dir: &Path) -> PathBuf {
    data_dir.join(F_RECOVERY)
}

pub fn load_recovery(data_dir: &Path) -> Result<Option<Recovery>> {
    let p = recovery_path(data_dir);
    if !p.exists() {
        return Ok(None);
    }
    let r: Recovery = serde_json::from_slice(&std::fs::read(&p)?)
        .with_context(|| format!("reading {}", p.display()))?;
    Ok(Some(r))
}

fn root_key(data_dir: &Path) -> Result<SigningKey> {
    let p = data_dir.join(F_ROOT);
    let s = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
    itcmdr_keys::key_from_seed(&hex::decode(s.trim()).context("root key is not hex")?)
}

fn recovery_for(words: &str, root: &VerifyingKey) -> Result<(Recovery, itcmdr_keys::Master)> {
    let m = itcmdr_keys::Master::from_words(words)?;
    let p = m.public();
    let root_hex = hex::encode(root.as_bytes());
    let sig = m
        .signing
        .sign(format!("{ROOT_CERT_MSG}{root_hex}").as_bytes());
    Ok((
        Recovery {
            v: 1,
            ed25519: hex::encode(p.ed25519.as_bytes()),
            x25519: hex::encode(p.x25519.as_bytes()),
            root: root_hex,
            root_sig: hex::encode(sig.to_bytes()),
            created_ms: crate::now_ms(),
        },
        m,
    ))
}

/// `backup init`: new recovery words (returned once, never stored). Refuses to replace an
/// existing recovery key unless `replace` (old archives then need the OLD words).
pub fn init(cfg: &Config, replace: bool) -> Result<(String, Recovery)> {
    crate::material::load(cfg)?; // root key and TLS exist
    if load_recovery(&cfg.data_dir)?.is_some() && !replace {
        bail!("backups are already set up (recovery key {}); use --replace to issue new words (existing archives still need the old words)",
            load_recovery(&cfg.data_dir)?.map(|r| r.fingerprint()).unwrap_or_default());
    }
    let words = itcmdr_keys::generate_words();
    let root = root_key(&cfg.data_dir)?;
    let (r, _) = recovery_for(&words, &root.verifying_key())?;
    write_private(
        &recovery_path(&cfg.data_dir),
        &serde_json::to_vec_pretty(&r)?,
    )?;
    Ok((words, r))
}

// ---------------------------------------------------------------- archive format

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub format: String,
    pub version: String,
    pub schema_version: i64,
    /// sqlite | postgres
    pub db: String,
    pub created_ms: i64,
    /// manual | nightly | pre-restore
    pub kind: String,
    pub files: Vec<FileEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Header {
    pub manifest: Manifest,
    pub recovery: Recovery,
    pub content_key: itcmdr_keys::Escrow,
    pub nonce: String,
    pub ciphertext_sha256: String,
    /// By the root key named in `recovery.root`
    pub sig: String,
}

impl Header {
    fn signed_message(&self) -> Result<Vec<u8>> {
        let mut m = FORMAT.as_bytes().to_vec();
        m.push(b'\n');
        m.extend(serde_json::to_vec(&self.manifest)?);
        m.push(b'\n');
        m.extend(serde_json::to_vec(&self.recovery)?);
        m.push(b'\n');
        m.extend(serde_json::to_vec(&self.content_key)?);
        m.extend(format!("\n{}\n{}", self.nonce, self.ciphertext_sha256).as_bytes());
        Ok(m)
    }
}

#[derive(Clone, Debug)]
pub struct BackupInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub manifest: Manifest,
}

pub fn archive_name(created_ms: i64, kind: &str) -> String {
    let t = chrono::DateTime::from_timestamp_millis(created_ms)
        .unwrap_or_default()
        .with_timezone(&chrono::Local);
    format!(
        "itcmdr-adminsvr-{}{:03}-{kind}{EXT}",
        t.format("%Y%m%d-%H%M%S"),
        created_ms.rem_euclid(1000)
    )
}

fn is_archive(name: &str) -> bool {
    name.starts_with("itcmdr-adminsvr-") && name.ends_with(EXT)
}

fn write_private(p: &Path, data: &[u8]) -> Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).with_context(|| format!("creating {}", d.display()))?;
    }
    let tmp = p.with_extension("tmp-write");
    std::fs::write(&tmp, data).with_context(|| format!("writing {}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, p).with_context(|| format!("writing {}", p.display()))?;
    Ok(())
}

fn tar_files(files: &Files) -> Result<Vec<u8>> {
    let mut b = tar::Builder::new(Vec::new());
    for (p, d) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(d.len() as u64);
        h.set_mode(0o600);
        h.set_mtime(0);
        h.set_cksum();
        b.append_data(&mut h, p, d.as_slice())?;
    }
    Ok(b.into_inner()?)
}

fn untar_files(data: &[u8]) -> Result<Files> {
    let mut out = Files::new();
    let mut a = tar::Archive::new(data);
    for e in a.entries()? {
        let mut e = e?;
        let p = e.path()?.to_string_lossy().replace('\\', "/");
        ensure!(
            allowed(&p),
            "archive holds an unexpected file {p:?}; refused"
        );
        let mut v = Vec::new();
        e.read_to_end(&mut v)?;
        out.insert(p, v);
    }
    Ok(out)
}

// ---------------------------------------------------------------- collect / create

/// The ITCMDR_ADMINSVR_* settings in effect (the env file is loaded into the environment first).
fn config_env() -> Vec<u8> {
    let mut v: Vec<(String, String)> = std::env::vars()
        .filter(|(k, _)| k.starts_with("ITCMDR_ADMINSVR_") && k != "ITCMDR_ADMINSVR_ENV_FILE")
        .collect();
    v.sort();
    let mut s =
        String::from("# adminsvr settings at backup time (ITCMDR_ADMINSVR_*); review before use\n");
    for (k, x) in v {
        s.push_str(&format!("{k}={x}\n"));
    }
    s.into_bytes()
}

fn read_dir_files(dir: &Path, prefix: &str, out: &mut Files) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.ends_with(".tmp-write") {
            continue;
        }
        let p = e.path();
        if p.is_dir() {
            read_dir_files(&p, &format!("{prefix}{name}/"), out)?;
        } else if p.is_file() {
            out.insert(format!("{prefix}{name}"), std::fs::read(&p)?);
        }
    }
    Ok(())
}

fn sql_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[derive(diesel::QueryableByName)]
struct TextRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    t: String,
}

#[derive(diesel::QueryableByName)]
struct PairRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    a: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    b: String,
}

fn pg_tables(c: &mut diesel::PgConnection) -> Result<Vec<String>> {
    use diesel::RunQueryDsl;
    let r: Vec<TextRow> = diesel::sql_query(
        "SELECT table_name::text AS t FROM information_schema.tables WHERE table_schema = current_schema() AND table_type = 'BASE TABLE' AND table_name <> '__diesel_schema_migrations' ORDER BY 1",
    )
    .load(c)?;
    Ok(r.into_iter().map(|x| x.t).collect())
}

/// Tables in an order where every referenced table comes first.
fn pg_order(c: &mut diesel::PgConnection, tables: &[String]) -> Result<Vec<String>> {
    use diesel::RunQueryDsl;
    let deps: Vec<PairRow> = diesel::sql_query(
        "SELECT cl.relname::text AS a, rf.relname::text AS b FROM pg_constraint k JOIN pg_class cl ON cl.oid = k.conrelid JOIN pg_class rf ON rf.oid = k.confrelid JOIN pg_namespace n ON n.oid = cl.relnamespace WHERE k.contype = 'f' AND n.nspname = current_schema()",
    )
    .load(c)?;
    let mut out: Vec<String> = Vec::new();
    let mut left: Vec<String> = tables.to_vec();
    while !left.is_empty() {
        let before = left.len();
        left.retain(|t| {
            let ready = deps
                .iter()
                .filter(|d| &d.a == t && d.b != *t)
                .all(|d| out.contains(&d.b) || !tables.contains(&d.b));
            if ready {
                out.push(t.clone());
            }
            !ready
        });
        if left.len() == before {
            out.append(&mut left); // a cycle: keep going, the insert reports any problem
        }
    }
    Ok(out)
}

fn pg_dump(c: &mut diesel::PgConnection) -> Result<Vec<u8>> {
    use diesel::RunQueryDsl;
    let mut m = serde_json::Map::new();
    for t in pg_tables(c)? {
        let r: Vec<TextRow> = diesel::sql_query(format!(
            "SELECT coalesce(json_agg(x), '[]'::json)::text AS t FROM \"{t}\" x"
        ))
        .load(c)?;
        let v: serde_json::Value =
            serde_json::from_str(&r.first().map(|x| x.t.clone()).unwrap_or("[]".into()))?;
        m.insert(t, v);
    }
    Ok(serde_json::to_vec(&serde_json::Value::Object(m))?)
}

fn pg_load(c: &mut diesel::PgConnection, dump: &[u8]) -> Result<()> {
    use diesel::connection::SimpleConnection;
    use diesel::{Connection, RunQueryDsl};
    let d: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(dump).context("database dump is not valid JSON")?;
    c.transaction(|c| {
        let tables = pg_tables(c)?;
        if !tables.is_empty() {
            let list = tables
                .iter()
                .map(|t| format!("\"{t}\""))
                .collect::<Vec<_>>()
                .join(", ");
            c.batch_execute(&format!("TRUNCATE {list} CASCADE"))?;
        }
        for t in pg_order(c, &tables)? {
            let Some(serde_json::Value::Array(rows)) = d.get(&t) else {
                continue;
            };
            if rows.is_empty() {
                continue;
            }
            let cols_now: Vec<TextRow> = diesel::sql_query(format!(
                "SELECT column_name::text AS t FROM information_schema.columns WHERE table_schema = current_schema() AND table_name = {}",
                sql_quote(&t)
            ))
            .load(c)?;
            let cols_now: Vec<String> = cols_now.into_iter().map(|x| x.t).collect();
            let mut cols: Vec<String> = rows
                .iter()
                .filter_map(|r| r.as_object())
                .flat_map(|o| o.keys().cloned())
                .filter(|k| cols_now.contains(k))
                .collect();
            cols.sort();
            cols.dedup();
            let cl = cols
                .iter()
                .map(|x| format!("\"{x}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let json = serde_json::to_string(rows)?;
            diesel::sql_query(format!(
                "INSERT INTO \"{t}\" ({cl}) SELECT {cl} FROM json_populate_recordset(NULL::\"{t}\", {}::json)",
                sql_quote(&json)
            ))
            .execute(c)
            .with_context(|| format!("restoring table {t}"))?;
            let seqs: Vec<PairRow> = diesel::sql_query(format!(
                "SELECT c::text AS a, pg_get_serial_sequence({}, c)::text AS b FROM unnest(ARRAY[{}]::text[]) c WHERE pg_get_serial_sequence({}, c) IS NOT NULL",
                sql_quote(&format!("\"{t}\"")),
                cols_now.iter().map(|x| sql_quote(x)).collect::<Vec<_>>().join(", "),
                sql_quote(&format!("\"{t}\""))
            ))
            .load(c)?;
            for sq in seqs {
                diesel::sql_query(format!(
                    "SELECT setval({}, (SELECT coalesce(max(\"{}\")::bigint, 1) FROM \"{t}\"))",
                    sql_quote(&sq.b),
                    sq.a
                ))
                .execute(c)?;
            }
        }
        Ok::<(), anyhow::Error>(())
    })
}

async fn db_snapshot(pool: &Pool, tmp: &Path) -> Result<(String, Vec<u8>)> {
    if pool.sqlite {
        let _ = std::fs::remove_file(tmp);
        pool.exec(&format!(
            "VACUUM INTO {}",
            sql_quote(&tmp.to_string_lossy())
        ))
        .await
        .context("database snapshot")?;
        let b = std::fs::read(tmp);
        let _ = std::fs::remove_file(tmp);
        Ok((F_SQLITE.into(), b?))
    } else {
        let b = pool
            .run(|c| match c {
                Db::Pg(c) => pg_dump(c),
                Db::Sqlite(_) => unreachable!(),
            })
            .await
            .context("database snapshot")?;
        Ok((F_PG.into(), b))
    }
}

fn acme_dir(cfg: &Config) -> PathBuf {
    cfg.acme
        .as_ref()
        .map(|a| a.cache_dir.clone())
        .unwrap_or_else(|| cfg.data_dir.join("acme"))
}

async fn collect(cfg: &Config, pool: &Pool, tmp_dir: &Path) -> Result<(Files, String)> {
    let mut f = Files::new();
    let (dbname, db) = db_snapshot(pool, &tmp_dir.join(".adminsvr-snapshot.sqlite")).await?;
    let dbkind = if dbname == F_SQLITE {
        "sqlite"
    } else {
        "postgres"
    };
    f.insert(dbname, db);
    f.insert(F_ROOT.into(), std::fs::read(cfg.data_dir.join(F_ROOT))?);
    for t in TLS_FILES {
        f.insert(t.into(), std::fs::read(cfg.data_dir.join(t))?);
    }
    f.insert(
        F_RECOVERY.into(),
        std::fs::read(recovery_path(&cfg.data_dir))?,
    );
    f.insert(F_CONFIG.into(), config_env());
    read_dir_files(&acme_dir(cfg), D_ACME, &mut f)?;
    for p in [&cfg.push.fcm_service_account, &cfg.push.apns_key]
        .into_iter()
        .flatten()
    {
        if p.is_file() {
            let n = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            f.insert(format!("{D_PUSH}{n}"), std::fs::read(p)?);
        }
    }
    for k in f.keys() {
        ensure!(allowed(k), "internal: {k} is not an allowed backup file");
    }
    Ok((f, dbkind.into()))
}

pub fn backup_dir(cfg: &Config, out: Option<&Path>) -> PathBuf {
    out.map(Path::to_path_buf)
        .unwrap_or_else(|| cfg.backup.dir.clone())
}

/// Make an archive. Needs `backup init`.
pub async fn create(
    cfg: &Config,
    pool: &Pool,
    kind: &str,
    out: Option<&Path>,
) -> Result<BackupInfo> {
    let rec = load_recovery(&cfg.data_dir)?.ok_or_else(|| {
        anyhow!("backups are not set up; run `itcmdr-adminsvr backup init` first")
    })?;
    rec.check()?;
    let root = root_key(&cfg.data_dir)?;
    ensure!(
        hex::encode(root.verifying_key().as_bytes()) == rec.root,
        "the root key changed since `backup init`; run `backup init --replace`"
    );
    let dir = backup_dir(cfg, out);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let (files, db) = collect(cfg, pool, &dir).await?;
    let created_ms = crate::now_ms();
    let manifest = Manifest {
        format: FORMAT.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        schema_version: store::SCHEMA_VERSION,
        db,
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
    let plain = Zeroizing::new(tar_files(&files)?);
    drop(files);
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
        content_key: itcmdr_keys::seal(&rec.x()?, ck.as_ref())?,
        recovery: rec,
        nonce: hex::encode(nonce),
        ciphertext_sha256: sha256_hex(&ct),
        sig: String::new(),
    };
    h.sig = hex::encode(root.sign(&h.signed_message()?).to_bytes());
    let hj = serde_json::to_vec(&h)?;
    let name = archive_name(created_ms, kind);
    let path = dir.join(&name);
    let tmp = dir.join(format!(".{name}.tmp"));
    {
        let mut f =
            std::fs::File::create(&tmp).with_context(|| format!("writing {}", tmp.display()))?;
        f.write_all(MAGIC)?;
        f.write_all(&(hj.len() as u32).to_le_bytes())?;
        f.write_all(&hj)?;
        f.write_all(&ct)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &path)?;
    let size = std::fs::metadata(&path)?.len();
    let _ = write_private(
        &cfg.data_dir.join("backup/last-backup.json"),
        &serde_json::to_vec(&serde_json::json!({"ms": created_ms, "name": name, "kind": kind}))?,
    );
    Ok(BackupInfo {
        name,
        path,
        size,
        manifest,
    })
}

// ---------------------------------------------------------------- verify / open

fn read_archive(path: &Path) -> Result<(Header, Vec<u8>)> {
    let b = std::fs::read(path)
        .with_context(|| format!("backup {} is missing or unreadable", path.display()))?;
    ensure!(
        b.len() > 12 && &b[..8] == MAGIC,
        "{} is not an adminsvr backup (bad magic or empty)",
        path.display()
    );
    let n = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
    ensure!(b.len() >= 12 + n, "backup is truncated (header)");
    let h: Header = serde_json::from_slice(&b[12..12 + n]).context("backup header is damaged")?;
    Ok((h, b[12 + n..].to_vec()))
}

fn verify_parsed(h: &Header, ct: &[u8], pinned: Option<&Recovery>) -> Result<()> {
    ensure!(
        h.manifest.format == FORMAT,
        "unknown backup format {}",
        h.manifest.format
    );
    ensure!(
        sha256_hex(ct) == h.ciphertext_sha256,
        "backup contents are damaged or truncated (hash mismatch)"
    );
    h.recovery.check()?;
    let root = itcmdr_keys::parse_public_key(&h.recovery.root)?;
    let sig: [u8; 64] = hex::decode(&h.sig)?
        .try_into()
        .map_err(|_| anyhow!("bad backup signature"))?;
    root.verify(&h.signed_message()?, &Signature::from_bytes(&sig))
        .map_err(|_| anyhow!("backup header signature is not valid (damaged or altered)"))?;
    if let Some(p) = pinned {
        ensure!(
            p.ed25519 == h.recovery.ed25519,
            "this backup was made for another recovery key ({}; this adminsvr uses {})",
            h.recovery.fingerprint(),
            p.fingerprint()
        );
    }
    Ok(())
}

/// Signature, hash and recovery key (pinned to this adminsvr's when it has one). No words needed.
pub fn verify(path: &Path, pinned: Option<&Recovery>) -> Result<Header> {
    let (h, ct) = read_archive(path)?;
    verify_parsed(&h, &ct, pinned)?;
    Ok(h)
}

/// Decrypt with the words and check every file. Nothing is written.
pub fn open(path: &Path, words: &str, pinned: Option<&Recovery>) -> Result<(Header, Files)> {
    let (h, ct) = read_archive(path)?;
    verify_parsed(&h, &ct, pinned)?;
    let m = itcmdr_keys::Master::from_words(words)?;
    ensure!(
        hex::encode(m.public().ed25519.as_bytes()) == h.recovery.ed25519,
        "these recovery words do not open this backup (wrong words, or another adminsvr's backup)"
    );
    let ck = itcmdr_keys::unseal(&m, &h.content_key)?;
    let nonce: [u8; 24] = hex::decode(&h.nonce)?
        .try_into()
        .map_err(|_| anyhow!("bad nonce"))?;
    let plain = Zeroizing::new(
        XChaCha20Poly1305::new_from_slice(&ck)
            .map_err(|_| anyhow!("bad content key"))?
            .decrypt(XNonce::from_slice(&nonce), Payload { msg: &ct, aad: AAD })
            .map_err(|_| anyhow!("backup does not decrypt (damaged)"))?,
    );
    let files = untar_files(&plain)?;
    ensure!(
        files.len() == h.manifest.files.len(),
        "backup holds {} files, the manifest lists {}",
        files.len(),
        h.manifest.files.len()
    );
    for e in &h.manifest.files {
        let b = files
            .get(&e.path)
            .ok_or_else(|| anyhow!("{} is missing from the backup", e.path))?;
        ensure!(
            b.len() as u64 == e.size && sha256_hex(b) == e.sha256,
            "{} is damaged (size or sha256)",
            e.path
        );
    }
    Ok((h, files))
}

pub fn list(dir: &Path) -> Result<Vec<BackupInfo>> {
    let mut v = Vec::new();
    if !dir.is_dir() {
        return Ok(v);
    }
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        if !is_archive(&name) {
            continue;
        }
        let path = e.path();
        let size = e.metadata()?.len();
        if let Ok((h, _)) = read_archive(&path) {
            v.push(BackupInfo {
                name,
                path,
                size,
                manifest: h.manifest,
            });
        } else {
            v.push(BackupInfo {
                name,
                path,
                size,
                manifest: Manifest {
                    format: "damaged".into(),
                    version: String::new(),
                    schema_version: 0,
                    db: String::new(),
                    created_ms: 0,
                    kind: String::new(),
                    files: vec![],
                },
            });
        }
    }
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

/// Keep the newest `keep` nightly archives; manual ones are never pruned.
pub fn prune(dir: &Path, keep: usize) -> Result<Vec<String>> {
    let mut n: Vec<String> = list(dir)?
        .into_iter()
        .map(|b| b.name)
        .filter(|n| n.ends_with(&format!("-nightly{EXT}")))
        .collect();
    n.sort();
    let drop_n = n.len().saturating_sub(keep.max(1));
    let gone: Vec<String> = n.into_iter().take(drop_n).collect();
    for g in &gone {
        std::fs::remove_file(dir.join(g))?;
    }
    Ok(gone)
}

// ---------------------------------------------------------------- drill

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrillReport {
    pub ms: i64,
    pub archive: String,
    pub deep: bool,
    pub ok: bool,
    pub error: String,
    pub files: usize,
    pub users: i64,
}

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

/// Restore-check a decrypted archive into a throwaway folder (never the live install).
async fn deep_check(h: &Header, files: &Files, scratch: &Path) -> Result<i64> {
    let root = itcmdr_keys::key_from_seed(&hex::decode(
        String::from_utf8_lossy(files.get(F_ROOT).ok_or_else(|| anyhow!("no root key"))?).trim(),
    )?)?;
    ensure!(
        hex::encode(root.verifying_key().as_bytes()) == h.recovery.root,
        "the archived root key does not match the header"
    );
    for t in TLS_FILES {
        let pem =
            String::from_utf8_lossy(files.get(t).ok_or_else(|| anyhow!("no {t}"))?).to_string();
        if t.ends_with(".pem") {
            itcmdr_agentlink::tls::pem_fingerprint(&pem)
                .with_context(|| format!("{t} is not a certificate"))?;
        }
    }
    ensure!(
        h.manifest.schema_version <= store::SCHEMA_VERSION,
        "backup schema {} is newer than this adminsvr ({}); upgrade first",
        h.manifest.schema_version,
        store::SCHEMA_VERSION
    );
    if let Some(db) = files.get(F_SQLITE) {
        let p = scratch.join("drill.sqlite");
        std::fs::write(&p, db)?;
        let pool = store::connect(&format!("sqlite://{}", p.display())).await?;
        let ok: Vec<TextRow> = pool
            .run(|c| {
                use diesel::RunQueryDsl;
                Ok(
                    diesel::sql_query("SELECT integrity_check AS t FROM pragma_integrity_check")
                        .load(c)?,
                )
            })
            .await?;
        ensure!(
            ok.len() == 1 && ok[0].t == "ok",
            "database integrity check failed"
        );
        store::migrate(&pool)
            .await
            .context("database does not migrate")?;
        let n: Vec<CountRow> = pool
            .run(|c| {
                use diesel::RunQueryDsl;
                Ok(diesel::sql_query("SELECT count(*) AS n FROM users").load(c)?)
            })
            .await?;
        return Ok(n.first().map(|x| x.n).unwrap_or(0));
    }
    let d: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(
        files
            .get(F_PG)
            .ok_or_else(|| anyhow!("no database in the backup"))?,
    )
    .context("database dump is damaged")?;
    Ok(d.get("users")
        .and_then(|u| u.as_array())
        .map(|a| a.len() as i64)
        .unwrap_or(0))
}

/// Copy the newest archive to a temp folder, check it (and with words: decrypt and restore it
/// into a throwaway database), delete the copy. The result is kept in backup/last-drill.json and
/// audited (`backup.drill`, archive name and ok/failed only).
pub async fn drill(
    cfg: &Config,
    pool: Option<&Pool>,
    dir: Option<&Path>,
    words: Option<&str>,
) -> Result<DrillReport> {
    let dir = backup_dir(cfg, dir);
    let newest = list(&dir)?.into_iter().last();
    let pinned = load_recovery(&cfg.data_dir)?;
    let mut rep = DrillReport {
        ms: crate::now_ms(),
        archive: newest.as_ref().map(|b| b.name.clone()).unwrap_or_default(),
        deep: words.is_some(),
        ok: false,
        error: String::new(),
        files: 0,
        users: 0,
    };
    let res: Result<()> = async {
        let b = newest.ok_or_else(|| anyhow!("no backups in {}", dir.display()))?;
        let scratch = tempfile::tempdir()?;
        let copy = scratch.path().join(&b.name);
        std::fs::copy(&b.path, &copy).context("copying the newest backup")?;
        match words {
            None => {
                let h = verify(&copy, pinned.as_ref())?;
                ensure!(
                    h.manifest.schema_version <= store::SCHEMA_VERSION,
                    "backup schema is newer than this adminsvr"
                );
                rep.files = h.manifest.files.len();
            }
            Some(w) => {
                let (h, files) = open(&copy, w, pinned.as_ref())?;
                rep.files = files.len();
                rep.users = deep_check(&h, &files, scratch.path()).await?;
            }
        }
        Ok(())
    }
    .await;
    match res {
        Ok(()) => rep.ok = true,
        Err(e) => rep.error = format!("{e:#}"),
    }
    let _ = write_private(
        &cfg.data_dir.join("backup/last-drill.json"),
        &serde_json::to_vec_pretty(&rep)?,
    );
    if let Some(p) = pool {
        let _ = crate::audit::append(
            p,
            &crate::audit::Entry {
                actor: "console".into(),
                action: "backup.drill".into(),
                target: rep.archive.clone(),
                payload: format!("deep={} ok={}", rep.deep, rep.ok).into_bytes(),
                ..Default::default()
            },
        )
        .await;
    }
    Ok(rep)
}

pub fn last_drill(data_dir: &Path) -> Option<DrillReport> {
    serde_json::from_slice(&std::fs::read(data_dir.join("backup/last-drill.json")).ok()?).ok()
}

// ---------------------------------------------------------------- restore

#[derive(Clone, Debug, Default)]
pub struct RestoreReport {
    pub archive: String,
    pub safety_copy: Option<PathBuf>,
    pub written: Vec<String>,
    pub review: Vec<PathBuf>,
    pub dry_run: bool,
}

/// True when something answers on the admin API port (the service is running).
fn api_running(cfg: &Config) -> bool {
    use std::net::ToSocketAddrs;
    let a = cfg
        .api_addr
        .replace("0.0.0.0", "127.0.0.1")
        .replace("[::]", "[::1]");
    a.to_socket_addrs()
        .ok()
        .and_then(|mut i| i.next())
        .map(|s| {
            std::net::TcpStream::connect_timeout(&s, std::time::Duration::from_millis(500)).is_ok()
        })
        .unwrap_or(false)
}

fn sqlite_path(cfg: &Config) -> Option<PathBuf> {
    match store::parse_url(&cfg.db_url).ok()? {
        store::Kind::Sqlite(p) => Some(PathBuf::from(p)),
        store::Kind::Postgres(_) => None,
    }
}

/// Where each restored file goes (None = review copy under <data_dir>/restored/).
fn target(cfg: &Config, path: &str) -> Option<PathBuf> {
    if let Some(rest) = path.strip_prefix(D_ACME) {
        return Some(acme_dir(cfg).join(rest));
    }
    if path == F_CONFIG || path.starts_with(D_PUSH) || path == F_PG || path == F_SQLITE {
        return None;
    }
    Some(cfg.data_dir.join(path))
}

fn copy_into(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for e in std::fs::read_dir(src)? {
            let e = e?;
            copy_into(&e.path(), &dst.join(e.file_name()))?;
        }
    } else if src.is_file() {
        if let Some(d) = dst.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

const STATE_DIRS: [&str; 3] = ["keys", "tls", "backup"];

/// Restore adminsvr from an archive. The service must be stopped. Steps: open and check
/// everything (nothing changes if that fails); copy the current state to
/// <data_dir>/pre-restore-<ts>/; write keys, TLS, ACME, recovery and the database; put the
/// archived config and push keys under <data_dir>/restored/ for review (the active config is
/// never overwritten); migrate. If writing fails, the safety copy is put back.
pub async fn restore(
    cfg: &Config,
    archive: &Path,
    words: &str,
    dry_run: bool,
) -> Result<RestoreReport> {
    let pinned = load_recovery(&cfg.data_dir)?;
    // A fresh install (no recovery key yet) accepts any backup the words open; an install that
    // has one only accepts its own unless --replace-recovery is wanted (then remove it first).
    let (h, files) = open(archive, words, pinned.as_ref())?;
    ensure!(
        h.manifest.schema_version <= store::SCHEMA_VERSION,
        "backup schema {} is newer than this adminsvr ({}); install the newer adminsvr first",
        h.manifest.schema_version,
        store::SCHEMA_VERSION
    );
    let sqlite = sqlite_path(cfg);
    match (&sqlite, h.manifest.db.as_str()) {
        (Some(_), "sqlite") | (None, "postgres") => {}
        (Some(_), _) => bail!("this backup is from a Postgres adminsvr; point --db-url at a Postgres database to restore it"),
        (None, _) => bail!("this backup is from a SQLite adminsvr; restore it with a SQLite --db-url"),
    }
    let mut rep = RestoreReport {
        archive: archive.display().to_string(),
        dry_run,
        ..Default::default()
    };
    for p in files.keys() {
        match target(cfg, p) {
            Some(t) => rep.written.push(t.display().to_string()),
            None if p.starts_with("db/") => {
                rep.written.push(format!("database ({})", h.manifest.db))
            }
            None => rep.review.push(cfg.data_dir.join("restored").join(p)),
        }
    }
    if dry_run {
        return Ok(rep);
    }
    ensure!(
        !api_running(cfg),
        "adminsvr is running (something answers on {}); stop the itcmdr-adminsvr service first",
        cfg.api_addr
    );
    std::fs::create_dir_all(&cfg.data_dir)?;
    // Safety copy of the current state.
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let safety = cfg.data_dir.join(format!("pre-restore-{ts}"));
    std::fs::create_dir_all(&safety)?;
    for d in STATE_DIRS {
        copy_into(&cfg.data_dir.join(d), &safety.join(d))?;
    }
    copy_into(&acme_dir(cfg), &safety.join("acme"))?;
    // SQLite: no connection is open while the file is replaced.
    let pg = if sqlite.is_none() {
        Some(store::connect(&cfg.db_url).await?)
    } else {
        None
    };
    if let Some(p) = &sqlite {
        for sfx in ["", "-wal", "-shm"] {
            let f = PathBuf::from(format!("{}{sfx}", p.display()));
            copy_into(&f, &safety.join(format!("db/adminsvr.sqlite{sfx}")))?;
        }
    } else if let Some(pool) = &pg {
        store::migrate(pool).await?;
        let cur = pool
            .run(|c| match c {
                Db::Pg(c) => pg_dump(c),
                Db::Sqlite(_) => unreachable!(),
            })
            .await?;
        write_private(&safety.join(F_PG), &cur)?;
    }
    rep.safety_copy = Some(safety.clone());

    let res: Result<()> = async {
        for (p, b) in &files {
            if let Some(t) = target(cfg, p) {
                write_private(&t, b)?;
            } else if !p.starts_with("db/") {
                write_private(&cfg.data_dir.join("restored").join(p), b)?;
            }
        }
        if let Some(p) = &sqlite {
            for sfx in ["-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{sfx}", p.display()));
            }
            write_private(p, files.get(F_SQLITE).unwrap())?;
        } else if let Some(pool) = &pg {
            let dump = files.get(F_PG).unwrap().clone();
            pool.run(move |c| match c {
                Db::Pg(c) => pg_load(c, &dump),
                Db::Sqlite(_) => unreachable!(),
            })
            .await?;
        }
        let pool = store::connect(&cfg.db_url).await?;
        store::migrate(&pool).await?;
        crate::material::load(cfg)?;
        crate::audit::append(
            &pool,
            &crate::audit::Entry {
                actor: "console".into(),
                action: "backup.restore".into(),
                target: archive
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                payload: h.manifest.created_ms.to_string().into_bytes(),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
    .await;
    if let Err(e) = res {
        // Put the safety copy back.
        for d in STATE_DIRS {
            let _ = std::fs::remove_dir_all(cfg.data_dir.join(d));
            let _ = copy_into(&safety.join(d), &cfg.data_dir.join(d));
        }
        let _ = std::fs::remove_dir_all(acme_dir(cfg));
        let _ = copy_into(&safety.join("acme"), &acme_dir(cfg));
        if let Some(p) = &sqlite {
            for sfx in ["", "-wal", "-shm"] {
                let f = PathBuf::from(format!("{}{sfx}", p.display()));
                let _ = std::fs::remove_file(&f);
                let _ = copy_into(&safety.join(format!("db/adminsvr.sqlite{sfx}")), &f);
            }
        } else if let Ok(cur) = std::fs::read(safety.join(F_PG)) {
            if let Ok(pool) = store::connect(&cfg.db_url).await {
                let _ = pool
                    .run(move |c| match c {
                        Db::Pg(c) => pg_load(c, &cur),
                        Db::Sqlite(_) => Ok(()),
                    })
                    .await;
            }
        }
        return Err(e.context(format!(
            "restore failed and was rolled back (the state before it is also in {})",
            safety.display()
        )));
    }
    Ok(rep)
}

// ---------------------------------------------------------------- schedule

/// Nightly backups inside `serve` (at `backup.hour`, local time, once a day), then prune.
/// Does nothing until `backup init` has been run.
pub async fn nightly(cfg: Config, pool: Pool, mut stop: tokio::sync::watch::Receiver<bool>) {
    let BackupConfig { hour, keep, .. } = cfg.backup.clone();
    let Some(hour) = hour else { return };
    let mut last_day = String::new();
    loop {
        tokio::select! {
            _ = stop.changed() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
        }
        use chrono::Timelike;
        let now = chrono::Local::now();
        let day = now.format("%Y%m%d").to_string();
        if now.hour() != hour || day == last_day {
            continue;
        }
        if !recovery_path(&cfg.data_dir).exists() {
            continue;
        }
        let already = list(&cfg.backup.dir)
            .map(|v| {
                v.iter().any(|b| {
                    b.name.starts_with(&format!("itcmdr-adminsvr-{day}"))
                        && b.name.ends_with(&format!("-nightly{EXT}"))
                })
            })
            .unwrap_or(false);
        last_day = day;
        if already {
            continue;
        }
        match create(&cfg, &pool, "nightly", None).await {
            Ok(b) => {
                tracing::info!(archive = %b.name, size = b.size, "adminsvr backup");
                if let Err(e) = prune(&cfg.backup.dir, keep) {
                    tracing::warn!(error = %e, "adminsvr backup prune");
                }
            }
            Err(e) => tracing::error!(error = %format!("{e:#}"), "adminsvr nightly backup failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allow_list() {
        for ok in [
            F_SQLITE,
            F_PG,
            F_ROOT,
            "tls/ca.key",
            "acme/account.json",
            "push/key.p8",
            F_CONFIG,
            F_RECOVERY,
        ] {
            assert!(allowed(ok), "{ok}");
        }
        for bad in [
            "itcmdr.database",
            "keys/db.key.escrow",
            "sites/x.itcb",
            "acme/",
            "acme/../keys/x",
            "/etc/passwd",
            "backups/itcmdr-1.itcb",
            "tls\\ca.key",
            "server.json",
        ] {
            assert!(!allowed(bad), "{bad}");
        }
    }
    #[test]
    fn names() {
        let n = archive_name(1_700_000_000_123, "nightly");
        assert!(is_archive(&n) && n.ends_with("123-nightly.itcab"), "{n}");
        assert!(!is_archive("itcmdr-20260101-000000000-nightly.itcb"));
    }
    #[test]
    fn untar_refuses_foreign_files() {
        let mut f = Files::new();
        f.insert("site-backups/office.itcb".into(), b"x".to_vec());
        let mut b = tar::Builder::new(Vec::new());
        for (p, d) in &f {
            let mut h = tar::Header::new_gnu();
            h.set_size(d.len() as u64);
            h.set_cksum();
            b.append_data(&mut h, p, d.as_slice()).unwrap();
        }
        let e = untar_files(&b.into_inner().unwrap())
            .unwrap_err()
            .to_string();
        assert!(e.contains("unexpected file"), "{e}");
    }
}
