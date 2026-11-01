//! Client side of the admin API: identity on this device, signed calls, enrollment, key backup.
//! The admin app does the same through itcmdr-keys (WASM on web).
use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::SigningKey;
use itcmdr_adminlink::pb;
use pb::admin_api_client::AdminApiClient;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tonic::transport::Channel;
use zeroize::Zeroizing;

pub use itcmdr_adminlink::pb as proto;

/// What this device keeps (0600). The device key never leaves it except as the encrypted backup.
#[derive(Serialize, Deserialize, Clone)]
pub struct Identity {
    pub addr: String,
    pub pin: String,
    pub email: String,
    pub device_seed: String,
}

impl Identity {
    pub fn key(&self) -> Result<SigningKey> {
        itcmdr_keys::key_from_seed(&hex::decode(&self.device_seed)?)
    }
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("identity.json")
    }
    pub fn load(dir: &Path) -> Result<Self> {
        let p = Self::path(dir);
        serde_json::from_slice(
            &std::fs::read(&p).with_context(|| {
                format!("no identity at {} (run itcmdr-admin enroll)", p.display())
            })?,
        )
        .context("identity file")
    }
    pub fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let p = Self::path(dir);
        std::fs::write(&p, serde_json::to_vec_pretty(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

pub fn default_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("ITCMDR_ADMIN_HOME") {
        return PathBuf::from(d);
    }
    directories::ProjectDirs::from("", "", "itcmdr-admin")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".itcmdr-admin"))
}

/// A signed client.
pub struct Client {
    pub api: AdminApiClient<Channel>,
    key: SigningKey,
}

pub fn signed<T: Message>(key: &SigningKey, method: &str, msg: T) -> tonic::Request<T> {
    let body = msg.encode_to_vec();
    let mut r = tonic::Request::new(msg);
    for (k, v) in itcmdr_keys::sign_request(key, method, &body) {
        r.metadata_mut().insert(k, v.parse().expect("ascii"));
    }
    r
}

pub async fn channel(addr: &str, pin: &str) -> Result<Channel> {
    itcmdr_agentlink::tls::connect(addr, pin, None)
        .await
        .with_context(|| format!("connect {addr}"))
}

macro_rules! call {
    ($self:ident, $m:ident, $name:literal, $req:expr) => {
        $self
            .api
            .clone()
            .$m(signed(
                &$self.key,
                concat!("/itcmdr.admin.AdminApi/", $name),
                $req,
            ))
            .await
            .map(|r| r.into_inner())
            .map_err(|s| anyhow!("{}", s.message()))
    };
}

