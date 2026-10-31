//! v0.14.0 install and first-run setup: server.json values (upgrades keep what is set), path
//! checks, and first-run (master key + first admin key, so signed mode works from day one).
use ::itcmdr_server as srv;
use srv::setup::{self, Level, Values};
use std::path::Path;

fn run(cfg: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_itcmdr-cli"))
        .args(args)
        .env("ITCMDR_SERVER_CONFIG", cfg)
        .env_remove("ITCMDR_DATA_DIR")
        .output()
        .unwrap()
}

#[test]
fn apply_creates_then_keeps_existing_values_on_upgrade() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("conf").join("server.json");
    let v = Values {
        logs_dir: Some(t.path().join("share")),
        backup_dir: Some(t.path().join("bk")),
        ..Default::default()
    };
    let ch = setup::apply(&p, &v, false).unwrap();
    assert_eq!(ch, ["logs_dir", "backup.dir"]);
    let j: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    assert_eq!(j["backup"]["keep_daily"], 7, "template defaults kept");
    assert_eq!(j["logs_dir"], t.path().join("share").display().to_string());
    // An admin edit, then an upgrade with other values: nothing that is set changes.
    let mut j = j;
    j["retention_days"] = 99.into();
    j["auth_mode"] = "both".into();
    std::fs::write(&p, serde_json::to_vec(&j).unwrap()).unwrap();
    let up = Values {
        logs_dir: Some("/elsewhere".into()),
        bind_addr: Some("127.0.0.1:50999".into()),
        ..Default::default()
    };
    // bind_addr is set in the template, so keep_existing leaves it too.
    assert!(setup::apply(&p, &up, true).unwrap().is_empty());
    let j2: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    assert_eq!(j2["retention_days"], 99);
    assert_eq!(j2["auth_mode"], "both");
    assert_eq!(j2["logs_dir"], j["logs_dir"]);
    // An empty value is filled on upgrade.
    j["logs_dir"] = "".into();
    std::fs::write(&p, serde_json::to_vec(&j).unwrap()).unwrap();
    assert_eq!(setup::apply(&p, &up, true).unwrap(), ["logs_dir"]);
    // Without keep_existing an explicit value wins; bad bind addresses are refused.
    assert_eq!(setup::apply(&p, &up, false).unwrap(), ["bind_addr"]);
    let bad = Values {
        bind_addr: Some("nope".into()),
        ..Default::default()
    };
    assert!(setup::apply(&p, &bad, false).is_err());
    // A BOM from Windows editors is fine.
    let txt = std::fs::read_to_string(&p).unwrap();
    std::fs::write(&p, format!("\u{feff}{txt}")).unwrap();
    assert!(setup::apply(&p, &up, true).is_ok());
}

