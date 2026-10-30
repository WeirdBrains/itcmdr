//! itcmdr-restore: the out-of-process restore helper started by `StageRestore` (decision 13).
//! Reads the 24 words from stdin (never from disk), then stops the service, restores, restarts,
//! health-checks and rolls back on failure. Progress: `<data>/restore/status.json`.
use ::itcmdr_server as srv;
use std::io::BufRead;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (job, data_dir) = srv::restore::parse_args(&args)?;
    let mut words = zeroize::Zeroizing::new(String::new());
    std::io::stdin().lock().read_line(&mut words)?;
    let mut cfg = srv::config::ServerConfig::load()?;
    cfg.data_dir_flag = data_dir;
    srv::db::track_pools();
    let rt = tokio::runtime::Runtime::new()?;
    let r = rt.block_on(srv::restore::run_helper(&cfg, &job, &words));
    rt.block_on(srv::db::close_tracked());
    let st = r?;
    println!("{}: {}", st.state, st.message);
    if st.state == "done" {
        Ok(())
    } else {
        std::process::exit(1)
    }
}
