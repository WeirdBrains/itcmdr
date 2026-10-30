//! v0.8.0 backup & restore (decision 13) and itcmdr-cli.
use ::itcmdr_server as srv;
use prost::Message;
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use std::path::Path;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::{Code, Request};

const TOKEN: &str = "legacy-token";

fn cfg(root: &Path) -> ServerConfig {
    let mut c = ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        admin_token_hash: Some(srv::api::sha256_hex(TOKEN)),
        ..ServerConfig::default()
    };
    c.backup.nightly = false;
    c.backup.dir = Some(root.join("share").join("backups"));
    c
}

async fn seed(pool: &srv::db::DbPool, n: i64) {
    for i in 0..n {
        sqlx::query("INSERT INTO runs(run_id,computer,ts_ms) VALUES (?, 'WS-01', ?)")
            .bind(format!("r{i}"))
            .bind(i)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn count(pool: &srv::db::DbPool, t: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {t}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn round_trip_onto_a_fresh_data_dir_with_only_the_words() {
    let a = tempfile::tempdir().unwrap();
    let ca = cfg(a.path());
    let pool = srv::db::init_db(&ca).await.unwrap();
    let out = srv::auth::init(&pool, &ca.layout()).await.unwrap();
    seed(&pool, 25).await;
    let old_root = out.root_fingerprint.clone();
    let b = srv::backup::create(&pool, &ca, "manual", None)
        .await
        .unwrap();
    pool.close().await;
    assert!(b.path.starts_with(a.path().join("share")));
    assert_eq!(b.manifest.schema_version, srv::db::SCHEMA_VERSION);
    let names: Vec<_> = b.manifest.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        names,
        ["itcmdr.database", "keys/db.key.escrow", "server.json"]
    );
    let raw = std::fs::read(&b.path).unwrap();
    for secret in [&out.words.as_bytes()[..20], b"WS-01".as_slice()] {
        assert!(
            !raw.windows(secret.len()).any(|w| w == secret),
            "archive is encrypted"
        );
    }

    // Fresh machine: empty data dir, nothing but the archive and the words.
    let fresh = tempfile::tempdir().unwrap();
    let cb = cfg(fresh.path());
    let other = srv::keys::generate_words();
    assert!(
        srv::backup::restore(&cb, &b.path, &other, false)
            .await
            .is_err(),
        "other words fail"
    );
    let dry = srv::backup::restore(&cb, &b.path, &out.words, true)
        .await
        .unwrap();
    assert!(dry.dry_run && dry.manifest.is_some());
    assert!(!cb.database_path().exists(), "dry run changes nothing");

    let r = srv::backup::restore(&cb, &b.path, &out.words, false)
        .await
        .unwrap();
    assert!(r.safety_copy.is_none(), "fresh dir: nothing to copy aside");
    assert!(r.restored_config.unwrap().exists());
    let new_root = r.new_root_fingerprint.unwrap();
    assert_ne!(new_root, old_root);
    let pool = srv::db::init_db(&cb).await.unwrap();
    assert_eq!(count(&pool, "runs").await, 25);
    let keys = srv::auth::list_keys(&pool).await.unwrap();
    assert!(
        keys.iter()
            .any(|k| k.fingerprint == old_root && k.revoked_at.is_some()),
        "old root revoked"
    );
    assert!(keys
        .iter()
        .any(|k| k.fingerprint == new_root && k.revoked_at.is_none()));
    // The restored server can back itself up again (new root key is certified by the same master).
    let b2 = srv::backup::create(&pool, &cb, "manual", None)
        .await
        .unwrap();
    let mp = srv::auth::master_public(&pool).await.unwrap().unwrap();
    srv::backup::verify(&b2.path, Some(&mp.ed25519)).unwrap();
    seed_one(&pool).await;
    pool.close().await;

    // Restoring over existing state copies it aside first.
    let r = srv::backup::restore(&cb, &b.path, &out.words, false)
        .await
        .unwrap();
    let safety = r.safety_copy.unwrap();
    assert!(safety.join("itcmdr.database").exists() && safety.join("db.key.escrow").exists());
    let pool = srv::db::init_db(&cb).await.unwrap();
    assert_eq!(count(&pool, "runs").await, 25, "back to the archived state");
}

async fn seed_one(pool: &srv::db::DbPool) {
    sqlx::query("INSERT INTO runs(run_id,computer,ts_ms) VALUES ('later','WS-02',99)")
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_tampered_or_foreign_archive_is_rejected() {
    let a = tempfile::tempdir().unwrap();
    let c = cfg(a.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    seed(&pool, 3).await;
    let b = srv::backup::create(&pool, &c, "manual", None)
        .await
        .unwrap();
    let mp = srv::auth::master_public(&pool).await.unwrap().unwrap();
    srv::backup::verify(&b.path, Some(&mp.ed25519)).unwrap();
    let master = srv::keys::Master::from_words(&out.words).unwrap();
    srv::backup::open(&b.path, &master).unwrap();
    let good = std::fs::read(&b.path).unwrap();
    let hlen = u32::from_le_bytes(good[8..12].try_into().unwrap()) as usize;
    let t = a.path().join("t.itcb");

    // 1. a flipped ciphertext byte
    let mut x = good.clone();
    let last = x.len() - 1;
    x[last] ^= 1;
    std::fs::write(&t, &x).unwrap();
    assert!(srv::backup::verify(&t, None).is_err());
    assert!(srv::backup::open(&t, &master).is_err());
    // 2. an edited manifest (schema version) with the old signature
    let mut h: serde_json::Value = serde_json::from_slice(&good[12..12 + hlen]).unwrap();
    h["manifest"]["schema_version"] = 99.into();
    let hj = serde_json::to_vec(&h).unwrap();
    let mut x = good[..8].to_vec();
    x.extend((hj.len() as u32).to_le_bytes());
    x.extend(&hj);
    x.extend(&good[12 + hlen..]);
    std::fs::write(&t, &x).unwrap();
    let e = srv::backup::verify(&t, None).unwrap_err().to_string();
    assert!(e.contains("signature"), "{e}");
    // 3. truncated
    std::fs::write(&t, &good[..good.len() / 2]).unwrap();
    assert!(srv::backup::verify(&t, None).is_err());
    // 4. another server's archive
    let o = tempfile::tempdir().unwrap();
    let oc = cfg(o.path());
    let op = srv::db::init_db(&oc).await.unwrap();
    srv::auth::init(&op, &oc.layout()).await.unwrap();
    let ob = srv::backup::create(&op, &oc, "manual", None).await.unwrap();
    srv::backup::verify(&ob.path, None).unwrap();
    assert!(srv::backup::verify(&ob.path, Some(&mp.ed25519)).is_err());
    assert!(srv::backup::restore(&c, &ob.path, &out.words, true)
        .await
        .is_err());
}

#[tokio::test]
async fn a_backup_taken_during_writes_is_consistent() {
    let a = tempfile::tempdir().unwrap();
    let c = cfg(a.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (wp, ws) = (pool.clone(), stop.clone());
    // Each transaction writes a run and its check together: a consistent snapshot has equal counts.
    let writer = tokio::spawn(async move {
        let mut i = 0i64;
        while !ws.load(std::sync::atomic::Ordering::Relaxed) {
            let mut tx = wp.begin().await.unwrap();
            sqlx::query("INSERT INTO runs(run_id,computer,ts_ms) VALUES (?, 'WS-01', ?)")
                .bind(format!("w{i}"))
                .bind(i)
                .execute(&mut *tx)
                .await
                .unwrap();
            sqlx::query("INSERT INTO checks(run_id,computer,category,check_id,severity,status,ts_ms) VALUES (?, 'WS-01','network','NET-DNS','info','ok',?)")
                .bind(format!("w{i}")).bind(i).execute(&mut *tx).await.unwrap();
            tx.commit().await.unwrap();
            i += 1;
        }
        i
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    let mut archives = Vec::new();
    for _ in 0..3 {
        archives.push(
            srv::backup::create(&pool, &c, "manual", None)
                .await
                .unwrap(),
        );
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let written = writer.await.unwrap();
    assert!(written > 0);
    for b in archives {
        let f = tempfile::tempdir().unwrap();
        let fc = cfg(f.path());
        srv::backup::restore(&fc, &b.path, &out.words, false)
            .await
            .unwrap();
        let p = srv::db::init_db(&fc).await.unwrap();
        let (r, k) = (count(&p, "runs").await, count(&p, "checks").await);
        assert_eq!(r, k, "snapshot cut through a transaction");
        p.close().await;
    }
}

#[test]
fn retention_keeps_daily_weekly_blocks_and_never_prunes_manual() {
    let day = 86_400_000i64;
    let now = chrono::Utc::now().timestamp_millis();
    let mut names: Vec<String> = (0..120)
        .map(|d| srv::backup::archive_name(now - d * day, "nightly"))
        .collect();
    names.push(srv::backup::archive_name(now - 400 * day, "manual"));
    names.push(srv::backup::archive_name(now - 401 * day, "pre-restore"));
    let del = srv::backup::prune_plan(&names, 7, 4, 13);
    let kept: Vec<_> = names.iter().filter(|n| !del.contains(n)).collect();
    for n in &names[..7] {
        assert!(kept.contains(&n), "the last 7 days are kept");
    }
    assert!(
        kept.iter().any(|n| n.contains("manual")) && kept.iter().any(|n| n.contains("pre-restore"))
    );
    // 7 daily + up to 4 weekly + one per 4-week block (overlapping) out of 120 nightlies.
    let nightly_kept = kept.iter().filter(|n| n.contains("nightly")).count();
    assert!((7..=24).contains(&nightly_kept), "{nightly_kept}");
    // Every 4-week block present in the input keeps an archive.
    let block = |n: &String| {
        let ms = srv::backup::parse_name_ms(n).unwrap();
        let t = chrono::TimeZone::timestamp_millis_opt(&chrono::Local, ms).unwrap();
        srv::backup::block_key(t.date_naive())
    };
    let mut all: Vec<_> = names
        .iter()
        .filter(|n| n.contains("nightly"))
        .map(block)
        .collect();
    all.sort();
    all.dedup();
    let mut kept_blocks: Vec<_> = kept
        .iter()
        .filter(|n| n.contains("nightly"))
        .map(|n| block(n))
        .collect();
    kept_blocks.sort();
    kept_blocks.dedup();
    assert_eq!(kept_blocks, all, "one archive per 4-week block");
    // Applying it twice deletes nothing more.
    let kept_owned: Vec<String> = kept.into_iter().cloned().collect();
    assert!(srv::backup::prune_plan(&kept_owned, 7, 4, 13).is_empty());
}

#[test]
fn four_week_blocks_follow_iso_weeks() {
    use chrono::NaiveDate as D;
    let k = |y, m, d| srv::backup::block_key(D::from_ymd_opt(y, m, d).unwrap());
    assert_eq!(k(2026, 1, 1), "2026-B01"); // ISO 2026-W01
    assert_eq!(k(2026, 1, 25), "2026-B01"); // W04, last week of block 1
    assert_eq!(k(2026, 1, 26), "2026-B02"); // W05
    assert_eq!(k(2026, 12, 27), "2026-B13"); // W52
    assert_eq!(k(2026, 12, 31), "2026-B13"); // W53 folds into block 13
    assert_eq!(k(2027, 1, 3), "2026-B13"); // still ISO year 2026, week 53
    assert_eq!(k(2027, 1, 4), "2027-B01");
    // A full ISO year (2026 has 53 weeks) of nightlies keeps exactly 13 block archives.
    let start = D::from_ymd_opt(2025, 12, 29)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let start = chrono::TimeZone::from_local_datetime(&chrono::Local, &start)
        .unwrap()
        .timestamp_millis();
    let names: Vec<String> = (0..371)
        .map(|d| srv::backup::archive_name(start + d * 86_400_000, "nightly"))
        .collect();
    let del = srv::backup::prune_plan(&names, 0, 0, 13);
    assert_eq!(names.len() - del.len(), 13);
}

async fn start(c: ServerConfig) -> (ItcmdrClient<Channel>, tokio::sync::oneshot::Sender<()>) {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut c = c;
    c.bind_addr = format!("127.0.0.1:{port}");
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(srv::run(c, async move {
        let _ = rx.await;
    }));
    for _ in 0..100 {
        if let Ok(cl) = ItcmdrClient::connect(format!("http://127.0.0.1:{port}")).await {
            return (cl, tx);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not start");
}

fn signed<T: Message>(key: &ed25519_dalek::SigningKey, method: &str, msg: T) -> Request<T> {
    let md = srv::auth::sign_request(key, method, &msg.encode_to_vec());
    let mut r = Request::new(msg);
    for (k, v) in md {
        r.metadata_mut().insert(k, v.parse().unwrap());
    }
    r
}

#[tokio::test]
async fn backup_rpcs_are_admin_only_and_audited() {
    let a = tempfile::tempdir().unwrap();
    let c = cfg(a.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let master = srv::keys::Master::from_words(&out.words).unwrap();
    let user = srv::keys::new_keypair();
    let cert = srv::keys::certify(
        &master.signing,
        "user",
        "Front Desk",
        "user",
        &user.verifying_key(),
    );
    let upk = hex::encode(user.verifying_key().as_bytes());
    srv::auth::register(
        &pool,
        srv::auth::NewKey {
            kind: "user",
            name: "Front Desk",
            role: srv::auth::Role::User,
            email: None,
            phone: None,
            public_key_hex: &upk,
            certificate: &cert,
        },
    )
    .await
    .unwrap();
    pool.close().await;
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let (mut cl, stop) = start(c.clone()).await;
    const CB: &str = "/itcmdr.Itcmdr/CreateBackup";
    const LB: &str = "/itcmdr.Itcmdr/ListBackups";
    const VB: &str = "/itcmdr.Itcmdr/VerifyBackup";

    assert_eq!(
        cl.create_backup(CreateBackupRequest {})
            .await
            .unwrap_err()
            .code(),
        Code::Unauthenticated
    );
    assert_eq!(
        cl.create_backup(signed(&user, CB, CreateBackupRequest {}))
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );
    let made = cl
        .create_backup(signed(&root, CB, CreateBackupRequest {}))
        .await
        .unwrap()
        .into_inner()
        .backup
        .unwrap();
    assert_eq!(made.kind, "manual");
    let list = cl
        .list_backups(signed(&root, LB, ListBackupsRequest {}))
        .await
        .unwrap()
        .into_inner()
        .backups;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, made.name);
    let v = cl
        .verify_backup(signed(
            &root,
            VB,
            VerifyBackupRequest {
                name: made.name.clone(),
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(v.ok, "{}", v.message);
    let bad = cl
        .verify_backup(signed(
            &root,
            VB,
            VerifyBackupRequest {
                name: "../x".into(),
            },
        ))
        .await
        .unwrap_err();
    assert_eq!(bad.code(), Code::InvalidArgument);
    let mut tok = Request::new(ListBackupsRequest {});
    tok.metadata_mut()
        .insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    cl.list_backups(tok)
        .await
        .expect("legacy token still works in mode both");
    let _ = stop.send(());
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pool = srv::db::init_db(&c).await.unwrap();
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_log WHERE action != 'schedule-seed' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        actions,
        [
            "backup-create",
            "backup-list",
            "backup-verify",
            "backup-list"
        ]
    );
}

fn cli(root: &Path) -> assert_cmd::Command {
    let cfgp = root.join("server.json");
    if !cfgp.exists() {
        let c = cfg(root);
        std::fs::write(&cfgp, serde_json::to_vec(&c).unwrap()).unwrap();
    }
    let mut cmd = assert_cmd::Command::cargo_bin("itcmdr-cli").unwrap();
    cmd.env("ITCMDR_SERVER_CONFIG", &cfgp)
        .env_remove("ITCMDR_DATA_DIR");
    cmd
}

#[test]
fn cli_init_certify_offline_register_backup_and_restore() {
    let a = tempfile::tempdir().unwrap();
    let o = cli(a.path())
        .arg("init")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let o = String::from_utf8(o).unwrap();
    let words: Vec<&str> = o
        .lines()
        .filter_map(|l| {
            l.trim()
                .split_once(". ")
                .filter(|(n, _)| n.parse::<u32>().is_ok())
                .map(|(_, w)| w)
        })
        .collect();
    assert_eq!(words.len(), 24);
    let words = words.join(" ") + "\n";
    cli(a.path()).arg("init").assert().failure();

    // Offline certification of a key generated elsewhere (no data dir needed).
    let k = srv::keys::new_keypair();
    let pk = hex::encode(k.verifying_key().as_bytes());
    let bundle = a.path().join("desk.json");
    assert_cmd::Command::cargo_bin("itcmdr-cli")
        .unwrap()
        .args([
            "keys",
            "certify",
            "--kind",
            "user",
            "--name",
            "Front Desk",
            "--role",
            "user",
            "--public-key",
            &pk,
            "--out",
        ])
        .arg(&bundle)
        .write_stdin(words.clone())
        .assert()
        .success();
    assert_cmd::Command::cargo_bin("itcmdr-cli")
        .unwrap()
        .args([
            "keys",
            "certify",
            "--kind",
            "user",
            "--name",
            "X",
            "--role",
            "user",
            "--public-key",
            &pk,
        ])
        .write_stdin("")
        .assert()
        .failure();
    cli(a.path())
        .args(["keys", "register"])
        .arg(&bundle)
        .assert()
        .success();
    cli(a.path())
        .args(["service", "register", "--name", "xray-watch", "--out"])
        .arg(a.path().join("xray.key"))
        .write_stdin(words.clone())
        .assert()
        .success();
    assert!(a.path().join("xray.key.pub.json").exists());
    cli(a.path())
        .args([
            "user",
            "add",
            "--name",
            "Dr A",
            "--role",
            "admin",
            "--email",
            "a@example.local",
            "--out",
        ])
        .arg(a.path().join("dra.key"))
        .write_stdin(words.clone())
        .assert()
        .success();
    let list = String::from_utf8(
        cli(a.path())
            .args(["keys", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    for n in ["root", "Front Desk", "xray-watch", "Dr A"] {
        assert!(list.contains(n), "{list}");
    }
    let fp = srv::keys::fingerprint(&k.verifying_key());
    cli(a.path())
        .args(["keys", "rotate", &fp, "--out"])
        .arg(a.path().join("desk2.key"))
        .write_stdin(words.clone())
        .assert()
        .success();
    let list = String::from_utf8(
        cli(a.path())
            .args(["keys", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(
        list.lines()
            .any(|l| l.contains(&fp) && l.contains("REVOKED")),
        "{list}"
    );

    cli(a.path()).args(["backup", "create"]).assert().success();
    let listing = String::from_utf8(
        cli(a.path())
            .args(["--json", "backup", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let b: serde_json::Value = serde_json::from_str(listing.lines().next().unwrap()).unwrap();
    let path = b["path"].as_str().unwrap().to_string();
    cli(a.path())
        .args(["backup", "verify", &path])
        .assert()
        .success();
    cli(a.path())
        .args(["backup", "verify", "--deep", &path])
        .write_stdin(words.clone())
        .assert()
        .success();
    cli(a.path())
        .args(["backup", "verify", "--deep", &path])
        .write_stdin(srv::keys::generate_words())
        .assert()
        .failure();

    let fresh = tempfile::tempdir().unwrap();
    cli(fresh.path())
        .args(["restore", "--dry-run", "--from", &path])
        .write_stdin(words.clone())
        .assert()
        .success();
    assert!(!fresh.path().join("data").join("itcmdr.database").exists());
    cli(fresh.path())
        .args(["restore", "--from", &path])
        .write_stdin(words.clone())
        .assert()
        .success();
    let list = String::from_utf8(
        cli(fresh.path())
            .args(["keys", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(
        list.contains("xray-watch") && list.contains("Dr A"),
        "{list}"
    );
}

#[tokio::test]
async fn backup_settings_are_admin_only_validated_and_audited() {
    let a = tempfile::tempdir().unwrap();
    let c = cfg(a.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let master = srv::keys::Master::from_words(&out.words).unwrap();
    let user = srv::keys::new_keypair();
    let cert = srv::keys::certify(
        &master.signing,
        "user",
        "Front Desk",
        "user",
        &user.verifying_key(),
    );
    let upk = hex::encode(user.verifying_key().as_bytes());
    srv::auth::register(
        &pool,
        srv::auth::NewKey {
            kind: "user",
            name: "Front Desk",
            role: srv::auth::Role::User,
            email: None,
            phone: None,
            public_key_hex: &upk,
            certificate: &cert,
        },
    )
    .await
    .unwrap();
    pool.close().await;
    let root = srv::auth::load_root_key(&c.layout()).unwrap();
    let (mut cl, stop) = start(c.clone()).await;
    const GS: &str = "/itcmdr.Itcmdr/GetSettings";
    const US: &str = "/itcmdr.Itcmdr/UpdateSettings";
    const CB: &str = "/itcmdr.Itcmdr/CreateBackup";

    let s = cl
        .get_settings(signed(&root, GS, GetSettingsRequest {}))
        .await
        .unwrap()
        .into_inner()
        .backup
        .unwrap();
    assert_eq!(
        (s.hour, s.keep_daily, s.keep_weekly, s.keep_blocks),
        (2, 7, 4, 13)
    );
    assert!(s.dir.ends_with("backups"), "{}", s.dir);
    assert_eq!(
        cl.get_settings(signed(&user, GS, GetSettingsRequest {}))
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );

    let newdir = a.path().join("elsewhere");
    let mut want = BackupSettings {
        dir: newdir.display().to_string(),
        nightly: true,
        hour: 3,
        keep_daily: 5,
        keep_weekly: 2,
        keep_blocks: 13,
    };
    let e = cl
        .update_settings(signed(
            &root,
            US,
            UpdateSettingsRequest {
                backup: Some(want.clone()),
            },
        ))
        .await
        .unwrap_err();
    assert_eq!(e.code(), Code::InvalidArgument, "missing folder refused");
    assert!(e.message().contains("does not exist"), "{}", e.message());
    std::fs::create_dir_all(&newdir).unwrap();
    assert_eq!(
        cl.update_settings(signed(
            &user,
            US,
            UpdateSettingsRequest {
                backup: Some(want.clone())
            }
        ))
        .await
        .unwrap_err()
        .code(),
        Code::PermissionDenied
    );
    let mut bad = want.clone();
    bad.hour = 24;
    assert_eq!(
        cl.update_settings(signed(
            &root,
            US,
            UpdateSettingsRequest { backup: Some(bad) }
        ))
        .await
        .unwrap_err()
        .code(),
        Code::InvalidArgument
    );
    cl.update_settings(signed(
        &root,
        US,
        UpdateSettingsRequest {
            backup: Some(want.clone()),
        },
    ))
    .await
    .unwrap();
    let got = cl
        .get_settings(signed(&root, GS, GetSettingsRequest {}))
        .await
        .unwrap()
        .into_inner()
        .backup
        .unwrap();
    assert_eq!(got, want);
    // Backups now go to the new folder.
    cl.create_backup(signed(&root, CB, CreateBackupRequest {}))
        .await
        .unwrap();
    assert_eq!(srv::backup::list(&newdir).unwrap().len(), 1);
    // Empty dir = default again.
    want.dir.clear();
    cl.update_settings(signed(
        &root,
        US,
        UpdateSettingsRequest { backup: Some(want) },
    ))
    .await
    .unwrap();
    let _ = stop.send(());
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pool = srv::db::init_db(&c).await.unwrap();
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='settings-update'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n, 2, "only successful updates are logged");
}

#[test]
fn cli_config_get_set_unset() {
    let a = tempfile::tempdir().unwrap();
    let get = |root: &Path| {
        String::from_utf8(
            cli(root)
                .args(["config", "get"])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone(),
        )
        .unwrap()
    };
    assert!(get(a.path()).contains("backup.hour = 2    [default]"));
    cli(a.path())
        .args(["config", "set", "backup.dir"])
        .arg(a.path().join("nope"))
        .assert()
        .failure();
    let d = a.path().join("bk");
    std::fs::create_dir_all(&d).unwrap();
    cli(a.path())
        .args(["config", "set", "backup.dir"])
        .arg(&d)
        .assert()
        .success();
    cli(a.path())
        .args(["config", "set", "backup.hour", "4"])
        .assert()
        .success();
    cli(a.path())
        .args(["config", "set", "backup.hour", "25"])
        .assert()
        .failure();
    cli(a.path())
        .args(["config", "set", "backup.color", "red"])
        .assert()
        .failure();
    let g = get(a.path());
    assert!(
        g.contains("backup.hour = 4    [db]")
            && g.contains(&format!("backup.dir = {}    [db]", d.display())),
        "{g}"
    );
    cli(a.path())
        .args(["config", "unset", "backup.hour"])
        .assert()
        .success();
    assert!(get(a.path()).contains("backup.hour = 2    [default]"));
    // every key, agent ones per machine
    let all = get(a.path());
    for k in [
        "server.retention_days = 400",
        "agent.quietMinutes = 10",
        "plugins.enable_on_install = true",
        "auth.max_skew_secs = 300",
    ] {
        assert!(all.contains(k), "{k} in {all}");
    }
    cli(a.path())
        .args([
            "config",
            "set",
            "agent.quietMinutes",
            "20",
            "--machine",
            "WS-01",
        ])
        .assert()
        .success();
    cli(a.path())
        .args([
            "config",
            "set",
            "server.retention_days",
            "30",
            "--machine",
            "WS-01",
        ])
        .assert()
        .failure();
    cli(a.path())
        .args(["config", "set", "server.bind_addr", "198.51.100.1:1"])
        .assert()
        .failure();
    let out = cli(a.path())
        .args(["config", "get", "agent.quietMinutes", "--machine", "WS-01"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8(out)
        .unwrap()
        .contains("agent.quietMinutes = 20    [db:WS-01]"));
    assert!(
        get(a.path()).contains("agent.quietMinutes = 10    [default]"),
        "global unaffected"
    );
}
