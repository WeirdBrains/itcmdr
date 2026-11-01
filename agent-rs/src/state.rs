//! The agent's files in its data folder:
//! - `agent.json`: id, machine, role, server address, pinned server fingerprint, the client
//!   certificate and the site CA certificate (public data);
//! - `keys/agent.key` (Ed25519 seed, signs Hello) and `keys/tls.key` (TLS client key, PEM):
//!   secret files, DPAPI LocalMachine on Windows, 0600 elsewhere. They never leave the PC.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub agent_id: String,
    pub machine: String,
    pub role: String,
    pub address: String,
    pub pin: String,
    pub cert_pem: String,
    pub ca_pem: String,
    pub cert_expires_ms: i64,
    pub enrolled_ms: i64,
}

pub fn default_dir() -> PathBuf {
    if let Ok(d) = std::env::var("ITCMDR_AGENT_DIR") {
        return PathBuf::from(d);
    }
    #[cfg(windows)]
    {
        let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
        PathBuf::from(pd).join("itcmdr").join("agent")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/var/lib/itcmdr-agent")
    }
}

/// Where the agent installer leaves `/EnrollToken=` (picked up by `run` when not enrolled).
pub fn default_token_file(dir: &Path) -> PathBuf {
    dir.parent()
        .map(|p| p.join("agent-enroll.token"))
        .unwrap_or_else(|| dir.join("agent-enroll.token"))
}

pub fn this_machine() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

pub fn state_path(dir: &Path) -> PathBuf {
    dir.join("agent.json")
}

pub fn load(dir: &Path) -> Result<Option<State>> {
    let p = state_path(dir);
    if !p.exists() {
        return Ok(None);
    }
    Ok(Some(
        serde_json::from_str(&std::fs::read_to_string(&p)?)
            .with_context(|| format!("reading {}", p.display()))?,
    ))
}

pub fn save(dir: &Path, s: &State) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let p = state_path(dir);
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(&tmp, &p)?;
    Ok(())
}

pub fn keys_dir(dir: &Path) -> PathBuf {
    dir.join("keys")
}

/// Write a secret file: DPAPI (LocalMachine) on Windows, 0600 elsewhere. Atomic.
pub fn write_secret(path: &Path, secret: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let data = protect(secret)?;
    let tmp = path.with_extension("tmp");
    {
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        use std::io::Write;
        let mut f = o.open(&tmp)?;
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
        if std::fs::metadata(path)?.permissions().mode() & 0o077 != 0 {
            anyhow::bail!(
                "{} is readable by group/others; chmod 600 it",
                path.display()
            );
        }
    }
    unprotect(&data)
}

#[cfg(not(windows))]
fn protect(s: &[u8]) -> Result<Vec<u8>> {
    Ok(hex::encode(s).into_bytes())
}
#[cfg(not(windows))]
fn unprotect(d: &[u8]) -> Result<Vec<u8>> {
    hex::decode(String::from_utf8_lossy(d).trim()).context("secret file is not valid hex")
}
#[cfg(windows)]
fn protect(s: &[u8]) -> Result<Vec<u8>> {
    dpapi::call(s, true)
}
#[cfg(windows)]
fn unprotect(d: &[u8]) -> Result<Vec<u8>> {
    dpapi::call(d, false)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod dpapi {
    use anyhow::{bail, Result};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_LOCAL_MACHINE,
        CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    pub fn call(input: &[u8], encrypt: bool) -> Result<Vec<u8>> {
        let inb = CRYPT_INTEGER_BLOB {
            cbData: input.len() as u32,
            pbData: input.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let flags = CRYPTPROTECT_LOCAL_MACHINE | CRYPTPROTECT_UI_FORBIDDEN;
        // SAFETY: valid in/out blobs; the output buffer is freed with LocalFree below.
        let ok = unsafe {
            if encrypt {
                CryptProtectData(
                    &inb,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    flags,
                    &mut out,
                )
            } else {
                CryptUnprotectData(
                    &inb,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    flags,
                    &mut out,
                )
            }
        };
        if ok == 0 {
            bail!("DPAPI failed: {}", std::io::Error::last_os_error());
        }
        // SAFETY: DPAPI returned a buffer of cbData bytes.
        let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec() };
        unsafe { LocalFree(out.pbData as _) };
        Ok(v)
    }
}
