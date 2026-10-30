//! v0.11.0: built-in default schedules, parity ingest, the cutover gate, switch and rollback.
use ::itcmdr_server as srv;
use chrono::{Duration, Local, TimeZone, Utc};
use srv::config::ServerConfig;
use srv::cutover::{self, AGENT_KEY, BACKUP_KEY};
use srv::schedule;
use std::path::Path;

fn cfg(root: &Path) -> ServerConfig {
    let mut c = ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        ..ServerConfig::default()
    };
    c.backup.dir = Some(root.join("backups"));
    c
}

#[tokio::test]
async fn seeding_is_idempotent_matches_the_task_and_respects_deletes() {
    let t = tempfile::tempdir().unwrap();
    let c = cfg(t.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::settings::set(&pool, "agent.nightlyTime", "01:45", None)
        .await
        .unwrap();
    srv::settings::set(&pool, "backup.hour", "3", None)
        .await
        .unwrap();
    assert_eq!(cutover::seed_defaults(&pool, &c).await.unwrap().len(), 2);
    assert!(cutover::seed_defaults(&pool, &c).await.unwrap().is_empty());
    assert_eq!(schedule::list(&pool).await.unwrap().len(), 2);

    let a = cutover::find_builtin(&pool, AGENT_KEY)
        .await
        .unwrap()
        .unwrap();
    // Same as the SYSTEM task: its time, all machines, every category, repairs on, guard on.
    assert_eq!(a.spec.when, "daily 01:45");
    assert!(a.spec.all_machines && a.spec.repair && a.spec.builtin);
    assert_eq!(
        a.spec.categories,
        cutover::CATEGORIES.map(String::from).to_vec()
    );
    assert_eq!(a.spec.guard, "report-only-in-hours");
    assert!(
        !a.enabled,
        "the task stays the source of truth until the switch"
    );
    let b = cutover::find_builtin(&pool, BACKUP_KEY)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (b.spec.when.as_str(), b.spec.action.as_str()),
        ("daily 03:00", "backup")
    );
    assert!(b.enabled && b.spec.missed == "catch-up");

    // Editable, but a built-in keeps what it does.
    let mut e = b.spec.clone();
    e.when = "daily 04:00".into();
    e.action = String::new();
    let b2 = schedule::update(&pool, &b.id, None, e).await.unwrap();
    assert_eq!(
        (b2.spec.when.as_str(), b2.spec.action.as_str()),
        ("daily 04:00", "backup")
    );
    assert!(b2.spec.builtin && b2.spec.builtin_key == BACKUP_KEY);

    // Not deletable by accident; a confirmed delete is remembered across upgrades.
    assert!(schedule::delete(&pool, &b.id, false, None).await.is_err());
    assert!(schedule::delete(&pool, &b.id, true, None).await.is_err());
    schedule::delete(&pool, &b.id, true, Some("Nightly backup"))
        .await
        .unwrap();
    assert!(cutover::seed_defaults(&pool, &c).await.unwrap().is_empty());
    assert!(cutover::find_builtin(&pool, BACKUP_KEY)
        .await
        .unwrap()
        .is_none());
    let r = cutover::reset_to_default(&pool, &c, BACKUP_KEY, "t")
        .await
        .unwrap();
    assert_eq!(r.spec.when, "daily 03:00");
    // Reset also restores edited timing.
    let mut e = a.spec.clone();
    e.when = "daily 05:00".into();
    schedule::update(&pool, &a.id, None, e).await.unwrap();
    let a2 = cutover::reset_to_default(&pool, &c, "Nightly agent run", "t")
        .await
        .unwrap();
    assert_eq!(
        (a2.id.as_str(), a2.spec.when.as_str()),
        (a.id.as_str(), "daily 01:45")
    );
    assert!(cutover::reset_to_default(&pool, &c, "nope", "t")
        .await
        .is_err());
}

