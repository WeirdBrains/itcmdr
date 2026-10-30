use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "itcmdr-sample-data-gen", about = "Generate demo agent logs (JSON Lines)")]
struct Args {
    /// Output folder (point the server's logs_dir here)
    #[arg(long, default_value = "./sample-maintenanceRuns")]
    out_dir: PathBuf,
    /// Number of nights to simulate
    #[arg(long, default_value_t = 90)]
    days: i64,
    /// RNG seed (same seed = same data shape)
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

fn main() -> anyhow::Result<()> {
    let a = Args::parse();
    let n = itcmdr_sample_data_gen::generate(&a.out_dir, a.days, a.seed, chrono::Utc::now())?;
    println!("Wrote {n} run logs to {}", a.out_dir.display());
    Ok(())
}
