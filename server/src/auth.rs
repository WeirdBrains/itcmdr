//! Master init, key registration/revocation, signed-request verification and roles.
//!
//! Signed request metadata (all required):
//! - `x-itcmdr-key`: key fingerprint (sha256 of the Ed25519 public key, hex)
//! - `x-itcmdr-ts`: unix millis, must be within +/- `MAX_SKEW_MS` of the server clock
//! - `x-itcmdr-nonce`: 16-128 chars of [A-Za-z0-9_-], never reused (replay protection)
//! - `x-itcmdr-sig`: hex Ed25519 signature over `keys::request_message(method, ts, nonce, body)`,
//!   where `method` is the gRPC path (e.g. `/itcmdr.Itcmdr/TriggerRuns`) and `body` is the protobuf
//!   encoding of the request message.
use crate::db::DbPool;
use crate::keys::{self, Certificate, Master, MasterPublic};
use crate::storage::Layout;
use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use parking_lot::Mutex;
use sqlx::Row;
use std::collections::HashMap;
use tonic::Status;
use zeroize::Zeroizing;

pub const MAX_SKEW_MS: i64 = 5 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Client = 0,
    User = 1,
    Admin = 2,
    Owner = 3,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "owner" => Some(Role::Owner),
            "admin" => Some(Role::Admin),
            "user" => Some(Role::User),
            "client" => Some(Role::Client),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Admin => "admin",
            Role::User => "user",
            Role::Client => "client",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Principal {
    pub name: String,
    pub role: Role,
    pub fingerprint: String,
    pub kind: String,
}

impl Principal {
    pub fn actor(&self) -> String {
        format!(
            "{} ({})",
            self.name,
            &self.fingerprint[..12.min(self.fingerprint.len())]
        )
    }
}

pub async fn master_public(pool: &DbPool) -> Result<Option<MasterPublic>> {
    let get = |k: &'static str| async move {
        sqlx::query_scalar::<_, String>("SELECT value FROM server_meta WHERE key = ?")
            .bind(k)
            .fetch_optional(pool)
            .await
    };
    let (Some(ed), Some(x)) = (
        get("master_ed25519_pub").await?,
        get("master_x25519_pub").await?,
    ) else {
        return Ok(None);
    };
    let xb: [u8; 32] = hex::decode(x)?
        .try_into()
        .map_err(|_| anyhow!("bad master_x25519_pub"))?;
    Ok(Some(MasterPublic {
        ed25519: keys::parse_public_key(&ed)?,
        x25519: x25519_dalek::PublicKey::from(xb),
    }))
}

pub struct InitOutput {
    /// Shown ONCE to the operator, never stored.
    pub words: Zeroizing<String>,
    pub master_fingerprint: String,
    pub root_fingerprint: String,
}

/// First-run setup: generate the 24 words, store the master PUBLIC keys, create the root client
/// (role owner) with its private key in `keys/root`, and escrow `keys/db.key` under the master.
/// Refuses when already initialized.
pub async fn init(pool: &DbPool, layout: &Layout) -> Result<InitOutput> {
    if master_public(pool).await?.is_some() {
        bail!("this server is already initialized (master key present); use the recovery words instead");
    }
    let words = Zeroizing::new(keys::generate_words());
    let master = Master::from_words(&words)?;
    let mp = master.public();
    let root = keys::new_keypair();
    let cert = keys::certify(
        &master.signing,
        "client",
        "root",
        "owner",
        &root.verifying_key(),
    );
    let mut tx = pool.begin().await?;
    for (k, v) in [
        ("master_ed25519_pub", hex::encode(mp.ed25519.as_bytes())),
        ("master_x25519_pub", hex::encode(mp.x25519.as_bytes())),
        ("initialized_at", chrono::Utc::now().to_rfc3339()),
    ] {
        sqlx::query("INSERT INTO server_meta(key, value) VALUES (?, ?)")
            .bind(k)
            .bind(v)
            .execute(&mut *tx)
            .await?;
    }
    insert_key(
        &mut tx,
        "client",
        "root",
        Role::Owner,
        None,
        None,
        &root,
        &cert,
    )
    .await?;
    layout.ensure()?;
    crate::storage::write_secret(&layout.keys().join("root"), root.as_bytes())?;
    escrow_db_key(layout, &mp)?;
    tx.commit().await?;
    Ok(InitOutput {
        words,
        master_fingerprint: keys::fingerprint(&mp.ed25519),
        root_fingerprint: cert.fingerprint,
    })
}

