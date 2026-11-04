//! v1.2.0 adminsvr's own backup: round trip into a fresh install, and real breaks (missing,
//! corrupt or truncated archive, wrong words, foreign recovery key, running service), the drill,
//! prune, and that only adminsvr's own files are ever archived.
mod common;
use itcmdr_adminsvr::backup as b;
use itcmdr_adminsvr::config::Config;
use std::path::Path;

fn cfg(dir: &Path, url: &str) -> Config {
    Config::for_test(
        dir.to_path_buf(),
        url,
        common::free_port(),
        common::free_port(),
    )
}

fn sqlite_url(dir: &Path) -> String {
    format!("sqlite://{}", dir.join("adminsvr.db").display())
}

/// An installed adminsvr with an owner, ACME files and backups set up. Returns the words.
async fn installed(c: &Config) -> String {
    itcmdr_adminsvr::api::init(c, "owner@example.invalid", "Owner")
        .await
        .unwrap();
    std::fs::create_dir_all(c.data_dir.join("acme")).unwrap();
    std::fs::write(c.data_dir.join("acme/account.json"), b"{\"acct\":1}").unwrap();
    let (w, _) = b::init(c, false).unwrap();
    w
}

async fn users(url: &str) -> i64 {
    let p = common::pool(url).await;
    p.run(|c| {
        use diesel::prelude::*;
        use itcmdr_adminsvr::schema::users::dsl::*;
        Ok(users.count().get_result::<i64>(c)?)
    })
    .await
    .unwrap()
}

async fn round_trip(src_url: String, dst_url: String) {
    let (a, z) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let ca = cfg(a.path(), &src_url);
    let words = installed(&ca).await;
    let pool = common::pool(&src_url).await;
    let info = b::create(&ca, &pool, "manual", None).await.unwrap();
    drop(pool);
    // Every archived file is on the allow list (adminsvr's own state, nothing from sites).
    for f in &info.manifest.files {
        assert!(b::allowed(&f.path), "{}", f.path);
    }
    let names: Vec<&str> = info
        .manifest
        .files
        .iter()
        .map(|f| f.path.as_str())
        .collect();
    for want in [
        b::F_ROOT,
        "tls/ca.key",
        "tls/server.pem",
        "acme/account.json",
        b::F_RECOVERY,
        b::F_CONFIG,
    ] {
        assert!(names.contains(&want), "{want} missing from {names:?}");
    }
    assert!(names.contains(&b::F_SQLITE) || names.contains(&b::F_PG));

    // Quick and deep drills pass.
    let pool = common::pool(&src_url).await;
    let q = b::drill(&ca, Some(&pool), None, None).await.unwrap();
    assert!(q.ok, "{}", q.error);
    let d = b::drill(&ca, Some(&pool), None, Some(&words))
        .await
        .unwrap();
    assert!(d.ok && d.deep && d.users == 1, "{d:?}");
    assert!(b::last_drill(&ca.data_dir).unwrap().ok);
    drop(pool);

    // Restore into a fresh install (another machine): same root key, CA, server cert, ACME.
    let cz = cfg(z.path(), &dst_url);
    let dry = b::restore(&cz, &info.path, &words, true).await.unwrap();
    assert!(dry.dry_run && !z.path().join("keys/root.key").exists());
    let r = b::restore(&cz, &info.path, &words, false).await.unwrap();
    assert!(r.safety_copy.unwrap().exists());
    for f in [
        "keys/root.key",
        "tls/ca.key",
        "tls/ca.pem",
        "tls/server.key",
        "tls/server.pem",
        "acme/account.json",
        "backup/recovery.json",
    ] {
        assert_eq!(
            std::fs::read(a.path().join(f)).unwrap(),
            std::fs::read(z.path().join(f)).unwrap(),
            "{f}"
        );
    }
    assert!(z.path().join("restored/config/adminsvr.env").exists());
    assert_eq!(users(&dst_url).await, 1);
    let mz = itcmdr_adminsvr::material::load(&cz).unwrap();
    let ma = itcmdr_adminsvr::material::load(&ca).unwrap();
    assert_eq!(mz.fingerprint, ma.fingerprint);
    assert_eq!(mz.root_hex(), ma.root_hex());
    let pz = common::pool(&dst_url).await;
    let av = itcmdr_adminsvr::audit::verify(&pz).await.unwrap();
    assert!(av.ok, "audit chain after restore: {av:?}");
    // The restored install makes and opens its own backups with the same words.
    let again = b::create(&cz, &pz, "manual", None).await.unwrap();
    b::open(
        &again.path,
        &words,
        b::load_recovery(&cz.data_dir).unwrap().as_ref(),
    )
    .unwrap();
}

