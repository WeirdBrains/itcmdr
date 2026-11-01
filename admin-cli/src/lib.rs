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
pub mod passkey;
use passkey::{Authenticator, LocalPasskey, Locked};
use std::sync::Arc;

/// What this device keeps (0600). The device key never leaves it except as the encrypted backup.
#[derive(Serialize, Deserialize, Clone)]
pub struct Identity {
    pub addr: String,
    pub pin: String,
    pub email: String,
    /// Empty while `locked` (v0.19.1).
    #[serde(default)]
    pub device_seed: String,
    /// Passkeys registered from this device (v0.19.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passkeys: Vec<LocalPasskey>,
    /// The device key encrypted under a passkey's hmac-secret (v0.19.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<Locked>,
}

impl Identity {
    pub fn new(addr: &str, pin: &str, email: &str, seed_hex: String) -> Self {
        Identity {
            addr: addr.into(),
            pin: pin.into(),
            email: email.into(),
            device_seed: seed_hex,
            passkeys: vec![],
            locked: None,
        }
    }
    /// The device key; a locked identity needs `auth` (one touch).
    pub fn key_with(&self, auth: Option<&dyn Authenticator>) -> Result<SigningKey> {
        if let Some(l) = &self.locked {
            let a = auth.ok_or_else(|| {
                anyhow!("this identity is locked with a passkey; plug in your security key")
            })?;
            let seed = passkey::unlock(a, l)?;
            return itcmdr_keys::key_from_seed(&hex::decode(seed.as_str())?);
        }
        itcmdr_keys::key_from_seed(&hex::decode(&self.device_seed)?)
    }
    pub fn key(&self) -> Result<SigningKey> {
        self.key_with(None)
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
    auth: Option<Arc<dyn Authenticator>>,
    /// Credentials to try for a step-up (empty = the ones adminsvr lists).
    pub prefer: Vec<Vec<u8>>,
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

/// A signed call; when adminsvr asks for a step-up and this client has an authenticator, touch
/// the key for this exact request and send it again.
macro_rules! call {
    ($self:ident, $m:ident, $name:literal, $req:expr) => {{
        let method = concat!("/itcmdr.admin.AdminApi/", $name);
        let req = $req;
        let first = $self
            .api
            .clone()
            .$m(signed(&$self.key, method, req.clone()))
            .await;
        match first {
            Err(s)
                if s.code() == tonic::Code::FailedPrecondition
                    && s.message().starts_with(STEPUP_REQUIRED)
                    && $self.auth.is_some() =>
            {
                let h = $self.stepup(method, &req.encode_to_vec()).await?;
                let mut r = signed(&$self.key, method, req);
                r.metadata_mut()
                    .insert("x-itcmdr-stepup", h.parse().expect("base64"));
                $self.api.clone().$m(r).await
            }
            other => other,
        }
        .map(|r| r.into_inner())
        .map_err(|s| anyhow!("{}", s.message()))
    }};
}

const STEPUP_REQUIRED: &str = "stepup-required";

impl Client {
    pub async fn connect(id: &Identity) -> Result<Self> {
        Self::connect_with(id, None).await
    }
    /// With an authenticator: unlocks a locked identity and answers step-ups.
    pub async fn connect_with(id: &Identity, auth: Option<Arc<dyn Authenticator>>) -> Result<Self> {
        Ok(Client {
            api: AdminApiClient::new(channel(&id.addr, &id.pin).await?),
            key: id.key_with(auth.as_deref())?,
            prefer: id
                .passkeys
                .iter()
                .filter_map(|p| itcmdr_adminlink::webauthn::from_b64url(&p.credential_id).ok())
                .collect(),
            auth,
        })
    }
    /// Signed with another key (the master key from the words).
    pub async fn with_key(addr: &str, pin: &str, key: SigningKey) -> Result<Self> {
        Ok(Client {
            api: AdminApiClient::new(channel(addr, pin).await?),
            key,
            auth: None,
            prefer: vec![],
        })
    }
    pub fn set_authenticator(&mut self, a: Option<Arc<dyn Authenticator>>) {
        self.auth = a;
    }

    /// A step-up header for `method` with this body: nonce from adminsvr, one touch.
    pub async fn stepup(&self, method: &str, body: &[u8]) -> Result<String> {
        use itcmdr_adminlink::webauthn as wa;
        let auth = self
            .auth
            .clone()
            .ok_or_else(|| anyhow!("a passkey touch is needed; plug in your security key"))?;
        let ch = self
            .api
            .clone()
            .begin_step_up(signed(
                &self.key,
                "/itcmdr.admin.AdminApi/BeginStepUp",
                pb::Empty {},
            ))
            .await
            .map_err(|s| anyhow!("{}", s.message()))?
            .into_inner();
        let challenge = wa::stepup_challenge(method, body, &ch.nonce);
        let cd = wa::client_data("webauthn.get", &challenge, wa::CLI_ORIGIN);
        let cdh = wa::client_data_hash(&cd);
        let mut ids: Vec<Vec<u8>> = self
            .prefer
            .iter()
            .filter(|i| ch.credential_ids.contains(i))
            .cloned()
            .collect();
        ids.extend(
            ch.credential_ids
                .iter()
                .filter(|i| !ids.contains(i))
                .cloned()
                .collect::<Vec<_>>(),
        );
        let mut last = anyhow!("you have no passkeys registered");
        for id in ids {
            match auth.get_assertion(&ch.rp_id, &id, &cdh, None) {
                Ok(a) => {
                    let s = pb::StepUpAssertion {
                        nonce: ch.nonce.clone(),
                        credential_id: id,
                        client_data_json: cd,
                        auth_data: itcmdr_adminlink::webauthn::unwrap_cbor_bytes(&a.auth_data),
                        signature: a.sig,
                    };
                    use base64::Engine;
                    return Ok(base64::engine::general_purpose::STANDARD.encode(s.encode_to_vec()));
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    /// Register a passkey on the authenticator (`hmac_secret`: usable to lock the device key).
    pub async fn add_passkey(
        &self,
        label: &str,
        hmac_secret: bool,
    ) -> Result<(LocalPasskey, pb::PasskeyList)> {
        use itcmdr_adminlink::webauthn as wa;
        passkey::check_label(label)?;
        let auth = self
            .auth
            .clone()
            .ok_or_else(|| anyhow!("no authenticator"))?;
        let ch = call!(
            self,
            begin_passkey_registration,
            "BeginPasskeyRegistration",
            pb::Empty {}
        )?;
        let cd = wa::client_data("webauthn.create", &ch.challenge, wa::CLI_ORIGIN);
        let m = auth.make_credential(
            &ch.rp_id,
            &ch.user_handle,
            &ch.user_name,
            &wa::client_data_hash(&cd),
            hmac_secret,
        )?;
        let ad = wa::unwrap_cbor_bytes(&m.auth_data);
        let id = wa::parse_auth_data(&ad)?.credential_id;
        let list = call!(
            self,
            finish_passkey_registration,
            "FinishPasskeyRegistration",
            pb::PasskeyRegistration {
                nonce: ch.nonce.clone(),
                label: label.into(),
                client_data_json: cd.clone(),
                fmt: m.fmt.clone(),
                auth_data: ad.clone(),
                att_sig: m.sig.clone(),
                x5c: m.x5c.clone(),
                hmac_secret,
                ..Default::default()
            }
        )?;
        Ok((
            LocalPasskey {
                credential_id: wa::b64url(&id),
                rp_id: ch.rp_id,
                label: label.into(),
                hmac_secret,
            },
            list,
        ))
    }
    pub async fn list_passkeys(&self, user: &str) -> Result<pb::PasskeyList> {
        call!(
            self,
            list_passkeys,
            "ListPasskeys",
            pb::UserRef { user: user.into() }
        )
    }
    pub async fn remove_passkey(&self, user: &str, credential_id: &str) -> Result<pb::PasskeyList> {
        call!(
            self,
            remove_passkey,
            "RemovePasskey",
            pb::PasskeyRef {
                user: user.into(),
                credential_id: credential_id.into()
            }
        )
    }
    pub async fn get_auth_policy(&self) -> Result<pb::AuthPolicy> {
        call!(self, get_auth_policy, "GetAuthPolicy", pb::Empty {})
    }
    pub async fn set_auth_policy(&self, p: pb::AuthPolicy) -> Result<pb::AuthPolicy> {
        call!(self, set_auth_policy, "SetAuthPolicy", p)
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
    /// v0.19.2: a site's status history (oldest first).
    pub async fn site_metrics(
        &self,
        site: &str,
        since_ms: i64,
        limit: i32,
    ) -> Result<pb::MetricsSeries> {
        call!(
            self,
            list_site_metrics,
            "ListSiteMetrics",
            pb::MetricsFilter {
                site: site.into(),
                since_ms,
                limit
            }
        )
    }
    /// v0.19.2: this device's push token (provider fcm | apns).
    pub async fn register_push(
        &self,
        provider: &str,
        token: &str,
        platform: &str,
    ) -> Result<pb::Empty> {
        call!(
            self,
            register_push,
            "RegisterPush",
            pb::PushRegistration {
                provider: provider.into(),
                token: token.into(),
                platform: platform.into()
            }
        )
    }
    /// Remove a push token ("" = every token of this device).
    pub async fn unregister_push(&self, token: &str) -> Result<pb::Empty> {
        call!(
            self,
            unregister_push,
            "UnregisterPush",
            pb::PushRegistration {
                token: token.into(),
                ..Default::default()
            }
        )
    }
    pub async fn test_push(&self) -> Result<pb::PushResult> {
        call!(self, test_push, "TestPush", pb::Empty {})
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
        identity: Identity::new(&c.addr, &c.pin, &c.email, hex::encode(seed.as_ref())),
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
    auth: Option<Arc<dyn Authenticator>>,
) -> Result<Identity> {
    let master = itcmdr_keys::Master::from_words(words)?;
    let mut mc = Client::with_key(addr, pin, master.signing.clone()).await?;
    mc.set_authenticator(auth);
    if from_backup {
        let kb = mc.get_key_backup().await?;
        let w: itcmdr_keys::wrap::Wrapped = serde_json::from_str(&kb.blob_json)?;
        let seed = itcmdr_keys::wrap::unwrap(words, &w)?;
        return Ok(Identity::new(addr, pin, email, hex::encode(seed.as_ref())));
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
    Ok(Identity::new(addr, pin, email, hex::encode(seed.as_ref())))
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
    ("BeginPasskeyRegistration", "passkey add"),
    ("FinishPasskeyRegistration", "passkey add"),
    ("ListPasskeys", "passkey list"),
    ("RemovePasskey", "passkey remove / passkey reset"),
    ("BeginStepUp", "(any step-up action)"),
    ("GetAuthPolicy", "policy show"),
    ("SetAuthPolicy", "policy set"),
    ("ListSiteMetrics", "site metrics"),
    ("RegisterPush", "push register"),
    ("UnregisterPush", "push unregister"),
    ("TestPush", "push test"),
];