#[allow(clippy::too_many_arguments)]
async fn insert_key(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    kind: &str,
    name: &str,
    role: Role,
    email: Option<&str>,
    phone: Option<&str>,
    key: &SigningKey,
    cert: &Certificate,
) -> Result<String> {
    insert_public(
        tx,
        kind,
        name,
        role,
        email,
        phone,
        &hex::encode(key.verifying_key().as_bytes()),
        cert,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn insert_public(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    kind: &str,
    name: &str,
    role: Role,
    email: Option<&str>,
    phone: Option<&str>,
    public_hex: &str,
    cert: &Certificate,
) -> Result<String> {
    let role_id: String = sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
        .bind(role.as_str())
        .fetch_one(&mut **tx)
        .await
        .context("role missing")?;
    let id = uuid::Uuid::new_v4().to_string();
    let cert_json = serde_json::to_string(cert)?;
    if kind == "user" {
        sqlx::query("INSERT INTO users(id,name,email,phone,role_id,public_key,fingerprint,certificate) VALUES (?,?,?,?,?,?,?,?)")
            .bind(&id)
            .bind(name)
            .bind(email)
            .bind(phone)
            .bind(&role_id)
            .bind(public_hex)
            .bind(&cert.fingerprint)
            .bind(&cert_json)
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query("INSERT INTO clients(id,name,role_id,public_key,fingerprint,certificate) VALUES (?,?,?,?,?,?)")
            .bind(&id)
            .bind(name)
            .bind(&role_id)
            .bind(public_hex)
            .bind(&cert.fingerprint)
            .bind(&cert_json)
            .execute(&mut **tx)
            .await?;
    }
    Ok(id)
}

pub struct NewKey<'a> {
    pub kind: &'a str, // "client" | "user"
    pub name: &'a str,
    pub role: Role,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub public_key_hex: &'a str,
    pub certificate: &'a Certificate,
}

/// Register a key certified by the master (certificate made offline with the words).
/// Returns the new record's UUID.
pub async fn register(pool: &DbPool, k: NewKey<'_>) -> Result<String> {
    let master = master_public(pool)
        .await?
        .ok_or_else(|| anyhow!("server is not initialized"))?;
    if k.kind != "client" && k.kind != "user" {
        bail!("kind must be client or user");
    }
    let pk = keys::parse_public_key(k.public_key_hex)?;
    keys::verify_certificate(&master.ed25519, k.certificate)?;
    let c = k.certificate;
    if c.fingerprint != keys::fingerprint(&pk)
        || c.kind != k.kind
        || c.role != k.role.as_str()
        || c.name != k.name
    {
        bail!("certificate does not match the key, kind, role or name being registered");
    }
    let mut tx = pool.begin().await?;
    let id = insert_public(
        &mut tx,
        k.kind,
        k.name,
        k.role,
        k.email,
        k.phone,
        k.public_key_hex,
        c,
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Revoke a key by fingerprint. Returns false when unknown or already revoked.
pub async fn revoke(pool: &DbPool, fingerprint: &str) -> Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut n = 0;
    for t in ["users", "clients"] {
        n += sqlx::query(&format!(
            "UPDATE {t} SET revoked_at = ? WHERE fingerprint = ? AND revoked_at IS NULL"
        ))
        .bind(&now)
        .bind(fingerprint)
        .execute(pool)
        .await?
        .rows_affected();
    }
    Ok(n > 0)
}

pub fn escrow_path(layout: &Layout) -> std::path::PathBuf {
    layout.keys().join("db.key.escrow")
}

/// Write `keys/db.key.escrow`: the current db.key sealed for the master X25519 key.
pub fn escrow_db_key(layout: &Layout, master: &MasterPublic) -> Result<()> {
    let key = Zeroizing::new(crate::storage::load_or_create_db_key(layout)?);
    let e = keys::seal(&master.x25519, key.as_ref())?;
    let p = escrow_path(layout);
    std::fs::write(&p, serde_json::to_vec_pretty(&e)?)
        .with_context(|| format!("writing {}", p.display()))?;
    Ok(())
}

/// Replacement machine: open the escrow with the words and write `keys/db.key` protected for this
/// machine (DPAPI / 0600). Refuses to overwrite an existing db.key unless `force`.
pub fn recover_db_key(layout: &Layout, words: &str, force: bool) -> Result<()> {
    let p = escrow_path(layout);
    let e: keys::Escrow = serde_json::from_slice(
        &std::fs::read(&p).with_context(|| format!("reading {}", p.display()))?,
    )?;
    let master = Master::from_words(words)?;
    let key = keys::unseal(&master, &e)?;
    if key.len() != 32 {
        bail!("escrowed key has the wrong length");
    }
    let dst = layout.keys().join("db.key");
    if dst.exists() && !force {
        bail!(
            "{} already exists; pass --force to replace it",
            dst.display()
        );
    }
    layout.ensure()?;
    crate::storage::write_secret(&dst, &key)?;
    Ok(())
}

/// Load the root client's private key from `keys/root`.
pub fn load_root_key(layout: &Layout) -> Result<SigningKey> {
    let b: [u8; 32] = crate::storage::read_secret(&layout.keys().join("root"))?
        .try_into()
        .map_err(|_| anyhow!("keys/root has the wrong length"))?;
    Ok(SigningKey::from_bytes(&b))
}

/// Metadata a client attaches to a signed call.
pub fn sign_request(key: &SigningKey, method: &str, body: &[u8]) -> Vec<(&'static str, String)> {
    let ts = chrono::Utc::now().timestamp_millis();
    let mut n = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut n);
    let nonce = hex::encode(n);
    let sig = key.sign(&keys::request_message(method, ts, &nonce, body));
    vec![
        ("x-itcmdr-key", keys::fingerprint(&key.verifying_key())),
        ("x-itcmdr-ts", ts.to_string()),
        ("x-itcmdr-nonce", nonce),
        ("x-itcmdr-sig", hex::encode(sig.to_bytes())),
    ]
}

/// Seen nonces (per key) until they fall out of the clock window.
#[derive(Default)]
pub struct NonceCache {
    seen: Mutex<HashMap<String, i64>>,
}

impl NonceCache {
    /// False when the (key, nonce) pair was already used.
    pub fn check_and_insert(&self, fp: &str, nonce: &str, now_ms: i64) -> bool {
        let mut m = self.seen.lock();
        if m.len() > 10_000 {
            m.retain(|_, exp| *exp > now_ms);
        }
        let k = format!("{fp}:{nonce}");
        if m.get(&k).is_some_and(|exp| *exp > now_ms) {
            return false;
        }
        m.insert(k, now_ms + 2 * MAX_SKEW_MS);
        true
    }
}

fn meta_str<'a>(meta: &'a tonic::metadata::MetadataMap, k: &str) -> Result<&'a str, Status> {
    meta.get(k)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Status::unauthenticated(format!("missing {k}")))
}

