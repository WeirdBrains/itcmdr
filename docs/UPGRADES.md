# Upgrades and watchdogs

The agents and the site server upgrade themselves from owner-signed release packages
(`itcmdr-agent-release`, `itcmdr-server-release`), with a watchdog that rolls back a bad release.

## Update window

`updates.auto` (default true) and `updates.window` (default `daily 01:00-04:00` in `updates.timezone`,
default the server's zone). The window holds the server upgrade, agent offers and ring advances. A range
like `Mon-Fri 22:00-02:00` belongs to the day it starts. A bad window fails closed (nothing installs).
Change it with `itcmdr-cli config set updates.window "Sat,Sun 02:00-05:00"`, the site app's Updates page,
or `itcmdr-admin config set <site> updates.window ...`. `agent.upgrades = false` holds one machine back.

## Agents: rings

    itcmdr-cli plugins install --path itcmdr-agent-release-<ver>.tar.gz
    itcmdr-cli upgrade start <ver> [--now]       # ring 0 first; --now ignores the window
    itcmdr-cli upgrade status | pause | resume | abort

A release is a service-plugin package with a `release.toml` (`version`, `min_from`), signed by an owner
key; it is never pushed as a plugin. Rings come from `agent.ring` (0 = a test PC, 1 = one PC,
2 = everything). When every agent of a ring runs the new version with fresh heartbeats, the ring soaks
for `agent.upgradeSoakMins` and the next ring starts. A failed apply, a watchdog rollback or a new
Critical alert on a ring machine pauses the rollout and raises a Critical alert.

## The site server

    itcmdr-cli upgrade server-start <ver> [--now]
    itcmdr-cli upgrade server-abort              # while it still waits for the window

The server takes a database safety copy and stages the new files; `itcmdr-server-watchdog` swaps
them in and health-checks. On failure it rolls back and restores the database
(`itcmdr-cli upgrade restore-db`) and opens a Critical `server-upgrade` alert. Agent releases that
need a newer server (`min_server`) wait for it.

## Watchdogs

`itcmdr-agent-watchdog` and `itcmdr-server-watchdog` restart their service when its health file is
older than `--stale-secs` (default 300) and perform the swap and rollback. `--once` runs one pass.

## From adminsvr

Approve a release once and roll it out to the sites you choose: `itcmdr-admin release approve|rollout`
([adminsvr](ADMINSVR.md)). The site must allow `release.rollout` (`itcmdr-cli adminsvr allow ...`).

## Installer logs (v1.2.1)
Every Windows installer run (new install or upgrade, interactive or `/VERYSILENT`) writes a log, also when it fails:
- Server, agent, office app, CLI, admin CLI and admin app: `%ProgramData%\itcmdr\install-logs\<part>-<version>-<yyyymmdd-hhnnss>.log`.
- adminsvr: `%ProgramData%\itcmdr-adminsvr\install-logs\`.
- Pruned (v1.3.0): each run keeps the newest 20 logs of its own part (by file time, the current run included) and deletes older ones; `last-<part>.json`, `last-<part>.reported.json` and other parts' logs are never touched. The log says `pruned N old install log(s)`.

The log has one timestamped line per step: the version found (fresh install or "upgrade from X"), each service stop and the wait for STOPPED, each process exit (forced with `taskkill /F` after 30 s), a lock probe of every program file (30 tries, 2 s apart, each retry logged), files replaced, each service start and the wait for RUNNING (one `sc start` retry), the result and the exit code. Inno Setup's own log is appended under `---- Inno Setup log ----`. The command line is not logged (it may hold an enrollment token). Old logs are not pruned.

A failed run is loud:
- Exit code **7**: failed before any file was changed (a service did not stop, a file stayed locked). Exit code **8**: files installed, but a service that was running before did not start. Otherwise Inno Setup's own exit code.
- The services that were running are started again (on the old version when nothing was replaced).
- An Application event log entry, source `itcmdr-setup`, id 100.
- `last-<part>.json` next to the logs: `part`, `from`, `to`, `ok`, `step`, `error`, `exit`, `log`, `time`.
- The site server reads `last-server.json` on its next self-upgrade tick and opens the Critical alert `installer-server` (a good run resolves it); each agent reads its own `last-agent.json` at start and every minute and sends `installer-agent`. The file is then renamed to `last-<part>.reported.json`.
- `itcmdr-cli setup check` shows `install.server` and `install.agent` (Ok, or Fail with the log path).

Testing the failure path: `/SIMULATEFAIL=lock` fails at the lock probe (exit 7, nothing changed); `/SIMULATEFAIL=start` fails the service start check (exit 8). Exit 9 (v1.3.4) means the files were installed but `/EnrollToken=` enroll or re-enroll failed (`agent.json` was rolled back, services restarted, `last-agent.json` records `step` `enroll`).
