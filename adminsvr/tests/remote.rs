//! v0.20.0 commands down and documents up (ADMINSVR.md sections 3 and 5), end to end with a real
//! site server linked to adminsvr:
//! - the config and plugin documents carry no paths, names or text from the site (random secrets
//!   in settings, schedule names and quarantined file names; wire tap and adminsvr's database);
//! - setting.set: dry run previews and changes nothing, then applies; link/auth/path/text keys and
//!   out-of-range values are refused by the site;
//! - schedule pause/resume, run;
//! - plugin push from the catalog: two-person (the requester can't approve; SREs can't), the site
//!   installs through its own validation; a tampered package is quarantined (reason code only);
//! - real break: an adminsvr that skips the approval (row flipped to pending) is refused by the
//!   site (needs-approval);
//! - global freeze: owners only; commands and approvals refused while frozen, sent after;
//! - metrics summaries arrive.
mod common;
use ::itcmdr_server as srv;
use itcmdr_admin::{proto as pb, Client};
use prost::Message;
use rand::Rng;
use std::time::Duration;

fn word(n: usize) -> String {
    let mut r = rand::thread_rng();
    (0..n).map(|_| (b'a' + r.gen_range(0..26)) as char).collect()
}

fn clean(what: &str, bytes: &[u8], debug: &str, bad: &[String]) {
    let lb = String::from_utf8_lossy(bytes).to_lowercase();
    let ld = debug.to_lowercase();
    for b in bad {
        let l = b.to_lowercase();
        assert!(!lb.contains(&l) && !ld.contains(&l), "{what} leaked {b:?}: {debug}");
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

async fn user(o: &Client, email: &str, role: &str) -> Client {
    let r = o.create_user(email, "User U", role).await.unwrap();
    let e = itcmdr_admin::enroll(&r.enroll_code, "test", true, None).await.unwrap();
    o.approve_device(email, &e.user.devices[0].fingerprint).await.unwrap();
    Client::connect(&e.identity).await.unwrap()
}

/// A plugin source folder signed into a package by `key`.
fn package(root: &std::path::Path, name: &str, version: &str, key: &ed25519_dalek::SigningKey) -> Vec<u8> {
    let d = root.join(format!("src-{name}-{version}"));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("run.ps1"), "Write-Output 'hi'\n").unwrap();
    std::fs::write(
        d.join("package.toml"),
        format!(
            "name = \"{name}\"\nversion = \"{version}\"\nfeatures = [\"verify\"]\n\n\
             [[files]]\nname = \"run.ps1\"\ntype = \"powershell\"\nmain = true\nchecksum = \"\"\n"
        ),
    )
    .unwrap();
    let out = root.join(format!("{name}-{version}.tar.gz"));
    srv::packages::pack(&d, key, &out).unwrap();
    std::fs::read(out).unwrap()
}

async fn done(o: &Client, id: &str) -> pb::CommandInfo {
    o.wait_command(id, Duration::from_secs(30)).await.unwrap()
}

both!(commands_down_documents_up, remote_sqlite, remote_postgres);

async fn commands_down_documents_up(db: String) {
    let a = common::start(&db).await;
    let (o, _) = common::owner(&a).await;
    let adm = user(&o, "admin@example.com", "admin").await;
    let sre = user(&o, "sre@example.com", "sre").await;

    // ---- the site, with secrets in places that must not go up
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let mut cfg = srv::config::ServerConfig {
        bind_addr: format!("127.0.0.1:{}", common::free_port()),
        agent_bind_addr: format!("127.0.0.1:{}", common::free_port()),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        auth_mode: "signed".into(),
        ..Default::default()
    };
    cfg.backup.nightly = false;
    let pool = srv::db::init_db(&cfg).await.unwrap();
    let init = srv::auth::init(&pool, &cfg.layout()).await.unwrap();
    let master = srv::keys::Master::from_words(&init.words).unwrap();
    let pkg_key = srv::keys::new_keypair();
    let cert = srv::keys::certify(&master.signing, "user", "Packager", "admin", &pkg_key.verifying_key());
    let pk = hex::encode(pkg_key.verifying_key().as_bytes());
    srv::auth::register(&pool, srv::auth::NewKey { kind: "user", name: "Packager", role: srv::auth::Role::Admin, email: None, phone: None, public_key_hex: &pk, certificate: &cert }).await.unwrap();

    let secret_dir = root.join(format!("share-{}", word(8)));
    std::fs::create_dir_all(&secret_dir).unwrap();
    let ps = format!("C:\\Users\\{}\\pwsh.exe", word(7));
    let sched_name = format!("{} {} nightly", word(6), word(6));
    let mut bad: Vec<String> = vec![secret_dir.display().to_string(), ps.clone(), sched_name.clone(), root.display().to_string()];
    srv::settings::set(&pool, "backup.dir", &secret_dir.display().to_string(), None).await.unwrap();
    srv::settings::set(&pool, "plugins.powershell", &ps, None).await.unwrap();
    let sch = srv::schedule::create(
        &pool,
        &sched_name,
        srv::schedule::Spec { when: "daily 03:00".into(), timezone: "UTC".into(), action: "backup".into(), guard: "none".into(), missed: "skip".into(), ..Default::default() },
        true,
    )
    .await
    .unwrap();

    let cs = o.create_site("Office A").await.unwrap();
    let st = srv::sitelink::enroll(&pool, &cfg, &cs.token, "test").await.unwrap();
    let site = st.site_id.clone();
    let (tap_tx, mut tap) = tokio::sync::mpsc::unbounded_channel();
    srv::sitelink::set_tap(Some(tap_tx));
    srv::settings::set(&pool, "adminsvr.heartbeatSecs", "5", None).await.unwrap();
    let srun = tokio::spawn(srv::sitelink::run(pool.clone(), cfg.clone()));

    // ---- the config document
    until("config document", 40, || async {
        o.site_config(&site).await.map(|i| i.config.is_some()).unwrap_or(false)
    })
    .await;
    let d = o.site_config(&site).await.unwrap().config.unwrap();
    let get = |d: &pb::SiteConfig, k: &str| d.settings.iter().find(|x| x.key == k).cloned().unwrap();
    let bd = get(&d, "backup.dir");
    assert!(bd.hidden && bd.value.is_empty() && !bd.remote, "{bd:?}");
    let kd = get(&d, "backup.keep_daily");
    assert!(!kd.hidden && kd.remote && kd.value == "7" && kd.max > 0, "{kd:?}");
    for k in ["adminsvr.enabled", "adminsvr.requireTwoPerson", "server.auth_mode", "auth.max_skew_secs", "plugins.powershell"] {
        assert!(!get(&d, k).remote, "{k} must not be settable remotely");
    }
    assert!(d.require_two_person);
    assert!(d.allowed.iter().any(|k| k == "plugin.install"), "{:?}", d.allowed);
    let s0 = d.schedules.iter().find(|x| x.when == "daily 03:00" && x.action == "backup").expect("schedule");
    assert!(s0.name.is_empty() && s0.id.starts_with("s-") && s0.enabled && s0.action == "backup", "{s0:?}");

    // ---- setting.set: dry run, then apply; refusals by the site
    let x = adm.command_ex(&site, "setting.set", &[("key", "backup.keep_daily"), ("value", "9")], true).await.unwrap();
    let x = done(&adm, &x.id).await;
    assert_eq!((x.state.as_str(), x.result_code.as_str()), ("previewed", "ok"), "{x:?}");
    assert_eq!((x.preview[0].target.as_str(), x.preview[0].before.as_str(), x.preview[0].after.as_str()), ("backup.keep_daily", "7", "9"));
    assert_eq!(srv::settings::get_i64(&pool, None, "backup.keep_daily").await, 7, "a dry run changed the site");
    let x = adm.command_ex(&site, "setting.set", &[("key", "backup.keep_daily"), ("value", "9")], false).await.unwrap();
    assert_eq!(done(&adm, &x.id).await.state, "applied");
    assert_eq!(srv::settings::get_i64(&pool, None, "backup.keep_daily").await, 9);
    until("config document updated", 40, || async {
        o.site_config(&site).await.ok().and_then(|i| i.config).is_some_and(|d| get(&d, "backup.keep_daily").value == "9")
    })
    .await;
    for (k, v, code) in [
        ("adminsvr.enabled", "false", "not-allowed"),
        ("adminsvr.requireTwoPerson", "false", "not-allowed"),
        ("backup.dir", "/tmp/elsewhere", "not-allowed"),
        ("server.auth_mode", "token", "not-allowed"),
        ("backup.keep_daily", "99999", "invalid"),
        ("no.such.key", "1", "invalid"),
    ] {
        let x = o.command_ex(&site, "setting.set", &[("key", k), ("value", v)], false).await.unwrap();
        let x = done(&o, &x.id).await;
        assert_eq!((x.state.as_str(), x.result_code.as_str()), ("failed", code), "{k}");
    }
    assert!(srv::settings::get_bool(&pool, None, "adminsvr.enabled").await);

    // ---- schedules (SRE scoped to the site)
    o.set_scopes("sre@example.com", vec![site.clone()]).await.unwrap();
    let x = sre.command_ex(&site, "schedule.pause", &[("schedule", &s0.id)], false).await.unwrap();
    assert_eq!(done(&sre, &x.id).await.state, "applied");
    assert!(!srv::schedule::get(&pool, &sch.id).await.unwrap().enabled);
    let x = sre.command_ex(&site, "schedule.resume", &[("schedule", &s0.id)], true).await.unwrap();
    let x = done(&sre, &x.id).await;
    assert_eq!((x.state.as_str(), x.preview[0].before.as_str(), x.preview[0].after.as_str()), ("previewed", "paused", "enabled"));
    assert!(!srv::schedule::get(&pool, &sch.id).await.unwrap().enabled);
    let x = sre.command_ex(&site, "schedule.resume", &[("schedule", &s0.id)], false).await.unwrap();
    assert_eq!(done(&sre, &x.id).await.state, "applied");
    assert!(srv::schedule::get(&pool, &sch.id).await.unwrap().enabled);
    let x = sre.command_ex(&site, "schedule.pause", &[("schedule", "s-0000000000000000")], false).await.unwrap();
    assert_eq!(done(&sre, &x.id).await.result_code, "invalid");

    // ---- plugin catalog and a two-person push
    let good = package(&root, "disk-watch", "1.2.0", &pkg_key);
    assert!(sre.upload_plugin(good.clone()).await.is_err(), "SREs can't add packages");
    assert!(adm.upload_plugin(b"not a package".to_vec()).await.is_err());
    let e = adm.upload_plugin(good.clone()).await.unwrap();
    assert_eq!((e.name.as_str(), e.version.as_str()), ("disk-watch", "1.2.0"));
    assert_eq!(o.catalog().await.unwrap().packages.len(), 1);
    let unknown = "ab".repeat(32);
    assert!(o.command_ex(&site, "plugin.install", &[("sha256", &unknown)], false).await.is_err());

    // dry run: no approval needed; the site fetches and validates, installs nothing
    let x = sre.command_ex(&site, "plugin.install", &[("sha256", &e.sha256)], true).await.unwrap();
    let x = done(&sre, &x.id).await;
    assert_eq!((x.state.as_str(), x.result_code.as_str()), ("previewed", "ok"), "{x:?}");
    assert_eq!(x.preview[0].after, "1.2.0");
    assert!(srv::packages::list(&pool, &cfg.layout()).await.unwrap().is_empty());

    let x = sre.command_ex(&site, "plugin.install", &[("sha256", &e.sha256)], false).await.unwrap();
    assert_eq!(x.state, "awaiting-approval");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(o.get_command(&x.id).await.unwrap().info.unwrap().state, "awaiting-approval", "sent without approval");
    assert!(sre.approve_command(&x.id).await.is_err(), "SREs can't approve");
    let x2 = adm.command_ex(&site, "plugin.install", &[("sha256", &e.sha256)], false).await.unwrap();
    assert!(adm.approve_command(&x2.id).await.is_err(), "the requester can't be the second person");
    adm.reject_command(&x2.id).await.unwrap();
    assert_eq!(o.get_command(&x2.id).await.unwrap().info.unwrap().state, "rejected");
    let ap = adm.approve_command(&x.id).await.unwrap();
    assert_eq!(ap.approved_by.is_empty(), false);
    let x = done(&o, &x.id).await;
    assert_eq!((x.state.as_str(), x.result_code.as_str()), ("applied", "ok"), "{x:?}");
    let inst = srv::packages::list(&pool, &cfg.layout()).await.unwrap();
    assert_eq!((inst.len(), inst[0].name.as_str(), inst[0].enabled), (1, "disk-watch", true));
    until("plugins document", 40, || async {
        o.site_plugins(&site).await.ok().and_then(|i| i.plugins).is_some_and(|p| p.installed.iter().any(|i| i.name == "disk-watch" && i.version == "1.2.0"))
    })
    .await;

    // ---- real break: adminsvr skips the approval (row flipped to pending): the site refuses
    let x = o.command_ex(&site, "plugin.disable", &[("name", "disk-watch")], false).await.unwrap();
    assert_eq!(x.state, "awaiting-approval");
    let apool = itcmdr_adminsvr::store::connect(&db).await.unwrap();
    let cid = x.id.clone();
    apool
        .run(move |c| {
            use diesel::prelude::*;
            use itcmdr_adminsvr::schema::commands::dsl::*;
            Ok(diesel::update(commands.filter(id.eq(cid))).set(state.eq("pending")).execute(c)?)
        })
        .await
        .unwrap();
    // (send_pending runs on its poll)
    let x = done(&o, &x.id).await;
    assert_eq!((x.state.as_str(), x.result_code.as_str()), ("failed", "needs-approval"), "{x:?}");
    assert!(srv::packages::list(&pool, &cfg.layout()).await.unwrap()[0].enabled);
    // ... and approved properly it runs
    let x = o.command_ex(&site, "plugin.disable", &[("name", "disk-watch")], false).await.unwrap();
    adm.approve_command(&x.id).await.unwrap();
    assert_eq!(done(&o, &x.id).await.state, "applied");
    assert!(!srv::packages::list(&pool, &cfg.layout()).await.unwrap()[0].enabled);

    // ---- a tampered package: the site quarantines it; only a reason code goes up
    let secret_file = format!("{}.ps1", word(9));
    bad.push(secret_file.clone());
    let mut ents: Vec<(String, Vec<u8>)> = srv::packages::read_tar_gz(&package(&root, "net-watch", "2.0.0", &pkg_key)).unwrap().into_iter().collect();
    ents.push((secret_file.clone(), b"evil".to_vec())); // not listed in the manifest
    let tampered = {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        {
            let mut t = tar::Builder::new(&mut gz);
            for (n, b) in &ents {
                let mut h = tar::Header::new_gnu();
                h.set_size(b.len() as u64);
                h.set_mode(0o644);
                h.set_cksum();
                t.append_data(&mut h, n, b.as_slice()).unwrap();
            }
            t.finish().unwrap();
        }
        gz.finish().unwrap()
    };
    let e2 = adm.upload_plugin(tampered).await.unwrap();
    let x = o.command_ex(&site, "plugin.install", &[("sha256", &e2.sha256)], false).await.unwrap();
    adm.approve_command(&x.id).await.unwrap();
    let x = done(&o, &x.id).await;
    assert_eq!((x.state.as_str(), x.result_code.as_str()), ("failed", "quarantined"), "{x:?}");
    until("quarantine in the plugins document", 40, || async {
        o.site_plugins(&site).await.ok().and_then(|i| i.plugins).is_some_and(|p| p.quarantine.iter().any(|q| q.sha256 == e2.sha256))
    })
    .await;
    let q = o.site_plugins(&site).await.unwrap().plugins.unwrap().quarantine;
    assert!(q.iter().all(|q| !q.reason.is_empty() && q.reason.chars().all(|c| c.is_ascii_lowercase() || c == '-')), "{q:?}");

    // ---- freeze (owners)
    assert!(adm.set_freeze(true).await.is_err());
    assert!(o.set_freeze(true).await.unwrap().frozen);
    assert!(adm.freeze().await.unwrap().frozen);
    let err = adm.command_ex(&site, "schedule.run", &[("schedule", &s0.id)], false).await.err().unwrap();
    assert!(format!("{err}").contains("frozen"), "{err}");
    assert!(o.command(&site, "backup.start", &[]).await.is_err());
    let pend = sre.command_ex(&site, "plugin.enable", &[("name", "disk-watch")], false);
    assert!(pend.await.is_err());
    assert!(!o.set_freeze(false).await.unwrap().frozen);
    let x = sre.command_ex(&site, "plugin.enable", &[("name", "disk-watch"), ("version", "1.2.0")], false).await.unwrap();
    adm.approve_command(&x.id).await.unwrap();
    assert_eq!(done(&o, &x.id).await.state, "applied");

    // ---- metrics summaries
    let ls = o.list_sites().await.unwrap().sites[0].last_status.clone().unwrap();
    assert!(ls.disk_total_bytes > 0 && ls.disk_free_bytes >= 0 && ls.disk_free_bytes <= ls.disk_total_bytes, "{ls:?}");

    // ---- nothing sensitive on the wire or in adminsvr's documents and commands
    let mut n = 0;
    while let Ok(m) = tap.try_recv() {
        clean("wire", &m.encode_to_vec(), &format!("{m:?}"), &bad);
        n += 1;
    }
    assert!(n > 5);
    let docs: Vec<(Option<Vec<u8>>, Option<Vec<u8>>)> = apool
        .run(|c| {
            use diesel::prelude::*;
            use itcmdr_adminsvr::schema::sites::dsl::*;
            Ok(sites.select((config, plugins)).load(c)?)
        })
        .await
        .unwrap();
    for (a1, b1) in docs {
        clean("adminsvr config", &a1.unwrap_or_default(), "", &bad);
        clean("adminsvr plugins", &b1.unwrap_or_default(), "", &bad);
    }
    let prev: Vec<Option<Vec<u8>>> = apool
        .run(|c| {
            use diesel::prelude::*;
            use itcmdr_adminsvr::schema::commands::dsl::*;
            Ok(commands.select(preview).load(c)?)
        })
        .await
        .unwrap();
    for p in prev {
        clean("adminsvr previews", &p.unwrap_or_default(), "", &bad);
    }
    // the site audit has every command with its identity
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action LIKE 'adminsvr.command.%'").fetch_one(&pool).await.unwrap();
    assert!(n >= 15, "{n}");
    assert!(o.verify_audit().await.unwrap().ok);
    srun.abort();
    srv::sitelink::set_tap(None);
}