pub fn has_signature(meta: &tonic::metadata::MetadataMap) -> bool {
    meta.contains_key("x-itcmdr-sig")
}

/// Verify a signed call and return who made it. Does not check the role.
pub async fn verify(
    pool: &DbPool,
    nonces: &NonceCache,
    meta: &tonic::metadata::MetadataMap,
    method: &str,
    body: &[u8],
    now_ms: i64,
) -> Result<Principal, Status> {
    let fp = meta_str(meta, "x-itcmdr-key")?;
    let ts: i64 = meta_str(meta, "x-itcmdr-ts")?
        .parse()
        .map_err(|_| Status::unauthenticated("bad x-itcmdr-ts"))?;
    let nonce = meta_str(meta, "x-itcmdr-nonce")?;
    let sig_hex = meta_str(meta, "x-itcmdr-sig")?;
    if (now_ms - ts).abs() > MAX_SKEW_MS {
        return Err(Status::unauthenticated(
            "request timestamp outside the allowed window (check the clock)",
        ));
    }
    if !(16..=128).contains(&nonce.len())
        || !nonce
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Status::unauthenticated("bad x-itcmdr-nonce"));
    }
    let master = master_public(pool)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| {
            Status::failed_precondition("server is not initialized (run itcmdr-server init)")
        })?;
    let row = sqlx::query(
        "SELECT 'user' AS kind, u.name, u.public_key, u.certificate, r.name AS role, u.revoked_at FROM users u JOIN roles r ON r.id = u.role_id WHERE u.fingerprint = ?1 \
         UNION ALL SELECT 'client', c.name, c.public_key, c.certificate, r.name, c.revoked_at FROM clients c JOIN roles r ON r.id = c.role_id WHERE c.fingerprint = ?1",
    )
    .bind(fp)
    .fetch_optional(pool)
    .await
    .map_err(|e| Status::internal(e.to_string()))?
    .ok_or_else(|| Status::unauthenticated("unknown key"))?;
    let revoked: Option<String> = row.get("revoked_at");
    if revoked.is_some() {
        return Err(Status::unauthenticated("key revoked"));
    }
    let deny = |m: &str| Status::unauthenticated(m.to_string());
    let pk = keys::parse_public_key(row.get::<String, _>("public_key").as_str())
        .map_err(|_| deny("stored key invalid"))?;
    let cert: Certificate = serde_json::from_str(
        row.get::<Option<String>, _>("certificate")
            .as_deref()
            .unwrap_or(""),
    )
    .map_err(|_| deny("key has no certificate"))?;
    let role_s: String = row.get("role");
    let kind: String = row.get("kind");
    let name: String = row.get("name");
    keys::verify_certificate(&master.ed25519, &cert).map_err(|_| deny("certificate invalid"))?;
    if cert.fingerprint != fp
        || keys::fingerprint(&pk) != fp
        || cert.role != role_s
        || cert.kind != kind
        || cert.name != name
    {
        return Err(deny("certificate does not match the key record"));
    }
    let sig: [u8; 64] = hex::decode(sig_hex)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| deny("bad x-itcmdr-sig"))?;
    pk.verify(
        &keys::request_message(method, ts, nonce, body),
        &Signature::from_bytes(&sig),
    )
    .map_err(|_| deny("bad signature"))?;
    if !nonces.check_and_insert(fp, nonce, now_ms) {
        return Err(deny("replayed request (nonce already used)"));
    }
    let role = Role::parse(&role_s).ok_or_else(|| deny("unknown role"))?;
    Ok(Principal {
        name,
        role,
        fingerprint: fp.to_string(),
        kind,
    })
}

