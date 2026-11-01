//! v0.19.2 push notifications for alerts: FCM HTTP v1 (Android) and APNs token auth (iOS/macOS).
//!
//! Payloads are redacted by construction: they carry only the opaque site id, the opaque alert id
//! and the severity (plus a fixed title). No site name, alert code, machine, plugin or user data
//! ever goes to Google or Apple; the app fetches the details over the signed API.
//! Every send is audited (`push.send`). Tokens belong to a device; a revoked or disabled device,
//! user or scope gets nothing.
use crate::config::PushConfig;
use crate::schema::{devices, push_tokens, site_scopes, users};
use crate::store::Pool;
use anyhow::{bail, Context, Result};
use diesel::prelude::*;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// What a push says. Only opaque ids and enums.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// alert | test
    pub kind: String,
    pub site: String,
    pub alert: String,
    pub severity: String,
}

impl Payload {
    pub fn title(&self) -> String {
        match self.kind.as_str() {
            "test" => "itcmdr: test notification".into(),
            _ => format!("itcmdr: {} alert", sev_word(&self.severity)),
        }
    }
    pub fn body(&self) -> &'static str {
        "Open itcmdr Admin to see it."
    }
    /// The data fields (both providers).
    pub fn data(&self) -> Value {
        json!({"t": self.kind, "site": self.site, "alert": self.alert, "sev": self.severity})
    }
}

fn sev_word(s: &str) -> &'static str {
    match s {
        "critical" => "critical",
        "cautious" => "cautious",
        _ => "informational",
    }
}

pub fn sev_rank(s: &str) -> u8 {
    match s {
        "critical" => 3,
        "cautious" => 2,
        "informational" => 1,
        _ => 0,
    }
}

/// The FCM v1 message (exposed for tests).
pub fn fcm_message(token: &str, p: &Payload) -> Value {
    json!({"message": {
        "token": token,
        "notification": {"title": p.title(), "body": p.body()},
        "data": p.data(),
        "android": {"priority": "high"},
    }})
}

/// The APNs body (exposed for tests).
pub fn apns_body(p: &Payload) -> Value {
    let mut v = p.data();
    v["aps"] = json!({"alert": {"title": p.title(), "body": p.body()}, "sound": "default"});
    v
}

#[derive(Deserialize)]
struct ServiceAccount {
    project_id: String,
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}
fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".into()
}

struct Fcm {
    sa: ServiceAccount,
    key: jsonwebtoken::EncodingKey,
    base: String,
    token: Mutex<Option<(String, i64)>>,
}

struct Apns {
    key: jsonwebtoken::EncodingKey,
    key_id: String,
    team_id: String,
    topic: String,
    base: String,
    jwt: Mutex<Option<(String, i64)>>,
}

#[derive(Default)]
pub struct Push {
    fcm: Option<Fcm>,
    apns: Option<Apns>,
    min: u8,
    http: reqwest::Client,
}

/// One delivery outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    Ok,
    /// The provider says the token is gone: it is deleted.
    Gone,
    Failed,
}

impl Push {
    pub fn new(c: &PushConfig) -> Result<Arc<Self>> {
        let fcm = match &c.fcm_service_account {
            Some(p) => {
                let sa: ServiceAccount = serde_json::from_slice(
                    &std::fs::read(p).with_context(|| format!("read {}", p.display()))?,
                )
                .context("FCM service account JSON")?;
                let key = jsonwebtoken::EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
                    .context("FCM service account private_key")?;
                Some(Fcm {
                    sa,
                    key,
                    base: or(&c.fcm_url, "https://fcm.googleapis.com"),
                    token: Mutex::new(None),
                })
            }
            None => None,
        };
        let apns = match &c.apns_key {
            Some(p) => {
                if c.apns_key_id.is_empty() || c.apns_team_id.is_empty() || c.apns_topic.is_empty()
                {
                    bail!("APNs needs ITCMDR_ADMINSVR_APNS_KEY_ID, _TEAM_ID and _TOPIC");
                }
                let pem = std::fs::read(p).with_context(|| format!("read {}", p.display()))?;
                Some(Apns {
                    key: jsonwebtoken::EncodingKey::from_ec_pem(&pem).context("APNs .p8 key")?,
                    key_id: c.apns_key_id.clone(),
                    team_id: c.apns_team_id.clone(),
                    topic: c.apns_topic.clone(),
                    base: or(&c.apns_url, "https://api.push.apple.com"),
                    jwt: Mutex::new(None),
                })
            }
            None => None,
        };
        let min = match sev_rank(&c.min_severity) {
            0 => 2,
            n => n,
        };
        Ok(Arc::new(Push {
            fcm,
            apns,
            min,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()?,
        }))
    }