#[tokio::test]
async fn the_backup_schedule_takes_the_nightly_backup_once_a_day() {
    let t = tempfile::tempdir().unwrap();
    let c = cfg(t.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    cutover::seed_defaults(&pool, &c).await.unwrap();
    let b = cutover::find_builtin(&pool, BACKUP_KEY)
        .await
        .unwrap()
        .unwrap();
    let now = Utc::now().timestamp_millis();
    let (o, _, n) = schedule::fire(&pool, &c, &b, now, "test", "t", true)
        .await
        .unwrap();
    assert_eq!(o, "skipped", "{n}");
    srv::auth::init(&pool, &c.layout()).await.unwrap();
    let (o, req, n) = schedule::fire(&pool, &c, &b, now, "test", "t", true)
        .await
        .unwrap();
    assert_eq!(o, "done", "{n}");
    assert!(req.is_none() && n.contains("written"), "{n}");
    let (o, _, n) = schedule::fire(&pool, &c, &b, now, "test", "t", true)
        .await
        .unwrap();
    assert!(o == "done" && n.contains("exists"), "{n}");
    let files = std::fs::read_dir(t.path().join("backups")).unwrap().count();
    assert_eq!(files, 1);
    // No request files: the backup runs on the server, not on agents.
    assert!(
        !c.requests_dir().exists() || std::fs::read_dir(c.requests_dir()).unwrap().count() == 0
    );
}

fn day_ts(days_ago: i64, hour: u32) -> chrono::DateTime<Local> {
    let d = Local::now().date_naive() - Duration::days(days_ago);
    Local
        .from_local_datetime(&d.and_hms_opt(hour, 30, 0).unwrap())
        .single()
        .unwrap()
}

/// A nightly log with run-start (cutover fields) and a parity line.
fn write_log(dir: &Path, machine: &str, days_ago: i64, mismatched: usize, seen: bool) {
    let ts = day_ts(days_ago, 1);
    let run = format!("{machine}-{days_ago}");
    let ss = if seen {
        format!("\"{}\"", ts.to_rfc3339())
    } else {
        "null".into()
    };
    let mis: Vec<String> = (0..mismatched)
        .map(|i| format!("{{\"checkId\":\"X-{i}\",\"builtin\":\"fail\",\"plugin\":\"ok\"}}"))
        .collect();
    let body = format!(
        "{{\"type\":\"run-start\",\"runId\":\"{run}\",\"computer\":\"{machine}\",\"role\":\"Workstation\",\"reportOnly\":false,\"started\":\"{t}\",\"trigger\":\"task\",\"cutover\":\"task\",\"scheduleStateTs\":{ss},\"pluginMode\":\"parallel\"}}\n\
         {{\"type\":\"check\",\"runId\":\"{run}\",\"computer\":\"{machine}\",\"category\":\"network\",\"checkId\":\"NET-IP-CORRECT\",\"status\":\"ok\",\"ts\":\"{t}\"}}\n\
         {{\"type\":\"parity\",\"runId\":\"{run}\",\"computer\":\"{machine}\",\"ts\":\"{t}\",\"matched\":12,\"mismatched\":[{m}],\"errors\":[],\"plugins\":[\"itcmdr-network@0.11.0 (bundled)\"]}}\n\
         {{\"type\":\"run-complete\",\"runId\":\"{run}\",\"computer\":\"{machine}\",\"finished\":\"{t}\",\"exitCode\":0}}\n",
        t = ts.to_rfc3339(),
        m = mis.join(",")
    );
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("{machine}_{days_ago}_{run}.log")), body).unwrap();
}

