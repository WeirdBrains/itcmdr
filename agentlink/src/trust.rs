//! v1.1.0 client trust for the apps and CLIs (admin app, site app bridge, itcmdr-admin,
//! itcmdr-cli): HTTPS by default with full validation against the system trust store (chain,
//! hostname, expiry). When that fails, the caller gets a [`CertProblem`] with the certificate's
//! SHA-256 fingerprint, subject and expiry so the user can choose "Trust this certificate", which
//! pins that exact certificate for that one server (trust on first use). A pinned certificate is
//! accepted while it is byte-for-byte the same; a different one that the system store also
//! refuses is blocked (`pin-changed`) until the user reviews and re-trusts it. A certificate the
//! system store accepts for the right name is accepted even with an older pin (same as the Flutter
//! apps); the pin is kept as the anchor for when the system store refuses. There is no global "ignore errors" switch.
//!
//! Sites and agents are not affected: they keep their own exact pins (`tls::connect`) and mTLS.
use crate::tls::{pem_to_der, provider, Pem};
use anyhow::{anyhow, Result};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, SignatureScheme};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// SNI sent by clients holding an adminsvr pin from an enrollment code: adminsvr answers it with
/// its own internal certificate even when it also serves a public (ACME) one for its domain.
pub const PINNED_SNI: &str = "pinned.itcmdr.invalid";

/// What the user is shown before trusting a certificate.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CertReport {
    /// SHA-256 of the leaf certificate (lower-case hex)
    pub fingerprint: String,
    pub subject: String,
    pub issuer: String,
    pub not_after_ms: i64,
    pub names: Vec<String>,
    /// ok | self-signed | untrusted | expired | not-yet-valid | name-mismatch | pin-changed | invalid
    pub problem: String,
    pub detail: String,
}

impl CertReport {
    /// Problems that need an explicit "trust anyway" (not just "trust this certificate").
    pub fn needs_override(&self) -> bool {
        matches!(
            self.problem.as_str(),
            "expired" | "not-yet-valid" | "name-mismatch"
        )
    }
    pub fn describe(&self) -> String {
        let exp = if self.not_after_ms > 0 {
            let s = self.not_after_ms / 1000;
            let (y, m, d) = civil(s.div_euclid(86_400));
            format!("{y:04}-{m:02}-{d:02}")
        } else {
            "?".into()
        };
        format!(
            "certificate problem: {} ({})\n  SHA-256: {}\n  subject: {}\n  issuer:  {}\n  expires: {}\n  names:   {}",
            self.problem,
            self.detail,
            self.fingerprint,
            self.subject,
            self.issuer,
            exp,
            self.names.join(", ")
        )
    }
}

/// Days since 1970-01-01 to (year, month, day).
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// The certificate was refused; `report` says why and what it is.
#[derive(Debug, Clone)]
pub struct CertProblem(pub CertReport);

impl std::fmt::Display for CertProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.problem == "pin-changed" {
            write!(
                f,
                "WARNING: THE TRUSTED CERTIFICATE FOR THIS SERVER HAS CHANGED. This can mean someone is \
                 intercepting the connection. Refusing to connect.\n{}",
                self.0.describe()
            )
        } else {
            write!(f, "{}", self.0.describe())
        }
    }
}
impl std::error::Error for CertProblem {}

/// How a client trusts one server.
#[derive(Clone, Debug, Default)]
pub struct Trust {
    /// A trusted certificate's SHA-256: only that exact certificate is accepted.
    pub pin: Option<String>,
    /// Send [`PINNED_SNI`] (adminsvr enrollment pins).
    pub internal: bool,
    /// Extra trusted root certificates (PEM), on top of the system store (tests, private CAs).
    pub extra_roots: Vec<String>,
    /// Validate (and send SNI) as this name instead of the host in the address.
    pub name: Option<String>,
}

