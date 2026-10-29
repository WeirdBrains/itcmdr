use anyhow::Result;
use clap::Parser;
use chrono::{Duration, Utc};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::Serialize;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "itcmdr-sample-data-gen", about = "Generate demo JSONL agent logs")]
struct Args {
    /// Output folder for JSONL files (one per run)
    #[arg(long, default_value = "./sample-maintenanceRuns")]
    out_dir: PathBuf,
    /// Number of days to simulate
    #[arg(long, default_value_t = 7)]
    days: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    run_id: String,
    computer: String,
    category: String,
    check_id: String,
    severity: String,
    status: String,
    detail: String,
    action: String,
    result: String,
    duration_ms: u64,
    ts_ms: i64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    create_dir_all(&args.out_dir)?;
    let mut rng = StdRng::seed_from_u64(42);
    let machines = [
        "SERVER01",
        "RCU-01",
        "WS-01",
        "WS-02",
        "WS-03",
        "PRN-01",
        "PRN-02",
    ];
    let now = Utc::now();
    for d in (0..args.days).rev() {
        for m in machines {
            let run_id = Uuid::new_v4().to_string();
            let ts = now - Duration::days(d);
            let mut f = File::create(args.out_dir.join(format!("{}_{}.jsonl", m, run_id)))?;
            // Emit 5-10 records
            for i in 0..rng.gen_range(5..=10) {
                let rec = Record {
                    run_id: run_id.clone(),
                    computer: m.to_string(),
                    category: if i % 2 == 0 { "connectivity" } else { "configuration" }.into(),
                    check_id: format!("CHK{:03}", i),
                    severity: if i % 3 == 0 { "warning" } else { "info" }.into(),
                    status: if i % 4 == 0 { "failed" } else { "ok" }.into(),
                    detail: "simulated".into(),
                    action: if i % 4 == 0 { "retry" } else { "none" }.into(),
                    result: if i % 4 == 0 { "success" } else { "skipped" }.into(),
                    duration_ms: rng.gen_range(50..5000),
                    ts_ms: (ts - Duration::minutes(i * 3)).timestamp_millis(),
                };
                writeln!(f, "{}", serde_json::to_string(&rec)?)?;
            }
        }
    }
    println!("Wrote sample logs to {}", args.out_dir.display());
    Ok(())
}

