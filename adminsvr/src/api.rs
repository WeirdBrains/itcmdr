//! The admin API (CLI and app). Every call except Info is signed; every change is audited.
use crate::audit::{self, Entry};
use crate::auth::{self, Nonces, Principal};
use crate::config::Config;
use crate::link::Hub;
use crate::material::Material;
use crate::pb;
use crate::rbac::{self, Role};
use crate::store::Pool;
use anyhow::{anyhow, Context, Result};
use prost::Message;
use rand::RngCore;
use sqlx::Row;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub const CODE_TTL_MS: i64 = 24 * 3600 * 1000;
pub const CMD_TTL_MS: i64 = 15 * 60 * 1000;

#[derive(Clone)]
pub struct Api {
    pool: Pool,
    cfg: Config,
    m: Material,
    hub: Hub,
    nonces: Arc<Nonces>,
}

fn internal(e: impl std::fmt::Display) -> Status {
    Status::internal(e.to_string())
}
fn denied(e: impl std::fmt::Display) -> Status {
    Status::permission_denied(e.to_string())
}

pub fn secret() -> String {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}

pub async fn find_user(pool: &Pool, r: &str) -> Result<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM users WHERE id = $1 OR lower(email) = lower($1)")
            .bind(r)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn active_owners(pool: &Pool) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM users WHERE role = 'owner' AND status = 'active'")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn user_info(pool: &Pool, id: &str) -> Result<pb::UserInfo> {
    let r = sqlx::query("SELECT u.*, (SELECT count(*) FROM key_backups k WHERE k.user_id = u.id) AS kb FROM users u WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow!("no such user"))?;
    let sites: Vec<String> =
        sqlx::query_scalar("SELECT site_id FROM site_scopes WHERE user_id = $1 ORDER BY site_id")
            .bind(id)
            .fetch_all(pool)
            .await?;
    Ok(pb::UserInfo {
        id: r.get("id"),
        email: r.get("email"),
        name: r.get("name"),
        role: r.get("role"),
        status: r.get("status"),
        sites,
        key_backup: r.get::<i64, _>("kb") > 0,
        key_backup_opt_out: r.get("key_backup_opt_out"),
        created_ms: r.get("created_ms"),
        devices: devices(pool, id).await?,
        master_fingerprint: r.get::<Option<String>, _>("master_fp").unwrap_or_default(),
    })
}

async fn devices(pool: &Pool, id: &str) -> Result<Vec<pb::DeviceInfo>> {
    Ok(
        sqlx::query("SELECT * FROM devices WHERE user_id = $1 ORDER BY created_ms")
            .bind(id)
            .fetch_all(pool)
            .await?
            .iter()
            .map(|r| pb::DeviceInfo {
                fingerprint: r.get("fingerprint"),
                platform: r.get("platform"),
                status: r.get("status"),
                created_ms: r.get("created_ms"),
                last_used_ms: r.get("last_used_ms"),
            })
            .collect(),
    )
}

/// A fresh one-time enrollment code for `user_id`.
pub async fn new_code(
    pool: &Pool,
    cfg: &Config,
    m: &Material,
    user_id: &str,
) -> Result<(String, i64)> {
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let s = secret();
    let exp = crate::now_ms() + CODE_TTL_MS;
    sqlx::query("INSERT INTO enroll_codes (secret_hash, user_id, expires_ms) VALUES ($1,$2,$3)")
        .bind(itcmdr_adminlink::sha256_hex(s.as_bytes()))
        .bind(user_id)
        .bind(exp)
        .execute(pool)
        .await?;
    let code = itcmdr_adminlink::EnrollCode {
        v: 1,
        addr: cfg.public_api.clone(),
        pin: m.fingerprint.clone(),
        secret: s,
        email,
    };
    Ok((code.encode(), exp))
}

/// Console: create the first owner and print an enrollment code. Refused once an owner exists.
pub async fn init(cfg: &Config, email: &str, name: &str) -> Result<String> {
    let pool = crate::store::connect(&cfg.db_url).await?;
    crate::store::migrate(&pool).await?;
    let m = crate::material::load(cfg)?;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await?;
    if n > 0 {
        anyhow::bail!("already initialized (users exist); use recover-owner at the console if every owner is lost");
    }
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (id, email, name, role, created_by, created_ms) VALUES ($1,$2,$3,'owner','console',$4)")
        .bind(&id).bind(email).bind(name).bind(crate::now_ms())
        .execute(&pool).await?;
    audit::append(
        &pool,
        &Entry {
            actor: "console".into(),
            action: "init".into(),
            target: id.clone(),
            payload: email.as_bytes().to_vec(),
            ..Default::default()
        },
    )
    .await?;
    Ok(new_code(&pool, cfg, &m, &id).await?.0)
}