    pub fn providers(&self) -> Vec<String> {
        let mut v = vec![];
        if self.fcm.is_some() {
            v.push("fcm".to_string());
        }
        if self.apns.is_some() {
            v.push("apns".to_string());
        }
        v
    }

    pub fn wants(&self, severity: &str) -> bool {
        (self.fcm.is_some() || self.apns.is_some()) && sev_rank(severity) >= self.min
    }

    async fn fcm_access(&self, f: &Fcm) -> Result<String> {
        let now = crate::now_ms() / 1000;
        if let Some((t, exp)) = f.token.lock().clone() {
            if exp > now + 60 {
                return Ok(t);
            }
        }
        let claims = json!({
            "iss": f.sa.client_email, "scope": "https://www.googleapis.com/auth/firebase.messaging",
            "aud": f.sa.token_uri, "iat": now, "exp": now + 3600,
        });
        let jwt = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &f.key,
        )?;
        #[derive(Deserialize)]
        struct T {
            access_token: String,
            #[serde(default)]
            expires_in: i64,
        }
        let r = self
            .http
            .post(&f.sa.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt.as_str()),
            ])
            .send()
            .await?;
        if !r.status().is_success() {
            bail!("FCM token exchange: HTTP {}", r.status());
        }
        let t: T = r.json().await?;
        let exp = now + if t.expires_in > 0 { t.expires_in } else { 3600 };
        *f.token.lock() = Some((t.access_token.clone(), exp));
        Ok(t.access_token)
    }

    fn apns_jwt(&self, a: &Apns) -> Result<String> {
        let now = crate::now_ms() / 1000;
        if let Some((t, iat)) = a.jwt.lock().clone() {
            if now - iat < 40 * 60 {
                return Ok(t);
            }
        }
        let mut h = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
        h.kid = Some(a.key_id.clone());
        let t = jsonwebtoken::encode(&h, &json!({"iss": a.team_id, "iat": now}), &a.key)?;
        *a.jwt.lock() = Some((t.clone(), now));
        Ok(t)
    }

    /// Deliver one push.
    pub async fn send(&self, provider: &str, token: &str, p: &Payload) -> Sent {
        let r = match provider {
            "fcm" => match &self.fcm {
                Some(f) => self.send_fcm(f, token, p).await,
                None => return Sent::Failed,
            },
            "apns" => match &self.apns {
                Some(a) => self.send_apns(a, token, p).await,
                None => return Sent::Failed,
            },
            _ => return Sent::Failed,
        };
        r.unwrap_or_else(|e| {
            tracing::warn!(provider, error=%e, "push failed");
            Sent::Failed
        })
    }

    async fn send_fcm(&self, f: &Fcm, token: &str, p: &Payload) -> Result<Sent> {
        let at = self.fcm_access(f).await?;
        let url = format!(
            "{}/v1/projects/{}/messages:send",
            f.base.trim_end_matches('/'),
            f.sa.project_id
        );
        let r = self
            .http
            .post(url)
            .bearer_auth(at)
            .json(&fcm_message(token, p))
            .send()
            .await?;
        let st = r.status();
        if st.is_success() {
            return Ok(Sent::Ok);
        }
        let body = r.text().await.unwrap_or_default();
        if st == reqwest::StatusCode::NOT_FOUND || body.contains("UNREGISTERED") {
            return Ok(Sent::Gone);
        }
        tracing::warn!(status=%st, "FCM refused a push");
        Ok(Sent::Failed)
    }

    async fn send_apns(&self, a: &Apns, token: &str, p: &Payload) -> Result<Sent> {
        if !token.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(Sent::Gone);
        }
        let url = format!("{}/3/device/{token}", a.base.trim_end_matches('/'));
        let r = self
            .http
            .post(url)
            .header("authorization", format!("bearer {}", self.apns_jwt(a)?))
            .header("apns-topic", &a.topic)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .json(&apns_body(p))
            .send()
            .await?;
        let st = r.status();
        if st.is_success() {
            return Ok(Sent::Ok);
        }
        let body = r.text().await.unwrap_or_default();
        if st == reqwest::StatusCode::GONE
            || body.contains("BadDeviceToken")
            || body.contains("Unregistered")
        {
            return Ok(Sent::Gone);
        }
        tracing::warn!(status=%st, "APNs refused a push");
        Ok(Sent::Failed)
    }
}

