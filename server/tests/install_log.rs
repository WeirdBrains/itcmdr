//! v1.2.1: a failed Windows installer run (last-server.json) opens a Critical alert once; a later
//! good run resolves it; `setup check` shows the last result.
use ::itcmdr_server as srv;
use srv::config::ServerConfig;
use srv::install_log::{dir, latest, report};

fn cfg(root: &std::path::Path) -> ServerConfig {
    ServerConfig {
        logs_dir: root.join("logs"),
        data_dir: Some(root.join("data")),
        ..ServerConfig::default()
    }
}

fn open_installer_alerts(a: &[srv::alerts::Alert]) -> Vec<&srv::alerts::Alert> {
    a.iter().filter(|a| a.key == "installer-server").collect()
}

#[tokio::test]
async fn failed_then_good_install() {
    let t = tempfile::tempdir().unwrap();
    let c = cfg(t.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let data = c.layout().root;
    let d = dir(&data);
    std::fs::create_dir_all(&d).unwrap();
    let w = |ok: bool| {
        std::fs::write(
            d.join("last-server.json"),
            format!(r#"{{"part":"server","from":"1.2.0","to":"1.2.1","ok":{ok},"step":"file locks","error":"server\\itcmdr-server.exe is locked","exit":"7","log":"C:\\ProgramData\\itcmdr\\install-logs\\server-1.2.1-x.log","time":"t"}}"#),
        )
        .unwrap()
    };
    let f = srv::alerts::Filter {
        state: Some("open".into()),
        limit: 100,
        ..Default::default()
    };
    w(false);
    assert_eq!(report(&pool, &data).await.unwrap(), 1);
    assert_eq!(report(&pool, &data).await.unwrap(), 0, "reported once");
    let all = srv::alerts::list(&pool, &f).await.unwrap();
    let a = open_installer_alerts(&all);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].level, "critical");
    assert!(
        a[0].title.contains("failed at file locks; still on 1.2.0"),
        "{}",
        a[0].title
    );
    assert!(a[0].detail.contains("exit code 7") && a[0].detail.contains("server-1.2.1-x.log"));
    assert!(!latest(&data, "server").unwrap().ok);
    let chk = srv::setup::check_paths(&c);
    assert!(chk
        .iter()
        .any(|x| x.name == "install.server" && x.level == srv::setup::Level::Fail));
    w(true);
    report(&pool, &data).await.unwrap();
    let all = srv::alerts::list(&pool, &f).await.unwrap();
    assert!(
        open_installer_alerts(&all).is_empty(),
        "resolved by a good install"
    );
    let chk = srv::setup::check_paths(&c);
    assert!(chk
        .iter()
        .any(|x| x.name == "install.server" && x.level == srv::setup::Level::Ok));
}
