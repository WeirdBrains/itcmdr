//! On-disk layout and encryption at rest.
//!
//! Data dir precedence: `--data-dir` flag > `ITCMDR_DATA_DIR` env > `data_dir` in server.json >
//! `%ProgramData%\itcmdr` (Windows) / `~/.itcmdr` (elsewhere).
//!
//! The SQLCipher key is 32 random bytes in `keys/db.key`: DPAPI-protected (LocalMachine scope) on
//! Windows; on other systems a hex file with mode 0600 in a 0700 folder (file-permission protection
//! only, so use full-disk encryption on Linux hosts).
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub const DB_FILE: &str = "itcmdr.database";
/// Pre-0.6 plaintext database name; migrated automatically.
pub const LEGACY_DB_FILE: &str = "itcmdr.sqlite";

pub fn default_data_dir() -> PathBuf {
    if cfg!(windows) {
        let pd = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        pd.join("itcmdr")
    } else {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(".itcmdr")
    }
}

/// Resolve the data dir from (flag, env, config) in that order.
pub fn resolve_data_dir(flag: Option<&Path>, env: Option<&str>, config: Option<&Path>) -> PathBuf {
    if let Some(f) = flag {
        return f.to_path_buf();
    }
    if let Some(e) = env.map(str::trim).filter(|e| !e.is_empty()) {
        return PathBuf::from(e);
    }
    if let Some(c) = config {
        return c.to_path_buf();
    }
    default_data_dir()
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn service_plugins(&self) -> PathBuf {
        self.root.join("service_plugins")
    }
    pub fn quarantine(&self) -> PathBuf {
        self.root.join("quarantine")
    }
    pub fn keys(&self) -> PathBuf {
        self.root.join("keys")
    }
    pub fn plugin_logs(&self) -> PathBuf {
        self.root.join("plugin-logs")
    }
    /// Default backup folder (server.json `backup.dir` overrides it, e.g. the share).
    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }
    pub fn database(&self) -> PathBuf {
        self.root.join(DB_FILE)
    }

    /// Create the folders; `keys/` is 0700 on unix.
    pub fn ensure(&self) -> Result<()> {
        for d in [
            self.root.clone(),
            self.service_plugins(),
            self.quarantine(),
            self.keys(),
            self.plugin_logs(),
        ] {
            std::fs::create_dir_all(&d).with_context(|| format!("creating {}", d.display()))?;
        }
        restrict_dir(&self.keys())?;
        Ok(())
    }
}

#[cfg(unix)]
fn restrict_dir(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}
#[cfg(not(unix))]
fn restrict_dir(_p: &Path) -> Result<()> {
    Ok(()) // ProgramData ACLs + DPAPI protect the contents on Windows.
}

/// Write a secret file: protected with DPAPI on Windows, 0600 elsewhere. Atomic (tmp + rename).
pub fn write_secret(path: &Path, secret: &[u8]) -> Result<()> {
    let data = protect(secret)?;
    let tmp = path.with_extension("tmp");
    {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        use std::io::Write;
        let mut f = opts
            .open(&tmp)
            .with_context(|| format!("writing {}", tmp.display()))?;
        f.write_all(&data)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_secret(path: &Path) -> Result<Vec<u8>> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            anyhow::bail!(
                "{} is readable by group/others (mode {:o}); chmod 600 it",
                path.display(),
                mode
            );
        }
    }
    unprotect(&data)
}

#[cfg(not(windows))]
fn protect(secret: &[u8]) -> Result<Vec<u8>> {
    Ok(hex::encode(secret).into_bytes())
}
#[cfg(not(windows))]
fn unprotect(data: &[u8]) -> Result<Vec<u8>> {
    hex::decode(String::from_utf8_lossy(data).trim()).context("secret file is not valid hex")
}

#[cfg(windows)]
fn protect(secret: &[u8]) -> Result<Vec<u8>> {
    dpapi::protect(secret)
}
#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>> {
    dpapi::unprotect(data)
}

#[cfg(windows)]
mod dpapi {
    use anyhow::{bail, Result};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_LOCAL_MACHINE,
        CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn call(input: &[u8], encrypt: bool) -> Result<Vec<u8>> {
        call_with(input, encrypt, CRYPTPROTECT_LOCAL_MACHINE, None)
    }

    pub(super) fn call_with(
        input: &[u8],
        encrypt: bool,
        scope: u32,
        entropy: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        let mut ent = CRYPT_INTEGER_BLOB {
            cbData: entropy.map_or(0, |e| e.len() as u32),
            pbData: entropy.map_or(std::ptr::null_mut(), |e| e.as_ptr() as *mut u8),
        };
        let entp: *mut CRYPT_INTEGER_BLOB = if entropy.is_some() {
            &mut ent
        } else {
            std::ptr::null_mut()
        };
        let mut inb = CRYPT_INTEGER_BLOB {
            cbData: input.len() as u32,
            pbData: input.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let flags = scope | CRYPTPROTECT_UI_FORBIDDEN;
        // SAFETY: valid in/out blobs; the output buffer is freed with LocalFree below.
        let ok = unsafe {
            if encrypt {
                CryptProtectData(
                    &mut inb,
                    std::ptr::null(),
                    entp,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    flags,
                    &mut out,
                )
            } else {
                CryptUnprotectData(
                    &mut inb,
                    std::ptr::null_mut(),
                    entp,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    flags,
                    &mut out,
                )
            }
        };
        if ok == 0 {
            bail!(
                "DPAPI {} failed: {}",
                if encrypt { "protect" } else { "unprotect" },
                std::io::Error::last_os_error()
            );
        }
        // SAFETY: DPAPI returned a buffer of cbData bytes.
        let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec() };
        unsafe { LocalFree(out.pbData as _) };
        Ok(v)
    }
    pub fn protect(d: &[u8]) -> Result<Vec<u8>> {
        call(d, true)
    }
    pub fn unprotect(d: &[u8]) -> Result<Vec<u8>> {
        call(d, false)
    }
}

