//! Site CA, server and agent certificates, and the pinned-TLS client connector.
//!
//! - The site CA (ECDSA P-256) issues the server's TLS certificate and every agent client
//!   certificate. The server requires and verifies client certificates on `Link` (mTLS).
//! - Agents do not trust any CA for the server: they pin the server certificate's SHA-256 from
//!   the enrollment token and check the handshake signature with that certificate's key.
use anyhow::{anyhow, Context, Result};
use rcgen::{
    BasicConstraints, CertificateParams, CertificateSigningRequestParams, DistinguishedName,
    DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SanType, SerialNumber,
};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::sync::Arc;

/// PEM certificate + PEM private key.
#[derive(Clone, Debug)]
pub struct Pem {
    pub cert: String,
    pub key: String,
}

fn rand_serial() -> SerialNumber {
    let mut b = [0u8; 16];
    // uuid-free randomness: rustls' ring provider
    ring_fill(&mut b);
    b[0] &= 0x7f;
    SerialNumber::from_slice(&b)
}

fn ring_fill(b: &mut [u8]) {
    let p = provider();
    p.secure_random.fill(b).expect("random");
}

/// The crypto provider used everywhere (ring).
pub fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn days_from_now(days: i64) -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc() + time::Duration::days(days)
}

/// A new self-signed site CA (10 years).
pub fn new_ca(name: &str) -> Result<Pem> {
    let mut p = CertificateParams::new(Vec::<String>::new())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, name);
    p.distinguished_name = dn;
    p.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    p.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    p.not_before = days_from_now(-1);
    p.not_after = days_from_now(3650);
    p.serial_number = Some(rand_serial());
    let key = KeyPair::generate()?;
    let cert = p.self_signed(&key)?;
    Ok(Pem {
        cert: cert.pem(),
        key: key.serialize_pem(),
    })
}

fn load_ca(ca: &Pem) -> Result<(rcgen::Certificate, KeyPair)> {
    let key = KeyPair::from_pem(&ca.key).context("site CA key")?;
    let params = CertificateParams::from_ca_cert_pem(&ca.cert).context("site CA certificate")?;
    let cert = params.self_signed(&key)?;
    Ok((cert, key))
}

/// The server's TLS certificate, issued by the site CA. Names are informational (agents pin).
pub fn issue_server(ca: &Pem, names: &[String], days: i64) -> Result<Pem> {
    let (ca_cert, ca_key) = load_ca(ca)?;
    let mut p = CertificateParams::new(Vec::<String>::new())?;
    p.subject_alt_names = names
        .iter()
        .filter_map(|n| match n.parse::<std::net::IpAddr>() {
            Ok(ip) => Some(SanType::IpAddress(ip)),
            Err(_) => n.clone().try_into().ok().map(SanType::DnsName),
        })
        .collect();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "itcmdr-server");
    p.distinguished_name = dn;
    p.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    p.not_before = days_from_now(-1);
    p.not_after = days_from_now(days);
    p.serial_number = Some(rand_serial());
    let key = KeyPair::generate()?;
    let cert = p.signed_by(&key, &ca_cert, &ca_key)?;
    Ok(Pem {
        cert: cert.pem(),
        key: key.serialize_pem(),
    })
}

/// A fresh agent TLS key and its CSR (PEM). The key stays on the agent.
pub fn new_csr(machine: &str) -> Result<(String, String)> {
    let mut p = CertificateParams::new(Vec::<String>::new())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, machine);
    p.distinguished_name = dn;
    let key = KeyPair::generate()?;
    let csr = p.serialize_request(&key)?;
    Ok((csr.pem()?, key.serialize_pem()))
}

/// An issued client certificate.
pub struct Issued {
    pub cert_pem: String,
    pub fingerprint: String,
    pub serial: String,
    pub not_after_ms: i64,
}

/// Sign an agent CSR: CN = agent id, SAN = machine, client auth only. Everything the CSR asks
/// for besides its public key is ignored.
pub fn sign_csr(
    ca: &Pem,
    csr_pem: &str,
    agent_id: &str,
    machine: &str,
    lifetime_mins: i64,
) -> Result<Issued> {
    let (ca_cert, ca_key) = load_ca(ca)?;
    let mut csr = CertificateSigningRequestParams::from_pem(csr_pem).context("invalid CSR")?;
    let mut p = CertificateParams::new(Vec::<String>::new())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, agent_id);
    p.distinguished_name = dn;
    p.subject_alt_names = vec![SanType::DnsName(
        machine
            .to_string()
            .try_into()
            .map_err(|_| anyhow!("bad machine name"))?,
    )];
    p.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    p.is_ca = IsCa::ExplicitNoCa;
    let not_after = time::OffsetDateTime::now_utc() + time::Duration::minutes(lifetime_mins);
    p.not_before = days_from_now(-1);
    p.not_after = not_after;
    let serial = rand_serial();
    p.serial_number = Some(serial.clone());
    csr.params = p;
    let cert = csr.signed_by(&ca_cert, &ca_key)?;
    Ok(Issued {
        fingerprint: crate::sha256_hex(cert.der()),
        cert_pem: cert.pem(),
        serial: hex::encode(serial.as_ref()),
        not_after_ms: (not_after.unix_timestamp_nanos() / 1_000_000) as i64,
    })
}

