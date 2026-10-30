use itcmdr_server::config::ServerConfig;
use itcmdr_server::generated::itcmdr::Rollup;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

fn cfg(root: &Path) -> ServerConfig {
    ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        database_path: root.join("db.sqlite"),
        ..ServerConfig::default()
    }
}

async fn count(pool: &itcmdr_server::db::DbPool, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await.unwrap()
}

fn write_run(dir: &Path, run_id: &str, day: &str, status: &str) {
    let mut f = File::create(dir.join(format!("WS-01_{}-010000_{}.log", day.replace('-', ""), run_id))).unwrap();
    writeln!(f, r#"{{"type":"run-start","runId":"{run_id}","computer":"WS-01","role":"Workstation","reportOnly":false,"started":"{day}T01:00:00.0000000-04:00"}}"#).unwrap();
    writeln!(f, r#"{{"type":"check","runId":"{run_id}","computer":"WS-01","category":"connectivity","checkId":"CHK001","severity":"warning","status":"{status}","attemptedRepair":true,"action":"repair","result":"failed","ts":"{day}T01:00:10.5000000-04:00"}}"#).unwrap();
    writeln!(f, r#"{{"type":"run-complete","runId":"{run_id}","computer":"WS-01","finished":"{day}T01:05:00-04:00","exitCode":0}}"#).unwrap();
}

#[tokio::test]
async fn uses_record_timestamps_and_reingest_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    create_dir_all(&c.logs_dir).unwrap();
    let today = chrono::Utc::now().date_naive();
    let d1 = (today - chrono::Duration::days(3)).to_string();
    let d2 = (today - chrono::Duration::days(1)).to_string();
    write_run(&c.logs_dir, "run-1", &d1, "failed");
    write_run(&c.logs_dir, "run-2", &d2, "ok");

    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    let s = itcmdr_server::ingest::ingest_all(&pool, &c.logs_dir).await.unwrap();
    assert_eq!((s.files, s.new_runs, s.new_checks), (2, 2, 2));

    // Timestamps come from the records, not ingest time.
    let ts: i64 = sqlx::query_scalar("SELECT ts_ms FROM runs WHERE run_id='run-1'").fetch_one(&pool).await.unwrap();
    let want = chrono::DateTime::parse_from_rfc3339(&format!("{d1}T01:00:00-04:00")).unwrap().timestamp_millis();
    assert_eq!(ts, want);
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM runs WHERE finished_ts_ms IS NOT NULL AND exit_code=0").await, 2);
    // run-1 failed -> incident opened; run-2 ok -> closed with MTTR = 2 days
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM incidents WHERE is_open=0 AND closed_ts_ms > opened_ts_ms").await, 1);

    // Unchanged files are skipped.
    let s2 = itcmdr_server::ingest::ingest_all(&pool, &c.logs_dir).await.unwrap();
    assert_eq!(s2.files, 0);

    // Same logs again inside a weekly zip nested in a block zip: nothing double counts.
    let weekly = {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut z = zip::ZipWriter::new(&mut buf);
        for e in std::fs::read_dir(&c.logs_dir).unwrap() {
            let p = e.unwrap().path();
            z.start_file(p.file_name().unwrap().to_string_lossy(), zip::write::FileOptions::default()).unwrap();
            z.write_all(&std::fs::read(&p).unwrap()).unwrap();
        }
        z.finish().unwrap();
        drop(z);
        buf.into_inner()
    };
    let arch = c.logs_dir.join("archive").join("blocks");
    create_dir_all(&arch).unwrap();
    {
        let f = File::create(arch.join("itcmdr-2026-B11.zip")).unwrap();
        let mut z = zip::ZipWriter::new(f);
        z.start_file("itcmdr-2026-W44.zip", zip::write::FileOptions::default()).unwrap();
        z.write_all(&weekly).unwrap();
        z.finish().unwrap();
    }
    let s3 = itcmdr_server::ingest::ingest_all(&pool, &c.logs_dir).await.unwrap();
    assert_eq!((s3.files, s3.new_runs, s3.new_checks), (1, 0, 0));
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM checks").await, 2);
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM incidents").await, 1);
    assert_eq!(count(&pool, "SELECT SUM(runs) FROM rollup_daily").await, 2);

    let pts = itcmdr_server::metrics::get_metrics(&pool, 7, Rollup::Daily).await.unwrap();
    let inc: Vec<f64> = pts.iter().filter(|p| p.key == "incidents:WS-01").map(|p| p.value).collect();
    assert_eq!(inc.iter().sum::<f64>(), 1.0);
    assert!(pts.iter().any(|p| p.key == "repair_failures:WS-01:CHK001" && p.value == 2.0));
}

#[tokio::test]
async fn sample_data_gives_non_flat_metrics() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let files = itcmdr_sample_data_gen::generate(&c.logs_dir, 60, 7, chrono::Utc::now()).unwrap();
    assert!(files > 300);
    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    let s = itcmdr_server::ingest::ingest_all(&pool, &c.logs_dir).await.unwrap();
    assert_eq!(s.new_runs, files);

    let pts = itcmdr_server::metrics::get_metrics(&pool, 60, Rollup::Weekly).await.unwrap();
    let weekly: Vec<f64> = pts.iter().filter(|p| p.key == "incidents:WS-04").map(|p| p.value).collect();
    assert!(weekly.len() >= 8, "expected 8+ weekly buckets, got {}", weekly.len());
    let (min, max) = weekly.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    assert!(max > min, "weekly incidents should vary: {weekly:?}");
    // Flaky machine ranks above the stable one.
    let total = |m: &str| pts.iter().find(|p| p.key == format!("incidents_total:{m}")).map(|p| p.value).unwrap_or(0.0);
    assert!(total("WS-04") > total("SERVER01"));
    // Some nights were missed and some repairs failed.
    assert!(pts.iter().any(|p| p.key.starts_with("missed_runs:") && p.value > 0.0));
    assert!(pts.iter().any(|p| p.key.starts_with("repair_failures:")));
    let up: Vec<f64> = pts.iter().filter(|p| p.key.starts_with("uptime:")).map(|p| p.value).collect();
    assert!(up.iter().any(|v| *v < 1.0) && up.contains(&1.0));
}

#[tokio::test]
async fn request_status_follows_response_files() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    itcmdr_server::db::insert_request(&pool, "r1", r#"{"machines":["WS-01","WS-02"]}"#).await.unwrap();
    let resp = c.responses_dir();
    create_dir_all(&resp).unwrap();
    let w = |m: &str, s: &str| std::fs::write(resp.join(format!("r1.{m}.json")), format!("\u{feff}{{\"requestId\":\"r1\",\"machine\":\"{m}\",\"status\":\"{s}\"}}")).unwrap();
    let st = |p| async move { itcmdr_server::db::get_request_status(p, "r1").await.unwrap().0 };
    assert_eq!(st(&pool).await, "queued");
    w("WS-01", "running");
    itcmdr_server::requests::sync_requests(&pool, &resp).await.unwrap();
    assert_eq!(st(&pool).await, "running");
    w("WS-01", "done");
    w("WS-02", "done");
    itcmdr_server::requests::sync_requests(&pool, &resp).await.unwrap();
    assert_eq!(st(&pool).await, "done");
}

#[tokio::test]
async fn exceptions_export_matches_agent_format() {
    let tmp = tempfile::tempdir().unwrap();
    let c = cfg(tmp.path());
    let pool = itcmdr_server::db::init_db(&c).await.unwrap();
    let doc = itcmdr_server::exceptions::ExceptionDoc {
        id: "x1".into(),
        scope: itcmdr_server::exceptions::ScopeDoc { machines: vec!["RCU-01".into()], role: None },
        check_id: "NET-SERVER-TCP-445".into(),
        action: "ignore".into(),
        reason: "test".into(),
        ..Default::default()
    };
    itcmdr_server::db::save_exception(&pool, "x1", &serde_json::to_string(&doc).unwrap(), true).await.unwrap();
    itcmdr_server::db::save_exception(&pool, "x2", &serde_json::to_string(&doc).unwrap(), true).await.unwrap();
    itcmdr_server::db::expire_exception(&pool, "x2").await.unwrap();
    let path = c.exceptions_export_path();
    assert_eq!(itcmdr_server::exceptions::export(&pool, &path).await.unwrap(), 1);
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let r = &v["rules"][0];
    assert_eq!(r["checkId"], "NET-SERVER-TCP-445");
    assert_eq!(r["scope"]["machines"][0], "RCU-01");
    assert_eq!(r["action"], "ignore");
}