/// Entropy the desktop app uses for its DPAPI key file (`app/lib/key_store.dart`).
pub const APP_KEY_ENTROPY: &[u8] = b"itcmdr-app-key/v1";

/// Windows: DPAPI for the CURRENT USER with the app's entropy, the format of
/// `%APPDATA%\itcmdr\app-key.dpapi` (the app's private key, a hex seed in UTF8).
#[cfg(windows)]
pub fn protect_for_app(data: &[u8]) -> Result<Vec<u8>> {
    dpapi::call_with(data, true, 0, Some(APP_KEY_ENTROPY))
}
#[cfg(windows)]
pub fn unprotect_for_app(data: &[u8]) -> Result<Vec<u8>> {
    dpapi::call_with(data, false, 0, Some(APP_KEY_ENTROPY))
}

/// Load `keys/db.key`, creating a random 32-byte key the first time.
pub fn load_or_create_db_key(layout: &Layout) -> Result<[u8; 32]> {
    let p = layout.keys().join("db.key");
    if p.exists() {
        let v = read_secret(&p)?;
        return v
            .try_into()
            .map_err(|_| anyhow::anyhow!("{} has the wrong length", p.display()));
    }
    let mut k = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut k);
    write_secret(&p, &k)?;
    Ok(k)
}

/// Overwrite a file with zeros (synced), then delete it. Best effort against recovery: on SSDs and
/// copy-on-write or journaling filesystems the old blocks may survive, so full-disk encryption
/// is still recommended.
pub fn shred_file(p: &Path) -> Result<()> {
    use std::io::Write;
    let len = std::fs::metadata(p)
        .with_context(|| format!("stat {}", p.display()))?
        .len();
    {
        let mut f = std::fs::OpenOptions::new().write(true).open(p)?;
        let zeros = vec![0u8; 64 * 1024];
        let mut left = len;
        while left > 0 {
            let n = left.min(zeros.len() as u64) as usize;
            f.write_all(&zeros[..n])?;
            left -= n as u64;
        }
        f.sync_all()?;
    }
    std::fs::remove_file(p)?;
    Ok(())
}

/// True when the file starts with the plaintext SQLite header (SQLCipher files look random).
pub fn is_plaintext_sqlite(p: &Path) -> bool {
    use std::io::Read;
    let mut h = [0u8; 16];
    std::fs::File::open(p)
        .and_then(|mut f| f.read_exact(&mut h))
        .is_ok()
        && &h == b"SQLite format 3\0"
}

/// SQLCipher raw-key pragma value: `"x'<64 hex>'"`.
pub fn key_pragma(key: &[u8; 32]) -> String {
    format!("\"x'{}'\"", hex::encode(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shred_overwrites_then_deletes() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x");
        std::fs::write(&p, vec![b'A'; 200_000]).unwrap();
        shred_file(&p).unwrap();
        assert!(!p.exists());
        assert!(shred_file(&p).is_err());
    }

    #[test]
    fn data_dir_precedence() {
        let f = Path::new("/flag");
        let c = Path::new("/cfg");
        assert_eq!(
            resolve_data_dir(Some(f), Some("/env"), Some(c)),
            PathBuf::from("/flag")
        );
        assert_eq!(
            resolve_data_dir(None, Some("/env"), Some(c)),
            PathBuf::from("/env")
        );
        assert_eq!(
            resolve_data_dir(None, Some("  "), Some(c)),
            PathBuf::from("/cfg")
        );
        assert_eq!(resolve_data_dir(None, None, None), default_data_dir());
        assert!(default_data_dir().ends_with(if cfg!(windows) { "itcmdr" } else { ".itcmdr" }));
    }

    #[test]
    fn layout_and_secret_files() {
        let t = tempfile::tempdir().unwrap();
        let l = Layout::new(t.path().join("d"));
        l.ensure().unwrap();
        for d in ["service_plugins", "quarantine", "keys", "plugin-logs"] {
            assert!(l.root.join(d).is_dir(), "{d}");
        }
        assert_eq!(l.database().file_name().unwrap(), DB_FILE);
        let k1 = load_or_create_db_key(&l).unwrap();
        let k2 = load_or_create_db_key(&l).unwrap();
        assert_eq!(k1, k2, "key is stable across loads");
        assert_ne!(k1, [0u8; 32]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = |p: PathBuf| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(m(l.keys()), 0o700);
            assert_eq!(m(l.keys().join("db.key")), 0o600);
            std::fs::set_permissions(
                l.keys().join("db.key"),
                std::fs::Permissions::from_mode(0o644),
            )
            .unwrap();
            assert!(
                load_or_create_db_key(&l).is_err(),
                "refuses a world-readable key"
            );
        }
    }
}
