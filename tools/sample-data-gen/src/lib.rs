//! Demo data in the exact agent log format (run-start / check / run-complete JSON Lines,
//! one file per run named `<COMPUTER>_<yyyyMMdd-HHmmss>_<runId>.log`). Only neutral
//! placeholder machine names are used.
use chrono::{DateTime, Duration, Utc};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde_json::json;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

pub const MACHINES: [(&str, &str, f64); 6] = [
    // name, role, failure probability per check
    ("SERVER01", "Server", 0.02),
    ("RCU-01", "RCU", 0.10),
    ("WS-01", "Workstation", 0.03),
    ("WS-02", "Workstation", 0.15),
    ("WS-03", "Workstation", 0.05),
    ("WS-04", "Workstation", 0.30),
];

pub const CHECKS: [(&str, &str); 6] = [
    ("NET-IP-CORRECT", "network"),
    ("NET-SERVER-TCP-445", "connectivity"),
    ("NET-SERVER-TCP-53", "connectivity"),
    ("SHARE-PDATA", "connectivity"),
    ("PRN-PRN-01-9100", "connectivity"),
    ("SIRONA-PLUGIN-HEALTH", "services"),
];

/// Write `days` nightly runs per machine ending at `end`. Returns the number of files.
pub fn generate(out_dir: &Path, days: i64, seed: u64, end: DateTime<Utc>) -> anyhow::Result<usize> {
    create_dir_all(out_dir)?;
    let mut rng = StdRng::seed_from_u64(seed);
    let mut files = 0;
    for d in (0..days).rev() {
        for (m, role, p_fail) in MACHINES {
            // ~4% of nights a machine is off and misses its run
            if rng.gen_bool(0.04) {
                continue;
            }
            let start = end - Duration::days(d) - Duration::minutes(rng.gen_range(0..30));
            let run_id = uuid::Uuid::new_v4().to_string();
            let name = format!("{}_{}_{}.log", m, start.format("%Y%m%d-%H%M%S"), run_id);
            let mut f = File::create(out_dir.join(name))?;
            writeln!(
                f,
                "{}",
                json!({"type":"run-start","runId":run_id,"computer":m,"role":role,"reportOnly":false,"started":start.to_rfc3339(),"agentVersion":"sample"})
            )?;
            let mut t = start;
            for (check, cat) in CHECKS {
                t += Duration::seconds(rng.gen_range(1..20));
                let broken = rng.gen_bool(p_fail);
                let (status, attempted, result) = if !broken {
                    ("ok", false, "skipped")
                } else if rng.gen_bool(0.6) {
                    ("ok", true, "success")
                } else {
                    ("failed", true, "failed")
                };
                writeln!(
                    f,
                    "{}",
                    json!({"type":"check","runId":run_id,"computer":m,"category":cat,"checkId":check,
                        "severity": if status == "failed" {"warning"} else {"info"},"status":status,"detail":"simulated",
                        "attemptedRepair":attempted,"action": if attempted {"repair"} else {""},"result":result,
                        "durationMs": rng.gen_range(50..5000),"ts":t.to_rfc3339(),"excepted":false,"exceptionReason":null})
                )?;
            }
            writeln!(
                f,
                "{}",
                json!({"type":"run-complete","runId":run_id,"computer":m,"finished":(t + Duration::seconds(2)).to_rfc3339(),"exitCode":0})
            )?;
            files += 1;
        }
    }
    Ok(files)
}
