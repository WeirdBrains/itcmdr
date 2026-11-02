//! itcmdr-agent-watchdog (v0.21.0): agent upgrades, rollback and restarts. See itcmdr_watchdog.
fn main() {
    itcmdr_watchdog::runner::main(itcmdr_watchdog::runner::Kind::Agent);
}