#[tokio::test]
async fn sqlite_round_trip_into_fresh_install() {
    let (s, d) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    round_trip(sqlite_url(s.path()), sqlite_url(d.path())).await;
}

#[tokio::test]
async fn postgres_round_trip_into_fresh_install() {
    let Some(src) = common::pg_db().await else {
        return;
    };
    let Some(dst) = common::pg_db().await else {
        return;
    };
    round_trip(src.url.clone(), dst.url.clone()).await;
}

#[tokio::test]
async fn real_breaks() {
    let a = tempfile::tempdir().unwrap();
    let url = sqlite_url(a.path());
    let ca = cfg(a.path(), &url);
    itcmdr_adminsvr::api::init(&ca, "owner@example.invalid", "Owner")
        .await
        .unwrap();
    let pool = common::pool(&url).await;
    // Not set up yet: refused.
    let e = b::create(&ca, &pool, "manual", None)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("backup init"), "{e}");
    let (words, rec) = b::init(&ca, false).unwrap();
    assert!(b::init(&ca, false).is_err(), "init twice must refuse");
    let info = b::create(&ca, &pool, "manual", None).await.unwrap();
    drop(pool);
    let bytes = std::fs::read(&info.path).unwrap();
    let wrong = itcmdr_keys::generate_words();

    // Missing archive.
    let e = b::verify(&a.path().join("nope.itcab"), Some(&rec))
        .unwrap_err()
        .to_string();
    assert!(e.contains("missing"), "{e}");
    // Empty / not an archive.
    let junk = a.path().join("junk.itcab");
    std::fs::write(&junk, b"").unwrap();
    assert!(b::verify(&junk, None)
        .unwrap_err()
        .to_string()
        .contains("not an adminsvr backup"));
    // Corrupt contents (one flipped byte near the end).
    let bad = a.path().join("bad.itcab");
    let mut v = bytes.clone();
    let n = v.len() - 10;
    v[n] ^= 0xff;
    std::fs::write(&bad, &v).unwrap();
    let e = b::verify(&bad, Some(&rec)).unwrap_err().to_string();
    assert!(e.contains("damaged"), "{e}");
    assert!(b::open(&bad, &words, Some(&rec)).is_err());
    // Truncated.
    let tr = a.path().join("trunc.itcab");
    std::fs::write(&tr, &bytes[..bytes.len() / 2]).unwrap();
    assert!(b::verify(&tr, Some(&rec)).is_err());
    // Altered header (manifest edited, the signature no longer matches).
    let alt = a.path().join("alt.itcab");
    let mut av = bytes.clone();
    let pos = av
        .windows(16)
        .position(|w| w == b"\"kind\":\"manual\",")
        .unwrap();
    av[pos + 8] = b'M';
    std::fs::write(&alt, &av).unwrap();
    let e = b::verify(&alt, Some(&rec))
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(e.contains("signature"), "{e}");
    // Wrong words: refused before anything is written.
    let e = b::open(&info.path, &wrong, Some(&rec))
        .unwrap_err()
        .to_string();
    assert!(e.contains("do not open"), "{e}");
    let z = tempfile::tempdir().unwrap();
    let cz = cfg(z.path(), &sqlite_url(z.path()));
    let e = b::restore(&cz, &info.path, &wrong, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("do not open"), "{e}");
    assert!(!z.path().join("keys").exists() && !z.path().join("adminsvr.db").exists());
    // Not words at all.
    assert!(b::open(&info.path, "not twenty four words", Some(&rec)).is_err());
    // Another adminsvr's recovery key is refused by a pinned install.
    let o = tempfile::tempdir().unwrap();
    let co = cfg(o.path(), &sqlite_url(o.path()));
    itcmdr_adminsvr::material::load(&co).unwrap();
    let (_, other) = b::init(&co, false).unwrap();
    let e = b::verify(&info.path, Some(&other)).unwrap_err().to_string();
    assert!(e.contains("another recovery key"), "{e}");
    // Restore refuses while the service is running (something answers on the API port).
    let l = std::net::TcpListener::bind(&cz.api_addr).unwrap();
    let e = b::restore(&cz, &info.path, &words, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("stop the itcmdr-adminsvr service"), "{e}");
    drop(l);
    // A Postgres install refuses a SQLite backup.
    let mut cp = cz.clone();
    cp.db_url = "postgres://u@127.0.0.1:1/x".into();
    let e = b::restore(&cp, &info.path, &words, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("SQLite"), "{e}");

    // Drill: a damaged newest archive is reported as failed and recorded.
    let dd = tempfile::tempdir().unwrap();
    std::fs::copy(&info.path, dd.path().join(&info.name)).unwrap();
    std::fs::write(
        dd.path()
            .join("itcmdr-adminsvr-29991231-235959999-manual.itcab"),
        &v,
    )
    .unwrap();
    let r = b::drill(&ca, None, Some(dd.path()), None).await.unwrap();
    assert!(!r.ok && r.error.contains("damaged"), "{r:?}");
    assert!(!b::last_drill(&ca.data_dir).unwrap().ok);
    // Drill with wrong words fails; no backups at all fails.
    let r = b::drill(&ca, None, None, Some(&wrong)).await.unwrap();
    assert!(!r.ok, "{r:?}");
    let empty = tempfile::tempdir().unwrap();
    let r = b::drill(&ca, None, Some(empty.path()), None).await.unwrap();
    assert!(!r.ok && r.error.contains("no backups"), "{r:?}");
}

