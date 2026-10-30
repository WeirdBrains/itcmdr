//! Server side of the contract: launch a plugin process, read the handshake, call it.
use crate::proto::plugin_service_client::PluginServiceClient;
use crate::{ENV_SECRET, HANDSHAKE, SECRET_HEADER};
use anyhow::{anyhow, bail, Context, Result};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;

/// Adds the secret header to every call.
#[derive(Clone)]
pub struct SecretInterceptor(MetadataValue<tonic::metadata::Ascii>);

impl tonic::service::Interceptor for SecretInterceptor {
    fn call(&mut self, mut r: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        r.metadata_mut().insert(SECRET_HEADER, self.0.clone());
        Ok(r)
    }
}

pub type Client = PluginServiceClient<InterceptedService<Channel, SecretInterceptor>>;

/// A running plugin process. Dropping it kills the process.
pub struct Launched {
    pub child: Child,
    pub client: Client,
    pub addr: String,
}

impl Launched {
    /// Close stdin (the plugin exits on EOF) and wait a little, then kill.
    pub async fn shutdown(mut self) {
        drop(self.child.stdin.take());
        if tokio::time::timeout(Duration::from_secs(3), self.child.wait())
            .await
            .is_err()
        {
            let _ = self.child.kill().await;
        }
    }
}

pub fn new_secret() -> String {
    use rand::RngCore;
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}

/// Parse `ITCMDR-PLUGIN|1|tcp|127.0.0.1:<port>`; only loopback addresses are accepted.
pub fn parse_handshake(line: &str) -> Result<String> {
    let addr = line.trim().strip_prefix(HANDSHAKE).ok_or_else(|| {
        anyhow!(
            "bad plugin handshake {:?} (want {HANDSHAKE}127.0.0.1:<port>)",
            line.trim()
        )
    })?;
    let sa: std::net::SocketAddr = addr.parse().context("handshake address")?;
    if !sa.ip().is_loopback() {
        bail!("plugin must listen on loopback, not {sa}");
    }
    Ok(addr.to_string())
}

/// Start `cmd` with a fresh secret, wait up to `timeout` for the handshake and connect.
pub async fn launch(mut cmd: Command, timeout: Duration) -> Result<Launched> {
    let secret = new_secret();
    cmd.env(ENV_SECRET, &secret)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn().context("starting the plugin process")?;
    let stdout = child.stdout.take().expect("piped");
    let mut lines = BufReader::new(stdout).lines();
    let line = tokio::time::timeout(timeout, lines.next_line())
        .await
        .map_err(|_| anyhow!("plugin did not print its handshake within {timeout:?}"))??
        .ok_or_else(|| anyhow!("plugin exited before the handshake"))?;
    let addr = parse_handshake(&line)?;
    // Keep draining stdout so the plugin never blocks on a full pipe.
    tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
    let client = connect(&addr, &secret, timeout).await?;
    Ok(Launched {
        child,
        client,
        addr,
    })
}

/// Connect to a plugin at `addr` with `secret`.
pub async fn connect(addr: &str, secret: &str, timeout: Duration) -> Result<Client> {
    let ch = Channel::from_shared(format!("http://{addr}"))?
        .connect_timeout(timeout)
        .connect()
        .await
        .context("connecting to the plugin")?;
    let v: MetadataValue<_> = secret.parse().context("secret")?;
    Ok(PluginServiceClient::with_interceptor(
        ch,
        SecretInterceptor(v),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::*;
    use crate::Plugin;

    struct T;
    impl Plugin for T {
        fn describe(&self) -> DescribeResponse {
            crate::info("t", "1.0.0", &["verify", "execute"])
        }
        fn verify(&self, ctx: &RunContext) -> std::result::Result<StageResponse, String> {
            if ctx.args.contains_key("fail") {
                Err("verify said no".into())
            } else {
                Ok(crate::ok("fine"))
            }
        }
        fn execute(&self, ctx: &RunContext) -> std::result::Result<StageResponse, String> {
            Ok(crate::ok(&format!("ran on {}", ctx.machine)))
        }
    }

    #[test]
    fn handshake_parsing() {
        assert_eq!(
            parse_handshake("ITCMDR-PLUGIN|1|tcp|127.0.0.1:4000\n").unwrap(),
            "127.0.0.1:4000"
        );
        assert!(parse_handshake("ITCMDR-PLUGIN|1|tcp|198.51.100.7:4000").is_err());
        assert!(parse_handshake("hello").is_err());
        assert!(parse_handshake("ITCMDR-PLUGIN|1|tcp|nonsense").is_err());
    }

    #[tokio::test]
    async fn serve_checks_the_secret_and_runs_stages() {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(crate::serve_on(l, T, "s3cret".into(), async {
            let _ = rx.await;
        }));
        let mut c = connect(&addr, "s3cret", Duration::from_secs(5))
            .await
            .unwrap();
        let d = c.describe(DescribeRequest {}).await.unwrap().into_inner();
        assert_eq!((d.name.as_str(), d.version.as_str()), ("t", "1.0.0"));
        let ctx = RunContext {
            machine: "SERVER01".into(),
            ..Default::default()
        };
        assert!(c.verify(ctx.clone()).await.unwrap().into_inner().ok);
        assert_eq!(
            c.execute(ctx.clone()).await.unwrap().into_inner().message,
            "ran on SERVER01"
        );
        let mut bad = ctx.clone();
        bad.args.insert("fail".into(), "1".into());
        let e = c.verify(bad).await.unwrap_err();
        assert_eq!(e.code(), tonic::Code::Aborted);
        assert!(e.message().contains("verify said no"));
        let mut wrong = connect(&addr, "nope", Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(
            wrong.describe(DescribeRequest {}).await.unwrap_err().code(),
            tonic::Code::Unauthenticated
        );
        let _ = tx.send(());
    }
}