impl Trust {
    pub fn system() -> Self {
        Trust::default()
    }
    pub fn pinned(fp: &str) -> Self {
        Trust {
            pin: Some(fp.trim().to_lowercase()),
            ..Default::default()
        }
    }
    /// From an `ITCMDR_EXTRA_ROOTS` style file list (path separated by the OS path separator).
    pub fn with_env_roots(mut self) -> Self {
        if let Some(v) = std::env::var_os("ITCMDR_EXTRA_ROOTS") {
            for p in std::env::split_paths(&v) {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    self.extra_roots.push(s);
                }
            }
        }
        self
    }
}

/// Parse a leaf certificate for the report.
pub fn report_for(der: &[u8], problem: &str, detail: &str) -> CertReport {
    let mut r = CertReport {
        fingerprint: crate::sha256_hex(der),
        problem: problem.into(),
        detail: detail.into(),
        ..Default::default()
    };
    if let Ok((_, c)) = x509_parser::parse_x509_certificate(der) {
        r.subject = c.subject().to_string();
        r.issuer = c.issuer().to_string();
        r.not_after_ms = c.validity().not_after.timestamp() * 1000;
        if let Ok(Some(san)) = c.subject_alternative_name() {
            for n in &san.value.general_names {
                match n {
                    x509_parser::extensions::GeneralName::DNSName(d) => r.names.push(d.to_string()),
                    x509_parser::extensions::GeneralName::IPAddress(b) => {
                        r.names.push(match b.len() {
                            4 => std::net::Ipv4Addr::from(<[u8; 4]>::try_from(*b).unwrap())
                                .to_string(),
                            16 => std::net::Ipv6Addr::from(<[u8; 16]>::try_from(*b).unwrap())
                                .to_string(),
                            _ => hex::encode(b),
                        })
                    }
                    _ => {}
                }
            }
        }
    }
    r
}

fn classify(e: &rustls::Error, der: &[u8]) -> (&'static str, String) {
    let self_signed = x509_parser::parse_x509_certificate(der)
        .map(|(_, c)| c.subject() == c.issuer())
        .unwrap_or(false);
    let p = match e {
        rustls::Error::InvalidCertificate(c) => match c {
            CertificateError::Expired | CertificateError::ExpiredContext { .. } => "expired",
            CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => {
                "not-yet-valid"
            }
            CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
                "name-mismatch"
            }
            CertificateError::UnknownIssuer if self_signed => "self-signed",
            CertificateError::UnknownIssuer => "untrusted",
            _ => "invalid",
        },
        _ => "invalid",
    };
    (p, e.to_string())
}

/// System roots (OS store) plus `extra`.
pub fn roots(extra: &[String]) -> rustls::RootCertStore {
    let mut r = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    for c in native.certs {
        let _ = r.add(c);
    }
    for pem in extra {
        for c in pem_certs(pem) {
            let _ = r.add(c);
        }
    }
    r
}

/// Every certificate in a PEM bundle.
pub fn pem_certs(pem: &str) -> Vec<CertificateDer<'static>> {
    let mut out = vec![];
    let mut rest = pem;
    while let Some(i) = rest.find("-----BEGIN CERTIFICATE-----") {
        let tail = &rest[i..];
        let Some(j) = tail.find("-----END CERTIFICATE-----") else {
            break;
        };
        let block = &tail[..j + "-----END CERTIFICATE-----".len()];
        if let Ok(d) = pem_to_der(block) {
            out.push(d);
        }
        rest = &tail[j + 1..];
    }
    out
}

/// `CertReport.detail` prefix when the system store accepted a certificate other than the pinned
/// one (the pin stays as the fallback anchor; nothing is replaced silently).
pub const SYSTEM_OVER_PIN: &str = "trusted by the system store, not the pin";

#[derive(Debug)]
struct TrustVerifier {
    pin: Option<String>,
    webpki: Option<Arc<WebPkiServerVerifier>>,
    provider: Arc<CryptoProvider>,
    seen: Arc<Mutex<Option<CertReport>>>,
}