pub fn require(p: &Principal, min: Role) -> Result<(), Status> {
    if p.role >= min {
        Ok(())
    } else {
        Err(Status::permission_denied(format!(
            "needs role {} (you have {})",
            min.as_str(),
            p.role.as_str()
        )))
    }
}

/// One registered key, for `itcmdr-cli keys list`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyInfo {
    pub kind: String,
    pub name: String,
    pub role: String,
    pub fingerprint: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

pub async fn list_keys(pool: &DbPool) -> Result<Vec<KeyInfo>> {
    let mut out = Vec::new();
    for (kind, t) in [("user", "users"), ("client", "clients")] {
        let rows = sqlx::query(&format!(
            "SELECT k.name, r.name AS role, k.fingerprint, k.created_at, k.revoked_at FROM {t} k \
             JOIN roles r ON r.id = k.role_id ORDER BY k.created_at"
        ))
        .fetch_all(pool)
        .await?;
        for r in rows {
            out.push(KeyInfo {
                kind: kind.into(),
                name: r.get("name"),
                role: r.get("role"),
                fingerprint: r.get("fingerprint"),
                created_at: r.get("created_at"),
                revoked_at: r.get("revoked_at"),
            });
        }
    }
    Ok(out)
}

/// The stored certificate of an active key.
pub async fn certificate_of(pool: &DbPool, fingerprint: &str) -> Result<Certificate> {
    for t in ["clients", "users"] {
        let c: Option<Option<String>> = sqlx::query_scalar(&format!(
            "SELECT certificate FROM {t} WHERE fingerprint = ? AND revoked_at IS NULL"
        ))
        .bind(fingerprint)
        .fetch_optional(pool)
        .await?;
        if let Some(Some(c)) = c {
            return Ok(serde_json::from_str(&c)?);
        }
    }
    bail!("no active key with fingerprint {fingerprint}")
}

/// Replace the root client's key (after a restore onto another machine, where `keys/root` from
/// the old machine can't be opened): new keypair certified by the words, old root keys revoked.
pub async fn rotate_root(pool: &DbPool, layout: &Layout, master: &Master) -> Result<String> {
    let mp = master_public(pool)
        .await?
        .ok_or_else(|| anyhow!("server is not initialized"))?;
    if mp.ed25519 != master.signing.verifying_key() {
        bail!("these recovery words do not belong to this server");
    }
    let root = keys::new_keypair();
    let cert = keys::certify(
        &master.signing,
        "client",
        "root",
        "owner",
        &root.verifying_key(),
    );
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE clients SET revoked_at = ? WHERE name = 'root' AND revoked_at IS NULL")
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    insert_key(
        &mut tx,
        "client",
        "root",
        Role::Owner,
        None,
        None,
        &root,
        &cert,
    )
    .await?;
    layout.ensure()?;
    crate::storage::write_secret(&layout.keys().join("root"), root.as_bytes())?;
    tx.commit().await?;
    Ok(cert.fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_ordered() {
        assert!(Role::Owner > Role::Admin && Role::Admin > Role::User && Role::User > Role::Client);
        for r in ["owner", "admin", "user", "client"] {
            assert_eq!(Role::parse(r).unwrap().as_str(), r);
        }
        assert!(Role::parse("root").is_none());
    }

    #[test]
    fn nonce_cache_blocks_replays() {
        let c = NonceCache::default();
        assert!(c.check_and_insert("fp", "n1", 0));
        assert!(!c.check_and_insert("fp", "n1", 1000));
        assert!(c.check_and_insert("other", "n1", 1000), "per key");
        assert!(
            c.check_and_insert("fp", "n1", 3 * MAX_SKEW_MS),
            "expired entries can be reused (the ts check rejects them anyway)"
        );
    }
}