/// DER of the first certificate in a PEM string.
pub fn pem_to_der(pem: &str) -> Result<CertificateDer<'static>> {
    let (_, p) =
        x509_parser::pem::parse_x509_pem(pem.as_bytes()).map_err(|e| anyhow!("PEM: {e}"))?;
    Ok(CertificateDer::from(p.contents))
}

/// Serial number (hex) of a DER certificate, as stored at issue time.
pub fn serial_of(der: &[u8]) -> Result<String> {
    let (_, c) =
        x509_parser::parse_x509_certificate(der).map_err(|e| anyhow!("certificate: {e}"))?;
    Ok(hex::encode(c.tbs_certificate.raw_serial()))
}

/// SHA-256 hex of the first certificate in a PEM string.
pub fn pem_fingerprint(pem: &str) -> Result<String> {
    Ok(crate::sha256_hex(pem_to_der(pem)?.as_ref()))
}

fn key_der(pem: &str) -> Result<PrivateKeyDer<'static>> {
    let k = KeyPair::from_pem(pem).context("private key")?;
    PrivateKeyDer::try_from(k.serialize_der()).map_err(|e| anyhow!("{e}"))
}

/// Accepts exactly one server certificate (by SHA-256) and checks the handshake signature.
#[derive(Debug)]
struct Pinned {
    pin: String,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let got = crate::sha256_hex(end_entity.as_ref());
        if got.eq_ignore_ascii_case(&self.pin) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(format!(
                "server certificate fingerprint {got} does not match the pinned {}",
                self.pin
            )))
        }
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// rustls client config: pinned server cert, optional client certificate, ALPN h2.
pub fn client_config(pin: &str, client: Option<&Pem>) -> Result<rustls::ClientConfig> {
    let prov = provider();
    let b = rustls::ClientConfig::builder_with_provider(prov.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Pinned {
            pin: pin.to_lowercase(),
            provider: prov,
        }));
    let mut c = match client {
        Some(p) => b.with_client_auth_cert(vec![pem_to_der(&p.cert)?], key_der(&p.key)?)?,
        None => b.with_no_client_auth(),
    };
    c.alpn_protocols = vec![b"h2".to_vec()];
    Ok(c)
}

/// Open a gRPC channel to `addr` (host:port) over pinned TLS.
pub async fn connect(
    addr: &str,
    pin: &str,
    client: Option<&Pem>,
) -> Result<tonic::transport::Channel> {
    let cfg = Arc::new(client_config(pin, client)?);
    let addr = addr.to_string();
    let target = addr.clone();
    let connector = tower::service_fn(move |_: http_uri::Uri| {
        let cfg = cfg.clone();
        let target = target.clone();
        async move {
            let tcp = tokio::net::TcpStream::connect(&target).await?;
            let host = target.rsplit_once(':').map(|(h, _)| h).unwrap_or(&target);
            let host = host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();
            let name = ServerName::try_from(host)
                .unwrap_or_else(|_| ServerName::try_from("itcmdr-server").unwrap())
                .to_owned();
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
    Ok(ep.connect_with_connector(connector).await?)
}

/// tonic server TLS: the server identity, and client certificates verified against the site CA
/// (optional at the TLS layer so `Enroll` works; `Link` requires one).
/// Decides whether a client certificate serial (hex) is refused in the handshake.
pub type SerialCheck = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// Client verifier: chain to the site CA (WebPKI), then refuse revoked/replaced serials, so a
/// revoked agent cannot even finish the TLS handshake. Client auth stays optional (Enroll).
#[derive(Debug)]
struct SerialVerifier {
    inner: Arc<dyn rustls::server::danger::ClientCertVerifier>,
    revoked: Revoked,
}

struct Revoked(SerialCheck);
impl std::fmt::Debug for Revoked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SerialCheck")
    }
}

impl rustls::server::danger::ClientCertVerifier for SerialVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }
    fn client_auth_mandatory(&self) -> bool {
        false
    }
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        self.inner.root_hint_subjects()
    }
    fn verify_client_cert(
        &self,
        end: &CertificateDer<'_>,
        inter: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        let ok = self.inner.verify_client_cert(end, inter, now)?;
        let serial = serial_of(end.as_ref()).map_err(|_| {
            rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding)
        })?;
        if (self.revoked.0)(&serial) {
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::Revoked,
            ));
        }
        Ok(ok)
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(m, c, d)
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(m, c, d)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Server TLS config for the agent link (TLS 1.3, h2, optional client certs checked by serial).
pub fn server_config(
    server: &Pem,
    ca_cert_pem: &str,
    revoked: SerialCheck,
) -> Result<rustls::ServerConfig> {
    let prov = provider();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(pem_to_der(ca_cert_pem)?)?;
    let inner =
        rustls::server::WebPkiClientVerifier::builder_with_provider(Arc::new(roots), prov.clone())
            .allow_unauthenticated()
            .build()
            .map_err(|e| anyhow!("client verifier: {e}"))?;
    let mut c = rustls::ServerConfig::builder_with_provider(prov)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(Arc::new(SerialVerifier {
            inner,
            revoked: Revoked(revoked),
        }))
        .with_single_cert(vec![pem_to_der(&server.cert)?], key_der(&server.key)?)?;
    c.alpn_protocols = vec![b"h2".to_vec()];
    Ok(c)
}

