//! v1.1.0 ACME (Let's Encrypt) certificates for the admin API, off by default.
//!
//! - Challenge: TLS-ALPN-01 on the API listener itself (no extra port; the CA connects to port
//!   443 of the domain, so forward 443 to `api_addr`), or HTTP-01 on `acme_http_addr` (port 80).
//! - Which certificate a client gets is decided per handshake (SNI): the configured domains get the
//!   ACME certificate; [`itcmdr_agentlink::trust::PINNED_SNI`], IPs and other names get adminsvr's
//!   internal certificate, which enrolled devices pin. The SiteLink never uses the ACME
//!   certificate: sites keep their pinned certificate and mTLS.
//! - The account key and the certificate live in the cache dir (0700, files 0600). Renewal runs
//!   `renew_days` before expiry and swaps the certificate in place (no restart). A failed renewal
//!   keeps the current certificate, records the error and opens the `tls-acme` alert; the next
//!   success resolves it.
use crate::config::{AcmeConfig, Challenge};
use crate::link::Hub;
use crate::store::Pool;
use anyhow::{anyhow, bail, Context, Result};
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
    OrderStatus, RetryPolicy,
};
use itcmdr_adminlink::pb;
use parking_lot::RwLock;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub const ALERT_ID: &str = "tls-acme";
pub const META_KEY: &str = "acme.status";
/// The scope used for adminsvr's own alerts (not a site).
pub const OWN_SITE: &str = "adminsvr";

/// Picks the certificate for each handshake.
pub struct Resolver {
    internal: Arc<CertifiedKey>,
    domains: Vec<String>,
    acme: RwLock<Option<Arc<CertifiedKey>>>,
    alpn: RwLock<HashMap<String, Arc<CertifiedKey>>>,
}

impl std::fmt::Debug for Resolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resolver")
            .field("domains", &self.domains)
            .finish()
    }
}

impl Resolver {
    pub fn new(internal: Arc<CertifiedKey>, domains: &[String]) -> Arc<Self> {
        Arc::new(Resolver {
            internal,
            domains: domains.iter().map(|d| d.to_ascii_lowercase()).collect(),
            acme: RwLock::new(None),
            alpn: RwLock::new(HashMap::new()),
        })
    }
    pub fn install(&self, k: Arc<CertifiedKey>) {
        *self.acme.write() = Some(k);
    }
    pub fn has_acme(&self) -> bool {
        self.acme.read().is_some()
    }
}

impl ResolvesServerCert for Resolver {
    fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = hello.server_name().map(|s| s.to_ascii_lowercase());
        let challenge = hello
            .alpn()
            .map(|mut a| a.any(|p| p == b"acme-tls/1"))
            .unwrap_or(false);
        if challenge {
            return sni.and_then(|s| self.alpn.read().get(&s).cloned());
        }
        if let Some(s) = &sni {
            if self.domains.iter().any(|d| d == s) {
                if let Some(k) = self.acme.read().clone() {
                    return Some(k);
                }
            }
        }
        Some(self.internal.clone())
    }
}

/// What `tls-status` and `Info` report.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Status {
    pub domains: Vec<String>,
    pub expires_ms: i64,
    pub renewed_ms: i64,
    pub fingerprint: String,
    pub issuer: String,
    pub last_attempt_ms: i64,
    pub last_error: String,
    pub last_error_ms: i64,
}

pub async fn load_status(pool: &Pool) -> Status {
    pool.run(|c| {
        use crate::schema::meta;
        use diesel::prelude::*;
        let v: Option<String> = meta::table
            .filter(meta::k.eq(META_KEY))
            .select(meta::v)
            .first(c)
            .optional()?;
        Ok(v.and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default())
    })
    .await
    .unwrap_or_default()
}

async fn save_status(pool: &Pool, s: &Status) {
    let v = serde_json::to_string(s).unwrap_or_default();
    if let Err(e) = pool
        .run(move |c| crate::passkeys::put_meta(c, META_KEY, &v))
        .await
    {
        tracing::warn!(error=%e, "saving the ACME status");
    }
}

fn write_private(p: &Path, data: &[u8]) -> Result<()> {
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, p)?;
    Ok(())
}