impl Client {
    pub async fn connect(id: &Identity) -> Result<Self> {
        Ok(Client {
            api: AdminApiClient::new(channel(&id.addr, &id.pin).await?),
            key: id.key()?,
        })
    }
    /// Signed with another key (the master key from the words).
    pub async fn with_key(addr: &str, pin: &str, key: SigningKey) -> Result<Self> {
        Ok(Client {
            api: AdminApiClient::new(channel(addr, pin).await?),
            key,
        })
    }
    pub fn key(&self) -> &SigningKey {
        &self.key
    }
    pub async fn whoami(&self) -> Result<pb::UserInfo> {
        call!(self, who_am_i, "WhoAmI", pb::Empty {})
    }
    pub async fn list_users(&self) -> Result<pb::UserList> {
        call!(self, list_users, "ListUsers", pb::Empty {})
    }
    pub async fn create_user(
        &self,
        email: &str,
        name: &str,
        role: &str,
    ) -> Result<pb::CreateUserResponse> {
        call!(
            self,
            create_user,
            "CreateUser",
            pb::CreateUserRequest {
                email: email.into(),
                name: name.into(),
                role: role.into()
            }
        )
    }
    pub async fn set_role(&self, user: &str, role: &str) -> Result<pb::UserInfo> {
        call!(
            self,
            set_role,
            "SetRole",
            pb::SetRoleRequest {
                user: user.into(),
                role: role.into()
            }
        )
    }
    pub async fn set_disabled(&self, user: &str, disabled: bool) -> Result<pb::UserInfo> {
        call!(
            self,
            set_user_status,
            "SetUserStatus",
            pb::SetUserStatusRequest {
                user: user.into(),
                disabled
            }
        )
    }
    pub async fn set_scopes(&self, user: &str, sites: Vec<String>) -> Result<pb::UserInfo> {
        call!(
            self,
            set_scopes,
            "SetScopes",
            pb::SetScopesRequest {
                user: user.into(),
                sites
            }
        )
    }
    pub async fn approve_device(&self, user: &str, fp: &str) -> Result<pb::UserInfo> {
        call!(
            self,
            approve_device,
            "ApproveDevice",
            pb::ApproveDeviceRequest {
                user: user.into(),
                device_fingerprint: fp.into()
            }
        )
    }
    pub async fn new_code(&self, user: &str) -> Result<pb::CreateUserResponse> {
        call!(
            self,
            new_enroll_code,
            "NewEnrollCode",
            pb::UserRef { user: user.into() }
        )
    }
    pub async fn list_devices(&self, user: &str) -> Result<pb::DeviceList> {
        call!(
            self,
            list_devices,
            "ListDevices",
            pb::UserRef { user: user.into() }
        )
    }
    pub async fn revoke_device(&self, user: &str, fp: &str) -> Result<pb::DeviceList> {
        call!(
            self,
            revoke_device,
            "RevokeDevice",
            pb::RevokeDeviceRequest {
                user: user.into(),
                device_fingerprint: fp.into()
            }
        )
    }
    pub async fn put_key_backup(&self, blob_json: String) -> Result<pb::Empty> {
        call!(
            self,
            put_key_backup,
            "PutKeyBackup",
            pb::KeyBackup {
                user: String::new(),
                blob_json
            }
        )
    }
    pub async fn get_key_backup(&self) -> Result<pb::KeyBackup> {
        call!(
            self,
            get_key_backup,
            "GetKeyBackup",
            pb::UserRef {
                user: String::new()
            }
        )
    }
    pub async fn key_backup_opt_out(&self, opt_out: bool) -> Result<pb::UserInfo> {
        call!(
            self,
            set_key_backup_opt_out,
            "SetKeyBackupOptOut",
            pb::KeyBackupOptOut { opt_out }
        )
    }
    pub async fn add_device(&self, r: pb::AddDeviceRequest) -> Result<pb::UserInfo> {
        call!(self, add_device, "AddDevice", r)
    }
    pub async fn create_site(&self, name: &str) -> Result<pb::CreateSiteResponse> {
        call!(
            self,
            create_site,
            "CreateSite",
            pb::CreateSiteRequest { name: name.into() }
        )
    }
    pub async fn list_sites(&self) -> Result<pb::SiteList> {
        call!(self, list_sites, "ListSites", pb::Empty {})
    }
    pub async fn revoke_site(&self, site: &str) -> Result<pb::SiteInfo> {
        call!(
            self,
            revoke_site,
            "RevokeSite",
            pb::SiteRef { site: site.into() }
        )
    }
    pub async fn list_alerts(&self, site: &str, state: &str, limit: i32) -> Result<pb::AlertList> {
        call!(
            self,
            list_alerts,
            "ListAlerts",
            pb::AlertFilter {
                site: site.into(),
                state: state.into(),
                limit
            }
        )
    }
    pub async fn list_commands(&self, site: &str) -> Result<pb::CommandList> {
        call!(
            self,
            list_commands,
            "ListCommands",
            pb::SiteRef { site: site.into() }
        )
    }
    pub async fn list_audit(&self, user: &str, limit: i32) -> Result<pb::AuditList> {
        call!(
            self,
            list_audit,
            "ListAudit",
            pb::AuditFilter {
                user: user.into(),
                limit
            }
        )
    }
    pub async fn verify_audit(&self) -> Result<pb::AuditVerify> {
        call!(self, verify_audit, "VerifyAudit", pb::Empty {})
    }

    /// Build and sign a command with this device key (adminsvr adds the root signature).
    pub async fn command(
        &self,
        site: &str,
        kind: &str,
        args: &[(&str, &str)],
    ) -> Result<pb::CommandInfo> {
        let me = self.whoami().await?;
        let now = itcmdr_keys::now_ms();
        let mut c = pb::Command {
            id: uuid::Uuid::new_v4().to_string(),
            site_id: site.into(),
            kind: kind.into(),
            args: args
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            user_id: me.id,
            user_role: me.role,
            issued_ms: now,
            expires_ms: now + 10 * 60 * 1000,
            ..Default::default()
        };
        itcmdr_adminlink::sign_user(&self.key, &mut c);
        let req = pb::CommandRequest { command: Some(c) };
        match kind {
            "backup.start" => call!(self, start_backup, "StartBackup", req),
            "alert.ack" => call!(self, ack_alert, "AckAlert", req),
            "alert.resolve" => call!(self, resolve_alert, "ResolveAlert", req),
            _ => bail!("unknown command {kind}"),
        }
    }
}

/// Result of a first enrollment on this device.
pub struct Enrolled {
    pub identity: Identity,
    pub words: Zeroizing<String>,
    pub user: pb::UserInfo,
}

