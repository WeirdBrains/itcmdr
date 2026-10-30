//! v0.9.0 service plugin packages: validation, quarantine, install/list/enable/disable/remove,
//! --url installs, plugins in backups.
use ::itcmdr_server as srv;
use srv::config::ServerConfig;
use srv::packages::{self, InstallError, Source};
use std::path::Path;

fn cfg(root: &Path) -> ServerConfig {
    let mut c = ServerConfig {
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        ..ServerConfig::default()
    };
    c.backup.nightly = false;
    c
}

struct Env {
    _tmp: tempfile::TempDir,
    root: std::path::PathBuf,
    c: ServerConfig,
    pool: srv::db::DbPool,
    words: String,
    admin: ed25519_dalek::SigningKey,
    user: ed25519_dalek::SigningKey,
}

async fn register(
    pool: &srv::db::DbPool,
    m: &srv::keys::Master,
    k: &ed25519_dalek::SigningKey,
    name: &str,
    role: srv::auth::Role,
) {
    let cert = srv::keys::certify(&m.signing, "user", name, role.as_str(), &k.verifying_key());
    let pk = hex::encode(k.verifying_key().as_bytes());
    srv::auth::register(
        pool,
        srv::auth::NewKey {
            kind: "user",
            name,
            role,
            email: None,
            phone: None,
            public_key_hex: &pk,
            certificate: &cert,
        },
    )
    .await
    .unwrap();
}

async fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let c = cfg(&root);
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let m = srv::keys::Master::from_words(&out.words).unwrap();
    let admin = srv::keys::new_keypair();
    let user = srv::keys::new_keypair();
    register(&pool, &m, &admin, "Dr A", srv::auth::Role::Admin).await;
    register(&pool, &m, &user, "Front Desk", srv::auth::Role::User).await;
    Env {
        _tmp: tmp,
        root,
        c,
        pool,
        words: out.words.to_string(),
        admin,
        user,
    }
}

/// A source dir with package.toml + files; returns the dir.
fn source(root: &Path, name: &str, version: &str) -> std::path::PathBuf {
    let d = root.join(format!("src-{name}-{version}"));
    std::fs::create_dir_all(d.join("lib")).unwrap();
    std::fs::write(d.join("run.ps1"), "Write-Output 'hi'\n").unwrap();
    std::fs::write(d.join("lib/helper.psm1"), "function X {}\n").unwrap();
    std::fs::write(
        d.join("package.toml"),
        format!(
            "name = \"{name}\"\nversion = \"{version}\"\nfeatures = [\"verify\", \"execute\", \"log\"]\n\n\
             [[files]]\nname = \"run.ps1\"\ntype = \"powershell\"\nmain = true\nchecksum = \"\"\n\n\
             [[files]]\nname = \"lib/helper.psm1\"\ntype = \"powershell\"\nchecksum = \"\"\n"
        ),
    )
    .unwrap();
    d
}

fn good(e: &Env, name: &str, version: &str) -> std::path::PathBuf {
    let out = e.root.join(format!("{name}-{version}.tar.gz"));
    packages::pack(&source(&e.root, name, version), &e.admin, &out).unwrap();
    out
}

/// Unpacked entries of a good package, for crafting bad ones.
fn entries(p: &Path) -> Vec<(String, Vec<u8>)> {
    packages::read_tar_gz(&std::fs::read(p).unwrap())
        .unwrap()
        .into_iter()
        .collect()
}

fn resign(
    e: &Env,
    mut ents: Vec<(String, Vec<u8>)>,
    key: &ed25519_dalek::SigningKey,
) -> Vec<(String, Vec<u8>)> {
    use ed25519_dalek::Signer;
    let m = ents
        .iter()
        .find(|(n, _)| n == "package.toml")
        .unwrap()
        .1
        .clone();
    let msg = format!("itcmdr-package/v1\n{}", packages::sha256_hex(&m));
    let sig = serde_json::json!({"v": 1, "signer": srv::keys::fingerprint(&key.verifying_key()), "sig": hex::encode(key.sign(msg.as_bytes()).to_bytes())});
    ents.retain(|(n, _)| n != "package.sig");
    ents.push(("package.sig".into(), serde_json::to_vec(&sig).unwrap()));
    let _ = e;
    ents
}

