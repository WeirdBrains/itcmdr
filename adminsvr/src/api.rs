//! The admin API (CLI and app). Every call except Info is signed; every change is audited.
use crate::audit::{self, Entry};
use crate::auth::{self, Nonces, Principal};
use crate::config::Config;
use crate::link::Hub;
use crate::material::Material;
use crate::pb;
use crate::rbac::{self, Role};
use crate::schema::*;
use crate::store::Pool;
use anyhow::{anyhow, Context, Result};
use diesel::prelude::*;
use prost::Message;
use rand::RngCore;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub const CODE_TTL_MS: i64 = 24 * 3600 * 1000;
pub const CMD_TTL_MS: i64 = 15 * 60 * 1000;
/// Two-person commands wait for the second person: they may live a day (v0.20.0).
pub const APPROVAL_TTL_MS: i64 = 24 * 3600 * 1000;
/// Kinds SendCommand takes (the alert and backup kinds keep their own RPCs too).
pub const SEND_KINDS: &[&str] = &[
    "setting.set",
    "schedule.run",
    "schedule.pause",
    "schedule.resume",
    "plugin.enable",
    "plugin.disable",
    "plugin.install",
    "backup.start",
    "alert.ack",
    "alert.resolve",
    "release.rollout",
];
/// Release package names (v0.21.0).
pub const RELEASES: &[&str] = &["itcmdr-agent-release", "itcmdr-server-release"];
/// Largest catalog package (MiB).
pub const MAX_PACKAGE_MIB: usize = 256;

#[derive(Clone)]
pub struct Api {
    pool: Pool,
    cfg: Config,
    m: Material,
    hub: Hub,
    nonces: Arc<Nonces>,
    pk: Arc<crate::passkeys::Passkeys>,
}

fn internal(e: impl std::fmt::Display) -> Status {
    Status::internal(e.to_string())
}
fn denied(e: impl std::fmt::Display) -> Status {
    Status::permission_denied(e.to_string())
}
/// Database errors: unique/foreign-key violations become already_exists/not_found.
fn dberr(e: anyhow::Error) -> Status {
    use diesel::result::{DatabaseErrorKind as K, Error as E};
    match e.downcast_ref::<E>() {
        Some(E::DatabaseError(K::UniqueViolation, m)) => {
            Status::already_exists(m.message().to_string())
        }
        Some(E::DatabaseError(K::ForeignKeyViolation, m)) => {
            Status::not_found(m.message().to_string())
        }
        Some(E::NotFound) => Status::not_found("not found"),
        _ => internal(e),
    }
}
diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

pub fn secret() -> String {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}