impl ServerCertVerifier for TrustVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let der = end_entity.as_ref();
        let fp = crate::sha256_hex(der);
        // The system store (plus extra roots) first, as the Flutter apps do: a certificate that
        // validates for this name (chain, hostname, expiry) is accepted even when an older pin
        // is stored. The pin is only the trust anchor for certificates the system store refuses.
        let system = self
            .webpki
            .as_ref()
            .map(|w| w.verify_server_cert(end_entity, intermediates, server_name, ocsp, now));
        let pin_ok = self
            .pin
            .as_ref()
            .is_some_and(|pin| fp.eq_ignore_ascii_case(pin));
        let (res, rep) = match (system, &self.pin) {
            (Some(Ok(v)), Some(pin)) if !pin_ok => (
                Ok(v),
                report_for(
                    der,
                    "ok",
                    &format!(
                        "{SYSTEM_OVER_PIN} (the pinned certificate {pin} is kept as a fallback)"
                    ),
                ),
            ),
            (Some(Ok(v)), _) => (Ok(v), report_for(der, "ok", "trusted by the system store")),
            (_, Some(_)) if pin_ok => (
                Ok(ServerCertVerified::assertion()),
                report_for(der, "ok", "pinned"),
            ),
            (_, Some(pin)) => (
                Err(rustls::Error::General(format!(
                    "server certificate {fp} is not the trusted {pin}"
                ))),
                report_for(
                    der,
                    "pin-changed",
                    &format!("trusted certificate was {pin}"),
                ),
            ),
            (Some(Err(e)), None) => {
                let (p, d) = classify(&e, der);
                (Err(e), report_for(der, p, &d))
            }
            (None, None) => (
                Err(rustls::Error::InvalidCertificate(
                    CertificateError::UnknownIssuer,
                )),
                report_for(der, "untrusted", "no trusted roots on this system"),
            ),
        };
        *self.seen.lock().unwrap() = Some(rep);
        res
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(m, c, d, &self.provider.signature_verification_algorithms)
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(m, c, d, &self.provider.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

type Seen = Arc<Mutex<Option<CertReport>>>;

/// rustls client config for `trust`; the slot receives the report of the last handshake.
pub fn client_config(
    trust: &Trust,
    client: Option<&Pem>,
    alpn: &[&[u8]],
) -> Result<(rustls::ClientConfig, Seen)> {
    let prov = provider();
    let r = roots(&trust.extra_roots);
    let webpki = if r.is_empty() {
        None
    } else {
        Some(
            WebPkiServerVerifier::builder_with_provider(Arc::new(r), prov.clone())
                .build()
                .map_err(|e| anyhow!("verifier: {e}"))?,
        )
    };
    let seen: Seen = Arc::new(Mutex::new(None));
    let b = rustls::ClientConfig::builder_with_provider(prov.clone())
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(TrustVerifier {
            pin: trust.pin.clone(),
            webpki,
            provider: prov,
            seen: seen.clone(),
        }));
    let mut c = match client {
        Some(p) => b.with_client_auth_cert(
            vec![pem_to_der(&p.cert)?],
            rustls::pki_types::PrivateKeyDer::try_from(
                rcgen::KeyPair::from_pem(&p.key)?.serialize_der(),
            )
            .map_err(|e| anyhow!("{e}"))?,
        )?,
        None => b.with_no_client_auth(),
    };
    c.alpn_protocols = alpn.iter().map(|a| a.to_vec()).collect();
    Ok((c, seen))
}

fn split_host(addr: &str) -> String {
    let host = addr.rsplit_once(':').map(|(h, _)| h).unwrap_or(addr);
    host.trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

fn server_name(addr: &str, trust: &Trust) -> ServerName<'static> {
    let h = if trust.internal && trust.pin.is_some() {
        PINNED_SNI.to_string()
    } else if let Some(n) = &trust.name {
        n.clone()
    } else {
        split_host(addr)
    };
    ServerName::try_from(h)
        .unwrap_or_else(|_| ServerName::try_from("itcmdr-server").unwrap())
        .to_owned()
}