fn with_manifest(
    ents: &[(String, Vec<u8>)],
    f: impl Fn(String) -> String,
) -> Vec<(String, Vec<u8>)> {
    ents.iter()
        .map(|(n, b)| {
            if n == "package.toml" {
                (
                    n.clone(),
                    f(String::from_utf8(b.clone()).unwrap()).into_bytes(),
                )
            } else {
                (n.clone(), b.clone())
            }
        })
        .collect()
}

async fn expect_quarantined(e: &Env, bytes: Vec<u8>, why: &str) {
    let p = e.root.join(format!("bad-{}.tar.gz", uuid::Uuid::new_v4()));
    std::fs::write(&p, &bytes).unwrap();
    let before = packages::list_quarantine(&e.c.layout()).unwrap().len();
    match packages::install(&e.pool, &e.c.layout(), Source::Path(&p), true, "test").await {
        Err(InstallError::Quarantined { reason, file }) => {
            assert!(reason.contains(why), "expected {why:?}, got {reason:?}");
            assert!(e.c.layout().quarantine().join(&file).exists());
        }
        other => panic!("{why}: expected quarantine, got {other:?}"),
    }
    let q = packages::list_quarantine(&e.c.layout()).unwrap();
    assert_eq!(q.len(), before + 1);
    assert_eq!(q.last().unwrap().sha256, packages::sha256_hex(&bytes));
    let installed = std::fs::read_dir(e.c.layout().service_plugins())
        .unwrap()
        .count();
    assert_eq!(installed, 0, "{why}: nothing may be installed");
}

fn tgz(e: &Env, ents: &[(String, Vec<u8>)]) -> Vec<u8> {
    let p = e.root.join(format!("t-{}.tar.gz", uuid::Uuid::new_v4()));
    packages::write_tar_gz(&p, ents).unwrap();
    std::fs::read(p).unwrap()
}

/// tar.gz with one raw entry of the given type (symlink, hard link) or path, bypassing tar-rs checks.
fn raw_tgz(ents: &[(String, Vec<u8>)], extra: (&str, tar::EntryType, Option<&str>)) -> Vec<u8> {
    let mut t = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    for (n, b) in ents {
        let mut h = tar::Header::new_gnu();
        h.set_size(b.len() as u64);
        h.set_mode(0o644);
        t.append_data(&mut h, n, &b[..]).unwrap();
    }
    let mut h = tar::Header::new_old();
    {
        let name = extra.0.as_bytes();
        let old = h.as_old_mut();
        old.name[..name.len()].copy_from_slice(name);
        if let Some(l) = extra.2 {
            old.linkname[..l.len()].copy_from_slice(l.as_bytes());
        }
    }
    h.set_entry_type(extra.1);
    h.set_size(if extra.1 == tar::EntryType::Regular {
        1
    } else {
        0
    });
    h.set_mode(0o644);
    h.set_cksum();
    t.append(
        &h,
        if extra.1 == tar::EntryType::Regular {
            &b"x"[..]
        } else {
            &b""[..]
        },
    )
    .unwrap();
    t.into_inner().unwrap().finish().unwrap()
}