/// A refusal inside a database closure (becomes permission_denied).
#[derive(Debug)]
pub struct Refuse(pub &'static str);
impl std::fmt::Display for Refuse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Refuse {}

/// Store or replace a user's key backup (no upsert: MultiConnection has none).
pub fn put_backup(c: &mut crate::store::Db, uid: &str, blob: &str, now: i64) -> Result<()> {
    let n = diesel::update(key_backups::table.filter(key_backups::user_id.eq(uid)))
        .set((key_backups::blob.eq(blob), key_backups::updated_ms.eq(now)))
        .execute(c)?;
    if n == 0 {
        diesel::insert_into(key_backups::table)
            .values((
                key_backups::user_id.eq(uid),
                key_backups::blob.eq(blob),
                key_backups::updated_ms.eq(now),
            ))
            .execute(c)?;
    }
    Ok(())
}

pub async fn find_user(pool: &Pool, r: &str) -> Result<Option<String>> {
    let r = r.to_string();
    pool.run(move |c| {
        Ok(users::table
            .filter(
                users::id
                    .eq(&r)
                    .or(lower(users::email).eq(r.to_lowercase())),
            )
            .select(users::id)
            .first(c)
            .optional()?)
    })
    .await
}

pub async fn active_owners(pool: &Pool) -> Result<i64> {
    pool.run(|c| {
        Ok(users::table
            .filter(users::role.eq("owner").and(users::status.eq("active")))
            .count()
            .get_result(c)?)
    })
    .await
}

pub async fn user_info(pool: &Pool, id: &str) -> Result<pb::UserInfo> {
    let i = id.to_string();
    let (u, kb, sites, pks) = pool
        .run(move |c| {
            let u: (
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                bool,
                i64,
            ) = users::table
                .filter(users::id.eq(&i))
                .select((
                    users::id,
                    users::email,
                    users::name,
                    users::role,
                    users::status,
                    users::master_fp,
                    users::key_backup_opt_out,
                    users::created_ms,
                ))
                .first(c)
                .optional()?
                .ok_or_else(|| anyhow!("no such user"))?;
            let kb: i64 = key_backups::table
                .filter(key_backups::user_id.eq(&i))
                .count()
                .get_result(c)?;
            let pks: i64 = passkeys::table
                .filter(passkeys::user_id.eq(&i))
                .count()
                .get_result(c)?;
            let sites: Vec<String> = site_scopes::table
                .filter(site_scopes::user_id.eq(&i))
                .select(site_scopes::site_id)
                .order(site_scopes::site_id)
                .load(c)?;
            Ok((u, kb, sites, pks))
        })
        .await?;
    Ok(pb::UserInfo {
        id: u.0,
        email: u.1,
        name: u.2,
        role: u.3,
        status: u.4,
        sites,
        key_backup: kb > 0,
        key_backup_opt_out: u.6,
        created_ms: u.7,
        devices: devices(pool, id).await?,
        master_fingerprint: u.5.unwrap_or_default(),
        passkeys: pks as i32,
    })
}

async fn devices(pool: &Pool, id: &str) -> Result<Vec<pb::DeviceInfo>> {
    let i = id.to_string();
    pool.run(move |c| {
        let rows: Vec<(String, String, String, i64, i64)> = devices::table
            .filter(devices::user_id.eq(&i))
            .order(devices::created_ms)
            .select((
                devices::fingerprint,
                devices::platform,
                devices::status,
                devices::created_ms,
                devices::last_used_ms,
            ))
            .load(c)?;
        Ok(rows
            .into_iter()
            .map(|r| pb::DeviceInfo {
                fingerprint: r.0,
                platform: r.1,
                status: r.2,
                created_ms: r.3,
                last_used_ms: r.4,
            })
            .collect())
    })
    .await
}

/// A fresh one-time enrollment code for `user_id`.
pub async fn new_code(
    pool: &Pool,
    cfg: &Config,
    m: &Material,
    user_id: &str,
) -> Result<(String, i64)> {
    let s = secret();
    let exp = crate::now_ms() + CODE_TTL_MS;
    let (uid, h) = (
        user_id.to_string(),
        itcmdr_adminlink::sha256_hex(s.as_bytes()),
    );
    let email: String = pool
        .run(move |c| {
            let e: String = users::table
                .filter(users::id.eq(&uid))
                .select(users::email)
                .first(c)?;
            diesel::insert_into(enroll_codes::table)
                .values((
                    enroll_codes::secret_hash.eq(h),
                    enroll_codes::user_id.eq(&uid),
                    enroll_codes::expires_ms.eq(exp),
                ))
                .execute(c)?;
            Ok(e)
        })
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
    let id = uuid::Uuid::new_v4().to_string();
    let (i, e, n) = (id.clone(), email.to_string(), name.to_string());
    pool.run(move |c| {
        let k: i64 = users::table.count().get_result(c)?;
        if k > 0 {
            anyhow::bail!("already initialized (users exist); use recover-owner at the console if every owner is lost");
        }
        diesel::insert_into(users::table)
            .values((users::id.eq(i), users::email.eq(e), users::name.eq(n), users::role.eq("owner"), users::created_by.eq("console"), users::created_ms.eq(crate::now_ms())))
            .execute(c)?;
        Ok(())
    })
    .await?;
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
    let i = id.clone();
    pool.run(move |c| {
        c.transaction::<_, anyhow::Error, _>(|c| {
            diesel::update(users::table.filter(users::id.eq(&i)))
                .set((
                    users::role.eq("owner"),
                    users::status.eq("active"),
                    users::master_pub.eq(None::<String>),
                    users::master_fp.eq(None::<String>),
                ))
                .execute(c)?;
            diesel::update(devices::table.filter(devices::user_id.eq(&i)))
                .set(devices::status.eq("revoked"))
                .execute(c)?;
            // The lost owner's passkeys go too (24 words + console = the last owner's reset).
            crate::passkeys::remove_all(c, &i)?;
            Ok(())
        })
    })
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
            pk: Arc::new(crate::passkeys::Passkeys {
                pool: pool.clone(),
                cfg: cfg.clone(),
                ch: Arc::default(),
            }),
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
        let p = auth::verify(
            &self.pool,
            &self.nonces,
            req.metadata(),
            method,
            &body,
            self.cfg.max_skew_secs * 1000,
            master,
            pending,
        )
        .await?;
        self.pk.check(req.metadata(), method, &body, &p).await?;
        Ok(p)
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
        let i = id.to_string();
        let r: String = self
            .pool
            .run(move |c| {
                Ok(users::table
                    .filter(users::id.eq(i))
                    .select(users::role)
                    .first(c)?)
            })
            .await
            .map_err(dberr)?;
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
        let (u, st) = (p.user_id.clone(), site.to_string());
        let n: i64 = self
            .pool
            .run(move |c| {
                Ok(site_scopes::table
                    .filter(site_scopes::user_id.eq(u).and(site_scopes::site_id.eq(st)))
                    .count()
                    .get_result(c)?)
            })
            .await
            .map_err(dberr)?;
        if n == 0 {
            return Err(denied("this site is not assigned to you"));
        }
        Ok(())
    }

    async fn command_by_id(&self, id: &str) -> Result<CommandRow, Status> {
        let i = id.to_string();
        self.pool
            .run(move |c| {
                Ok(commands::table
                    .filter(commands::id.eq(i))
                    .select(command_cols())
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(dberr)?
            .ok_or_else(|| Status::not_found("no such command"))
    }

    async fn freeze_state(&self) -> Result<pb::FreezeState, Status> {
        let v: Option<String> = self
            .pool
            .run(|c| {
                Ok(meta::table
                    .filter(meta::k.eq("freeze"))
                    .select(meta::v)
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(dberr)?;
        let v = v.unwrap_or_default();
        let mut it = v.split('|');
        Ok(pb::FreezeState {
            frozen: it.next() == Some("1"),
            by: it.next().unwrap_or("").to_string(),
            since_ms: it.next().and_then(|x| x.parse().ok()).unwrap_or(0),
        })
    }

    /// v0.21.0: the organization's "require a second approver" setting (default off).
    pub async fn second_required(&self) -> Result<bool, Status> {
        let v: Option<String> = self
            .pool
            .run(|c| {
                Ok(meta::table
                    .filter(meta::k.eq("org.require_second_approver"))
                    .select(meta::v)
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(dberr)?;
        Ok(v.as_deref() == Some("true"))
    }

    async fn org_policy(&self) -> Result<pb::OrgPolicy, Status> {
        let rows: Vec<(String, String)> = self
            .pool
            .run(|c| Ok(meta::table.filter(meta::k.like("org.%")).load(c)?))
            .await
            .map_err(dberr)?;
        let get = |k: &str| rows.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        Ok(pb::OrgPolicy {
            require_second_approver: get("org.require_second_approver").as_deref() == Some("true"),
            updated_by: get("org.policy_updated_by").unwrap_or_default(),
            updated_ms: get("org.policy_updated_ms")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        })
    }

    async fn release_approvals(&self, sha: &str) -> Result<Vec<pb::ReleaseApproval>, Status> {
        let s = sha.to_string();
        let rows: Vec<(String, String, String, String, String, i64)> = self
            .pool
            .run(move |c| {
                Ok(release_approvals::table
                    .filter(release_approvals::sha256.eq(s))
                    .select((
                        release_approvals::user_id,
                        release_approvals::role,
                        release_approvals::pubkey,
                        release_approvals::endorsement,
                        release_approvals::sig,
                        release_approvals::ts_ms,
                    ))
                    .order(release_approvals::ts_ms.asc())
                    .load(c)?)
            })
            .await
            .map_err(dberr)?;
        Ok(rows
            .into_iter()
            .map(
                |(user_id, role, key, endorsement, sig, ts_ms)| pb::ReleaseApproval {
                    user_id,
                    role,
                    key,
                    endorsement,
                    sig,
                    ts_ms,
                },
            )
            .collect())
    }

    async fn release_state(&self, sha: &str) -> Result<pb::ReleaseState, Status> {
        let s = sha.to_string();
        let row: Option<(String, String)> = self
            .pool
            .run(move |c| {
                Ok(catalog::table
                    .filter(catalog::sha256.eq(s))
                    .select((catalog::name, catalog::version))
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(dberr)?;
        let Some((name, version)) = row else {
            return Err(Status::not_found("no such package in the catalog"));
        };
        if !RELEASES.contains(&name.as_str()) {
            return Err(Status::invalid_argument(format!(
                "{name} is not a release package"
            )));
        }
        let needed = if self.second_required().await? { 2 } else { 1 };
        let root = self.m.root.verifying_key();
        let ap = self.release_approvals(sha).await?;
        let ok =
            itcmdr_adminlink::check_release_approvals(&root, &ap, &name, &version, sha, needed)
                .is_ok();
        Ok(pb::ReleaseState {
            sha256: sha.into(),
            name,
            version,
            approved_by: ap.into_iter().map(|a| a.user_id).collect(),
            needed: needed as i32,
            approved: ok,
        })
    }

    async fn command(
        &self,
        req: Request<pb::CommandRequest>,
        method: &str,
        kinds: &[&str],
    ) -> Result<Response<pb::CommandInfo>, Status> {
        let p = self.signed(&req, method).await?;
        let mut c = req
            .into_inner()
            .command
            .ok_or_else(|| Status::invalid_argument("command missing"))?;
        if !kinds.contains(&c.kind.as_str()) {
            return Err(Status::invalid_argument(format!(
                "command kind must be one of {}",
                kinds.join(", ")
            )));
        }
        let kind = c.kind.clone();
        let kind = kind.as_str();
        if crate::link::frozen(&self.pool).await.map_err(dberr)? {
            return Err(Status::failed_precondition(
                "frozen: an owner has frozen all commands (GetFreeze)",
            ));
        }
        if !c.approver_id.is_empty() || !c.approver_sig.is_empty() {
            return Err(Status::invalid_argument(
                "approver fields are filled by ApproveCommand",
            ));
        }
        if !c.release_approvals.is_empty() {
            return Err(Status::invalid_argument(
                "release approvals are attached by adminsvr (ApproveRelease)",
            ));
        }
        let second = self.second_required().await?;
        let gated = itcmdr_adminlink::TWO_PERSON.contains(&kind) || kind == "release.rollout";
        if c.two_person != (second && gated) {
            return Err(Status::invalid_argument(format!(
                "two_person must be {} (the organization's second-approver setting, GetOrgPolicy)",
                second && gated
            )));
        }
        let two = !c.dry_run && second && itcmdr_adminlink::TWO_PERSON.contains(&kind);
        if kind == "release.rollout" && p.role < Role::Admin {
            return Err(denied("only owners and admins roll releases out"));
        }
        self.may_site(&p, &c.site_id).await?;
        let now = crate::now_ms();
        if c.user_id != p.user_id || c.user_role != p.role.as_str() || c.user_key != p.key_hex {
            return Err(denied("command is not yours (user, role or key differ)"));
        }
        let ttl = if two { APPROVAL_TTL_MS } else { CMD_TTL_MS };
        if (c.issued_ms - now).abs() > self.cfg.max_skew_secs * 1000
            || c.expires_ms <= now
            || c.expires_ms - c.issued_ms > ttl
        {
            return Err(Status::invalid_argument("command times are out of range"));
        }
        if uuid::Uuid::parse_str(&c.id).is_err() {
            return Err(Status::invalid_argument("command id must be a uuid"));
        }
        if kind.starts_with("alert.") && c.args.get("alert_id").is_none_or(|a| a.is_empty()) {
            return Err(Status::invalid_argument("alert_id missing"));
        }
        let need = |k: &str| c.args.get(k).is_none_or(|a| a.is_empty() || a.len() > 512);
        let missing = match kind {
            "setting.set" => need("key") || !c.args.contains_key("value"),
            "schedule.run" | "schedule.pause" | "schedule.resume" => need("schedule"),
            "plugin.enable" | "plugin.disable" => need("name"),
            "plugin.install" => need("sha256"),
            "release.rollout" => need("sha256") || need("name") || need("version"),
            _ => false,
        };
        if missing || c.args.len() > 8 {
            return Err(Status::invalid_argument(format!(
                "arguments for {kind} are missing or too long"
            )));
        }
        if kind == "plugin.install" {
            let sha = c.args["sha256"].to_lowercase();
            let n: i64 = self
                .pool
                .run(move |db| {
                    Ok(catalog::table
                        .filter(catalog::sha256.eq(sha))
                        .count()
                        .get_result(db)?)
                })
                .await
                .map_err(dberr)?;
            if n == 0 {
                return Err(Status::not_found("no such package in the catalog"));
            }
        }
        let sid = c.site_id.clone();
        let st: Option<String> = self
            .pool
            .run(move |c| {
                Ok(sites::table
                    .filter(sites::id.eq(sid))
                    .select(sites::status)
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(dberr)?;
        match st.as_deref() {
            None => return Err(Status::not_found("no such site")),
            Some("revoked") | Some("pending") => {
                return Err(Status::failed_precondition("site is not linked"))
            }
            _ => {}
        }
        if kind == "release.rollout" {
            let sha = c.args["sha256"].to_lowercase();
            let st = self.release_state(&sha).await?;
            if st.name != c.args["name"] || st.version != c.args["version"] {
                return Err(Status::invalid_argument(
                    "name/version do not match the catalog package",
                ));
            }
            if !st.approved {
                return Err(Status::failed_precondition(format!(
                    "{} {} has {} of {} approvals (ApproveRelease)",
                    st.name,
                    st.version,
                    st.approved_by.len(),
                    st.needed
                )));
            }
            c.release_approvals = self.release_approvals(&sha).await?;
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
        let (cc, uid) = (c.clone(), p.user_id.clone());
        self.pool
            .run(move |db| {
                diesel::insert_into(commands::table)
                    .values((
                        commands::id.eq(&cc.id),
                        commands::site_id.eq(&cc.site_id),
                        commands::kind.eq(&cc.kind),
                        commands::requested_by.eq(uid),
                        commands::state.eq(if two { "awaiting-approval" } else { "pending" }),
                        commands::created_ms.eq(now),
                        commands::body.eq(cc.encode_to_vec()),
                        commands::dry_run.eq(cc.dry_run),
                    ))
                    .execute(db)?;
                Ok(())
            })
            .await
            .map_err(dberr)?;
        let action = format!("command.{kind}{}", if c.dry_run { ".dry-run" } else { "" });
        self.audit(&p, &action, &c.site_id, &c).await?;
        if !two {
            self.hub.kick(&c.site_id);
        }
        Ok(Response::new(pb::CommandInfo {
            id: c.id,
            site: c.site_id,
            kind: c.kind,
            state: if two { "awaiting-approval" } else { "pending" }.into(),
            result_code: String::new(),
            requested_by: p.user_id,
            created_ms: now,
            dry_run: c.dry_run,
            args: c.args,
            ..Default::default()
        }))
    }
}

pub type CommandRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    bool,
    Option<Vec<u8>>,
    String,
    Vec<u8>,
);
pub fn command_cols() -> (
    commands::id,
    commands::site_id,
    commands::kind,
    commands::state,
    commands::result_code,
    commands::requested_by,
    commands::created_ms,
    commands::dry_run,
    commands::preview,
    commands::approved_by,
    commands::body,
) {
    (
        commands::id,
        commands::site_id,
        commands::kind,
        commands::state,
        commands::result_code,
        commands::requested_by,
        commands::created_ms,
        commands::dry_run,
        commands::preview,
        commands::approved_by,
        commands::body,
    )
}
pub fn command_row(r: CommandRow) -> pb::CommandInfo {
    pb::CommandInfo {
        id: r.0,
        site: r.1,
        kind: r.2,
        state: r.3,
        result_code: r.4,
        requested_by: r.5,
        created_ms: r.6,
        dry_run: r.7,
        preview: r
            .8
            .and_then(|b| pb::CommandResult::decode(b.as_slice()).ok())
            .map(|x| x.preview)
            .unwrap_or_default(),
        approved_by: r.9,
        args: pb::Command::decode(r.10.as_slice())
            .map(|c| c.args)
            .unwrap_or_default(),
    }
}

pub type SiteRow = (String, String, String, String, i64, bool, Option<Vec<u8>>);
pub fn site_cols() -> (
    sites::id,
    sites::name,
    sites::status,
    sites::version,
    sites::last_seen_ms,
    sites::share_names,
    sites::last_status,
) {
    (
        sites::id,
        sites::name,
        sites::status,
        sites::version,
        sites::last_seen_ms,
        sites::share_names,
        sites::last_status,
    )
}
pub fn site_info(r: SiteRow) -> pb::SiteInfo {
    pb::SiteInfo {
        id: r.0,
        name: r.1,
        status: r.2,
        version: r.3,
        last_seen_ms: r.4,
        share_display_names: r.5,
        last_status: r.6.and_then(|b| pb::SiteStatus::decode(b.as_slice()).ok()),
    }
}

async fn get_site(pool: &Pool, id: &str) -> Result<pb::SiteInfo, Status> {
    let id = id.to_string();
    pool.run(move |c| {
        Ok(sites::table
            .filter(sites::id.eq(id))
            .select(site_cols())
            .first::<SiteRow>(c)?)
    })
    .await
    .map(site_info)
    .map_err(dberr)
}

#[tonic::async_trait]
impl pb::admin_api_server::AdminApi for Api {
    async fn info(&self, _: Request<pb::Empty>) -> Result<Response<pb::InfoResponse>, Status> {
        let n: i64 = self
            .pool
            .run(|c| Ok(users::table.count().get_result(c)?))
            .await
            .map_err(dberr)?;
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
        if let Some(kb) = &r.key_backup {
            serde_json::from_str::<itcmdr_keys::wrap::Wrapped>(&kb.blob_json)
                .map_err(|_| Status::invalid_argument("bad key backup"))?;
        }
        let mfp = itcmdr_keys::fingerprint(&mk);
        let rr = r.clone();
        let cfp = cert.fingerprint.clone();
        let uid: String = self
            .pool
            .run(move |c| {
                c.transaction::<_, anyhow::Error, _>(|c| {
                    let now = crate::now_ms();
                    let row: Option<(String, i64, Option<i64>)> = enroll_codes::table
                        .filter(enroll_codes::secret_hash.eq(&h))
                        .select((
                            enroll_codes::user_id,
                            enroll_codes::expires_ms,
                            enroll_codes::used_ms,
                        ))
                        .first(c)
                        .optional()?;
                    let (uid, exp, used) =
                        row.ok_or_else(|| anyhow!(Refuse("unknown enrollment code")))?;
                    if used.is_some() || exp < now {
                        return Err(anyhow!(Refuse("enrollment code used or expired")));
                    }
                    let (old, role, by): (Option<String>, String, String) = users::table
                        .filter(users::id.eq(&uid))
                        .select((users::master_fp, users::role, users::created_by))
                        .first(c)?;
                    if old.is_some_and(|o| o != mfp) {
                        return Err(anyhow!(Refuse(
                            "this user already has another master key (use the 24 words)"
                        )));
                    }
                    // The first owner (console init / recover-owner) is active at once; others wait for approval.
                    let status = if by == "console" && role == "owner" {
                        "active"
                    } else {
                        "pending"
                    };
                    // Single use even under concurrency: only one UPDATE can see used_ms NULL.
                    let n = diesel::update(
                        enroll_codes::table.filter(
                            enroll_codes::secret_hash
                                .eq(&h)
                                .and(enroll_codes::used_ms.is_null()),
                        ),
                    )
                    .set(enroll_codes::used_ms.eq(now))
                    .execute(c)?;
                    if n != 1 {
                        return Err(anyhow!(Refuse("enrollment code used or expired")));
                    }
                    diesel::update(users::table.filter(users::id.eq(&uid)))
                        .set((
                            users::master_pub.eq(&rr.master_public_key),
                            users::master_fp.eq(&mfp),
                            users::key_backup_opt_out.eq(rr.key_backup.is_none()),
                        ))
                        .execute(c)?;
                    diesel::insert_into(devices::table)
                        .values((
                            devices::fingerprint.eq(&cfp),
                            devices::user_id.eq(&uid),
                            devices::public_key.eq(&rr.device_public_key),
                            devices::certificate.eq(&rr.certificate_json),
                            devices::platform.eq(&rr.platform),
                            devices::status.eq(status),
                            devices::created_ms.eq(now),
                        ))
                        .execute(c)?;
                    if let Some(kb) = &rr.key_backup {
                        put_backup(c, &uid, &kb.blob_json, now)?;
                    }
                    Ok(uid)
                })
            })
            .await
            .map_err(|e| match e.downcast_ref::<Refuse>() {
                Some(Refuse(m)) => Status::permission_denied(*m),
                None => dberr(e),
            })?;
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
            self.pool
                .run(|c| {
                    Ok(users::table
                        .order(users::created_ms)
                        .select(users::id)
                        .load(c)?)
                })
                .await
                .map_err(dberr)?
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
        let (i, e, n, by) = (
            id.clone(),
            r.email.trim().to_string(),
            r.name.trim().to_string(),
            p.user_id.clone(),
        );
        let lc = e.to_lowercase();
        self.pool
            .run(move |c| {
                let dup: i64 = users::table
                    .filter(lower(users::email).eq(lc))
                    .count()
                    .get_result(c)?;
                if dup > 0 {
                    return Err(anyhow!(diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        Box::new("a user with this email exists".to_string()),
                    )));
                }
                diesel::insert_into(users::table)
                    .values((
                        users::id.eq(i),
                        users::email.eq(e),
                        users::name.eq(n),
                        users::role.eq(role.as_str()),
                        users::created_by.eq(by),
                        users::created_ms.eq(crate::now_ms()),
                    ))
                    .execute(c)?;
                Ok(())
            })
            .await
            .map_err(dberr)?;
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
        let i = id.clone();
        self.pool
            .run(move |c| {
                Ok(diesel::update(users::table.filter(users::id.eq(i)))
                    .set(users::role.eq(to.as_str()))
                    .execute(c)?)
            })
            .await
            .map_err(dberr)?;
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
        let (i, st) = (id.clone(), if r.disabled { "disabled" } else { "active" });
        self.pool
            .run(move |c| {
                Ok(diesel::update(users::table.filter(users::id.eq(i)))
                    .set(users::status.eq(st))
                    .execute(c)?)
            })
            .await
            .map_err(dberr)?;
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
        let (i, list) = (id.clone(), r.sites.clone());
        self.pool
            .run(move |c| {
                c.transaction::<_, anyhow::Error, _>(|c| {
                    diesel::delete(site_scopes::table.filter(site_scopes::user_id.eq(&i)))
                        .execute(c)?;
                    for s in &list {
                        let n: i64 = sites::table.filter(sites::id.eq(s)).count().get_result(c)?;
                        if n == 0 {
                            return Err(anyhow!(diesel::result::Error::NotFound));
                        }
                        diesel::insert_into(site_scopes::table)
                            .values((site_scopes::user_id.eq(&i), site_scopes::site_id.eq(s)))
                            .execute(c)?;
                    }
                    Ok(())
                })
            })
            .await
            .map_err(|_| Status::not_found("no such site"))?;
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
        // v0.21.0: with the organization's second-approver setting on, someone else approves your
        // devices; off (default), you may approve your own new device from an active one.
        if id == p.user_id && self.second_required().await? {
            return Err(denied("another owner or admin approves your devices"));
        }
        rbac::can_edit(p.role, false, self.role_of(&id).await?).map_err(denied)?;
        let (i, f) = (id.clone(), r.device_fingerprint.clone());
        let n = self
            .pool
            .run(move |c| {
                Ok(diesel::update(
                    devices::table.filter(
                        devices::user_id
                            .eq(i)
                            .and(devices::fingerprint.eq(f))
                            .and(devices::status.eq("pending")),
                    ),
                )
                .set(devices::status.eq("active"))
                .execute(c)?)
            })
            .await
            .map_err(dberr)?;
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
        let (i, f) = (id.clone(), r.device_fingerprint.clone());
        let n = self
            .pool
            .run(move |c| {
                Ok(diesel::update(
                    devices::table.filter(
                        devices::user_id
                            .eq(i)
                            .and(devices::fingerprint.eq(f))
                            .and(devices::status.ne("revoked")),
                    ),
                )
                .set(devices::status.eq("revoked"))
                .execute(c)?)
            })
            .await
            .map_err(dberr)?;
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
        let (u, b) = (p.user_id.clone(), r.blob_json.clone());
        self.pool
            .run(move |c| {
                c.transaction::<_, anyhow::Error, _>(|c| {
                    put_backup(c, &u, &b, crate::now_ms())?;
                    diesel::update(users::table.filter(users::id.eq(&u)))
                        .set(users::key_backup_opt_out.eq(false))
                        .execute(c)?;
                    Ok(())
                })
            })
            .await
            .map_err(dberr)?;
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
        let i = id.clone();
        let blob: String = self
            .pool
            .run(move |c| {
                Ok(key_backups::table
                    .filter(key_backups::user_id.eq(i))
                    .select(key_backups::blob)
                    .first(c)
                    .optional()?)
            })
            .await
            .map_err(dberr)?
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
        let u = p.user_id.clone();
        self.pool
            .run(move |c| {
                c.transaction::<_, anyhow::Error, _>(|c| {
                    diesel::update(users::table.filter(users::id.eq(&u)))
                        .set(users::key_backup_opt_out.eq(o))
                        .execute(c)?;
                    if o {
                        diesel::delete(key_backups::table.filter(key_backups::user_id.eq(&u)))
                            .execute(c)?;
                    }
                    Ok(())
                })
            })
            .await
            .map_err(dberr)?;
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
        let (f, u, rr) = (cert.fingerprint.clone(), p.user_id.clone(), r.clone());
        self.pool
            .run(move |c| {
                diesel::insert_into(devices::table)
                    .values((
                        devices::fingerprint.eq(f),
                        devices::user_id.eq(u),
                        devices::public_key.eq(rr.device_public_key),
                        devices::certificate.eq(rr.certificate_json),
                        devices::platform.eq(rr.platform),
                        devices::status.eq("active"),
                        devices::created_ms.eq(crate::now_ms()),
                    ))
                    .execute(c)?;
                Ok(())
            })
            .await
            .map_err(dberr)?;
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
        let s_tmp = secret();
        let exp = now + CODE_TTL_MS;
        let (i, n, th) = (
            id.clone(),
            name.clone(),
            itcmdr_adminlink::sha256_hex(s_tmp.as_bytes()),
        );
        self.pool
            .run(move |c| {
                c.transaction::<_, anyhow::Error, _>(|c| {
                    diesel::insert_into(sites::table)
                        .values((
                            sites::id.eq(&i),
                            sites::name.eq(n),
                            sites::status.eq("pending"),
                            sites::created_ms.eq(now),
                        ))
                        .execute(c)?;
                    diesel::insert_into(site_tokens::table)
                        .values((
                            site_tokens::secret_hash.eq(th),
                            site_tokens::site_id.eq(&i),
                            site_tokens::expires_ms.eq(exp),
                        ))
                        .execute(c)?;
                    Ok(())
                })
            })
            .await
            .map_err(dberr)?;
        self.audit(&p, "site.create", &id, req.get_ref()).await?;
        let token = itcmdr_adminlink::SiteToken {
            v: 1,
            addr: self.cfg.public_site.clone(),
            pin: self.m.fingerprint.clone(),
            secret: s_tmp,
            root: self.m.root_hex(),
        }
        .encode();
        Ok(Response::new(pb::CreateSiteResponse {
            site: Some(get_site(&self.pool, &id).await?),
            token,
            expires_ms: exp,
        }))
    }

    async fn list_sites(&self, req: Request<pb::Empty>) -> Result<Response<pb::SiteList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListSites")
            .await?;
        let (admin, u) = (p.role >= Role::Admin, p.user_id.clone());
        let rows: Vec<SiteRow> = self
            .pool
            .run(move |c| {
                let mut q = sites::table
                    .select(site_cols())
                    .order(sites::name)
                    .into_boxed();
                if !admin {
                    q = q.filter(
                        sites::id.eq_any(
                            site_scopes::table
                                .filter(site_scopes::user_id.eq(u))
                                .select(site_scopes::site_id),
                        ),
                    );
                }
                Ok(q.load(c)?)
            })
            .await
            .map_err(dberr)?;
        let sites = rows.into_iter().map(site_info).collect();
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
        let i = id.clone();
        let n = self
            .pool
            .run(move |c| {
                Ok(diesel::update(sites::table.filter(sites::id.eq(i)))
                    .set(sites::status.eq("revoked"))
                    .execute(c)?)
            })
            .await
            .map_err(dberr)?;
        if n == 0 {
            return Err(Status::not_found("no such site"));
        }
        self.hub.drop_site(&id);
        crate::link::refresh_revoked(&self.pool).await;
        self.audit(&p, "site.revoke", &id, req.get_ref()).await?;
        Ok(Response::new(get_site(&self.pool, &id).await?))
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
        let (site, st, admin, u) = (
            f.site.clone(),
            state.to_string(),
            p.role >= Role::Admin,
            p.user_id.clone(),
        );
        let mut rows: Vec<(String, String, String, i64, Vec<u8>)> = self
            .pool
            .run(move |c| {
                let mut q = alerts::table
                    .select((
                        alerts::site_id,
                        alerts::origin,
                        alerts::severity,
                        alerts::last_seen_ms,
                        alerts::body,
                    ))
                    .into_boxed();
                if !site.is_empty() {
                    q = q.filter(alerts::site_id.eq(site));
                }
                match st.as_str() {
                    "all" => {}
                    "open" => q = q.filter(alerts::state.eq_any(["open", "acked", "silenced"])),
                    s => q = q.filter(alerts::state.eq(s.to_string())),
                }
                if !admin {
                    q = q.filter(
                        alerts::site_id.eq_any(
                            site_scopes::table
                                .filter(site_scopes::user_id.eq(u))
                                .select(site_scopes::site_id),
                        ),
                    );
                }
                Ok(q.load(c)?)
            })
            .await
            .map_err(dberr)?;
        let rank = |s: &str| match s {
            "critical" => 0,
            "cautious" => 1,
            _ => 2,
        };
        rows.sort_by(|a, b| rank(&a.2).cmp(&rank(&b.2)).then(b.3.cmp(&a.3)));
        rows.truncate(limit as usize);
        let alerts = rows
            .into_iter()
            .map(|r| pb::AlertInfo {
                site: r.0,
                alert: pb::SiteAlert::decode(r.4.as_slice()).ok(),
                origin: r.1,
            })
            .collect();
        Ok(Response::new(pb::AlertList { alerts }))
    }

    async fn send_command(
        &self,
        req: Request<pb::CommandRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        self.command(req, "/itcmdr.admin.AdminApi/SendCommand", SEND_KINDS)
            .await
    }

    async fn get_command(
        &self,
        req: Request<pb::CommandRef>,
    ) -> Result<Response<pb::CommandDetail>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/GetCommand")
            .await?;
        let id = req.get_ref().id.clone();
        let row = self.command_by_id(&id).await?;
        self.may_site(&p, &row.1).await?;
        let command = pb::Command::decode(row.10.as_slice()).ok();
        Ok(Response::new(pb::CommandDetail {
            info: Some(command_row(row)),
            command,
        }))
    }

    async fn approve_command(
        &self,
        req: Request<pb::ApproveRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ApproveCommand")
            .await?;
        let r = req.get_ref().clone();
        if p.role < Role::Admin {
            return Err(denied("only owners and admins can approve"));
        }
        if crate::link::frozen(&self.pool).await.map_err(dberr)? {
            return Err(Status::failed_precondition("frozen"));
        }
        let row = self.command_by_id(&r.id).await?;
        self.may_site(&p, &row.1).await?;
        if row.3 != "awaiting-approval" {
            return Err(Status::failed_precondition(format!("command is {}", row.3)));
        }
        if row.5 == p.user_id {
            return Err(denied(
                "the second person must be someone other than the requester",
            ));
        }
        if r.approver_key != p.key_hex {
            return Err(Status::invalid_argument(
                "approver_key must be the calling device's key",
            ));
        }
        let mut c =
            pb::Command::decode(row.10.as_slice()).map_err(|_| Status::internal("command body"))?;
        if c.expires_ms <= crate::now_ms() {
            return Err(Status::failed_precondition("command expired"));
        }
        c.approver_id = p.user_id.clone();
        c.approver_role = p.role.as_str().into();
        c.approver_key = p.key_hex.clone();
        c.approver_sig = r.approver_sig.clone();
        itcmdr_adminlink::endorse_approver(&self.m.root, &mut c);
        let root = self.m.root.verifying_key();
        if !matches!(itcmdr_adminlink::check_approver(&root, &c), Ok(true)) {
            return Err(Status::unauthenticated("bad approval signature"));
        }
        let (cc, uid, id) = (c.clone(), p.user_id.clone(), r.id.clone());
        let n = self
            .pool
            .run(move |db| {
                Ok(diesel::update(
                    commands::table.filter(
                        commands::id
                            .eq(id)
                            .and(commands::state.eq("awaiting-approval")),
                    ),
                )
                .set((
                    commands::state.eq("pending"),
                    commands::approved_by.eq(uid),
                    commands::body.eq(cc.encode_to_vec()),
                ))
                .execute(db)?)
            })
            .await
            .map_err(dberr)?;
        if n == 0 {
            return Err(Status::failed_precondition("command was changed meanwhile"));
        }
        self.audit(&p, &format!("command.approve.{}", c.kind), &c.site_id, &r)
            .await?;
        self.hub.kick(&c.site_id);
        let row = self.command_by_id(&r.id).await?;
        Ok(Response::new(command_row(row)))
    }

    async fn reject_command(
        &self,
        req: Request<pb::CommandRef>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/RejectCommand")
            .await?;
        let id = req.get_ref().id.clone();
        let row = self.command_by_id(&id).await?;
        self.may_site(&p, &row.1).await?;
        if row.5 != p.user_id && p.role < Role::Admin {
            return Err(denied(
                "only the requester, an owner or an admin can reject",
            ));
        }
        let i = id.clone();
        let n = self
            .pool
            .run(move |db| {
                Ok(diesel::update(
                    commands::table.filter(
                        commands::id
                            .eq(i)
                            .and(commands::state.eq("awaiting-approval")),
                    ),
                )
                .set(commands::state.eq("rejected"))
                .execute(db)?)
            })
            .await
            .map_err(dberr)?;
        if n == 0 {
            return Err(Status::failed_precondition(format!("command is {}", row.3)));
        }
        self.audit(
            &p,
            &format!("command.reject.{}", row.2),
            &row.1,
            req.get_ref(),
        )
        .await?;
        Ok(Response::new(command_row(self.command_by_id(&id).await?)))
    }

    async fn get_site_config(
        &self,
        req: Request<pb::SiteRef>,
    ) -> Result<Response<pb::SiteConfigInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/GetSiteConfig")
            .await?;
        let site = req.get_ref().site.clone();
        self.may_site(&p, &site).await?;
        let s = site.clone();
        let (b, ms): (Option<Vec<u8>>, i64) = self
            .pool
            .run(move |c| {
                Ok(sites::table
                    .filter(sites::id.eq(s))
                    .select((sites::config, sites::config_ms))
                    .first(c)
                    .optional()?
                    .unwrap_or((None, 0)))
            })
            .await
            .map_err(dberr)?;
        Ok(Response::new(pb::SiteConfigInfo {
            site,
            config: b.and_then(|b| pb::SiteConfig::decode(b.as_slice()).ok()),
            received_ms: ms,
        }))
    }

    async fn get_site_plugins(
        &self,
        req: Request<pb::SiteRef>,
    ) -> Result<Response<pb::SitePluginsInfo>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/GetSitePlugins")
            .await?;
        let site = req.get_ref().site.clone();
        self.may_site(&p, &site).await?;
        let s = site.clone();
        let (b, ms): (Option<Vec<u8>>, i64) = self
            .pool
            .run(move |c| {
                Ok(sites::table
                    .filter(sites::id.eq(s))
                    .select((sites::plugins, sites::plugins_ms))
                    .first(c)
                    .optional()?
                    .unwrap_or((None, 0)))
            })
            .await
            .map_err(dberr)?;
        Ok(Response::new(pb::SitePluginsInfo {
            site,
            plugins: b.and_then(|b| pb::SitePlugins::decode(b.as_slice()).ok()),
            received_ms: ms,
        }))
    }

    async fn upload_plugin(
        &self,
        req: Request<pb::PluginUpload>,
    ) -> Result<Response<pb::CatalogEntry>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/UploadPlugin")
            .await?;
        if p.role < Role::Admin {
            return Err(denied("only owners and admins can add packages"));
        }
        let bytes = req.get_ref().package.clone();
        if bytes.len() > MAX_PACKAGE_MIB * 1024 * 1024 {
            return Err(Status::invalid_argument("package too large"));
        }
        let (name, version) = package_name_version(&bytes)
            .map_err(|e| Status::invalid_argument(format!("not a plugin package: {e}")))?;
        let sha = itcmdr_adminlink::sha256_hex(&bytes);
        let e = pb::CatalogEntry {
            sha256: sha.clone(),
            name,
            version,
            size: bytes.len() as i64,
            uploaded_by: p.user_id.clone(),
            uploaded_ms: crate::now_ms(),
            description: String::new(),
        };
        let ee = e.clone();
        self.pool
            .run(move |c| {
                let have: i64 = catalog::table
                    .filter(catalog::sha256.eq(&ee.sha256))
                    .count()
                    .get_result(c)?;
                if have == 0 {
                    diesel::insert_into(catalog::table)
                        .values((
                            catalog::sha256.eq(&ee.sha256),
                            catalog::name.eq(&ee.name),
                            catalog::version.eq(&ee.version),
                            catalog::size.eq(ee.size),
                            catalog::description.eq(""),
                            catalog::uploaded_by.eq(&ee.uploaded_by),
                            catalog::uploaded_ms.eq(ee.uploaded_ms),
                            catalog::body.eq(&bytes),
                        ))
                        .execute(c)?;
                }
                Ok(())
            })
            .await
            .map_err(dberr)?;
        // The package bytes are not in the audit, only what identifies them.
        self.audit(
            &p,
            "catalog.upload",
            &sha,
            &pb::CatalogEntry { ..e.clone() },
        )
        .await?;
        Ok(Response::new(e))
    }

    async fn list_catalog(&self, req: Request<pb::Empty>) -> Result<Response<pb::Catalog>, Status> {
        self.signed(&req, "/itcmdr.admin.AdminApi/ListCatalog")
            .await?;
        type Row = (String, String, String, i64, String, String, i64);
        let rows: Vec<Row> = self
            .pool
            .run(|c| {
                Ok(catalog::table
                    .order((catalog::name, catalog::uploaded_ms.desc()))
                    .select((
                        catalog::sha256,
                        catalog::name,
                        catalog::version,
                        catalog::size,
                        catalog::description,
                        catalog::uploaded_by,
                        catalog::uploaded_ms,
                    ))
                    .load(c)?)
            })
            .await
            .map_err(dberr)?;
        Ok(Response::new(pb::Catalog {
            packages: rows
                .into_iter()
                .map(|r| pb::CatalogEntry {
                    sha256: r.0,
                    name: r.1,
                    version: r.2,
                    size: r.3,
                    description: r.4,
                    uploaded_by: r.5,
                    uploaded_ms: r.6,
                })
                .collect(),
        }))
    }

    async fn get_freeze(
        &self,
        req: Request<pb::Empty>,
    ) -> Result<Response<pb::FreezeState>, Status> {
        self.signed(&req, "/itcmdr.admin.AdminApi/GetFreeze")
            .await?;
        Ok(Response::new(self.freeze_state().await?))
    }

    async fn set_freeze(
        &self,
        req: Request<pb::FreezeState>,
    ) -> Result<Response<pb::FreezeState>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/SetFreeze")
            .await?;
        if p.role != Role::Owner {
            return Err(denied("only owners can freeze or unfreeze"));
        }
        let v = if req.get_ref().frozen {
            format!("1|{}|{}", p.user_id, crate::now_ms())
        } else {
            "0".to_string()
        };
        self.pool
            .run(move |c| crate::passkeys::put_meta(c, "freeze", &v))
            .await
            .map_err(dberr)?;
        let st = self.freeze_state().await?;
        self.audit(
            &p,
            if st.frozen { "freeze.on" } else { "freeze.off" },
            "",
            req.get_ref(),
        )
        .await?;
        if !st.frozen {
            self.hub.kick_all();
        }
        Ok(Response::new(st))
    }

    async fn ack_alert(
        &self,
        req: Request<pb::CommandRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        self.command(req, "/itcmdr.admin.AdminApi/AckAlert", &["alert.ack"])
            .await
    }
    async fn resolve_alert(
        &self,
        req: Request<pb::CommandRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        self.command(
            req,
            "/itcmdr.admin.AdminApi/ResolveAlert",
            &["alert.resolve"],
        )
        .await
    }
    async fn start_backup(
        &self,
        req: Request<pb::CommandRequest>,
    ) -> Result<Response<pb::CommandInfo>, Status> {
        self.command(req, "/itcmdr.admin.AdminApi/StartBackup", &["backup.start"])
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
        let (admin, u, sid) = (p.role >= Role::Admin, p.user_id.clone(), site.clone());
        let rows: Vec<CommandRow> = self
            .pool
            .run(move |c| {
                let mut q = commands::table
                    .select(command_cols())
                    .order(commands::created_ms.desc())
                    .limit(200)
                    .into_boxed();
                if !sid.is_empty() {
                    q = q.filter(commands::site_id.eq(sid));
                }
                if !admin {
                    q = q.filter(
                        commands::site_id.eq_any(
                            site_scopes::table
                                .filter(site_scopes::user_id.eq(u))
                                .select(site_scopes::site_id),
                        ),
                    );
                }
                Ok(q.load(c)?)
            })
            .await
            .map_err(dberr)?;
        Ok(Response::new(pb::CommandList {
            commands: rows.into_iter().map(command_row).collect(),
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

    async fn begin_passkey_registration(
        &self,
        req: Request<pb::Empty>,
    ) -> Result<Response<pb::PasskeyChallenge>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/BeginPasskeyRegistration")
            .await?;
        self.pk.begin(&p, true).await.map(Response::new)
    }

    async fn finish_passkey_registration(
        &self,
        req: Request<pb::PasskeyRegistration>,
    ) -> Result<Response<pb::PasskeyList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/FinishPasskeyRegistration")
            .await?;
        let l = self.pk.finish(&p, req.get_ref()).await?;
        let mut r = req.get_ref().clone();
        r.nonce.clear();
        self.audit(&p, "passkey.add", &p.user_id, &r).await?;
        Ok(Response::new(l))
    }

    async fn list_passkeys(
        &self,
        req: Request<pb::UserRef>,
    ) -> Result<Response<pb::PasskeyList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListPasskeys")
            .await?;
        let id = self.target(&req.get_ref().user, &p).await?;
        if id != p.user_id {
            rbac::require(p.role, Role::Admin).map_err(denied)?;
        }
        crate::passkeys::list(&self.pool, &id)
            .await
            .map(Response::new)
            .map_err(internal)
    }

    async fn remove_passkey(
        &self,
        req: Request<pb::PasskeyRef>,
    ) -> Result<Response<pb::PasskeyList>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/RemovePasskey")
            .await?;
        let r = req.get_ref();
        let id = self.target(&r.user, &p).await?;
        let other = id != p.user_id;
        if other {
            // Lost passkey: an owner or admin resets it (with their own step-up); admins not for owners.
            rbac::require(p.role, Role::Admin).map_err(denied)?;
            rbac::can_edit(p.role, false, self.role_of(&id).await?).map_err(denied)?;
        }
        if !self.pk.remove(&id, &r.credential_id).await? {
            return Err(Status::not_found("no such passkey"));
        }
        self.audit(
            &p,
            if other {
                "passkey.reset"
            } else {
                "passkey.remove"
            },
            &id,
            r,
        )
        .await?;
        crate::passkeys::list(&self.pool, &id)
            .await
            .map(Response::new)
            .map_err(internal)
    }

    async fn begin_step_up(
        &self,
        req: Request<pb::Empty>,
    ) -> Result<Response<pb::PasskeyChallenge>, Status> {
        let p = self
            .signed_opts(&req, "/itcmdr.admin.AdminApi/BeginStepUp", true, false)
            .await?;
        self.pk.begin(&p, false).await.map(Response::new)
    }

    async fn get_org_policy(
        &self,
        req: Request<pb::Empty>,
    ) -> Result<Response<pb::OrgPolicy>, Status> {
        self.signed(&req, "/itcmdr.admin.AdminApi/GetOrgPolicy")
            .await?;
        self.org_policy().await.map(Response::new)
    }

    async fn set_org_policy(
        &self,
        req: Request<pb::OrgPolicy>,
    ) -> Result<Response<pb::OrgPolicy>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/SetOrgPolicy")
            .await?;
        rbac::require(p.role, Role::Owner).map_err(denied)?;
        let on = req.get_ref().require_second_approver;
        let (uid, now) = (p.user_id.clone(), crate::now_ms());
        self.pool
            .run(move |c| {
                crate::passkeys::put_meta(
                    c,
                    "org.require_second_approver",
                    if on { "true" } else { "false" },
                )?;
                crate::passkeys::put_meta(c, "org.policy_updated_by", &uid)?;
                crate::passkeys::put_meta(c, "org.policy_updated_ms", &now.to_string())?;
                Ok(())
            })
            .await
            .map_err(dberr)?;
        self.audit(&p, "org.policy", "org", req.get_ref()).await?;
        self.org_policy().await.map(Response::new)
    }

    async fn list_releases(
        &self,
        req: Request<pb::Empty>,
    ) -> Result<Response<pb::ReleaseList>, Status> {
        self.signed(&req, "/itcmdr.admin.AdminApi/ListReleases")
            .await?;
        let shas: Vec<String> = self
            .pool
            .run(|c| {
                Ok(catalog::table
                    .filter(catalog::name.eq_any(RELEASES))
                    .order(catalog::uploaded_ms.desc())
                    .select(catalog::sha256)
                    .load(c)?)
            })
            .await
            .map_err(dberr)?;
        let mut releases = vec![];
        for s in shas {
            releases.push(self.release_state(&s).await?);
        }
        Ok(Response::new(pb::ReleaseList { releases }))
    }

    async fn approve_release(
        &self,
        req: Request<pb::ApproveReleaseRequest>,
    ) -> Result<Response<pb::ReleaseState>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ApproveRelease")
            .await?;
        if p.role < Role::Admin {
            return Err(denied("only owners and admins approve releases"));
        }
        let sha = req.get_ref().sha256.to_lowercase();
        let st = self.release_state(&sha).await?;
        let mut a = pb::ReleaseApproval {
            user_id: p.user_id.clone(),
            role: p.role.as_str().into(),
            key: p.key_hex.clone(),
            endorsement: String::new(),
            sig: req.get_ref().sig.clone(),
            ts_ms: crate::now_ms(),
        };
        itcmdr_adminlink::endorse_release(&self.m.root, &mut a);
        let root = self.m.root.verifying_key();
        if itcmdr_adminlink::check_release_approvals(
            &root,
            std::slice::from_ref(&a),
            &st.name,
            &st.version,
            &sha,
            1,
        )
        .is_err()
        {
            return Err(Status::unauthenticated("bad release approval signature"));
        }
        let (aa, s2) = (a.clone(), sha.clone());
        self.pool
            .run(move |c| {
                diesel::delete(
                    release_approvals::table.filter(
                        release_approvals::sha256
                            .eq(&s2)
                            .and(release_approvals::user_id.eq(&aa.user_id)),
                    ),
                )
                .execute(c)?;
                diesel::insert_into(release_approvals::table)
                    .values((
                        release_approvals::sha256.eq(&s2),
                        release_approvals::user_id.eq(&aa.user_id),
                        release_approvals::role.eq(&aa.role),
                        release_approvals::pubkey.eq(&aa.key),
                        release_approvals::endorsement.eq(&aa.endorsement),
                        release_approvals::sig.eq(&aa.sig),
                        release_approvals::ts_ms.eq(aa.ts_ms),
                    ))
                    .execute(c)?;
                Ok(())
            })
            .await
            .map_err(dberr)?;
        self.audit(
            &p,
            "release.approve",
            &format!("{} {}", st.name, st.version),
            req.get_ref(),
        )
        .await?;
        self.release_state(&sha).await.map(Response::new)
    }

    async fn get_auth_policy(
        &self,
        req: Request<pb::Empty>,
    ) -> Result<Response<pb::AuthPolicy>, Status> {
        self.signed_opts(&req, "/itcmdr.admin.AdminApi/GetAuthPolicy", true, false)
            .await?;
        self.pool
            .run(crate::passkeys::load_policy)
            .await
            .map(|p| Response::new(p.to_pb()))
            .map_err(internal)
    }

    async fn set_auth_policy(
        &self,
        req: Request<pb::AuthPolicy>,
    ) -> Result<Response<pb::AuthPolicy>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/SetAuthPolicy")
            .await?;
        rbac::require(p.role, Role::Owner).map_err(denied)?;
        let r = req.get_ref().clone();
        let pol = self
            .pool
            .run(move |c| Ok(crate::passkeys::save_policy(c, &r)))
            .await
            .map_err(internal)?
            .map_err(Status::invalid_argument)?;
        self.audit(&p, "auth.policy", "auth", req.get_ref()).await?;
        Ok(Response::new(pol.to_pb()))
    }

    async fn list_site_metrics(
        &self,
        req: Request<pb::MetricsFilter>,
    ) -> Result<Response<pb::MetricsSeries>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/ListSiteMetrics")
            .await?;
        let f = req.get_ref().clone();
        if f.site.is_empty() {
            return Err(Status::invalid_argument("site required"));
        }
        self.may_site(&p, &f.site).await?;
        let limit = if f.limit <= 0 {
            1440
        } else {
            f.limit.min(20000)
        } as i64;
        let (site, since) = (f.site.clone(), f.since_ms);
        let mut rows: Vec<(i64, Vec<u8>)> = self
            .pool
            .run(move |c| {
                Ok(site_metrics::table
                    .filter(
                        site_metrics::site_id
                            .eq(site)
                            .and(site_metrics::ts_ms.ge(since)),
                    )
                    .order(site_metrics::ts_ms.desc())
                    .limit(limit)
                    .select((site_metrics::ts_ms, site_metrics::body))
                    .load(c)?)
            })
            .await
            .map_err(dberr)?;
        rows.reverse();
        let samples = rows
            .into_iter()
            .filter_map(|(ts, b)| {
                pb::SiteStatus::decode(b.as_slice())
                    .ok()
                    .map(|st| pb::MetricsSample {
                        ts_ms: ts,
                        status: Some(st),
                    })
            })
            .collect();
        Ok(Response::new(pb::MetricsSeries {
            site: f.site,
            samples,
        }))
    }

    async fn register_push(
        &self,
        req: Request<pb::PushRegistration>,
    ) -> Result<Response<pb::Empty>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/RegisterPush")
            .await?;
        let r = req.get_ref().clone();
        if !matches!(r.provider.as_str(), "fcm" | "apns") {
            return Err(Status::invalid_argument("provider: fcm | apns"));
        }
        if r.token.is_empty() || r.token.len() > 4096 || r.platform.len() > 32 {
            return Err(Status::invalid_argument("bad token"));
        }
        let (u, fp, rr) = (p.user_id.clone(), p.key_fp.clone(), r.clone());
        self.pool
            .run(move |c| {
                c.transaction::<_, anyhow::Error, _>(|c| {
                    diesel::delete(push_tokens::table.filter(push_tokens::token.eq(&rr.token)))
                        .execute(c)?;
                    diesel::insert_into(push_tokens::table)
                        .values((
                            push_tokens::token.eq(&rr.token),
                            push_tokens::provider.eq(&rr.provider),
                            push_tokens::user_id.eq(&u),
                            push_tokens::device_fp.eq(&fp),
                            push_tokens::platform.eq(&rr.platform),
                            push_tokens::created_ms.eq(crate::now_ms()),
                        ))
                        .execute(c)?;
                    Ok(())
                })
            })
            .await
            .map_err(dberr)?;
        // The audit keeps only the payload hash, so the token is not in the chain.
        self.audit(&p, "push.register", &p.key_fp, &r).await?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn unregister_push(
        &self,
        req: Request<pb::PushRegistration>,
    ) -> Result<Response<pb::Empty>, Status> {
        let p = self
            .signed(&req, "/itcmdr.admin.AdminApi/UnregisterPush")
            .await?;
        let r = req.get_ref().clone();
        let (u, fp, t) = (p.user_id.clone(), p.key_fp.clone(), r.token.clone());
        self.pool
            .run(move |c| {
                let mine = push_tokens::user_id.eq(u);
                Ok(if t.is_empty() {
                    diesel::delete(
                        push_tokens::table.filter(mine.and(push_tokens::device_fp.eq(fp))),
                    )
                    .execute(c)?
                } else {
                    diesel::delete(push_tokens::table.filter(mine.and(push_tokens::token.eq(t))))
                        .execute(c)?
                })
            })
            .await
            .map_err(dberr)?;
        self.audit(&p, "push.unregister", &p.key_fp, &r).await?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn test_push(&self, req: Request<pb::Empty>) -> Result<Response<pb::PushResult>, Status> {
        let p = self.signed(&req, "/itcmdr.admin.AdminApi/TestPush").await?;
        let push = self.hub.push.clone();
        let targets = crate::push::recipients(&self.pool, None, Some(&p.user_id))
            .await
            .map_err(internal)?;
        let devices = targets.len() as i32;
        let pl = crate::push::Payload {
            kind: "test".into(),
            site: String::new(),
            alert: String::new(),
            severity: "informational".into(),
        };
        let (sent, failed) =
            crate::push::deliver(&push, &self.pool, targets, &pl, &p.user_id).await;
        Ok(Response::new(pb::PushResult {
            sent,
            failed,
            devices,
            providers: push.providers(),
        }))
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

/// name and version from an uploaded package's package.toml (the site validates the rest).
pub fn package_name_version(bytes: &[u8]) -> anyhow::Result<(String, String)> {
    use std::io::Read;
    let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    for e in ar.entries()? {
        let e = e?;
        let path = e
            .path()?
            .to_string_lossy()
            .trim_start_matches("./")
            .to_string();
        if path == "package.toml" {
            let mut s = String::new();
            e.take(1024 * 1024).read_to_string(&mut s)?;
            #[derive(serde::Deserialize)]
            struct M {
                name: String,
                version: String,
            }
            let m: M = toml::from_str(&s)?;
            let ok = |v: &str| {
                !v.is_empty()
                    && v.len() <= 64
                    && v.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
            };
            anyhow::ensure!(ok(&m.name) && ok(&m.version), "bad name or version");
            return Ok((m.name, m.version));
        }
    }
    anyhow::bail!("package.toml is missing")
}