/// Console: a fresh owner enrollment code for an existing owner (or a new owner) when every
/// owner device and the words are lost. Audited.
pub async fn recover_owner(cfg: &Config, email: &str) -> Result<String> {
    let pool = crate::store::connect(&cfg.db_url).await?;
    crate::store::migrate(&pool).await?;
    let m = crate::material::load(cfg)?;
    let id = find_user(&pool, email)
        .await?
        .ok_or_else(|| anyhow!("no such user"))?;
    sqlx::query("UPDATE users SET role = 'owner', status = 'active', master_pub = NULL, master_fp = NULL WHERE id = $1")
        .bind(&id).execute(&pool).await?;
    sqlx::query("UPDATE devices SET status = 'revoked' WHERE user_id = $1")
        .bind(&id)
        .execute(&pool)
        .await?;
    audit::append(
        &pool,
        &Entry {
            actor: "console".into(),
            action: "recover-owner".into(),
            target: id.clone(),
            payload: email.as_bytes().to_vec(),
            ..Default::default()
        },
    )
    .await?;
    Ok(new_code(&pool, cfg, &m, &id).await?.0)
}

impl Api {
    pub fn new(pool: Pool, cfg: Config, m: Material, hub: Hub) -> Self {
        Api {
            pool,
            cfg,
            m,
            hub,
            nonces: Arc::new(Nonces::default()),
        }
    }

    async fn signed<T: Message>(
        &self,
        req: &Request<T>,
        method: &str,
    ) -> Result<Principal, Status> {
        self.signed_opts(req, method, false, false).await
    }

    async fn signed_opts<T: Message>(
        &self,
        req: &Request<T>,
        method: &str,
        master: bool,
        pending: bool,
    ) -> Result<Principal, Status> {
        let body = req.get_ref().encode_to_vec();
        auth::verify(
            &self.pool,
            &self.nonces,
            req.metadata(),
            method,
            &body,
            self.cfg.max_skew_secs * 1000,
            master,
            pending,
        )
        .await
    }

    async fn audit<T: Message>(
        &self,
        p: &Principal,
        action: &str,
        target: &str,
        req: &T,
    ) -> Result<(), Status> {
        audit::append(
            &self.pool,
            &Entry {
                actor: p.user_id.clone(),
                actor_key: p.key_fp.clone(),
                action: action.into(),
                target: target.into(),
                payload: req.encode_to_vec(),
                request_sig: p.sig.clone(),
            },
        )
        .await
        .map(|_| ())
        .map_err(internal)
    }

    async fn target(&self, r: &str, p: &Principal) -> Result<String, Status> {
        if r.is_empty() {
            return Ok(p.user_id.clone());
        }
        find_user(&self.pool, r)
            .await
            .map_err(internal)?
            .ok_or_else(|| Status::not_found("no such user"))
    }

    async fn role_of(&self, id: &str) -> Result<Role, Status> {
        let r: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(internal)?;
        Role::parse(&r).ok_or_else(|| internal("bad role"))
    }

    async fn info(&self, id: &str) -> Result<Response<pb::UserInfo>, Status> {
        user_info(&self.pool, id)
            .await
            .map(Response::new)
            .map_err(internal)
    }