/// Enroll this device with a one-time code: new 24 words and master key, a device key certified
/// by the master, and (unless `no_backup`) the client-side encrypted key backup.
/// `words`: reuse existing words (an existing user re-enrolling with a code).
pub async fn enroll(
    code: &str,
    platform: &str,
    no_backup: bool,
    words: Option<&str>,
) -> Result<Enrolled> {
    let c = itcmdr_adminlink::EnrollCode::decode(code)?;
    let words = Zeroizing::new(match words {
        Some(w) => w.to_string(),
        None => itcmdr_keys::generate_words(),
    });
    let master = itcmdr_keys::Master::from_words(&words)?;
    let seed = itcmdr_keys::random_seed();
    let dk = itcmdr_keys::key_from_seed(seed.as_ref())?;
    let cert = itcmdr_keys::certify(
        &master.signing,
        "user",
        &c.email,
        "adminsvr-device",
        &dk.verifying_key(),
    );
    let key_backup = if no_backup {
        None
    } else {
        Some(pb::KeyBackup {
            user: String::new(),
            blob_json: serde_json::to_string(&itcmdr_keys::wrap::wrap(
                &words,
                &c.email.to_lowercase(),
                &seed,
            )?)?,
        })
    };
    let req = pb::EnrollDeviceRequest {
        code: c.secret.clone(),
        master_public_key: hex::encode(master.signing.verifying_key().as_bytes()),
        device_public_key: hex::encode(dk.verifying_key().as_bytes()),
        certificate_json: serde_json::to_string(&cert)?,
        platform: platform.into(),
        key_backup,
    };
    let mut api = AdminApiClient::new(channel(&c.addr, &c.pin).await?);
    let user = api
        .enroll_device(signed(&dk, "/itcmdr.admin.AdminApi/EnrollDevice", req))
        .await
        .map_err(|s| anyhow!("{}", s.message()))?
        .into_inner();
    Ok(Enrolled {
        identity: Identity {
            addr: c.addr,
            pin: c.pin,
            email: c.email,
            device_seed: hex::encode(seed.as_ref()),
        },
        words,
        user,
    })
}

/// New device from the 24 words: restore the device key from the encrypted backup when there
/// is one (`from_backup`), else certify a fresh device key with the master (AddDevice).
pub async fn recover(
    addr: &str,
    pin: &str,
    email: &str,
    words: &str,
    from_backup: bool,
    platform: &str,
) -> Result<Identity> {
    let master = itcmdr_keys::Master::from_words(words)?;
    let mc = Client::with_key(addr, pin, master.signing.clone()).await?;
    if from_backup {
        let kb = mc.get_key_backup().await?;
        let w: itcmdr_keys::wrap::Wrapped = serde_json::from_str(&kb.blob_json)?;
        let seed = itcmdr_keys::wrap::unwrap(words, &w)?;
        return Ok(Identity {
            addr: addr.into(),
            pin: pin.into(),
            email: email.into(),
            device_seed: hex::encode(seed.as_ref()),
        });
    }
    let seed = itcmdr_keys::random_seed();
    let dk = itcmdr_keys::key_from_seed(seed.as_ref())?;
    let cert = itcmdr_keys::certify(
        &master.signing,
        "user",
        email,
        "adminsvr-device",
        &dk.verifying_key(),
    );
    mc.add_device(pb::AddDeviceRequest {
        device_public_key: hex::encode(dk.verifying_key().as_bytes()),
        certificate_json: serde_json::to_string(&cert)?,
        platform: platform.into(),
    })
    .await?;
    Ok(Identity {
        addr: addr.into(),
        pin: pin.into(),
        email: email.into(),
        device_seed: hex::encode(seed.as_ref()),
    })
}

/// Every app action and its CLI command (the parity list; checked by a test against the API).
pub const PARITY: &[(&str, &str)] = &[
    ("Info", "info"),
    ("EnrollDevice", "enroll"),
    ("WhoAmI", "whoami"),
    ("ListUsers", "user list"),
    ("CreateUser", "user create"),
    ("SetRole", "user role"),
    ("SetUserStatus", "user disable / user enable"),
    ("SetScopes", "user scopes"),
    ("ApproveDevice", "user approve"),
    ("NewEnrollCode", "user code"),
    ("ListDevices", "device list"),
    ("RevokeDevice", "device revoke"),
    ("PutKeyBackup", "key-backup on"),
    ("GetKeyBackup", "recover --from-backup"),
    ("SetKeyBackupOptOut", "key-backup off"),
    ("AddDevice", "recover"),
    ("CreateSite", "site create"),
    ("ListSites", "site list"),
    ("RevokeSite", "site revoke"),
    ("ListAlerts", "alert list"),
    ("AckAlert", "alert ack"),
    ("ResolveAlert", "alert resolve"),
    ("StartBackup", "backup start"),
    ("ListCommands", "command list"),
    ("ListAudit", "audit list"),
    ("VerifyAudit", "audit verify"),
];