#[test]
fn path_checks_report_missing_and_unwritable_folders() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("server.json");
    let v = Values {
        logs_dir: Some(t.path().join("missing-share")),
        data_dir: Some(t.path().join("data")),
        ..Default::default()
    };
    setup::apply(&p, &v, false).unwrap();
    let cfg = srv::config::ServerConfig::from_path(p.clone()).unwrap();
    let ch = setup::check_paths(&cfg);
    let get = |n: &str| ch.iter().find(|c| c.name == n).unwrap().clone();
    assert_eq!(get("config").level, Level::Ok);
    assert_eq!(get("data_dir").level, Level::Ok, "data dir is created");
    assert!(t.path().join("data").is_dir());
    assert_eq!(
        get("logs_dir").level,
        Level::Fail,
        "the share is never created"
    );
    assert_eq!(get("backup.dir").level, Level::Warn);
    assert_eq!(setup::worst(&ch), Level::Fail);
    let o = run(&p, &["setup", "check"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(String::from_utf8_lossy(&o.stdout).contains("FAIL logs_dir"));
    // A file where a folder should be.
    std::fs::write(t.path().join("missing-share"), b"x").unwrap();
    assert!(setup::writable_dir(&t.path().join("missing-share"), false).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let ro = t.path().join("ro");
        std::fs::create_dir(&ro).unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o500)).unwrap();
        let root = unsafe { libc_geteuid() } == 0;
        if !root {
            assert!(setup::writable_dir(&ro, false)
                .unwrap_err()
                .contains("cannot write"));
        }
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

#[test]
fn word_confirmation() {
    let w = (1..=24)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(setup::confirm_ok(&w, &[2, 10, 24], " W2  w10 w24 "));
    assert!(!setup::confirm_ok(&w, &[2, 10, 24], "w2 w10"));
    assert!(!setup::confirm_ok(&w, &[2, 10, 24], "w2 w11 w24"));
    for _ in 0..50 {
        let p = setup::confirm_positions();
        assert!(
            p[0] < p[1] && p[1] < p[2] && p[0] >= 1 && p[2] <= 24,
            "{p:?}"
        );
    }
}

#[test]
fn first_run_registers_the_first_admin_so_signed_mode_works() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("server.json");
    std::fs::create_dir(t.path().join("share")).unwrap();
    let v = Values {
        logs_dir: Some(t.path().join("share")),
        data_dir: Some(t.path().join("data")),
        backup_dir: Some(t.path().join("share")),
        ..Default::default()
    };
    setup::apply(&p, &v, false).unwrap();
    let o = run(&p, &["setup", "check"]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{o:?}");
    assert!(out.contains("WARN initialized"), "{out}");

    // Not in a console: refused, nothing is created and no words are printed.
    let admin = srv::keys::new_keypair();
    let pk = hex::encode(admin.verifying_key().as_bytes());
    let o = run(
        &p,
        &[
            "setup",
            "first-run",
            "--admin-name",
            "Dr Example",
            "--admin-public-key",
            &pk,
        ],
    );
    assert!(!o.status.success());
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("console"),
        "{o:?}"
    );
    assert!(!String::from_utf8_lossy(&o.stdout).contains(" 1. "));

    let o = run(
        &p,
        &[
            "setup",
            "first-run",
            "--admin-name",
            "Dr Example",
            "--admin-public-key",
            &pk,
            "--no-confirm",
        ],
    );
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{o:?}");
    let words: Vec<String> = out
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|w| w[0].ends_with('.') && w[0].trim_end_matches('.').parse::<u32>().is_ok())
        .map(|w| w[1].to_string())
        .collect();
    assert_eq!(words.len(), 24, "{out}");
    assert!(out.contains("first admin key registered"), "{out}");
    // Nothing in the data dir holds the words.
    let joined = words.join(" ");
    for e in walk(&t.path().join("data")) {
        let b = std::fs::read(&e).unwrap();
        assert!(
            !String::from_utf8_lossy(&b).contains(&joined),
            "{}",
            e.display()
        );
    }
    // The words really are the master: they certify another key.
    let m = srv::keys::Master::from_words(&joined).unwrap();
    let o = run(&p, &["setup", "check"]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(
        out.contains("OK   initialized") && out.contains("Dr Example"),
        "{out}"
    );
    // Signed-only from day one: set-mode signed no longer refuses (an app key can sign).
    let o = run(&p, &["auth", "set-mode", "signed"]);
    assert!(o.status.success(), "{o:?}");
    // A second first-run is refused.
    let o = run(&p, &["setup", "first-run", "--no-confirm"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("already initialized"));
    // The registered key is the one passed in, as an owner, certified by that master.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let cfg = srv::config::ServerConfig::from_path(p.clone()).unwrap();
        let pool = srv::db::init_db(&cfg).await.unwrap();
        let k = srv::auth::list_keys(&pool)
            .await
            .unwrap()
            .into_iter()
            .find(|k| k.name == "Dr Example")
            .unwrap();
        assert_eq!(k.role, "owner");
        assert_eq!(
            k.fingerprint,
            srv::keys::fingerprint(&admin.verifying_key())
        );
        let cert = srv::auth::certificate_of(&pool, &k.fingerprint)
            .await
            .unwrap();
        srv::keys::verify_certificate(&m.public().ed25519, &cert).unwrap();
        pool.close().await;
    });
}

fn walk(d: &Path) -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    for e in std::fs::read_dir(d).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            v.extend(walk(&p));
        } else {
            v.push(p);
        }
    }
    v
}