    /// Sites `p` may see/act on.
    pub async fn may_site(&self, p: &Principal, site: &str) -> Result<(), Status> {
        if p.role >= Role::Admin {
            return Ok(());
        }
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM site_scopes WHERE user_id = $1 AND site_id = $2",
        )
        .bind(&p.user_id)
        .bind(site)
        .fetch_one(&self.pool)
        .await
        .map_err(internal)?;
        if n == 0 {
            return Err(denied("this site is not assigned to you"));
        }
        Ok(())
    }

    async fn command(
        &self,
        req: Request<pb::CommandRequest>,
        method: &str,
        kind: &str,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        let p = self.signed(&req, method).await?;
        let mut c = req
            .into_inner()
            .command
            .ok_or_else(|| Status::invalid_argument("command missing"))?;
        if c.kind != kind {
            return Err(Status::invalid_argument(format!(
                "command kind must be {kind}"
            )));
        }
        self.may_site(&p, &c.site_id).await?;
        let now = crate::now_ms();
        if c.user_id != p.user_id || c.user_role != p.role.as_str() || c.user_key != p.key_hex {
            return Err(denied("command is not yours (user, role or key differ)"));
        }
        if (c.issued_ms - now).abs() > self.cfg.max_skew_secs * 1000
            || c.expires_ms <= now
            || c.expires_ms - c.issued_ms > CMD_TTL_MS
        {
            return Err(Status::invalid_argument("command times are out of range"));
        }
        if uuid::Uuid::parse_str(&c.id).is_err() {
            return Err(Status::invalid_argument("command id must be a uuid"));
        }
        if kind.starts_with("alert.") && c.args.get("alert_id").is_none_or(|a| a.is_empty()) {
            return Err(Status::invalid_argument("alert_id missing"));
        }
        let st: Option<String> = sqlx::query_scalar("SELECT status FROM sites WHERE id = $1")
            .bind(&c.site_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?;
        match st.as_deref() {
            None => return Err(Status::not_found("no such site")),
            Some("revoked") | Some("pending") => {
                return Err(Status::failed_precondition("site is not linked"))
            }
            _ => {}
        }
        // The user's signature must already be valid; adminsvr then signs as root.
        let uk = itcmdr_keys::parse_public_key(&c.user_key)
            .map_err(|_| Status::invalid_argument("bad user key"))?;
        let us: [u8; 64] = hex::decode(&c.user_sig)
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| Status::unauthenticated("bad command signature"))?;
        use ed25519_dalek::Verifier;
        uk.verify(
            &itcmdr_adminlink::command_bytes(&c),
            &ed25519_dalek::Signature::from_bytes(&us),
        )
        .map_err(|_| Status::unauthenticated("bad command signature"))?;
        itcmdr_adminlink::sign_root(&self.m.root, &mut c);
        sqlx::query("INSERT INTO commands (id, site_id, kind, requested_by, state, created_ms, body) VALUES ($1,$2,$3,$4,'pending',$5,$6)")
            .bind(&c.id).bind(&c.site_id).bind(&c.kind).bind(&p.user_id).bind(now).bind(c.encode_to_vec())
            .execute(&self.pool).await.map_err(|e| Status::already_exists(e.to_string()))?;
        self.audit(&p, &format!("command.{kind}"), &c.site_id, &c)
            .await?;
        self.hub.kick(&c.site_id);
        Ok(Response::new(pb::CommandInfo {
            id: c.id,
            site: c.site_id,
            kind: c.kind,
            state: "pending".into(),
            result_code: String::new(),
            requested_by: p.user_id,
            created_ms: now,
        }))
    }
}

pub fn command_row(r: &sqlx::postgres::PgRow) -> pb::CommandInfo {
    pb::CommandInfo {
        id: r.get("id"),
        site: r.get("site_id"),
        kind: r.get("kind"),
        state: r.get("state"),
        result_code: r.get("result_code"),
        requested_by: r.get("requested_by"),
        created_ms: r.get("created_ms"),
    }
}

pub async fn site_info(pool: &Pool, r: &sqlx::postgres::PgRow) -> pb::SiteInfo {
    let _ = pool;
    pb::SiteInfo {
        id: r.get("id"),
        name: r.get("name"),
        status: r.get("status"),
        version: r.get("version"),
        last_seen_ms: r.get("last_seen_ms"),
        share_display_names: r.get("share_names"),
        last_status: r
            .get::<Option<Vec<u8>>, _>("last_status")
            .and_then(|b| pb::SiteStatus::decode(b.as_slice()).ok()),
    }
}

#[tonic::async_trait]
impl pb::admin_api_server::AdminApi for Api {
    async fn info(&self, _: Request<pb::Empty>) -> Result<Response<pb::InfoResponse>, Status> {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(internal)?;
        Ok(Response::new(pb::InfoResponse {
            version: env!("CARGO_PKG_VERSION").into(),
            initialized: n > 0,
            root_public_key: self.m.root_hex(),
        }))
    }

