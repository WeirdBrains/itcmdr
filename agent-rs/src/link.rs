//! Enrollment and the heartbeat link.
use crate::state::{self, State};
use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use itcmdr_agentlink::pb::{self, agent_service_client::AgentServiceClient};
use itcmdr_agentlink::tls::{self, Pem};
use std::path::Path;
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn os() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Enroll with a one-time token: new keys, a CSR, the client certificate.
pub async fn enroll(dir: &Path, token: &str, machine: Option<&str>, role: &str) -> Result<State> {
    let t = itcmdr_agentlink::Token::decode(token)?;
    let machine = machine
        .map(str::to_string)
        .unwrap_or_else(state::this_machine);
    if !t.machine.is_empty() && !t.machine.eq_ignore_ascii_case(&machine) {
        bail!("this token is for {}, but this machine is {machine} (use --machine if the name is right)", t.machine);
    }
    let sk = SigningKey::generate(&mut rand::rngs::OsRng);
    let (csr, tls_key) = tls::new_csr(&machine)?;
    let ch = tls::connect(&t.addr, &t.pin, None)
        .await
        .with_context(|| format!("connecting to {} (pinned TLS)", t.addr))?;
    let r = AgentServiceClient::new(ch)
        .enroll(pb::EnrollRequest {
            token_secret: t.secret.clone(),
            machine: machine.clone(),
            public_key: hex::encode(sk.verifying_key().to_bytes()),
            csr_pem: csr,
            os: os(),
            agent_version: VERSION.into(),
            role: role.into(),
        })
        .await
        .map_err(|s| anyhow::anyhow!("{}", s.message()))?
        .into_inner();
    verify_ca(&t.root, &r)?;
    let kd = state::keys_dir(dir);
    state::write_secret(&kd.join("agent.key"), hex::encode(sk.to_bytes()).as_bytes())?;
    state::write_secret(&kd.join("tls.key"), tls_key.as_bytes())?;
    let st = State {
        agent_id: r.agent_id,
        machine,
        role: role.into(),
        address: t.addr,
        pin: t.pin,
        cert_pem: r.client_cert_pem,
        ca_pem: r.ca_cert_pem,
        cert_expires_ms: r.cert_expires_ms,
        enrolled_ms: now(),
        cert_issued_ms: now(),
        queue_max_mib: 256,
    };
    state::save(dir, &st)?;
    Ok(st)
}

/// The token names the server root key; the site CA must be certified by it (v0.17.1).
fn verify_ca(root: &str, r: &pb::EnrollResponse) -> Result<()> {
    if root.is_empty() {
        return Ok(()); // token from a v0.17.0 server
    }
    if !r.root_public_key.eq_ignore_ascii_case(root) {
        bail!("the server's root key is not the one in the enrollment token");
    }
    let pk: [u8; 32] = hex::decode(root)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("bad root key in the token"))?;
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk)?;
    let sig: [u8; 64] = hex::decode(&r.ca_signature)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("the server sent no CA certification"))?;
    let der = tls::pem_to_der(&r.ca_cert_pem)?;
    vk.verify_strict(
        &itcmdr_agentlink::ca_cert_bytes(der.as_ref()),
        &ed25519_dalek::Signature::from_bytes(&sig),
    )
    .map_err(|_| anyhow::anyhow!("the site CA is not certified by the server root key"))?;
    Ok(())
}

fn signing_key(dir: &Path) -> Result<SigningKey> {
    let hexs = state::read_secret(&state::keys_dir(dir).join("agent.key"))?;
    let b: [u8; 32] = hex::decode(String::from_utf8_lossy(&hexs).trim())?
        .try_into()
        .map_err(|_| anyhow::anyhow!("agent.key is not 32 bytes"))?;
    Ok(SigningKey::from_bytes(&b))
}