/// Strip an `https://` prefix; refuse `http://` (plain HTTP is a separate, explicit call).
pub fn https_addr(addr: &str) -> Result<String> {
    let a = addr.trim();
    if a.starts_with("http://") {
        anyhow::bail!(
            "{a} is plain HTTP (not encrypted); connect with the explicit plain-HTTP option if you really mean it"
        );
    }
    Ok(a.trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string())
}

/// Connect a gRPC channel over HTTPS with `trust`. A refused certificate gives a [`CertProblem`]
/// error (use `downcast_ref`).
pub async fn connect(
    addr: &str,
    trust: &Trust,
    client: Option<&Pem>,
) -> Result<tonic::transport::Channel> {
    let addr = https_addr(addr)?;
    let (cfg, seen) = client_config(trust, client, &[b"h2"])?;
    let cfg = Arc::new(cfg);
    let name = server_name(&addr, trust);
    let target = addr.clone();
    let connector = tower::service_fn(move |_: crate::tls::http_uri::Uri| {
        let (cfg, target, name) = (cfg.clone(), target.clone(), name.clone());
        async move {
            let tcp = tokio::net::TcpStream::connect(&target).await?;
            let tls = tokio_rustls::TlsConnector::from(cfg)
                .connect(name, tcp)
                .await?;
            Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(tls))
        }
    });
    let ep = tonic::transport::Endpoint::from_shared(format!("http://{addr}"))?
        .connect_timeout(std::time::Duration::from_secs(10))
        .http2_keep_alive_interval(std::time::Duration::from_secs(30))
        .keep_alive_while_idle(true);
    match ep.connect_with_connector(connector).await {
        Ok(c) => Ok(c),
        Err(e) => {
            let rep = seen.lock().unwrap().clone();
            match rep {
                Some(r) if r.problem != "ok" => Err(anyhow::Error::new(CertProblem(r))),
                _ => Err(anyhow!("connect {addr}: {e}")),
            }
        }
    }
}

/// Plain HTTP (no encryption). Only for an explicit opt-in; callers print a warning.
pub async fn connect_plain(addr: &str) -> Result<tonic::transport::Channel> {
    let a = addr.trim().trim_start_matches("http://");
    Ok(
        tonic::transport::Endpoint::from_shared(format!("http://{a}"))?
            .connect_timeout(std::time::Duration::from_secs(10))
            .connect()
            .await?,
    )
}

/// Look at a server's certificate without trusting it (for the warning shown to the user).
pub async fn inspect(addr: &str, trust: &Trust) -> Result<CertReport> {
    let addr = https_addr(addr)?;
    let (cfg, seen) = client_config(trust, None, &[b"h2"])?;
    let tcp = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| anyhow!("connect {addr}: timeout"))??;
    let _ = tokio_rustls::TlsConnector::from(Arc::new(cfg))
        .connect(server_name(&addr, trust), tcp)
        .await;
    let r = seen.lock().unwrap().clone();
    r.ok_or_else(|| anyhow!("{addr} did not present a certificate (is it HTTPS?)"))
}

/// Accept `got` as the user's choice only when it is the certificate the server shows now.
pub fn check_choice(report: &CertReport, chosen: &str) -> Result<String> {
    let c = chosen.trim().to_lowercase().replace(':', "");
    if c.len() != 64 || !c.chars().all(|x| x.is_ascii_hexdigit()) {
        anyhow::bail!("a SHA-256 fingerprint is 64 hex characters");
    }
    if c != report.fingerprint {
        anyhow::bail!(
            "the server presents {} (not {c}); nothing was trusted",
            report.fingerprint
        );
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_and_choice() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_759), (2026, 11, 2));
        let r = CertReport {
            fingerprint: "ab".repeat(32),
            ..Default::default()
        };
        assert!(check_choice(&r, &"AB".repeat(32)).is_ok());
        assert!(check_choice(&r, &"cd".repeat(32)).is_err());
        assert!(check_choice(&r, "xyz").is_err());
        assert!(https_addr("http://h:1").is_err());
        assert_eq!(https_addr("https://h:1/").unwrap(), "h:1");
    }
}