#[tokio::test]
async fn restore_over_changed_install_keeps_safety_copy() {
    let a = tempfile::tempdir().unwrap();
    let url = sqlite_url(a.path());
    let ca = cfg(a.path(), &url);
    let words = installed(&ca).await;
    let pool = common::pool(&url).await;
    let info = b::create(&ca, &pool, "manual", None).await.unwrap();
    pool.exec("INSERT INTO users (id, email, name, role, created_by, created_ms) VALUES ('u2','b@example.invalid','B','admin','t',1)")
        .await
        .unwrap();
    drop(pool);
    assert_eq!(users(&url).await, 2);
    let r = b::restore(&ca, &info.path, &words, false).await.unwrap();
    assert_eq!(users(&url).await, 1);
    let safety = r.safety_copy.unwrap();
    assert!(safety.join("db/adminsvr.sqlite").exists() && safety.join("keys/root.key").exists());
}

#[tokio::test]
async fn prune_keeps_newest_nightly_only() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..5 {
        std::fs::write(
            d.path().join(format!(
                "itcmdr-adminsvr-2026010{i}-030000000-nightly.itcab"
            )),
            b"x",
        )
        .unwrap();
    }
    std::fs::write(
        d.path()
            .join("itcmdr-adminsvr-20250101-030000000-manual.itcab"),
        b"x",
    )
    .unwrap();
    let gone = b::prune(d.path(), 2).unwrap();
    assert_eq!(gone.len(), 3);
    let left: Vec<String> = b::list(d.path())
        .unwrap()
        .into_iter()
        .map(|x| x.name)
        .collect();
    assert_eq!(left.len(), 3);
    assert!(left.iter().any(|n| n.contains("manual")));
    assert!(left.iter().any(|n| n.contains("20260104")));
}