/// v1.1.0: like [`server_config`] but the server certificate comes from `resolver` (adminsvr's
/// ACME/pinned resolver) and the ALPN list is `alpn` (adds `acme-tls/1` for TLS-ALPN-01).
pub fn server_config_resolver(
    resolver: Arc<dyn rustls::server::ResolvesServerCert>,
    ca_cert_pem: &str,
    revoked: SerialCheck,
    alpn: &[&[u8]],
) -> Result<rustls::ServerConfig> {
    let prov = provider();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(pem_to_der(ca_cert_pem)?)?;
    let inner =
        rustls::server::WebPkiClientVerifier::builder_with_provider(Arc::new(roots), prov.clone())
            .allow_unauthenticated()
            .build()
            .map_err(|e| anyhow!("client verifier: {e}"))?;
    let mut c = rustls::ServerConfig::builder_with_provider(prov)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])?
        .with_client_cert_verifier(Arc::new(SerialVerifier {
            inner,
            revoked: Revoked(revoked),
        }))
        .with_cert_resolver(resolver);
    c.alpn_protocols = alpn.iter().map(|a| a.to_vec()).collect();
    Ok(c)
}

/// A rustls certified key from PEM (chain + key).
pub fn certified_key(chain_pem: &str, key_pem: &str) -> Result<Arc<rustls::sign::CertifiedKey>> {
    let chain = crate::trust::pem_certs(chain_pem);
    if chain.is_empty() {
        anyhow::bail!("no certificate in the PEM");
    }
    let key = provider()
        .key_provider
        .load_private_key(key_der(key_pem)?)
        .map_err(|e| anyhow!("private key: {e}"))?;
    Ok(Arc::new(rustls::sign::CertifiedKey::new(chain, key)))
}

/// TLS connections accepted on `listener` (each handshake runs in its own task, with a timeout;
/// failed handshakes are logged by the caller-supplied hook and dropped).
pub fn incoming(
    listener: tokio::net::TcpListener,
    cfg: rustls::ServerConfig,
    on_fail: Arc<dyn Fn(std::net::SocketAddr, String) + Send + Sync>,
) -> impl tokio_stream::Stream<
    Item = std::io::Result<tokio_rustls::server::TlsStream<tokio::net::TcpStream>>,
> {
    let acc = tokio_rustls::TlsAcceptor::from(Arc::new(cfg));
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(async move {
        loop {
            let (tcp, addr) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let (acc, tx2, on_fail) = (acc.clone(), tx.clone(), on_fail.clone());
            tokio::spawn(async move {
                let r =
                    tokio::time::timeout(std::time::Duration::from_secs(20), acc.accept(tcp)).await;
                match r {
                    // TLS-ALPN-01 validation handshakes end here (RFC 8737).
                    Ok(Ok(s)) if s.get_ref().1.alpn_protocol() == Some(b"acme-tls/1") => {}
                    Ok(Ok(s)) => {
                        let _ = tx2.send(Ok(s)).await;
                    }
                    Ok(Err(e)) => on_fail(addr, e.to_string()),
                    Err(_) => on_fail(addr, "handshake timeout".into()),
                }
            });
            if tx.is_closed() {
                break;
            }
        }
    });
    tokio_stream::wrappers::ReceiverStream::new(rx)
}