/// How one link session ended.
#[derive(Debug, PartialEq)]
pub enum End {
    /// The server said Bye (revoked or replaced): stop retrying until re-enrolled.
    Bye(String),
    /// Refused for good (revoked, unknown agent, wrong certificate).
    Refused(String),
    /// Stream dropped or never came up: retry with backoff.
    Lost(String),
    /// The certificate was renewed: reconnect at once with the new one.
    Renewed,
}

/// One session: connect, Hello, answer pings until the stream ends.
/// `connected` is called with the server's Welcome.
pub async fn session(dir: &Path, st: &State, mut connected: impl FnMut(&pb::Welcome)) -> End {
    let sk = match signing_key(dir) {
        Ok(k) => k,
        Err(e) => return End::Refused(format!("agent key: {e:#}")),
    };
    let tls_key = match state::read_secret(&state::keys_dir(dir).join("tls.key")) {
        Ok(k) => String::from_utf8_lossy(&k).to_string(),
        Err(e) => return End::Refused(format!("tls key: {e:#}")),
    };
    let client = Pem {
        cert: st.cert_pem.clone(),
        key: tls_key,
    };
    let fp = match tls::pem_fingerprint(&st.cert_pem) {
        Ok(f) => f,
        Err(e) => return End::Refused(format!("client certificate: {e:#}")),
    };
    let ch = match tls::connect(&st.address, &st.pin, Some(&client)).await {
        Ok(c) => c,
        Err(e) => return End::Lost(format!("connect {}: {e:#}", st.address)),
    };
    let (tx, rx) = tokio::sync::mpsc::channel::<pb::AgentMessage>(16);
    let mut seq = 1u64;
    let ts = now();
    let nonce = hex::encode(rand::random::<[u8; 16]>());
    let sig = sk.sign(&itcmdr_agentlink::hello_bytes(
        &st.agent_id,
        &st.machine,
        ts,
        &nonce,
        &fp,
    ));
    let hello = pb::Hello {
        agent_id: st.agent_id.clone(),
        machine: st.machine.clone(),
        agent_version: VERSION.into(),
        os: os(),
        role: st.role.clone(),
        ts_ms: ts,
        nonce,
        signature: hex::encode(sig.to_bytes()),
    };
    let _ = tx
        .send(pb::AgentMessage {
            seq,
            body: Some(pb::agent_message::Body::Hello(hello)),
        })
        .await;
    let resp = AgentServiceClient::new(ch)
        .link(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await;
    let mut inbound = match resp {
        Ok(r) => r.into_inner(),
        Err(s) => {
            return classify(&s);
        }
    };
    let mut pending_key: Option<String> = None;
    let mut queue_mib = st.queue_max_mib;
    let mut sent: std::collections::HashSet<u64> = Default::default();
    let mut qtick = tokio::time::interval(Duration::from_secs(2));
    let issued = if st.cert_issued_ms > 0 {
        st.cert_issued_ms
    } else {
        st.enrolled_ms
    };
    loop {
        let m = tokio::select! {
            m = inbound.message() => m,
            _ = qtick.tick() => {
                match crate::queue::ingest(dir, queue_mib) {
                    Ok(items) => for it in items {
                        if !sent.insert(it.seq) { continue; }
                        let msg = pb::AgentMessage { seq: it.seq, body: Some(pb::agent_message::Body::Alert(pb::AgentAlert {
                            key: it.key, level: it.level, title: it.title, detail: it.detail, ts_ms: it.ts_ms, resolve: it.resolve,
                        })) };
                        if tx.send(msg).await.is_err() { return End::Lost("send failed".into()); }
                    },
                    Err(e) => tracing::warn!(error=%format!("{e:#}"), "offline queue"),
                }
                continue;
            }
        };
        let mut renew = false;
        match m {
            Ok(Some(m)) => match m.body {
                Some(pb::server_message::Body::Welcome(w)) => {
                    connected(&w);
                    if w.queue_max_mib > 0 && w.queue_max_mib as u64 != st.queue_max_mib {
                        queue_mib = w.queue_max_mib as u64;
                        let mut s2 = st.clone();
                        s2.queue_max_mib = queue_mib;
                        let _ = state::save(dir, &s2);
                    }
                    renew = itcmdr_agentlink::renew_due(issued, st.cert_expires_ms, now());
                }
                Some(pb::server_message::Body::RenewNow(r)) => {
                    tracing::info!(reason=%r.reason, "the server asked for a certificate renewal");
                    renew = true;
                }
                Some(pb::server_message::Body::Renewed(r)) => {
                    let Some(key) = pending_key.take() else {
                        continue;
                    };
                    if let Err(e) = save_renewed(dir, st, &key, &r) {
                        tracing::error!(error=%format!("{e:#}"), "saving the renewed certificate");
                        continue;
                    }
                    tracing::info!(
                        expires_ms = r.expires_ms,
                        "certificate renewed; reconnecting"
                    );
                    return End::Renewed;
                }
                Some(pb::server_message::Body::Ping(p)) => {
                    seq += 1;
                    let pong = pb::Pong {
                        nonce: p.nonce,
                        agent_ts_ms: now(),
                    };
                    if tx
                        .send(pb::AgentMessage {
                            seq,
                            body: Some(pb::agent_message::Body::Pong(pong)),
                        })
                        .await
                        .is_err()
                    {
                        return End::Lost("send failed".into());
                    }
                }
                Some(pb::server_message::Body::Bye(b)) => return End::Bye(b.reason),
                None => crate::queue::ack(dir, m.ack_seq),
            },
            Ok(None) => return End::Lost("server closed the stream".into()),
            Err(s) => {
                return classify(&s);
            }
        }
        if renew && pending_key.is_none() {
            match tls::new_csr(&st.machine) {
                Ok((csr, key)) => {
                    seq += 1;
                    pending_key = Some(key);
                    let msg = pb::AgentMessage {
                        seq,
                        body: Some(pb::agent_message::Body::Renew(pb::Renew { csr_pem: csr })),
                    };
                    if tx.send(msg).await.is_err() {
                        return End::Lost("send failed".into());
                    }
                }
                Err(e) => tracing::error!(error=%format!("{e:#}"), "renewal CSR"),
            }
        }
    }
}

fn save_renewed(dir: &Path, st: &State, key: &str, r: &pb::Renewed) -> Result<()> {
    let fp_new = tls::pem_fingerprint(&r.cert_pem)?;
    ensure_new(&fp_new, &st.cert_pem)?;
    // Key first: if we crash between the two writes, the old cert + new key fail and the
    // previous key is gone, so keep a copy until the state is saved.
    let kd = state::keys_dir(dir);
    let old = kd.join("tls.key");
    let bak = kd.join("tls.key.prev");
    let _ = std::fs::copy(&old, &bak);
    state::write_secret(&old, key.as_bytes())?;
    let mut s2 = st.clone();
    s2.cert_pem = r.cert_pem.clone();
    s2.cert_expires_ms = r.expires_ms;
    s2.cert_issued_ms = now();
    if let Ok(Some(cur)) = state::load(dir) {
        s2.queue_max_mib = cur.queue_max_mib;
    }
    state::save(dir, &s2)?;
    let _ = std::fs::remove_file(&bak);
    Ok(())
}

fn ensure_new(fp_new: &str, old_pem: &str) -> Result<()> {
    if tls::pem_fingerprint(old_pem).is_ok_and(|o| o == fp_new) {
        bail!("the renewed certificate is the old one");
    }
    Ok(())
}

/// Map a link error: refusals (including a certificate revoked in the TLS handshake) stop the
/// retry loop; anything else is a lost link.
fn classify(s: &tonic::Status) -> End {
    let mut chain = s.message().to_string();
    let mut e: Option<&dyn std::error::Error> = std::error::Error::source(s);
    while let Some(x) = e {
        chain.push_str(&format!(": {x}"));
        e = x.source();
    }
    match s.code() {
        tonic::Code::PermissionDenied | tonic::Code::Unauthenticated => {
            End::Refused(s.message().to_string())
        }
        _ if chain.contains("CertificateRevoked") || chain.contains("certificate revoked") => {
            End::Refused("the client certificate was revoked (refused in the TLS handshake)".into())
        }
        _ => End::Lost(chain),
    }
}

/// Full-jitter exponential backoff: 1 s doubling to `cap`.
pub fn backoff(attempt: u32, cap_secs: u64) -> Duration {
    let ceil = (1u64 << attempt.min(16)).min(cap_secs.max(1));
    Duration::from_millis(rand::random::<u64>() % (ceil * 1000 + 1))
}

/// Run until `stop` fires: enroll from the token file when needed, then keep the link up.
pub async fn run(
    dir: &Path,
    token_file: &Path,
    mut stop: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let mut reconnect_cap = 300u64;
    let mut warned = false;
    loop {
        let st = match state::load(dir)? {
            Some(s) => s,
            None => {
                if token_file.exists() {
                    let tok = std::fs::read_to_string(token_file)?;
                    match enroll(dir, &tok, None, "workstation").await {
                        Ok(s) => {
                            tracing::info!(machine=%s.machine, agent=%s.agent_id, "enrolled");
                            let _ = std::fs::remove_file(token_file);
                            continue;
                        }
                        Err(e) => {
                            tracing::error!(error=%format!("{e:#}"), "enrollment failed; retrying in 1 min")
                        }
                    }
                    if wait(&mut stop, Duration::from_secs(60)).await {
                        return Ok(());
                    }
                    continue;
                }
                if !warned {
                    tracing::warn!(token=%token_file.display(), "not enrolled and no token yet; checking every 10 s");
                    warned = true;
                }
                if wait(&mut stop, Duration::from_secs(10)).await {
                    return Ok(());
                }
                continue;
            }
        };
        let mut attempt = 0u32;
        loop {
            let started = std::time::Instant::now();
            let end = tokio::select! {
                e = session(dir, &st, |w| {
                    reconnect_cap = w.reconnect_max_secs.max(5) as u64;
                    tracing::info!(heartbeat_secs = w.heartbeat_secs, "linked to the server");
                }) => e,
                _ = stop.changed() => return Ok(()),
            };
            if started.elapsed() > Duration::from_secs(60) {
                attempt = 0;
            }
            match end {
                End::Renewed => break, // reload state with the new certificate
                End::Lost(why) => {
                    // Offline: still move spooled alerts into the capped queue.
                    if let Err(e) = crate::queue::ingest(dir, st.queue_max_mib) {
                        tracing::warn!(error=%format!("{e:#}"), "offline queue");
                    }
                    let d = backoff(attempt, reconnect_cap);
                    attempt = attempt.saturating_add(1);
                    tracing::warn!(%why, retry_in_ms = d.as_millis() as u64, "link lost");
                    if wait(&mut stop, d).await {
                        return Ok(());
                    }
                }
                End::Bye(why) | End::Refused(why) => {
                    tracing::error!(%why, "the server refused this agent; enroll it again (checking again in 15 min)");
                    if wait(&mut stop, Duration::from_secs(900)).await {
                        return Ok(());
                    }
                    break; // reload state: a re-enrollment may have replaced it
                }
            }
        }
    }
}

/// Sleep `d`; true when stopped meanwhile.
async fn wait(stop: &mut tokio::sync::watch::Receiver<bool>, d: Duration) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(d) => *stop.borrow(),
        _ = stop.changed() => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backoff_is_capped() {
        for a in 0..40 {
            assert!(backoff(a, 300) <= Duration::from_secs(300));
        }
        assert!(backoff(0, 300) <= Duration::from_secs(1));
    }
}