fn ensure_dir(d: &Path) -> Result<()> {
    std::fs::create_dir_all(d).with_context(|| format!("ACME cache dir {}", d.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Leaf facts of a PEM chain: (not_after_ms, fingerprint, issuer, names).
pub fn leaf_info(chain_pem: &str) -> Result<(i64, String, String, Vec<String>)> {
    let certs = itcmdr_agentlink::trust::pem_certs(chain_pem);
    let leaf = certs
        .first()
        .ok_or_else(|| anyhow!("empty certificate chain"))?;
    let r = itcmdr_agentlink::trust::report_for(leaf.as_ref(), "ok", "");
    Ok((r.not_after_ms, r.fingerprint, r.issuer, r.names))
}

/// The cached certificate, when it covers every configured domain.
fn cached(cfg: &AcmeConfig) -> Option<(String, String, i64)> {
    let chain = std::fs::read_to_string(cfg.cache_dir.join("cert.pem")).ok()?;
    let key = std::fs::read_to_string(cfg.cache_dir.join("key.pem")).ok()?;
    let (exp, _, _, names) = leaf_info(&chain).ok()?;
    let names: Vec<String> = names.iter().map(|n| n.to_ascii_lowercase()).collect();
    cfg.domains
        .iter()
        .all(|d| names.contains(&d.to_ascii_lowercase()))
        .then_some((chain, key, exp))
}

async fn account(cfg: &AcmeConfig) -> Result<Account> {
    let builder = || -> Result<_> {
        Ok(match &cfg.directory_root {
            Some(p) => Account::builder_with_root(p)?,
            None => Account::builder()?,
        })
    };
    let f = cfg.cache_dir.join("account.json");
    if let Ok(b) = std::fs::read(&f) {
        let creds: instant_acme::AccountCredentials =
            serde_json::from_slice(&b).context("ACME account file")?;
        return Ok(builder()?.from_credentials(creds).await?);
    }
    let contact = cfg
        .contact
        .iter()
        .map(|c| {
            if c.starts_with("mailto:") {
                c.clone()
            } else {
                format!("mailto:{c}")
            }
        })
        .collect::<Vec<_>>();
    let refs: Vec<&str> = contact.iter().map(|s| s.as_str()).collect();
    let (acc, creds) = builder()?
        .create(
            &NewAccount {
                contact: &refs,
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            directory_url(&cfg.directory),
            None,
        )
        .await?;
    write_private(&f, &serde_json::to_vec(&creds)?)?;
    Ok(acc)
}

pub fn directory_url(d: &str) -> String {
    match d.trim() {
        "" | "production" => LetsEncrypt::Production.url().to_string(),
        "staging" => LetsEncrypt::Staging.url().to_string(),
        u => u.to_string(),
    }
}

/// HTTP-01 tokens served on `acme_http_addr`.
pub type Tokens = Arc<RwLock<HashMap<String, String>>>;

/// One order: (chain PEM, key PEM).
async fn issue(cfg: &AcmeConfig, res: &Resolver, tokens: &Tokens) -> Result<(String, String)> {
    let acc = account(cfg).await?;
    let ids: Vec<Identifier> = cfg
        .domains
        .iter()
        .map(|d| Identifier::Dns(d.clone()))
        .collect();
    let mut order = acc.new_order(&NewOrder::new(&ids)).await?;
    let mut auths = order.authorizations();
    while let Some(a) = auths.next().await {
        let mut a = a?;
        match a.status {
            AuthorizationStatus::Pending => {}
            AuthorizationStatus::Valid => continue,
            s => bail!("authorization is {s:?}"),
        }
        let ty = match cfg.challenge {
            Challenge::TlsAlpn01 => ChallengeType::TlsAlpn01,
            Challenge::Http01 => ChallengeType::Http01,
        };
        let mut ch = a
            .challenge(ty.clone())
            .ok_or_else(|| anyhow!("the CA offers no {ty:?} challenge"))?;
        let domain = ch.identifier().to_string().to_ascii_lowercase();
        let ka = ch.key_authorization();
        match cfg.challenge {
            Challenge::TlsAlpn01 => {
                let mut p = rcgen::CertificateParams::new(vec![domain.clone()])?;
                p.custom_extensions = vec![rcgen::CustomExtension::new_acme_identifier(
                    ka.digest().as_ref(),
                )];
                let k = rcgen::KeyPair::generate()?;
                let c = p.self_signed(&k)?;
                let ck = itcmdr_agentlink::tls::certified_key(&c.pem(), &k.serialize_pem())?;
                res.alpn.write().insert(domain, ck);
            }
            Challenge::Http01 => {
                tokens
                    .write()
                    .insert(ch.token.clone(), ka.as_str().to_string());
            }
        }
        ch.set_ready().await?;
    }
    let retry = RetryPolicy::default();
    let st = order.poll_ready(&retry).await;
    res.alpn.write().clear();
    tokens.write().clear();
    let st = st?;
    if st != OrderStatus::Ready {
        bail!("order is {st:?}");
    }
    let key = order.finalize().await?;
    let chain = order.poll_certificate(&retry).await?;
    Ok((chain, key))
}

async fn alert(pool: &Pool, hub: &Hub, open: bool, critical: bool) {
    let now = crate::now_ms();
    let a = pb::SiteAlert {
        id: ALERT_ID.into(),
        code: "tls-acme-renewal-failed".into(),
        severity: if critical { "critical" } else { "cautious" }.into(),
        state: if open { "open" } else { "resolved" }.into(),
        source: "adminsvr".into(),
        first_seen_ms: now,
        last_seen_ms: now,
        count: 1,
        ..Default::default()
    };
    if !open {
        // Only resolve an alert that exists.
        let exists = pool
            .run(|c| {
                use crate::schema::alerts;
                use diesel::prelude::*;
                let n: i64 = alerts::table
                    .filter(
                        alerts::site_id
                            .eq(OWN_SITE)
                            .and(alerts::alert_id.eq(ALERT_ID))
                            .and(alerts::state.ne("resolved")),
                    )
                    .count()
                    .get_result(c)?;
                Ok(n > 0)
            })
            .await
            .unwrap_or(false);
        if !exists {
            return;
        }
    }
    if let Err(e) = crate::link::store_and_push(pool, hub, OWN_SITE, "adminsvr", &a).await {
        tracing::warn!(error=%e, "ACME alert");
    }
}

/// Serve HTTP-01 tokens (and nothing else) on `addr`.
pub async fn serve_http01(addr: String, tokens: Tokens) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let l = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("bind acme_http_addr {addr}"))?;
    tracing::info!(%addr, "ACME HTTP-01 listener");
    loop {
        let (mut s, _) = match l.accept().await {
            Ok(x) => x,
            Err(_) => continue,
        };
        let tokens = tokens.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];
            let n = match tokio::time::timeout(Duration::from_secs(10), s.read(&mut buf)).await {
                Ok(Ok(n)) => n,
                _ => return,
            };
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req.split_whitespace().nth(1).unwrap_or("");
            let body = path
                .strip_prefix("/.well-known/acme-challenge/")
                .and_then(|t| tokens.read().get(t).cloned());
            let resp = match body {
                Some(b) => format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}",
                    b.len()
                ),
                None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
            };
            let _ = s.write_all(resp.as_bytes()).await;
        });
    }
}