#[tokio::test]
async fn a_signed_package_installs_and_can_be_managed() {
    let e = env().await;
    let p = good(&e, "xray-watch", "1.0.6");
    let i = packages::install(&e.pool, &e.c.layout(), Source::Path(&p), false, "test")
        .await
        .unwrap();
    assert_eq!(
        i.dir,
        e.c.layout().service_plugins().join("xray-watch-1.0.6")
    );
    for f in ["package.toml", "package.sig", "run.ps1", "lib/helper.psm1"] {
        assert!(i.dir.join(f).exists(), "{f}");
    }
    assert!(!i.enabled, "installed disabled by default");
    assert!(i.signer.starts_with("Dr A"));
    // same version again: refused, not quarantined
    let again = packages::install(&e.pool, &e.c.layout(), Source::Path(&p), false, "test").await;
    assert!(matches!(again, Err(InstallError::Other(_))));
    assert!(packages::list_quarantine(&e.c.layout()).unwrap().is_empty());
    // second version; enabling one disables the other
    let p2 = good(&e, "xray-watch", "1.1.0");
    packages::install(&e.pool, &e.c.layout(), Source::Path(&p2), true, "test")
        .await
        .unwrap();
    assert!(
        packages::enable(&e.pool, "xray-watch", "t").await.is_err(),
        "ambiguous without @version"
    );
    packages::enable(&e.pool, "xray-watch@1.0.6", "t")
        .await
        .unwrap();
    let l = packages::list(&e.pool, &e.c.layout()).await.unwrap();
    let en: Vec<_> = l
        .iter()
        .filter(|p| p.enabled)
        .map(|p| p.version.as_str())
        .collect();
    assert_eq!(en, ["1.0.6"]);
    assert!(
        packages::remove(&e.pool, &e.c.layout(), "xray-watch@1.0.6", false, "t")
            .await
            .is_err(),
        "enabled needs --force"
    );
    packages::disable(&e.pool, "xray-watch", "t").await.unwrap();
    packages::remove(&e.pool, &e.c.layout(), "xray-watch@1.0.6", false, "t")
        .await
        .unwrap();
    assert!(!i.dir.exists());
    assert_eq!(
        packages::list(&e.pool, &e.c.layout()).await.unwrap().len(),
        1
    );
    let actions: Vec<String> = sqlx::query_scalar("SELECT action FROM audit_log ORDER BY id")
        .fetch_all(&e.pool)
        .await
        .unwrap();
    assert_eq!(
        actions,
        [
            "plugin-install",
            "plugin-install",
            "plugin-enable",
            "plugin-disable",
            "plugin-remove"
        ]
    );
}

