//! itcmdr-server-watchdog (v0.21.0): server upgrades, rollback and restarts. See itcmdr_watchdog.
fn main() {
    itcmdr_watchdog::runner::main(itcmdr_watchdog::runner::Kind::Server);
}