    async fn enroll_device(
        &self,
        req: Request<pb::EnrollDeviceRequest>,
    ) -> Result<Response<pb::UserInfo>, Status> {
        let body = req.get_ref().encode_to_vec();
        let r = req.get_ref().clone();
        let dk = itcmdr_keys::parse_public_key(&r.device_public_key)
            .map_err(|_| Status::invalid_argument("bad device key"))?;
        let s = auth::check(
            &self.nonces,
            req.metadata(),
            &dk,
            "/itcmdr.admin.AdminApi/EnrollDevice",
            &body,
            self.cfg.max_skew_secs * 1000,
        )?;
        let mk = itcmdr_keys::parse_public_key(&r.master_public_key)
            .map_err(|_| Status::invalid_argument("bad master key"))?;
        let cert: itcmdr_keys::Certificate = serde_json::from_str(&r.certificate_json)
            .map_err(|_| Status::invalid_argument("bad certificate"))?;
        itcmdr_keys::verify_certificate(&mk, &cert)
            .map_err(|_| Status::unauthenticated("certificate is not signed by this master key"))?;
        if cert.fingerprint != itcmdr_keys::fingerprint(&dk) {
            return Err(Status::invalid_argument("certificate is for another key"));
        }
        let h = itcmdr_adminlink::sha256_hex(r.code.as_bytes());
        let mut tx = self.pool.begin().await.map_err(internal)?;
        let row = sqlx::query("SELECT user_id, expires_ms, used_ms FROM enroll_codes WHERE secret_hash = $1 FOR UPDATE")
            .bind(&h).fetch_optional(&mut *tx).await.map_err(internal)?
            .ok_or_else(|| Status::permission_denied("unknown enrollment code"))?;
        if row.get::<Option<i64>, _>("used_ms").is_some()
            || row.get::<i64, _>("expires_ms") < crate::now_ms()
        {
            return Err(Status::permission_denied("enrollment code used or expired"));
        }
        let uid: String = row.get("user_id");
        let u = sqlx::query("SELECT master_fp, role, created_by FROM users WHERE id = $1")
            .bind(&uid)
            .fetch_one(&mut *tx)
            .await
            .map_err(internal)?;
        let mfp = itcmdr_keys::fingerprint(&mk);
        if let Some(old) = u.get::<Option<String>, _>("master_fp") {
            if old != mfp {
                return Err(Status::permission_denied(
                    "this user already has another master key (use the 24 words)",
                ));
            }
        }
        let now = crate::now_ms();
        // The first owner (console init / recover-owner) is active at once; others wait for approval.
        let status = if u.get::<String, _>("created_by") == "console"
            && u.get::<String, _>("role") == "owner"
        {
            "active"
        } else {
            "pending"
        };
        sqlx::query("UPDATE enroll_codes SET used_ms = $1 WHERE secret_hash = $2")
            .bind(now)
            .bind(&h)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("UPDATE users SET master_pub = $1, master_fp = $2, key_backup_opt_out = $3 WHERE id = $4")
            .bind(&r.master_public_key).bind(&mfp).bind(r.key_backup.is_none()).bind(&uid).execute(&mut *tx).await.map_err(internal)?;
        sqlx::query("INSERT INTO devices (fingerprint, user_id, public_key, certificate, platform, status, created_ms) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(&cert.fingerprint).bind(&uid).bind(&r.device_public_key).bind(&r.certificate_json).bind(&r.platform).bind(status).bind(now)
            .execute(&mut *tx).await.map_err(|e| Status::already_exists(e.to_string()))?;
        if let Some(kb) = &r.key_backup {
            serde_json::from_str::<itcmdr_keys::wrap::Wrapped>(&kb.blob_json)
                .map_err(|_| Status::invalid_argument("bad key backup"))?;
            sqlx::query("INSERT INTO key_backups (user_id, blob, updated_ms) VALUES ($1,$2,$3) ON CONFLICT (user_id) DO UPDATE SET blob = EXCLUDED.blob, updated_ms = EXCLUDED.updated_ms")
                .bind(&uid).bind(&kb.blob_json).bind(now).execute(&mut *tx).await.map_err(internal)?;
        }
        tx.commit().await.map_err(internal)?;
        let p = Principal {
            user_id: uid.clone(),
            email: String::new(),
            role: Role::Sre,
            key_fp: cert.fingerprint.clone(),
            key_hex: r.device_public_key.clone(),
            via_master: false,
            sig: s.sig.to_string(),
        };
        let mut redacted = r.clone();
        redacted.code = itcmdr_adminlink::sha256_hex(r.code.as_bytes());
        self.audit(&p, "device.enroll", &uid, &redacted).await?;
        self.info(&uid).await
    }

    async fn who_am_i(&self, req: Request<pb::Empty>) -> Result<Response<pb::UserInfo>, Status> {
        let p = self
            .signed_opts(&req, "/itcmdr.admin.AdminApi/WhoAmI", true, true)
            .await?;
        self.info(&p.user_id).await
    }

    async fn list_users(&self, req: Request<pb::Empty>) -> Result<Response<pb::UserList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListUsers")
            .await?;
        let ids: Vec<String> = if p.role >= Role::Admin {
            sqlx::query_scalar("SELECT id FROM users ORDER BY created_ms")
                .fetch_all(&self.pool)
                .await
                .map_err(internal)?
        } else {
            vec![p.user_id.clone()] // SREs see only their own account
        };
        let mut users = vec![];
        for id in ids {
            users.push(user_info(&self.pool, &id).await.map_err(internal)?);
        }
        Ok(Response::new(pb::UserList { users }))
    }