fn or(s: &str, d: &str) -> String {
    if s.trim().is_empty() {
        d.into()
    } else {
        s.trim().into()
    }
}

/// (token, provider) of active devices of active users who may see `site` (None = only `user`).
pub async fn recipients(
    pool: &Pool,
    site: Option<&str>,
    user: Option<&str>,
) -> Result<Vec<(String, String)>> {
    let (site, user) = (site.map(str::to_string), user.map(str::to_string));
    pool.run(move |c| {
        let toks: Vec<(String, String, String, String)> = push_tokens::table
            .select((
                push_tokens::token,
                push_tokens::provider,
                push_tokens::user_id,
                push_tokens::device_fp,
            ))
            .load(c)?;
        let active_dev: HashSet<String> = devices::table
            .filter(devices::status.eq("active"))
            .select(devices::fingerprint)
            .load::<String>(c)?
            .into_iter()
            .collect();
        let us: HashMap<String, (String, String)> = users::table
            .select((users::id, users::role, users::status))
            .load::<(String, String, String)>(c)?
            .into_iter()
            .map(|(i, r, s)| (i, (r, s)))
            .collect();
        let scoped: HashSet<String> = match &site {
            Some(s) => site_scopes::table
                .filter(site_scopes::site_id.eq(s))
                .select(site_scopes::user_id)
                .load::<String>(c)?
                .into_iter()
                .collect(),
            None => HashSet::new(),
        };
        Ok(toks
            .into_iter()
            .filter(|(_, _, u, fp)| {
                let Some((role, status)) = us.get(u) else {
                    return false;
                };
                status == "active"
                    && active_dev.contains(fp)
                    && match (&user, &site) {
                        (Some(only), _) => only == u,
                        (None, Some(_)) => role != "sre" || scoped.contains(u),
                        (None, None) => false,
                    }
            })
            .map(|(t, p, _, _)| (t, p))
            .collect())
    })
    .await
}

/// Push `p` to `targets`, delete gone tokens, audit the send. Returns (sent, failed).
pub async fn deliver(
    push: &Push,
    pool: &Pool,
    targets: Vec<(String, String)>,
    p: &Payload,
    actor: &str,
) -> (i32, i32) {
    let (mut ok, mut bad, mut gone) = (0, 0, vec![]);
    for (t, prov) in &targets {
        match push.send(prov, t, p).await {
            Sent::Ok => ok += 1,
            Sent::Gone => {
                bad += 1;
                gone.push(t.clone());
            }
            Sent::Failed => bad += 1,
        }
    }
    if !gone.is_empty() {
        let g = gone.clone();
        let _ = pool
            .run(move |c| {
                Ok(
                    diesel::delete(push_tokens::table.filter(push_tokens::token.eq_any(g)))
                        .execute(c)?,
                )
            })
            .await;
    }
    let payload = json!({
        "kind": p.kind, "site": p.site, "alert": p.alert, "severity": p.severity,
        "devices": targets.len(), "sent": ok, "failed": bad, "tokens_removed": gone.len(),
    });
    if let Err(e) = crate::audit::append(
        pool,
        &crate::audit::Entry {
            actor: actor.into(),
            actor_key: String::new(),
            action: "push.send".into(),
            target: p.site.clone(),
            payload: payload.to_string().into_bytes(),
            request_sig: String::new(),
        },
    )
    .await
    {
        tracing::warn!(error=%e, "push audit");
    }
    (ok, bad)
}

/// A newly opened alert: push it (in the background) to everyone who may see the site.
pub fn alert_opened(push: Arc<Push>, pool: Pool, site: String, alert: String, severity: String) {
    if !push.wants(&severity) {
        return;
    }
    tokio::spawn(async move {
        let targets = match recipients(&pool, Some(&site), None).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error=%e, "push recipients");
                return;
            }
        };
        if targets.is_empty() {
            return;
        }
        let p = Payload {
            kind: "alert".into(),
            site,
            alert,
            severity,
        };
        deliver(&push, &pool, targets, &p, "adminsvr").await;
    });
}
