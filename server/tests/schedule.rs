//! v0.10.0 server-side scheduler (decision 14): ticks queue through the on-demand path,
//! concurrency, missed runs (skip / catch-up), the office-hours guard, groups, RPC roles + audit,
//! and the `schedule` CLI.
use ::itcmdr_server as srv;
use chrono::{TimeZone, Utc};
use prost::Message;
use srv::config::ServerConfig;
use srv::generated::itcmdr::itcmdr_client::ItcmdrClient;
use srv::generated::itcmdr::*;
use srv::schedule::{self, Spec};
use std::path::Path;
use std::time::Duration;
use tonic::{Code, Request};

fn cfg(root: &Path) -> ServerConfig {
    let mut c = ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        logs_dir: root.join("maintenanceRuns"),
        data_dir: Some(root.join("data")),
        ..ServerConfig::default()
    };
    c.backup.nightly = false;
    c
}

fn utc(d: u32, h: u32, m: u32, s: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, d, h, m, s).unwrap()
}

async fn options(pool: &srv::db::DbPool, id: &str) -> serde_json::Value {
    let o: String = sqlx::query_scalar("SELECT options_json FROM requests WHERE request_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    serde_json::from_str(&o).unwrap()
}

async fn outcomes(pool: &srv::db::DbPool, id: &str) -> Vec<String> {
    let mut v: Vec<String> = schedule::history(pool, Some(id), 100)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.outcome)
        .collect();
    v.reverse();
    v
}

#[tokio::test]
async fn tick_fires_queues_and_handles_concurrency_and_missed_runs() {
    let t = tempfile::tempdir().unwrap();
    let c = cfg(t.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    // Outside office hours whenever the test runs.
    srv::settings::set(&pool, "schedule.office_hours", "Sun 00:00-00:01", None)
        .await
        .unwrap();
    let spec = Spec {
        when: "hourly".into(),
        timezone: "UTC".into(),
        machines: vec!["WS-01".into()],
        categories: vec!["network".into()],
        repair: true,
        ..Default::default()
    };
    let s = schedule::create(&pool, "hourly-net", spec, true)
        .await
        .unwrap();
    assert_eq!(s.spec.guard, "report-only-in-hours");
    assert_eq!(s.spec.missed, "skip");
    schedule::set_last_fire(&pool, &s.id, utc(1, 10, 0, 30).timestamp_millis())
        .await
        .unwrap();

    // Nothing due yet.
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 10, 59, 0)).await.unwrap(),
        0
    );
    // Due at 11:00: queued through the on-demand path (request row + agent request file).
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 11, 0, 10)).await.unwrap(),
        1
    );
    let h = schedule::history(&pool, Some("hourly-net"), 10)
        .await
        .unwrap();
    let rid = h[0].request_id.clone().unwrap();
    assert_eq!(h[0].due_ms, utc(1, 11, 0, 0).timestamp_millis());
    let o = options(&pool, &rid).await;
    assert_eq!(o["scheduleId"], s.id.as_str());
    assert_eq!(o["machines"], serde_json::json!(["WS-01"]));
    assert_eq!(
        (o["repair"].as_bool(), o["reportOnly"].as_bool()),
        (Some(true), Some(false))
    );
    assert!(c.requests_dir().join(format!("{rid}.json")).is_file());
    // Same tick again: nothing new.
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 11, 0, 20)).await.unwrap(),
        0
    );

    // 12:00 while the 11:00 request is still queued: skipped, one at a time.
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 12, 0, 5)).await.unwrap(),
        0
    );
    assert_eq!(outcomes(&pool, &s.id).await, ["queued", "skipped"]);
    srv::db::set_request_status(&pool, &rid, "done", "")
        .await
        .unwrap();

    // Server "down" from 12:00 to 15:30: 13, 14, 15 missed and 15:00 is past the grace -> skip.
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 15, 30, 0)).await.unwrap(),
        0
    );
    let h = schedule::history(&pool, Some(&s.id), 1).await.unwrap();
    assert_eq!(h[0].outcome, "missed");
    assert!(h[0].note.contains("3 run(s) missed"), "{}", h[0].note);
    // ...and they are not replayed later.
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 15, 40, 0)).await.unwrap(),
        0
    );

    // catch-up: one run for the lot.
    let mut sp = s.spec.clone();
    sp.missed = "catch-up".into();
    schedule::update(&pool, &s.id, None, sp).await.unwrap();
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 19, 30, 0)).await.unwrap(),
        1
    );
    let h = schedule::history(&pool, Some(&s.id), 2).await.unwrap();
    assert_eq!(
        (h[0].outcome.as_str(), h[1].outcome.as_str()),
        ("queued", "missed")
    );
    assert!(h[0].note.contains("catch-up"), "{}", h[0].note);

    // Paused schedules don't fire; resume doesn't replay the pause.
    schedule::set_enabled(&pool, &s.id, false).await.unwrap();
    let rid2 = h[0].request_id.clone().unwrap();
    srv::db::set_request_status(&pool, &rid2, "done", "")
        .await
        .unwrap();
    assert_eq!(
        schedule::tick(&pool, &c, utc(1, 21, 0, 5)).await.unwrap(),
        0
    );
    let s2 = schedule::set_enabled(&pool, &s.id, true).await.unwrap();
    assert!(s2.last_fire_ms > utc(1, 21, 0, 5).timestamp_millis());
}