    async fn create_user(
        &self,
        req: Request<pb::CreateUserRequest>,
    ) -> Result<Response<pb::CreateUserResponse>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/CreateUser")
            .await?;
        let r = req.get_ref();
        let role = if r.role.is_empty() {
            rbac::DEFAULT_ROLE
        } else {
            Role::parse(&r.role)
                .ok_or_else(|| Status::invalid_argument("role is owner, admin or sre"))?
        };
        rbac::can_create(p.role, role).map_err(denied)?;
        if !r.email.contains('@') || r.email.len() > 200 || r.name.trim().is_empty() {
            return Err(Status::invalid_argument("email and name are required"));
        }
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email, name, role, created_by, created_ms) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(&id).bind(r.email.trim()).bind(r.name.trim()).bind(role.as_str()).bind(&p.user_id).bind(crate::now_ms())
            .execute(&self.pool).await.map_err(|e| Status::already_exists(e.to_string()))?;
        self.audit(&p, "user.create", &id, r).await?;
        let (code, exp) = new_code(&self.pool, &self.cfg, &self.m, &id)
            .await
            .map_err(internal)?;
        Ok(Response::new(pb::CreateUserResponse {
            user: Some(user_info(&self.pool, &id).await.map_err(internal)?),
            enroll_code: code,
            expires_ms: exp,
        }))
    }

    async fn set_role(
        &self,
        req: Request<pb::SetRoleRequest>,
    ) -> Result<Response<pb::UserInfo>, Status> {
        let p = self.signed(&req, "/itcmdr.admin.AdminApi/SetRole").await?;
        let r = req.get_ref();
        let id = self.target(&r.user, &p).await?;
        let to = Role::parse(&r.role)
            .ok_or_else(|| Status::invalid_argument("role is owner, admin or sre"))?;
        let from = self.role_of(&id).await?;
        let owners = active_owners(&self.pool).await.map_err(internal)?;
        rbac::can_set_role(p.role, id == p.user_id, from, to, owners).map_err(denied)?;
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(to.as_str())
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        self.audit(&p, "user.role", &id, r).await?;
        self.info(&id).await
    }

    async fn set_user_status(
        &self,
        req: Request<pb::SetUserStatusRequest>,
    ) -> Result<Response<pb::UserInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/SetUserStatus")
            .await?;
        let r = req.get_ref();
        let id = self.target(&r.user, &p).await?;
        let role = self.role_of(&id).await?;
        if r.disabled {
            rbac::can_disable(
                p.role,
                id == p.user_id,
                role,
                active_owners(&self.pool).await.map_err(internal)?,
            )
            .map_err(denied)?;
        } else {
            rbac::can_edit(p.role, false, role).map_err(denied)?;
        }
        sqlx::query("UPDATE users SET status = $1 WHERE id = $2")
            .bind(if r.disabled { "disabled" } else { "active" })
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        self.audit(
            &p,
            if r.disabled {
                "user.disable"
            } else {
                "user.enable"
            },
            &id,
            r,
        )
        .await?;
        self.info(&id).await
    }

    async fn set_scopes(
        &self,
        req: Request<pb::SetScopesRequest>,
    ) -> Result<Response<pb::UserInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/SetScopes")
            .await?;
        let r = req.get_ref();
        rbac::require(p.role, Role::Admin).map_err(denied)?;
        let id = self.target(&r.user, &p).await?;
        rbac::can_edit(p.role, false, self.role_of(&id).await?).map_err(denied)?;
        let mut tx = self.pool.begin().await.map_err(internal)?;
        sqlx::query("DELETE FROM site_scopes WHERE user_id = $1")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        for s in &r.sites {
            sqlx::query("INSERT INTO site_scopes (user_id, site_id) VALUES ($1,$2)")
                .bind(&id)
                .bind(s)
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::not_found(format!("no such site {s}")))?;
        }
        tx.commit().await.map_err(internal)?;
        self.audit(&p, "user.scopes", &id, r).await?;
        self.info(&id).await
    }

    async fn approve_device(
        &self,
        req: Request<pb::ApproveDeviceRequest>,
    ) -> Result<Response<pb::UserInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ApproveDevice")
            .await?;
        let r = req.get_ref();
        rbac::require(p.role, Role::Admin).map_err(denied)?;
        let id = self.target(&r.user, &p).await?;
        if id == p.user_id {
            return Err(denied("another owner or admin approves your devices"));
        }
        rbac::can_edit(p.role, false, self.role_of(&id).await?).map_err(denied)?;
        let n = sqlx::query("UPDATE devices SET status = 'active' WHERE user_id = $1 AND fingerprint = $2 AND status = 'pending'")
            .bind(&id).bind(&r.device_fingerprint).execute(&self.pool).await.map_err(internal)?.rows_affected();
        if n == 0 {
            return Err(Status::not_found("no pending device with that fingerprint"));
        }
        self.audit(&p, "device.approve", &id, r).await?;
        self.info(&id).await
    }

    async fn new_enroll_code(
        &self,
        req: Request<pb::UserRef>,
    ) -> Result<Response<pb::CreateUserResponse>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/NewEnrollCode")
            .await?;
        let id = self.target(&req.get_ref().user, &p).await?;
        rbac::require(p.role, Role::Admin).map_err(denied)?;
        rbac::can_edit(p.role, false, self.role_of(&id).await?).map_err(denied)?;
        self.audit(&p, "user.code", &id, req.get_ref()).await?;
        let (code, exp) = new_code(&self.pool, &self.cfg, &self.m, &id)
            .await
            .map_err(internal)?;
        Ok(Response::new(pb::CreateUserResponse {
            user: Some(user_info(&self.pool, &id).await.map_err(internal)?),
            enroll_code: code,
            expires_ms: exp,
        }))
    }

    async fn list_devices(
        &self,
        req: Request<pb::UserRef>,
    ) -> Result<Response<pb::DeviceList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListDevices")
            .await?;
        let id = self.target(&req.get_ref().user, &p).await?;
        if id != p.user_id {
            rbac::require(p.role, Role::Admin).map_err(denied)?;
        }
        Ok(Response::new(pb::DeviceList {
            devices: devices(&self.pool, &id).await.map_err(internal)?,
        }))
    }

    async fn revoke_device(
        &self,
        req: Request<pb::RevokeDeviceRequest>,
    ) -> Result<Response<pb::DeviceList>, Status> {
        let p = self
            .signed_opts(&req, "/itcmdr.admin.AdminApi/RevokeDevice", true, false)
            .await?;
        let r = req.get_ref();
        let id = self.target(&r.user, &p).await?;
        rbac::can_edit(p.role, id == p.user_id, self.role_of(&id).await?).map_err(denied)?;
        let n = sqlx::query("UPDATE devices SET status = 'revoked' WHERE user_id = $1 AND fingerprint = $2 AND status <> 'revoked'")
            .bind(&id).bind(&r.device_fingerprint).execute(&self.pool).await.map_err(internal)?.rows_affected();
        if n == 0 {
            return Err(Status::not_found("no such device"));
        }
        self.audit(&p, "device.revoke", &id, r).await?;
        Ok(Response::new(pb::DeviceList {
            devices: devices(&self.pool, &id).await.map_err(internal)?,
        }))
    }

    async fn put_key_backup(
        &self,
        req: Request<pb::KeyBackup>,
    ) -> Result<Response<pb::Empty>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/PutKeyBackup")
            .await?;
        let r = req.get_ref();
        let w: itcmdr_keys::wrap::Wrapped = serde_json::from_str(&r.blob_json)
            .map_err(|_| Status::invalid_argument("bad key backup"))?;
        if !w.user.eq_ignore_ascii_case(&p.email) {
            return Err(Status::invalid_argument("key backup is for another user"));
        }
        sqlx::query("INSERT INTO key_backups (user_id, blob, updated_ms) VALUES ($1,$2,$3) ON CONFLICT (user_id) DO UPDATE SET blob = EXCLUDED.blob, updated_ms = EXCLUDED.updated_ms")
            .bind(&p.user_id).bind(&r.blob_json).bind(crate::now_ms()).execute(&self.pool).await.map_err(internal)?;
        sqlx::query("UPDATE users SET key_backup_opt_out = false WHERE id = $1")
            .bind(&p.user_id)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        self.audit(&p, "keybackup.put", &p.user_id, r).await?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn get_key_backup(
        &self,
        req: Request<pb::UserRef>,
    ) -> Result<Response<pb::KeyBackup>, Status> {
        let p = self
            .signed_opts(&req, "/itcmdr.admin.AdminApi/GetKeyBackup", true, false)
            .await?;
        let id = self.target(&req.get_ref().user, &p).await?;
        if id != p.user_id {
            return Err(denied("only the user can fetch their own key backup"));
        }
        let blob: String = sqlx::query_scalar("SELECT blob FROM key_backups WHERE user_id = $1")
            .bind(&id)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?
            .ok_or_else(|| Status::not_found("no key backup stored"))?;
        self.audit(&p, "keybackup.get", &id, req.get_ref()).await?;
        Ok(Response::new(pb::KeyBackup {
            user: id,
            blob_json: blob,
        }))
    }

    async fn set_key_backup_opt_out(
        &self,
        req: Request<pb::KeyBackupOptOut>,
    ) -> Result<Response<pb::UserInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/SetKeyBackupOptOut")
            .await?;
        let o = req.get_ref().opt_out;
        sqlx::query("UPDATE users SET key_backup_opt_out = $1 WHERE id = $2")
            .bind(o)
            .bind(&p.user_id)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        if o {
            sqlx::query("DELETE FROM key_backups WHERE user_id = $1")
                .bind(&p.user_id)
                .execute(&self.pool)
                .await
                .map_err(internal)?;
        }
        self.audit(
            &p,
            if o {
                "keybackup.opt-out"
            } else {
                "keybackup.opt-in"
            },
            &p.user_id,
            req.get_ref(),
        )
        .await?;
        self.info(&p.user_id).await
    }

    async fn add_device(
        &self,
        req: Request<pb::AddDeviceRequest>,
    ) -> Result<Response<pb::UserInfo>, Status> {
        let p = self
            .signed_opts(&req, "/itcmdr.admin.AdminApi/AddDevice", true, false)
            .await?;
        if !p.via_master {
            return Err(denied("sign AddDevice with the master key (24 words)"));
        }
        let r = req.get_ref();
        let dk = itcmdr_keys::parse_public_key(&r.device_public_key)
            .map_err(|_| Status::invalid_argument("bad device key"))?;
        let mk = itcmdr_keys::parse_public_key(&p.key_hex).map_err(internal)?;
        let cert: itcmdr_keys::Certificate = serde_json::from_str(&r.certificate_json)
            .map_err(|_| Status::invalid_argument("bad certificate"))?;
        itcmdr_keys::verify_certificate(&mk, &cert)
            .map_err(|_| Status::unauthenticated("certificate is not signed by your master key"))?;
        if cert.fingerprint != itcmdr_keys::fingerprint(&dk) {
            return Err(Status::invalid_argument("certificate is for another key"));
        }
        sqlx::query("INSERT INTO devices (fingerprint, user_id, public_key, certificate, platform, status, created_ms) VALUES ($1,$2,$3,$4,$5,'active',$6)")
            .bind(&cert.fingerprint).bind(&p.user_id).bind(&r.device_public_key).bind(&r.certificate_json).bind(&r.platform).bind(crate::now_ms())
            .execute(&self.pool).await.map_err(|e| Status::already_exists(e.to_string()))?;
        self.audit(&p, "device.add", &p.user_id, r).await?;
        self.info(&p.user_id).await
    }

    async fn create_site(
        &self,
        req: Request<pb::CreateSiteRequest>,
    ) -> Result<Response<pb::CreateSiteResponse>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/CreateSite")
            .await?;
        rbac::require(p.role, Role::Admin).map_err(denied)?;
        let name = req.get_ref().name.trim().to_string();
        if name.is_empty() || name.len() > 100 {
            return Err(Status::invalid_argument("site name is required"));
        }
        let id = format!("site-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
        let now = crate::now_ms();
        sqlx::query("INSERT INTO sites (id, name, status, created_ms) VALUES ($1,$2,'pending',$3)")
            .bind(&id)
            .bind(&name)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        let s = secret();
        let exp = now + CODE_TTL_MS;
        sqlx::query("INSERT INTO site_tokens (secret_hash, site_id, expires_ms) VALUES ($1,$2,$3)")
            .bind(itcmdr_adminlink::sha256_hex(s.as_bytes()))
            .bind(&id)
            .bind(exp)
            .execute(&self.pool)
            .await
            .map_err(internal)?;
        self.audit(&p, "site.create", &id, req.get_ref()).await?;
        let token = itcmdr_adminlink::SiteToken {
            v: 1,
            addr: self.cfg.public_site.clone(),
            pin: self.m.fingerprint.clone(),
            secret: s,
            root: self.m.root_hex(),
        }
        .encode();
        let r = sqlx::query("SELECT * FROM sites WHERE id = $1")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(internal)?;
        Ok(Response::new(pb::CreateSiteResponse {
            site: Some(site_info(&self.pool, &r).await),
            token,
            expires_ms: exp,
        }))
    }

    async fn list_sites(&self, req: Request<pb::Empty>) -> Result<Response<pb::SiteList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListSites")
            .await?;
        let rows = if p.role >= Role::Admin {
            sqlx::query("SELECT * FROM sites ORDER BY name").fetch_all(&self.pool).await
        } else {
            sqlx::query("SELECT s.* FROM sites s JOIN site_scopes x ON x.site_id = s.id WHERE x.user_id = $1 ORDER BY s.name").bind(&p.user_id).fetch_all(&self.pool).await
        }.map_err(internal)?;
        let mut sites = vec![];
        for r in &rows {
            sites.push(site_info(&self.pool, r).await);
        }
        Ok(Response::new(pb::SiteList { sites }))
    }

    async fn revoke_site(
        &self,
        req: Request<pb::SiteRef>,
    ) -> Result<Response<pb::SiteInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/RevokeSite")
            .await?;
        rbac::require(p.role, Role::Admin).map_err(denied)?;
        let id = req.get_ref().site.clone();
        let n = sqlx::query("UPDATE sites SET status = 'revoked' WHERE id = $1")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(internal)?
            .rows_affected();
        if n == 0 {
            return Err(Status::not_found("no such site"));
        }
        self.hub.drop_site(&id);
        crate::link::refresh_revoked(&self.pool).await;
        self.audit(&p, "site.revoke", &id, req.get_ref()).await?;
        let r = sqlx::query("SELECT * FROM sites WHERE id = $1")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(internal)?;
        Ok(Response::new(site_info(&self.pool, &r).await))
    }

    async fn list_alerts(
        &self,
        req: Request<pb::AlertFilter>,
    ) -> Result<Response<pb::AlertList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListAlerts")
            .await?;
        let f = req.get_ref();
        if !f.site.is_empty() {
            self.may_site(&p, &f.site).await?;
        }
        let limit = if f.limit <= 0 { 200 } else { f.limit.min(5000) } as i64;
        let state = if f.state.is_empty() {
            "open"
        } else {
            f.state.as_str()
        };
        let rows = sqlx::query(
            "SELECT a.* FROM alerts a WHERE ($1 = '' OR a.site_id = $1) \
             AND ($2 = 'all' OR ($2 = 'open' AND a.state IN ('open','acked','silenced')) OR a.state = $2) \
             AND ($3 OR a.site_id IN (SELECT site_id FROM site_scopes WHERE user_id = $4)) \
             ORDER BY CASE a.severity WHEN 'critical' THEN 0 WHEN 'cautious' THEN 1 ELSE 2 END, a.last_seen_ms DESC LIMIT $5",
        )
        .bind(&f.site).bind(state).bind(p.role >= Role::Admin).bind(&p.user_id).bind(limit)
        .fetch_all(&self.pool).await.map_err(internal)?;
        let alerts = rows
            .iter()
            .map(|r| pb::AlertInfo {
                site: r.get("site_id"),
                alert: pb::SiteAlert::decode(r.get::<Vec<u8>, _>("body").as_slice()).ok(),
                origin: r.get("origin"),
            })
            .collect();
        Ok(Response::new(pb::AlertList { alerts }))
    }

    async fn ack_alert(
        &self,
        req: Request<pb::CommandRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        self.command(req, "/itcmdr.admin.AdminApi/AckAlert", "alert.ack")
            .await
    }
    async fn resolve_alert(
        &self,
        req: Request<pb::CommandRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        self.command(req, "/itcmdr.admin.AdminApi/ResolveAlert", "alert.resolve")
            .await
    }
    async fn start_backup(
        &self,
        req: Request<pb::CommandRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        self.command(req, "/itcmdr.admin.AdminApi/StartBackup", "backup.start")
            .await
    }

    async fn list_commands(
        &self,
        req: Request<pb::SiteRef>,
    ) -> Result<Response<pb::CommandList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListCommands")
            .await?;
        let site = req.get_ref().site.clone();
        if !site.is_empty() {
            self.may_site(&p, &site).await?;
        }
        let rows = sqlx::query(
            "SELECT * FROM commands WHERE ($1 = '' OR site_id = $1) AND ($2 OR site_id IN (SELECT site_id FROM site_scopes WHERE user_id = $3)) ORDER BY created_ms DESC LIMIT 200",
        ).bind(&site).bind(p.role >= Role::Admin).bind(&p.user_id).fetch_all(&self.pool).await.map_err(internal)?;
        Ok(Response::new(pb::CommandList {
            commands: rows.iter().map(command_row).collect(),
        }))
    }

    async fn list_audit(
        &self,
        req: Request<pb::AuditFilter>,
    ) -> Result<Response<pb::AuditList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListAudit")
            .await?;
        let f = req.get_ref();
        let limit = if f.limit <= 0 { 200 } else { f.limit.min(5000) } as i64;
        let actor = if p.role < Role::Admin {
            Some(p.user_id.clone())
        } else if f.user.is_empty() {
            None
        } else {
            Some(self.target(&f.user, &p).await?)
        };
        Ok(Response::new(pb::AuditList {
            entries: audit::list(&self.pool, actor.as_deref(), limit)
                .await
                .map_err(internal)?,
        }))
    }

    async fn verify_audit(
        &self,
        req: Request<pb::Empty>,
    ) -> Result<Response<pb::AuditVerify>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/VerifyAudit")
            .await?;
        rbac::require(p.role, Role::Admin).map_err(denied)?;
        audit::verify(&self.pool)
            .await
            .map(Response::new)
            .map_err(internal)
    }
}

/// Serve the admin API over pinned TLS on `cfg.api_addr`.
pub async fn serve(
    pool: Pool,
    cfg: Config,
    m: Material,
    hub: Hub,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let addr: std::net::SocketAddr = cfg.api_addr.parse().context("api_addr")?;
    let tls_cfg =
        itcmdr_agentlink::tls::server_config(&m.server, &m.ca.cert, Arc::new(|_: &str| false))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("bind api_addr")?;
    tracing::info!(%addr, fingerprint=%m.fingerprint, "admin API listening");
    let on_fail = Arc::new(
        |peer: std::net::SocketAddr, e: String| tracing::debug!(peer=%peer.ip(), error=%e, "api handshake failed"),
    );
    tonic::transport::Server::builder()
        .add_service(pb::admin_api_server::AdminApiServer::new(Api::new(
            pool, cfg, m, hub,
        )))
        .serve_with_incoming_shutdown(
            itcmdr_agentlink::tls::incoming(listener, tls_cfg, on_fail),
            shutdown,
        )
        .await?;
    Ok(())
}