pub(crate) mod http_uri {
    pub use tonic::codegen::http::Uri;
}

/// v1.1.0 site server API: HTTPS, or plain HTTP only where the caller allows it (loopback, or an
/// explicit opt-in). The first byte decides (0x16 = a TLS handshake).
pub enum MaybeTls {
    Tls(Box<tokio_rustls::server::TlsStream<tokio::net::TcpStream>>),
    Plain(tokio::net::TcpStream),
}

impl tokio::io::AsyncRead for MaybeTls {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_read(cx, buf),
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for MaybeTls {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            MaybeTls::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_write(cx, buf),
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_flush(cx),
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_shutdown(cx),
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl tonic::transport::server::Connected for MaybeTls {
    type ConnectInfo = tonic::transport::server::TcpConnectInfo;
    fn connect_info(&self) -> Self::ConnectInfo {
        match self {
            MaybeTls::Tls(s) => s.get_ref().0.connect_info(),
            MaybeTls::Plain(s) => s.connect_info(),
        }
    }
}

/// Server config for the site API: one certificate, no client certificates, TLS 1.2/1.3, ALPN
/// h2 and http/1.1 (gRPC-Web).
pub fn api_server_config(server: &Pem) -> Result<rustls::ServerConfig> {
    let mut c = rustls::ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])?
        .with_no_client_auth()
        .with_single_cert(vec![pem_to_der(&server.cert)?], key_der(&server.key)?)?;
    c.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(c)
}

pub type PlainCheck = Arc<dyn Fn(std::net::SocketAddr) -> bool + Send + Sync>;

/// Connections on `listener`: TLS ones are handshaken with `cfg`; plain ones are passed through
/// only when `allow_plain(peer)` says so, and dropped otherwise.
pub fn incoming_mixed(
    listener: tokio::net::TcpListener,
    cfg: rustls::ServerConfig,
    allow_plain: PlainCheck,
    on_fail: Arc<dyn Fn(std::net::SocketAddr, String) + Send + Sync>,
) -> impl tokio_stream::Stream<Item = std::io::Result<MaybeTls>> {
    let acc = tokio_rustls::TlsAcceptor::from(Arc::new(cfg));
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(async move {
        loop {
            let (tcp, addr) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let _ = tcp.set_nodelay(true);
            let (acc, tx2, on_fail, allow) = (
                acc.clone(),
                tx.clone(),
                on_fail.clone(),
                allow_plain.clone(),
            );
            tokio::spawn(async move {
                let mut b = [0u8; 1];
                let first =
                    tokio::time::timeout(std::time::Duration::from_secs(20), tcp.peek(&mut b))
                        .await;
                match first {
                    Ok(Ok(1)) if b[0] == 0x16 => {
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(20),
                            acc.accept(tcp),
                        )
                        .await
                        {
                            Ok(Ok(s)) => {
                                let _ = tx2.send(Ok(MaybeTls::Tls(Box::new(s)))).await;
                            }
                            Ok(Err(e)) => on_fail(addr, e.to_string()),
                            Err(_) => on_fail(addr, "handshake timeout".into()),
                        }
                    }
                    Ok(Ok(1)) if allow(addr) => {
                        let _ = tx2.send(Ok(MaybeTls::Plain(tcp))).await;
                    }
                    Ok(Ok(1)) => on_fail(addr, "plain HTTP refused (HTTPS only)".into()),
                    _ => {}
                }
            });
            if tx.is_closed() {
                break;
            }
        }
    });
    tokio_stream::wrappers::ReceiverStream::new(rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ca_issues_server_and_client() {
        let ca = new_ca("test CA").unwrap();
        let s = issue_server(&ca, &["localhost".into(), "127.0.0.1".into()], 30).unwrap();
        assert_eq!(pem_fingerprint(&s.cert).unwrap().len(), 64);
        let (csr, _key) = new_csr("WS-01").unwrap();
        let i = sign_csr(&ca, &csr, "agent-1", "WS-01", 90 * 1440).unwrap();
        assert_eq!(i.fingerprint, pem_fingerprint(&i.cert_pem).unwrap());
        assert!(i.not_after_ms > 0);
        assert!(sign_csr(&ca, "junk", "a", "WS-01", 90).is_err());
        assert_eq!(
            serial_of(&pem_to_der(&i.cert_pem).unwrap()).unwrap(),
            i.serial
        );
    }
}