#[tokio::test]
async fn invalid_packages_are_quarantined_and_denied() {
    let e = env().await;
    let ok = entries(&good(&e, "probe", "1.0.0"));
    let drop = |n: &str| {
        ok.iter()
            .filter(|(x, _)| x != n)
            .cloned()
            .collect::<Vec<_>>()
    };

    expect_quarantined(&e, b"not a package".to_vec(), "not a .tar.gz").await;
    expect_quarantined(
        &e,
        tgz(&e, &drop("package.toml")),
        "package.toml is missing",
    )
    .await;
    expect_quarantined(&e, tgz(&e, &drop("package.sig")), "package.sig is missing").await;
    expect_quarantined(&e, tgz(&e, &drop("lib/helper.psm1")), "listed but missing").await;
    let mut extra = ok.clone();
    extra.push(("evil.exe".into(), b"MZ".to_vec()));
    expect_quarantined(&e, tgz(&e, &extra), "not listed").await;
    let tampered: Vec<_> = ok
        .iter()
        .map(|(n, b)| {
            if n == "run.ps1" {
                (n.clone(), b"Remove-Item C:\\ -Recurse".to_vec())
            } else {
                (n.clone(), b.clone())
            }
        })
        .collect();
    expect_quarantined(&e, tgz(&e, &tampered), "sha256 does not match").await;
    // manifest edited after signing (e.g. version bump) -> signature fails
    let edited = with_manifest(&ok, |m| m.replace("1.0.0", "1.0.1"));
    expect_quarantined(&e, tgz(&e, &edited), "signature is invalid").await;
    // signed by keys that may not sign
    let stranger = srv::keys::new_keypair();
    expect_quarantined(
        &e,
        tgz(&e, &resign(&e, ok.clone(), &stranger)),
        "not an active admin or owner",
    )
    .await;
    expect_quarantined(
        &e,
        tgz(&e, &resign(&e, ok.clone(), &e.user)),
        "not an active admin or owner",
    )
    .await;
    srv::auth::revoke(&e.pool, &srv::keys::fingerprint(&e.admin.verifying_key()))
        .await
        .unwrap();
    expect_quarantined(&e, tgz(&e, &ok), "not an active admin or owner").await;
    // the root (owner) key can sign; malformed manifests are caught even when signed
    let root = srv::auth::load_root_key(&e.c.layout()).unwrap();
    let bad_toml = resign(
        &e,
        with_manifest(&ok, |m| m.replace("main = true", "main = \"yes\"")),
        &root,
    );
    expect_quarantined(&e, tgz(&e, &bad_toml), "package.toml is invalid").await;
    let two_mains = resign(
        &e,
        with_manifest(&ok, |m| {
            let mut v: toml::Value = toml::from_str(&m).unwrap();
            for f in v["files"].as_array_mut().unwrap() {
                f.as_table_mut()
                    .unwrap()
                    .insert("main".into(), toml::Value::Boolean(true));
            }
            toml::to_string(&v).unwrap()
        }),
        &root,
    );
    expect_quarantined(&e, tgz(&e, &two_mains), "exactly one file").await;
    let traversal = resign(
        &e,
        with_manifest(&ok, |m| m.replace("lib/helper.psm1", "../helper.psm1")),
        &root,
    );
    expect_quarantined(&e, tgz(&e, &traversal), "unsafe path").await;
    // unsafe tar entries
    let good_root = resign(&e, ok.clone(), &root);
    expect_quarantined(
        &e,
        raw_tgz(
            &good_root,
            ("../../escape.ps1", tar::EntryType::Regular, None),
        ),
        "unsafe path",
    )
    .await;
    expect_quarantined(
        &e,
        raw_tgz(&good_root, ("/etc/passwd", tar::EntryType::Regular, None)),
        "unsafe path",
    )
    .await;
    expect_quarantined(
        &e,
        raw_tgz(
            &good_root,
            ("link", tar::EntryType::Symlink, Some("/etc/passwd")),
        ),
        "only regular files",
    )
    .await;
    expect_quarantined(
        &e,
        raw_tgz(&good_root, ("hard", tar::EntryType::Link, Some("run.ps1"))),
        "only regular files",
    )
    .await;
    // ...and the root-signed original installs fine
    let p = e.root.join("root-signed.tar.gz");
    std::fs::write(&p, tgz(&e, &good_root)).unwrap();
    packages::install(&e.pool, &e.c.layout(), Source::Path(&p), false, "test")
        .await
        .unwrap();
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action = 'plugin-quarantine'")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!(
        n as usize,
        packages::list_quarantine(&e.c.layout()).unwrap().len()
    );
}

