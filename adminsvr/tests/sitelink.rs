//! v0.19.0 SiteLink end to end: a site server links to adminsvr and sends only redacted data.
//! Required tests (ADMINSVR.md decisions 18 and 19):
//! - no hostname, IP, path, username or message text from the site in any outbound message
//!   (random values each run, checked on the wire tap and in adminsvr's database);
//! - with the owner's opt-in, display names appear and nothing else changes;
//! - no backup bytes or key material on the link; `backup.start` runs locally;
//! - a restore command from adminsvr is rejected and audited on the site; bad signatures and
//!   commands off the local allowlist are refused;
//! - a refused root key leaves no enrolled site; kill switch, unreachable alert, delivery after
//!   reconnect, revocation.
mod common;
use ::itcmdr_server as srv;
use itcmdr_admin::proto as pb;
use prost::Message;
use rand::Rng;
use std::time::Duration;

macro_rules! need_pg {
    () => {
        match common::db().await {
            Some(u) => u,
            None => {
                eprintln!("no Postgres (ITCMDR_PG_BIN / ITCMDR_TEST_PG_URL): skipped");
                return;
            }
        }
    };
}

struct Secrets {
    machines: Vec<String>,
    other: Vec<String>, // IPs, paths, usernames, domains, message text
}

fn word(r: &mut impl Rng, n: usize) -> String {
    (0..n)
        .map(|_| (b'a' + r.gen_range(0..26)) as char)
        .collect()
}

fn secrets() -> Secrets {
    let mut r = rand::thread_rng();
    let machines = (0..4)
        .map(|i| {
            format!(
                "{}-{}{i}",
                word(&mut r, 6).to_uppercase(),
                r.gen_range(10..99)
            )
        })
        .collect();
    let mut other = vec![];
    for _ in 0..3 {
        let user = format!("{}.{}", word(&mut r, 5), word(&mut r, 7));
        other.push(user.clone());
        other.push(format!(
            "{}.{}.{}.{}",
            r.gen_range(11..200),
            r.gen_range(0..255),
            r.gen_range(0..255),
            r.gen_range(1..254)
        ));
        other.push(format!(
            "C:\\Users\\{user}\\Documents\\{}.docx",
            word(&mut r, 8)
        ));
        other.push(format!("/srv/{}/{}", word(&mut r, 6), word(&mut r, 6)));
        other.push(format!("{}.{}.lan", word(&mut r, 7), word(&mut r, 5)));
        other.push(format!(
            "patient {} {} visit",
            word(&mut r, 6),
            word(&mut r, 8)
        ));
    }
    Secrets { machines, other }
}

fn clean(what: &str, bytes: &[u8], debug: &str, bad: &[String]) {
    let lb = String::from_utf8_lossy(bytes).to_lowercase();
    let ld = debug.to_lowercase();
    for b in bad {
        let l = b.to_lowercase();
        assert!(
            !lb.contains(&l) && !ld.contains(&l),
            "{what} leaked {b:?}: {debug}"
        );
    }
}