/// Load the cached certificate (if any) into `res`; returns its expiry.
pub fn load_cached(cfg: &AcmeConfig, res: &Resolver) -> Option<i64> {
    let (chain, key, exp) = cached(cfg)?;
    match itcmdr_agentlink::tls::certified_key(&chain, &key) {
        Ok(k) => {
            res.install(k);
            Some(exp)
        }
        Err(e) => {
            tracing::warn!(error=%e, "cached ACME certificate unusable");
            None
        }
    }
}

/// One renewal check: issue when there is no certificate or it expires within `renew_days`.
/// Returns the expiry of the certificate now being served (0 = none).
pub async fn check_once(
    pool: &Pool,
    hub: &Hub,
    cfg: &AcmeConfig,
    res: &Resolver,
    tokens: &Tokens,
) -> i64 {
    let mut st = load_status(pool).await;
    st.domains = cfg.domains.clone();
    let now = crate::now_ms();
    let current = cached(cfg).map(|(_, _, e)| e).filter(|_| res.has_acme());
    let due = match current {
        None => true,
        Some(e) => e - now < cfg.renew_days * 86_400_000,
    };
    if !due {
        return current.unwrap_or(0);
    }
    st.last_attempt_ms = now;
    let r = async {
        ensure_dir(&cfg.cache_dir)?;
        let (chain, key) = issue(cfg, res, tokens).await?;
        let k = itcmdr_agentlink::tls::certified_key(&chain, &key)?;
        let (exp, fp, issuer, _) = leaf_info(&chain)?;
        write_private(&cfg.cache_dir.join("key.pem"), key.as_bytes())?;
        write_private(&cfg.cache_dir.join("cert.pem"), chain.as_bytes())?;
        res.install(k);
        Ok::<_, anyhow::Error>((exp, fp, issuer))
    }
    .await;
    match r {
        Ok((exp, fp, issuer)) => {
            tracing::info!(expires_ms = exp, "ACME certificate installed");
            st.expires_ms = exp;
            st.renewed_ms = crate::now_ms();
            st.fingerprint = fp;
            st.issuer = issuer;
            st.last_error.clear();
            st.last_error_ms = 0;
            save_status(pool, &st).await;
            alert(pool, hub, false, false).await;
            exp
        }
        Err(e) => {
            let msg = format!("{e:#}");
            tracing::warn!(error=%msg, "ACME renewal failed; keeping the current certificate");
            st.last_error = msg.chars().take(500).collect();
            st.last_error_ms = crate::now_ms();
            save_status(pool, &st).await;
            let left = current.map(|e| e - now).unwrap_or(0);
            alert(pool, hub, true, left < 7 * 86_400_000).await;
            current.unwrap_or(0)
        }
    }
}

/// Renewal loop until `stop` flips.
pub async fn run(
    pool: Pool,
    hub: Hub,
    cfg: AcmeConfig,
    res: Arc<Resolver>,
    tokens: Tokens,
    mut stop: tokio::sync::watch::Receiver<bool>,
) {
    if let Some(exp) = load_cached(&cfg, &res) {
        let mut st = load_status(&pool).await;
        if st.expires_ms != exp {
            st.expires_ms = exp;
            save_status(&pool, &st).await;
        }
    }
    loop {
        let before = load_status(&pool).await.last_error_ms;
        let exp = check_once(&pool, &hub, &cfg, &res, &tokens).await;
        let failed = load_status(&pool).await.last_error_ms > before;
        let now = crate::now_ms();
        let wait_ms = if failed {
            cfg.retry_secs as i64 * 1000
        } else {
            let renew_at = exp - cfg.renew_days * 86_400_000;
            (renew_at - now).clamp(60_000, 12 * 3_600_000)
        };
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(wait_ms.max(1000) as u64)) => {}
            _ = stop.changed() => return,
        }
    }
}