#[tokio::test]
async fn install_from_url() {
    let e = env().await;
    let bytes = std::fs::read(good(&e, "net-probe", "0.1.0")).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for s in listener.incoming().take(2) {
            let mut s = s.unwrap();
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            if req.starts_with("GET /pkg.tar.gz") {
                write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                )
                .unwrap();
                s.write_all(&bytes).unwrap();
            } else {
                write!(
                    s,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            }
        }
    });
    let url = format!("http://127.0.0.1:{port}/pkg.tar.gz");
    let i = packages::install(&e.pool, &e.c.layout(), Source::Url(&url), false, "test").await;
    assert_eq!(i.unwrap().name, "net-probe");
    let missing = format!("http://127.0.0.1:{port}/nope");
    let r = packages::install(&e.pool, &e.c.layout(), Source::Url(&missing), false, "test").await;
    assert!(
        matches!(r, Err(InstallError::Other(_))),
        "a failed download is not a quarantine"
    );
    let r = packages::install(
        &e.pool,
        &e.c.layout(),
        Source::Url("ftp://example.local/x"),
        false,
        "test",
    )
    .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn plugins_and_the_quarantine_index_are_in_backups() {
    let e = env().await;
    let p = good(&e, "xray-watch", "1.0.6");
    packages::install(&e.pool, &e.c.layout(), Source::Path(&p), true, "test")
        .await
        .unwrap();
    let bad = e.root.join("bad.tar.gz");
    std::fs::write(&bad, b"junk").unwrap();
    let _ = packages::install(&e.pool, &e.c.layout(), Source::Path(&bad), false, "test").await;
    let b = srv::backup::create(&e.pool, &e.c, "manual", None)
        .await
        .unwrap();
    let names: Vec<_> = b.manifest.files.iter().map(|f| f.path.as_str()).collect();
    for want in [
        "service_plugins/xray-watch-1.0.6/package.toml",
        "service_plugins/xray-watch-1.0.6/lib/helper.psm1",
        "quarantine/index.jsonl",
    ] {
        assert!(names.contains(&want), "{want} in {names:?}");
    }
    let fresh = tempfile::tempdir().unwrap();
    let fc = cfg(fresh.path());
    srv::backup::restore(&fc, &b.path, &e.words, false)
        .await
        .unwrap();
    let pool = srv::db::init_db(&fc).await.unwrap();
    let l = packages::list(&pool, &fc.layout()).await.unwrap();
    assert_eq!(l.len(), 1);
    assert!(l[0].enabled && l[0].dir.join("run.ps1").exists());
    assert_eq!(packages::list_quarantine(&fc.layout()).unwrap().len(), 1);
}

#[test]
fn cli_pack_install_list_enable_remove() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let cfgp = root.join("server.json");
    std::fs::write(&cfgp, serde_json::to_vec(&cfg(root)).unwrap()).unwrap();
    let cli = || {
        let mut c = assert_cmd::Command::cargo_bin("itcmdr-cli").unwrap();
        c.env("ITCMDR_SERVER_CONFIG", &cfgp)
            .env_remove("ITCMDR_DATA_DIR");
        c
    };
    let out = String::from_utf8(
        cli()
            .arg("init")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let words = out
        .lines()
        .filter_map(|l| {
            l.trim()
                .split_once(". ")
                .filter(|(n, _)| n.parse::<u32>().is_ok())
                .map(|(_, w)| w)
        })
        .collect::<Vec<_>>()
        .join(" ")
        + "\n";
    let key = root.join("dra.key");
    cli()
        .args(["user", "add", "--name", "Dr A", "--role", "admin", "--out"])
        .arg(&key)
        .write_stdin(words)
        .assert()
        .success();
    let src = source(root, "xray-watch", "1.0.6");
    let pkg = root.join("xray.tar.gz");
    cli()
        .args(["plugins", "pack", "--dir"])
        .arg(&src)
        .arg("--key")
        .arg(&key)
        .arg("--out")
        .arg(&pkg)
        .assert()
        .success();
    cli()
        .args(["service", "plugins", "install", "--path"])
        .arg(&pkg)
        .assert()
        .success();
    cli().args(["plugins", "install"]).assert().failure();
    let l = String::from_utf8(
        cli()
            .args(["plugins", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(l.contains("xray-watch") && l.contains("disabled"), "{l}");
    cli()
        .args(["plugins", "enable", "xray-watch"])
        .assert()
        .success();
    cli()
        .args(["plugins", "remove", "xray-watch"])
        .assert()
        .failure();
    cli()
        .args(["plugins", "remove", "xray-watch", "--force"])
        .assert()
        .success();
    std::fs::write(root.join("junk.tar.gz"), b"junk").unwrap();
    cli()
        .args(["plugins", "install", "--path"])
        .arg(root.join("junk.tar.gz"))
        .assert()
        .failure();
    let q = String::from_utf8(
        cli()
            .args(["plugins", "quarantine"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(q.contains("not a .tar.gz"), "{q}");
}