async fn until<F, Fut>(what: &str, secs: u64, mut f: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..secs * 5 {
        if f().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("timed out waiting for {what}");
}

struct Site {
    _tmp: tempfile::TempDir,
    cfg: srv::config::ServerConfig,
    pool: srv::db::DbPool,
}

async fn site() -> Site {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let mut c = srv::config::ServerConfig {
        bind_addr: format!("127.0.0.1:{}", common::free_port()),
        agent_bind_addr: format!("127.0.0.1:{}", common::free_port()),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        auth_mode: "signed".into(),
        ..Default::default()
    };
    c.backup.nightly = false;
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::auth::init(&pool, &c.layout()).await.unwrap();
    Site {
        _tmp: tmp,
        cfg: c,
        pool,
    }
}

async fn raise(
    pool: &srv::db::DbPool,
    key: String,
    level: &str,
    source: &str,
    machine: &str,
    title: String,
    detail: String,
) {
    srv::alerts::raise(
        pool,
        &srv::alerts::Raise {
            key,
            level: level.into(),
            source: source.into(),
            machine: machine.into(),
            title,
            detail,
            run_id: None,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn site_link_is_redacted_and_commands_are_checked() {
    let db = need_pg!();
    let a = common::start(&db).await;
    let (o, _) = common::owner(&a).await;
    let s = site().await;
    let sec = secrets();
    let (m, x) = (&sec.machines, &sec.other);
    // Site data full of names, IPs, paths, usernames and message text.
    for (i, mach) in m.iter().enumerate() {
        sqlx::query("INSERT INTO agents (id, machine, role, public_key, cert_fp, cert_serial, cert_expires_ms, state, enrolled_ms, last_seen_ms, version) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
            .bind(format!("a{i}")).bind(mach).bind("workstation").bind("00").bind("ff").bind(format!("s{i}")).bind(0i64).bind(if i == 0 { "late" } else { "online" }).bind(0i64).bind(1i64).bind("0.19.0")
            .execute(&s.pool).await.unwrap();
    }
    raise(
        &s.pool,
        format!("agent:{}:disk-low", m[0]),
        "critical",
        "agent",
        &m[0],
        format!("{} full at {}", x[2], x[1]),
        format!("user {} on {} ({})", x[0], x[4], x[5]),
    )
    .await;
    raise(
        &s.pool,
        format!("plugin:disk-free:fail:{}", m[1]),
        "cautious",
        "plugin",
        &m[1],
        x[5].clone(),
        x[3].clone(),
    )
    .await;
    raise(
        &s.pool,
        format!("agent:{}:{}-{}", m[2], x[6], x[7]),
        "informational",
        "agent",
        &m[2],
        x[8].clone(),
        x[9].clone(),
    )
    .await;
    raise(
        &s.pool,
        format!("quarantine:{}", x[3]),
        "cautious",
        "quarantine",
        "",
        x[10].clone(),
        x[11].clone(),
    )
    .await;
    raise(
        &s.pool,
        format!("custom:{}:{}", x[13], x[16].to_lowercase()),
        "critical",
        "agent",
        &m[3],
        x[12].clone(),
        x[17].clone(),
    )
    .await;

    // Enrollment: a token with another root key is refused before anything is recorded.
    let cs = o.create_site("Office A").await.unwrap();
    let mut bad = itcmdr_adminlink::SiteToken::decode(&cs.token).unwrap();
    bad.root = hex::encode(itcmdr_keys::new_keypair().verifying_key().as_bytes());
    let e = srv::sitelink::enroll(&s.pool, &s.cfg, &bad.encode(), "test")
        .await
        .err()
        .expect("refused");
    assert!(format!("{e:#}").contains("root key"), "{e:#}");
    assert_eq!(o.list_sites().await.unwrap().sites[0].status, "pending");
    let st = srv::sitelink::enroll(&s.pool, &s.cfg, &cs.token, "test")
        .await
        .unwrap();
    assert!(srv::sitelink::enroll(&s.pool, &s.cfg, &cs.token, "test")
        .await
        .is_err());

    let (tap_tx, mut tap) = tokio::sync::mpsc::unbounded_channel();
    srv::sitelink::set_tap(Some(tap_tx));
    srv::settings::set(&s.pool, "adminsvr.heartbeatSecs", "5", None)
        .await
        .unwrap();
    let srun = tokio::spawn(srv::sitelink::run(s.pool.clone(), s.cfg.clone()));

    until("site online with alerts", 40, || async {
        o.list_alerts(&st.site_id, "all", 100)
            .await
            .map(|l| l.alerts.len() >= 5)
            .unwrap_or(false)
            && o.list_sites().await.unwrap().sites[0].last_status.is_some()
    })
    .await;
    let site_info = &o.list_sites().await.unwrap().sites[0];
    assert_eq!(site_info.status, "online");
    let ls = site_info.last_status.clone().unwrap();
    assert_eq!((ls.agents_online, ls.agents_late), (3, 1));
    assert_eq!(ls.machines.len(), 4);
    assert!(ls
        .machines
        .iter()
        .all(|mm| mm.id.starts_with("m-") && mm.display_name.is_empty()));
    let alerts = o.list_alerts(&st.site_id, "all", 100).await.unwrap().alerts;
    let codes: Vec<String> = alerts
        .iter()
        .map(|a| a.alert.as_ref().unwrap().code.clone())
        .collect();
    assert!(
        codes
            .iter()
            .any(|c| c.starts_with("agent:m-") && c.ends_with(":disk-low")),
        "{codes:?}"
    );
    assert!(codes.iter().any(|c| c.starts_with("plugin:")), "{codes:?}");

    // Nothing sensitive in anything that went up (wire tap) or that adminsvr stored.
    let mut bad_all: Vec<String> = m.clone();
    bad_all.extend(x.iter().cloned());
    bad_all.push(s.cfg.layout().root.display().to_string());
    let mut wire = vec![];
    while let Ok(msg) = tap.try_recv() {
        clean("wire", &msg.encode_to_vec(), &format!("{msg:?}"), &bad_all);
        wire.extend(msg.encode_to_vec());
    }
    assert!(!wire.is_empty());
    let apool = itcmdr_adminsvr::store::connect(&db).await.unwrap();
    let bodies: Vec<Vec<u8>> = sqlx::query_scalar("SELECT body FROM alerts")
        .fetch_all(&apool)
        .await
        .unwrap();
    for b in &bodies {
        clean("adminsvr alerts", b, "", &bad_all);
    }
    let stat: Vec<u8> = sqlx::query_scalar("SELECT last_status FROM sites")
        .fetch_one(&apool)
        .await
        .unwrap();
    clean("adminsvr status", &stat, "", &bad_all);

    // Remote start of a local backup: the archive stays on the site; no bytes or keys go up.
    let c = o.command(&st.site_id, "backup.start", &[]).await.unwrap();
    until("backup applied", 60, || async {
        o.list_commands(&st.site_id)
            .await
            .unwrap()
            .commands
            .iter()
            .any(|x| x.id == c.id && x.state == "applied")
    })
    .await;
    let bdir = s.cfg.layout().backups();
    let arch = std::fs::read_dir(&bdir)
        .unwrap()
        .flatten()
        .next()
        .expect("backup written on the site")
        .path();
    let data = std::fs::read(&arch).unwrap();
    tokio::time::sleep(Duration::from_secs(6)).await; // a status pass after the backup
    while let Ok(msg) = tap.try_recv() {
        wire.extend(msg.encode_to_vec());
    }
    let has = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).any(|w| w == needle);
    for chunk in data
        .chunks(32)
        .filter(|c| c.len() == 32)
        .step_by(7)
        .take(200)
    {
        assert!(!has(&wire, chunk), "backup bytes on the link");
    }
    for f in std::fs::read_dir(s.cfg.layout().keys()).unwrap().flatten() {
        let b = std::fs::read(f.path()).unwrap();
        if b.len() >= 32 && f.file_name() != "adminsvr-link.json" {
            for chunk in b.chunks(32).filter(|c| c.len() == 32) {
                assert!(
                    !has(&wire, chunk),
                    "key material from {:?} on the link",
                    f.file_name()
                );
            }
        }
    }
    assert!(
        !has(&wire, st.site_key.as_bytes())
            && !has(&wire, st.id_secret.as_bytes())
            && !has(&wire, st.client.key.as_bytes())
    );
    let ls = o.list_sites().await.unwrap().sites[0]
        .last_status
        .clone()
        .unwrap();
    assert!(
        ls.backup_age_secs >= 0 && ls.backup_size_bytes > 0 && ls.backup_last_result == "ok",
        "{ls:?}"
    );

    // Ack from adminsvr maps the opaque id back on the site.
    let disk = alerts
        .iter()
        .find(|a| a.alert.as_ref().unwrap().code.ends_with(":disk-low"))
        .unwrap();
    let c = o
        .command(
            &st.site_id,
            "alert.ack",
            &[("alert_id", &disk.alert.as_ref().unwrap().id)],
        )
        .await
        .unwrap();
    until("ack applied", 30, || async {
        o.list_commands(&st.site_id)
            .await
            .unwrap()
            .commands
            .iter()
            .any(|x| x.id == c.id && x.state == "applied")
    })
    .await;
    let local = srv::alerts::list(&s.pool, &Default::default())
        .await
        .unwrap();
    assert!(local
        .iter()
        .any(|l| l.key.ends_with(":disk-low") && l.state == "acked"));

    // Forged commands: restore (never remote), a bad root signature, off the allowlist.
    let root = itcmdr_keys::key_from_seed(
        &hex::decode(
            std::fs::read_to_string(a.cfg.data_dir.join("keys/root.key"))
                .unwrap()
                .trim(),
        )
        .unwrap(),
    )
    .unwrap();
    let me = o.whoami().await.unwrap();
    let forge = |kind: &str, root: &ed25519_dalek::SigningKey| {
        let now = chrono::Utc::now().timestamp_millis();
        let mut c = pb::Command {
            id: uuid::Uuid::new_v4().to_string(),
            site_id: st.site_id.clone(),
            kind: kind.into(),
            user_id: me.id.clone(),
            user_role: me.role.clone(),
            issued_ms: now,
            expires_ms: now + 60_000,
            ..Default::default()
        };
        itcmdr_adminlink::sign_user(o.key(), &mut c);
        itcmdr_adminlink::sign_root(root, &mut c);
        c
    };
    let queue = |c: pb::Command| {
        let apool = apool.clone();
        async move {
            sqlx::query("INSERT INTO commands (id, site_id, kind, requested_by, state, created_ms, body) VALUES ($1,$2,$3,'test','pending',$4,$5)")
                .bind(&c.id).bind(&c.site_id).bind(&c.kind).bind(c.issued_ms).bind(c.encode_to_vec()).execute(&apool).await.unwrap();
            c.id
        }
    };
    let r1 = queue(forge("restore", &root)).await;
    let r2 = queue(forge("backup.start", &itcmdr_keys::new_keypair())).await;
    srv::sitelink::set_allowed(&s.pool, &s.cfg, &["alert.ack".to_string()], "test")
        .await
        .unwrap();
    let r3 = queue(forge("alert.resolve", &root)).await;
    let result = |id: String| {
        let apool = apool.clone();
        async move {
            sqlx::query_as::<_, (String, String)>(
                "SELECT state, result_code FROM commands WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&apool)
            .await
            .unwrap()
        }
    };
    until("forged commands answered", 30, || async {
        result(r1.clone()).await.0 == "failed"
            && result(r2.clone()).await.0 == "failed"
            && result(r3.clone()).await.0 == "failed"
    })
    .await;
    assert_eq!(result(r1).await.1, "not-allowed");
    assert_eq!(result(r2).await.1, "bad-signature");
    assert_eq!(result(r3).await.1, "not-allowed");
    let refused: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action = 'adminsvr.command.refused' AND actor LIKE ?",
    )
    .bind(format!("adminsvr:%:{}", me.id))
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(refused, 3);
    // adminsvr itself never offers a restore: no such API method or command kind.
    assert!(
        !std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../proto/admin.proto"))
            .unwrap()
            .contains("rpc Restore")
    );
    assert!(!itcmdr_adminlink::COMMANDS
        .iter()
        .any(|k| k.contains("restore")));

    // Opt-in: display names appear, nothing else changes.
    srv::sitelink::set_share_names(&s.pool, &s.cfg, true, "test")
        .await
        .unwrap();
    while tap.try_recv().is_ok() {}
    raise(
        &s.pool,
        format!("agent:{}:service-down", m[1]),
        "critical",
        "agent",
        &m[1],
        format!("{} at {}", x[2], x[1]),
        x[0].clone(),
    )
    .await;
    until("named alert", 30, || async {
        o.list_alerts(&st.site_id, "open", 100)
            .await
            .unwrap()
            .alerts
            .iter()
            .any(|a| a.alert.as_ref().unwrap().machine_display_name == m[1])
    })
    .await;
    while let Ok(msg) = tap.try_recv() {
        clean(
            "wire (names on)",
            &msg.encode_to_vec(),
            &format!("{msg:?}"),
            x,
        );
    }

    // Kill switch: the link closes, adminsvr reports the site unreachable; an alert raised
    // meanwhile is delivered once the link is back.
    srv::settings::set(&s.pool, "adminsvr.enabled", "false", None)
        .await
        .unwrap();
    until("site unreachable alert", 40, || async {
        o.list_alerts(&st.site_id, "open", 100)
            .await
            .unwrap()
            .alerts
            .iter()
            .any(|a| a.origin == "adminsvr" && a.alert.as_ref().unwrap().code == "site-unreachable")
    })
    .await;
    assert_eq!(o.list_sites().await.unwrap().sites[0].status, "unreachable");
    raise(
        &s.pool,
        format!("agent:{}:disk-full", m[3]),
        "critical",
        "agent",
        &m[3],
        "x".into(),
        "y".into(),
    )
    .await;
    srv::settings::set(&s.pool, "adminsvr.enabled", "true", None)
        .await
        .unwrap();
    until("reconnected and delivered", 60, || async {
        let l = o
            .list_alerts(&st.site_id, "open", 100)
            .await
            .unwrap()
            .alerts;
        l.iter()
            .any(|a| a.alert.as_ref().unwrap().code.ends_with(":disk-full"))
            && !l
                .iter()
                .any(|a| a.alert.as_ref().unwrap().code == "site-unreachable")
    })
    .await;

    // Revoked: the link is cut and the certificate refused from then on.
    o.revoke_site(&st.site_id).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    let client = itcmdr_agentlink::tls::Pem {
        cert: st.client.cert.clone(),
        key: st.client.key.clone(),
    };
    let r = async {
        let ch = itcmdr_agentlink::tls::connect(&st.addr, &st.pin, Some(&client)).await?;
        let mut c = pb::site_link_client::SiteLinkClient::new(ch);
        let (_tx, rx) = tokio::sync::mpsc::channel::<pb::SiteMessage>(1);
        c.link(tokio_stream::wrappers::ReceiverStream::new(rx))
            .await?;
        anyhow::Ok(())
    }
    .await;
    assert!(r.is_err(), "revoked site still linked");
    assert_eq!(o.list_sites().await.unwrap().sites[0].status, "revoked");
    srun.abort();
    srv::sitelink::set_tap(None);
}

#[test]
fn redactor_property_no_names_in_codes() {
    let mut r = rand::thread_rng();
    for _ in 0..200 {
        let machines: Vec<String> = (0..3)
            .map(|_| format!("{}-{}", word(&mut r, 5).to_uppercase(), r.gen_range(1..99)))
            .collect();
        let user = word(&mut r, 6);
        let ip = format!(
            "{}.{}.{}.{}",
            r.gen_range(1..250),
            r.gen_range(0..255),
            r.gen_range(0..255),
            r.gen_range(1..254)
        );
        let path = format!("/home/{user}/{}", word(&mut r, 5));
        let red = srv::sitelink::Redactor::new(
            "secret",
            false,
            vec![user.clone(), path.clone()],
            machines.clone(),
            vec!["disk-free".into()],
            vec!["DISK-C".into()],
        );
        let pick = |r: &mut rand::rngs::ThreadRng| -> String {
            match r.gen_range(0..7) {
                0 => machines[r.gen_range(0..3)].clone(),
                1 => user.clone(),
                2 => ip.clone(),
                3 => path.clone(),
                4 => word(r, 7),
                5 => "disk-low".into(),
                _ => "DISK-C".into(),
            }
        };
        let key: Vec<String> = (0..r.gen_range(1..6)).map(|_| pick(&mut r)).collect();
        let code = red.code(&key.join(":"));
        for b in machines.iter().chain([&user, &ip, &path]) {
            assert!(
                !code.to_lowercase().contains(&b.to_lowercase()),
                "{b} in {code}"
            );
        }
        // unknown words never pass (deny by default)
        for seg in code.split(':') {
            assert!(
                seg == "disk-low"
                    || seg == "DISK-C"
                    || seg.starts_with("m-")
                    || seg.starts_with("x-"),
                "{seg} in {code}"
            );
        }
    }
}
