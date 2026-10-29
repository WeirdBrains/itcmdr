use itcmdr_server as _; // crate name is itcmdr-server; tests run within this crate path via dev-deps
use std::fs::{create_dir_all, File};
use std::io::Write;

#[tokio::test]
async fn ingest_and_metrics_basic() {
    let tmp = tempfile::tempdir().unwrap();
    let logs = tmp.path().join("maintenanceRuns");
    create_dir_all(&logs).unwrap();

    // Write a minimal run with one failed then ok check
    let run_id = "run-1";
    let computer = "WS-01";
    let mut f = File::create(logs.join(format!("{}_20240101-010000_{}.log", computer, run_id))).unwrap();
    writeln!(f, r#"{{"type":"run-start","runId":"{}","computer":"{}","role":"Workstation","reportOnly":false,"started":"2024-01-01T01:00:00Z"}}"#, run_id, computer).unwrap();
    writeln!(f, r#"{{"type":"check","runId":"{}","computer":"{}","category":"connectivity","checkId":"CHK001","severity":"warning","status":"failed","attemptedRepair":true,"action":"repair","result":"failed","ts":"2024-01-01T01:00:10Z"}}"#, run_id, computer).unwrap();
    writeln!(f, r#"{{"type":"check","runId":"{}","computer":"{}","category":"connectivity","checkId":"CHK001","severity":"info","status":"ok","attemptedRepair":false,"action":"","result":"success","ts":"2024-01-01T01:02:00Z"}}"#, run_id, computer).unwrap();
    writeln!(f, r#"{{"type":"run-complete","runId":"{}","computer":"{}","finished":"2024-01-01T01:05:00Z","exitCode":0}}"#, run_id, computer).unwrap();

    let cfg = itcmdr_server::config::ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: logs.clone(),
        database_path: tmp.path().join("db.sqlite"),
        requests_dir: None,
        admin_token_hash: None,
        retention_days: 400,
        metrics: itcmdr_server::config::MetricsConfig::default(),
    };
    let pool = itcmdr_server::db::init_db(&cfg).await.unwrap();
    itcmdr_server::ingest::ingest_all(&pool, &logs).await.unwrap();

    let pts = itcmdr_server::metrics::get_metrics(&pool, 7, itcmdr_server::generated::itcmdr::Rollup::Daily).await.unwrap();
    assert!(pts.iter().any(|p| p.key.starts_with("incidents:WS-01")));
}