#[tokio::test]
async fn the_switch_needs_a_clean_week_the_vm_flag_and_agents_that_see_schedules() {
    let t = tempfile::tempdir().unwrap();
    let c = cfg(t.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    cutover::seed_defaults(&pool, &c).await.unwrap();
    let logs = c.logs_dir.clone();
    // Six clean days only.
    for d in 1..=6 {
        write_log(&logs, "WS-01", d, 0, true);
    }
    srv::ingest::ingest_all(&pool, &logs).await.unwrap();
    let g = cutover::gate(&pool, Utc::now()).await.unwrap();
    assert!(!g.ok);
    assert!(
        g.problems.iter().any(|p| p.contains("no parity results")),
        "{:?}",
        g.problems
    );
    assert!(g.problems.iter().any(|p| p.contains("VM verification")));
    assert!(cutover::switch(&pool, &c, "t", Utc::now()).await.is_err());

    // Day 7 has a mismatch: still blocked.
    write_log(&logs, "WS-01", 7, 2, true);
    srv::ingest::ingest_all(&pool, &logs).await.unwrap();
    cutover::set_vm_verified(&pool, "t", true).await.unwrap();
    let g = cutover::gate(&pool, Utc::now()).await.unwrap();
    assert_eq!(g.problems.len(), 1, "{:?}", g.problems);
    assert!(g.problems[0].contains("2 mismatched"));

    // A second machine that ran without parity and never read schedule-state.json.
    write_log(&logs, "WS-02", 7, 0, false);
    std::fs::write(
        logs.join("WS-02_x.log"),
        format!(
            "{{\"type\":\"run-start\",\"runId\":\"w2\",\"computer\":\"WS-02\",\"started\":\"{}\"}}\n",
            day_ts(1, 2).to_rfc3339()
        ),
    )
    .unwrap();
    // Fix day 7 for WS-01 (re-run replaces the parity row).
    write_log(&logs, "WS-01", 7, 0, true);
    srv::ingest::ingest_all(&pool, &logs).await.unwrap();
    let g = cutover::gate(&pool, Utc::now()).await.unwrap();
    assert!(
        g.problems
            .iter()
            .any(|p| p.contains("WS-02 ran without a parity result")),
        "{:?}",
        g.problems
    );
    assert!(
        g.problems.iter().any(|p| p.contains("WS-02: has not read")),
        "{:?}",
        g.problems
    );

    // Clean: WS-02 gets parity every day it ran and reads the state file.
    for d in 1..=7 {
        write_log(&logs, "WS-02", d, 0, true);
    }
    std::fs::remove_file(logs.join("WS-02_x.log")).unwrap();
    sqlx::query("DELETE FROM runs WHERE run_id = 'w2'")
        .execute(&pool)
        .await
        .unwrap();
    srv::ingest::ingest_all(&pool, &logs).await.unwrap();
    let g = cutover::gate(&pool, Utc::now()).await.unwrap();
    assert!(g.ok, "{:?}", g.problems);
    assert_eq!(g.days.len(), 7);

    // Switch: agent schedule on, state exported for the agents.
    let g = cutover::switch(&pool, &c, "t", Utc::now()).await.unwrap();
    assert_eq!(g.state, "schedule");
    assert!(
        cutover::find_builtin(&pool, AGENT_KEY)
            .await
            .unwrap()
            .unwrap()
            .enabled
    );
    let st: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cutover::state_path(&c)).unwrap()).unwrap();
    assert_eq!(st["cutover"], "schedule");
    assert_eq!(st["nightly"]["enabled"], true);
    assert!(st["nightly"]["next"].is_string());

    // One-command rollback.
    cutover::rollback(&pool, &c, "t").await.unwrap();
    assert_eq!(cutover::state(&pool).await.unwrap(), "task");
    assert!(
        !cutover::find_builtin(&pool, AGENT_KEY)
            .await
            .unwrap()
            .unwrap()
            .enabled
    );
    let st: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cutover::state_path(&c)).unwrap()).unwrap();
    assert_eq!(
        (st["cutover"].as_str(), st["nightly"]["enabled"].as_bool()),
        (Some("task"), Some(false))
    );
    let audit: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_log WHERE action LIKE 'cutover-%' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        audit,
        ["cutover-vm-verified", "cutover-switch", "cutover-rollback"]
    );
}