#[tokio::test]
async fn office_hours_guard_and_groups() {
    let t = tempfile::tempdir().unwrap();
    let c = cfg(t.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    srv::settings::set(&pool, "schedule.office_hours", "Mon-Sun 00:00-23:59", None)
        .await
        .unwrap();
    for (run, pc, role) in [
        ("r1", "SERVER01", "Server"),
        ("r2", "WS-01", "Workstation"),
        ("r3", "WS-02", "Workstation"),
    ] {
        sqlx::query("INSERT INTO runs(run_id, computer, ts_ms, role) VALUES (?,?,?,?)")
            .bind(run)
            .bind(pc)
            .bind(1_000i64)
            .bind(role)
            .execute(&pool)
            .await
            .unwrap();
    }
    let mk = |name: &str, guard: &str| {
        let pool = pool.clone();
        let (name, guard) = (name.to_string(), guard.to_string());
        async move {
            schedule::create(
                &pool,
                &name,
                Spec {
                    when: "daily 01:00".into(),
                    groups: vec!["workstation".into()],
                    repair: true,
                    guard,
                    ..Default::default()
                },
                true,
            )
            .await
            .unwrap()
        }
    };
    let now = Utc::now().timestamp_millis();
    if (Utc::now().time() - chrono::NaiveTime::from_hms_opt(23, 59, 0).unwrap()).num_seconds() >= 0
    {
        eprintln!("too close to midnight for the guard test: skipped");
        return;
    }
    let ro = mk("ro", "report-only-in-hours").await;
    let (o, rid, note) = schedule::fire(&pool, &c, &ro, now, "scheduled", "scheduler", true)
        .await
        .unwrap();
    assert_eq!(o, "queued");
    assert!(note.contains("report-only: inside office hours"), "{note}");
    let op = options(&pool, &rid.unwrap()).await;
    assert_eq!(op["repair"], false);
    let mut m: Vec<String> = serde_json::from_value(op["machines"].clone()).unwrap();
    m.sort();
    assert_eq!(m, ["WS-01", "WS-02"]);

    let sk = mk("sk", "skip-in-hours").await;
    let (o, rid, _) = schedule::fire(&pool, &c, &sk, now, "scheduled", "scheduler", true)
        .await
        .unwrap();
    assert_eq!((o.as_str(), rid), ("skipped", None));
    // run-now ignores the server guard.
    let (o, ..) = schedule::fire(&pool, &c, &sk, now, "run now", "tester", false)
        .await
        .unwrap();
    assert_eq!(o, "queued");

    let none = mk("none", "none").await;
    let (_, rid, _) = schedule::fire(&pool, &c, &none, now, "scheduled", "scheduler", true)
        .await
        .unwrap();
    assert_eq!(options(&pool, &rid.unwrap()).await["repair"], true);

    // Groups with nobody in them: skipped.
    let empty = schedule::create(
        &pool,
        "rcu",
        Spec {
            when: "hourly".into(),
            groups: vec!["RCU".into()],
            ..Default::default()
        },
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        schedule::fire(&pool, &c, &empty, now, "x", "t", false)
            .await
            .unwrap()
            .0,
        "skipped"
    );

    // Validation.
    for bad in [
        Spec {
            when: "sometimes".into(),
            ..Default::default()
        },
        Spec {
            when: "hourly".into(),
            guard: "maybe".into(),
            ..Default::default()
        },
        Spec {
            when: "hourly".into(),
            missed: "later".into(),
            ..Default::default()
        },
        Spec {
            when: "hourly".into(),
            timezone: "Mars/Olympus".into(),
            ..Default::default()
        },
    ] {
        assert!(schedule::create(&pool, "bad", bad, true).await.is_err());
    }
    assert!(
        schedule::create(
            &pool,
            "ro",
            Spec {
                when: "hourly".into(),
                ..Default::default()
            },
            true
        )
        .await
        .is_err(),
        "duplicate name"
    );

    // Built-in schedules need force to delete.
    sqlx::query(
        "UPDATE schedules SET spec = json_set(spec, '$.builtin', json('true')) WHERE name = 'none'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(schedule::delete(&pool, "none", false).await.is_err());
    schedule::delete(&pool, "none", true).await.unwrap();
}

async fn start(
    c: ServerConfig,
) -> (
    ItcmdrClient<tonic::transport::Channel>,
    tokio::sync::oneshot::Sender<()>,
) {
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
async fn schedule_rpcs_roles_and_audit() {
    let t = tempfile::tempdir().unwrap();
    let c = cfg(t.path());
    let pool = srv::db::init_db(&c).await.unwrap();
    let out = srv::auth::init(&pool, &c.layout()).await.unwrap();
    let m = srv::keys::Master::from_words(&out.words).unwrap();
    let user = srv::keys::new_keypair();
    let cert = srv::keys::certify(
        &m.signing,
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
    let (mut cl, _stop) = start(c.clone()).await;
    const CR: &str = "/itcmdr.Itcmdr/CreateSchedule";
    const UP: &str = "/itcmdr.Itcmdr/UpdateSchedule";
    const LS: &str = "/itcmdr.Itcmdr/ListSchedules";
    const RN: &str = "/itcmdr.Itcmdr/RunScheduleNow";
    const PA: &str = "/itcmdr.Itcmdr/PauseSchedule";
    const DL: &str = "/itcmdr.Itcmdr/DeleteSchedule";
    const HI: &str = "/itcmdr.Itcmdr/ListScheduleRuns";
    let sch = Schedule {
        enabled: true,
        name: "nightly".into(),
        all_machines: true,
        when: "daily 01:30".into(),
        timezone: "America/New_York".into(),
        ..Default::default()
    };
    let save = SaveScheduleRequest {
        schedule: Some(sch.clone()),
    };
    assert_eq!(
        cl.create_schedule(signed(&user, CR, save.clone()))
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );
    let bad = SaveScheduleRequest {
        schedule: Some(Schedule {
            when: "whenever".into(),
            ..sch.clone()
        }),
    };
    assert_eq!(
        cl.create_schedule(signed(&root, CR, bad))
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    let s = cl
        .create_schedule(signed(&root, CR, save))
        .await
        .unwrap()
        .into_inner()
        .schedule
        .unwrap();
    assert!(s.enabled && !s.id.is_empty());
    assert_eq!(s.next_runs.len(), 3);
    assert_eq!(s.guard, "report-only-in-hours");

    let mut upd = s.clone();
    upd.when = "weekdays 06:00".into();
    let r = cl
        .update_schedule(signed(
            &root,
            UP,
            SaveScheduleRequest {
                schedule: Some(upd),
            },
        ))
        .await
        .unwrap()
        .into_inner()
        .schedule
        .unwrap();
    assert_eq!(r.when, "weekdays 06:00");

    let l = cl
        .list_schedules(signed(&user, LS, ListSchedulesRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(l.schedules.len(), 1);
    let rf = ScheduleRef {
        id: "nightly".into(),
        force: false,
    };
    assert_eq!(
        cl.pause_schedule(signed(&user, PA, rf.clone()))
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );
    let p = cl
        .pause_schedule(signed(&root, PA, rf.clone()))
        .await
        .unwrap()
        .into_inner()
        .schedule
        .unwrap();
    assert!(!p.enabled && p.next_runs.is_empty());
    // Users may run a schedule now.
    let n = cl
        .run_schedule_now(signed(&user, RN, rf.clone()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(n.outcome, "queued");
    assert!(!n.request_id.is_empty());
    let h = cl
        .list_schedule_runs(signed(
            &user,
            HI,
            ListScheduleRunsRequest {
                id: "nightly".into(),
                limit: 10,
            },
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(h.runs.len(), 1);
    assert_eq!(h.runs[0].request_id, n.request_id);
    assert_eq!(
        cl.delete_schedule(signed(&user, DL, rf.clone()))
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );
    cl.delete_schedule(signed(&root, DL, rf.clone()))
        .await
        .unwrap();
    assert_eq!(
        cl.run_schedule_now(signed(&root, RN, rf))
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );

    let pool = srv::db::init_db(&c).await.unwrap();
    let acts: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_log WHERE action LIKE 'schedule-%' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for a in [
        "schedule-create",
        "schedule-update",
        "schedule-pause",
        "schedule-fire",
        "schedule-delete",
    ] {
        assert!(acts.contains(&a.to_string()), "{a} in {acts:?}");
    }
}

#[test]
fn cli_schedule_commands() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let cfgp = root.join("server.json");
    std::fs::write(&cfgp, serde_json::to_vec(&cfg(root)).unwrap()).unwrap();
    let cli = |args: &[&str]| {
        let out = assert_cmd::Command::cargo_bin("itcmdr-cli")
            .unwrap()
            .env("ITCMDR_SERVER_CONFIG", &cfgp)
            .env_remove("ITCMDR_DATA_DIR")
            .args(args)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr),
        )
    };
    let (ok, o) = cli(&[
        "schedule",
        "add",
        "--name",
        "nightly",
        "--when",
        "daily 01:30",
        "--all",
        "--categories",
        "network,disk",
    ]);
    assert!(ok, "{o}");
    assert!(
        o.contains("nightly") && o.contains("report-only") && o.contains("all machines"),
        "{o}"
    );
    let (ok, o) = cli(&["schedule", "add", "--name", "bad", "--when", "sometimes"]);
    assert!(!ok, "{o}");
    let (ok, o) = cli(&[
        "schedule",
        "edit",
        "nightly",
        "--when",
        "weekly sat 03:00",
        "--repair",
        "--guard",
        "none",
    ]);
    assert!(
        ok && o.contains("weekly sat 03:00") && o.contains("repair") && o.contains("guard=none"),
        "{o}"
    );
    let (ok, o) = cli(&["schedule", "pause", "nightly"]);
    assert!(ok && o.contains("paused"), "{o}");
    let (ok, o) = cli(&["schedule", "resume", "nightly"]);
    assert!(ok && o.contains("[on]"), "{o}");
    let (ok, o) = cli(&["--json", "schedule", "list"]);
    assert!(ok, "{o}");
    let v: serde_json::Value = serde_json::from_str(o.lines().next().unwrap()).unwrap();
    assert_eq!(
        v["schedule"]["spec"]["categories"],
        serde_json::json!(["network", "disk"])
    );
    assert_eq!(v["next"].as_array().unwrap().len(), 3);
    let (ok, o) = cli(&["schedule", "run-now", "nightly"]);
    assert!(ok && o.starts_with("queued "), "{o}");
    let (ok, o) = cli(&["schedule", "history", "nightly"]);
    assert!(ok && o.contains("nightly") && o.contains("queued"), "{o}");
    let (ok, o) = cli(&["schedule", "rm", "nightly"]);
    assert!(ok && o.contains("deleted nightly"), "{o}");
    let (ok, o) = cli(&["schedule", "list"]);
    assert!(ok && o.trim().is_empty(), "{o}");
}
