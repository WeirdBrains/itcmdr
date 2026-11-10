# itcmdr handover notes

Keep this file current: update it at the start of each work item and in every commit. Phase statuses live in the root `PLAN.md` (update it in every commit too).
Placeholders only (SERVER01, WS-01, RCU-01, 198.51.100.x). Real site notes go in the
untracked mirror `HANDOVER.local.md` in the repo root (gitignored, never committed).

## Current state (updated 2026-11-02)

- **v1.1.0 released 2026-11-04 (tag v1.1.0):** Let's Encrypt for adminsvr (off by default, see docs/ADMINSVR.md "HTTPS certificates and Let's Encrypt"), HTTPS with certificate checks and per-server trust in both apps and the CLIs, site server API on HTTPS. See PLAN.md and docs/historical/1.0/SERVER-PLAN.md v1.1.0.
- Branch: `main` only (no PRs, no feature branches). Latest tag: **v1.5.0** (Fixed all remaining bugs). Previous: **v1.4.0** (plugin tags and supported platforms, mock built-in plugins, TLS startup race fix and release tooling). Previous: **v1.3.8** (Windows VM upgrade smoke `e2e/v137.ps1` in the repo; no runtime change since 1.3.7). Previous: **v1.3.7** (the site server starts `itcmdr-server-watchdog` when it is stopped; installers set `failureflag` 1 on the server, agent and both watchdogs). Previous: **v1.3.6** (`/EnrollToken=` on an already-enrolled PC verifies the link with a shared read of `agent.log`; 1.3.5 could falsely roll back). Previous: **v1.3.5** (1.3.4 plus the Unicode Inno Setup type fix so the agent installer compiles). Previous: **v1.3.4** (Windows agent installer re-enrolls when `/EnrollToken=` is given; a failed enroll exits 9 and rolls `agent.json` back). Previous: **v1.3.3** (Info, Check and revoked-agent handling; older unsigned `itcmdr-admin info` shows an empty version; agents before 1.3.3 are not told they were revoked, but reconnects are still refused). Previous: **v1.3.2** (Windows parts run without the Visual C++ runtime: static CRT for Rust, runtime DLLs next to the Flutter apps; demo workflow scripts/demo.sh). Previous: **v1.3.1** (release guard: every asset built from the tag, build stamp with commit and help hash checked by `release-local.sh check`/`publish` and `scripts/release-windows.ps1`; see docs/RELEASE-LOCAL.md). Previous: **v1.3.0** (installer logs pruned to the newest 20 per part; admin app Deep backup checks screen; VM end-to-end check with real breaks, 48/0). Previous: **v1.2.1** (installer logs, loud failed upgrades). Previous: **v1.2.0** (adminsvr's own backup, real app ids). Previous: **v1.1.0** (HTTPS everywhere, Let's Encrypt for adminsvr, docs as in-app help). Previous: **v1.0.0** (1.0: fresh install with no share tested on the VM, README and INSTALL for 1.0, versions 1.0.0; deep-check reminder: the site keeps the date and result of the last `backup drill --deep`, sends only those to adminsvr, and the site app, `itcmdr-admin site list` and the admin app warn when never run, failed or older than the org limit (`org policy --deep-check-days`, default 90); v1.0.0 tests: Linux fmt/clippy clean, Rust 190 (SQLite+Postgres), Flutter 43 (+1 skipped)/26, Pester 105 (10 skipped); site branch updated with 1.0 links (not live); launch drafts in the site branch drafts/, never posted). Previous: v0.22.0 (share flows retired after the agent cutover: no schedule-state/agent-settings rewrites, on-demand runs as RunNow `*` for 0.22+ agents and per plugin for older ones, poller skips `requests\`; on-site restore drill `backup drill [--deep]` + adminsvr `backup.drill` (ok/failed only) + monthly runbook in docs/BACKUP.md; fix for a server-upgrade rollback loop). v0.22.0 tests: Linux fmt/clippy clean, Rust 189+ (SQLite+Postgres), Flutter 41 (+1 skipped)/26, Pester 105 (10 skipped); Windows VM v22a/v22b: 0.21.0 install, agent pushed to 0.22.0-pre1 with the watchdog result reported 6 s after the new agent linked, rollout done, real server self-upgrade 0.21.0 -> 0.22.0-pre1, broken server release rolled back with the database restored, Critical alert and no re-staging (14/14 after the fix); VM restored from C:\e2e\v22-orig. Previous: v0.21.0 (agent and server self-upgrade with watchdogs and rollback; update window `updates.*` with push-now; approve-once releases from adminsvr (`release.rollout`, migration 000005); two-person approval as an org setting, off by default). v0.21.0 tests: Linux fmt/clippy clean, Rust 188 (SQLite+Postgres), Flutter 41 (+1 skipped)/26, Pester 104 (10 skipped); Windows VM v21a break tests on pre-releases: window holds, push installs at once, good upgrade, two broken releases rolled back, killed/stopped watchdog, stale agent, stopped server (25/26 checks; the 26th found the result-report race fixed in this release); VM restored from C:\e2e\v21-orig. Previous: v0.20.1 (agent ConfigUpdate over the link, gated `cutover switch --to agent` with rollback, nightly-task self-heal by the Rust agent and the PowerShell poller, idempotent switch). v0.20.1 tests: Linux fmt/clippy clean, Rust 166 (SQLite+Postgres), Flutter 38 (+1 skipped)/24, Pester 104 (10 skipped); Windows VM install test on a fresh test DB: config delivered, offline agent refused, real switch accepted (repeat is a no-op), real runs, re-enabled task disabled again with `nightly-task-reenabled`, tampered config.json rejected/alerted/resent without re-enabling the task, rollback re-enables the task and resolves the alert; the VM's original data restored afterwards. v0.21.0 details: docs/historical/1.0/SERVER-PLAN.md "v0.21.0". Next: v0.22.0, v1.0.0 (approved 2026-11-01). Previous: v0.20.0 (commands down with dry run, two-person approval, freeze, plugin catalog + FetchPackage, config/plugin documents, CLI + app UI; Flutter apps on FRB native assets + Swift Package Manager, no CocoaPods) (itcmdr Admin app in admin-app/ (Flutter + Rust bridge over admin-cli); FCM/APNs push with redacted payloads (stub-tested; accounts pending); metrics history; SysV/OpenRC/launchd/Windows service packaging; podman-tested container). Previous: v0.19.1 (adminsvr store on Diesel with SQLite (default) or Postgres; passkeys/security keys as an optional second factor with request-bound step-up, owner policy, reset, CLI via libfido2 tools, hmac-secret identity lock; real-key path untested). Previous: v0.19.0 (adminsvr core: `itcmdr-adminsvr` on Postgres with signed-request auth, owner/admin/sre RBAC, hash-chained audit, encrypted key backup on by default; `itcmdr-admin` CLI; site SiteLink `server/src/sitelink.rs` with deny-by-default redaction, dual-signed allowlisted commands `backup.start`/`alert.ack`/`alert.resolve`, no restore, kill switch `adminsvr.enabled`; see docs/ADMINSVR.md. adminsvr tests start a user-local Postgres from `ITCMDR_PG_BIN` (default ~/Projects/tools/pgsql/bin) or use `ITCMDR_TEST_PG_URL`. SRE app, push and a tested container image are v0.19.2). Previous: v0.18.0 (Rust agent runs plugins and schedules, report-only: `agent.plugins` selection pushed with a root-key endorsement and checked file checksums, PluginRemove, ScheduleSet fired by the agent also offline, RunNow `itcmdr-cli agent run/runs/plugins`, RunReports through the durable queue with artifacts, per-machine plugin logs and parity `agent:<run>` vs nightly checks, `agent-plugin:`/`agent-run:` alerts; `Prepare` RPC (refused root key leaves no enrollment), `Check` RPC (revoked agent backs off to 15 min); agent package ships itcmdr-plugin-ps.exe; schema 9; settings 49; versions 0.18.0, pubspec +30). v0.18.0 tests: Linux Rust 138, Flutter 38 (+1 skipped); Windows VM Rust 137, Pester 102, clippy clean; VM install test (v180s/v180f): plugin pushed and applied, RunNow report in 4 s with parity, schedule fired while the server was stopped and delivered after restart, a tampered server copy refused (checksum mismatch, alert raised then resolved), removal via `agent.plugins none`, tampered root leaves no PHCHECK row and the token still works, revoked agent logs refusal and backs off. v0.17.1: (agent certificates renewed over the link at 2/3 of `agent.certLifetimeMins` with a 24 h overlap and `agent-cert:<machine>` expiry alerts, `itcmdr-cli agent renew`; revoked/replaced client certificates refused in the TLS handshake by a serial allow-list in a custom acceptor; site CA signed by the root key, root public key in the token and verified by the agent; `agent.officeHours`; offline alert queue `itcmdr-agent alert` with durable seq, server dedupe and acks, `agent.queueMaxMib`; schema 8; versions 0.17.1, pubspec +29). v0.17.1 tests: Linux Rust 134, Flutter 38 (+1 skipped), Pester 102/112; Windows VM Rust 133 (restore_app rollback test hit a transient Access denied once, passed on rerun), Pester 102, clippy clean; VM install test (v171s): upgrade, CA signature file + root in token, tampered root refused, 60-min cert gives a Critical expiry alert, `agent renew` renews in ~5 s and resolves it, a revoked serial is refused in the TLS handshake (server log `handshake refused ... Revoked`), office hours keep a stopped agent at late with no alert outside the hours and alert inside, alerts queued while the server was stopped are delivered once after it starts and resolved via `--resolve`. v0.17.0: (Rust `itcmdr-agent` in the agent package: enrollment tokens, pinned TLS + mTLS client certs from a site CA, signed Hello, heartbeats + non-responsive alerts, maintenance, revoke, `itcmdr-cli agent`, app Agents page; schema 7; docs/AGENT.md; versions server/keys/app/installer/AgentVersion/agent 0.17.0). v0.17.0 tests: Linux Rust 129, Flutter 38 (+1 skipped), Pester 102/112 (fault suite skipped); Windows VM Rust 128, Pester 102, clippy clean; VM install test: server upgrade opens 50052 + firewall rule, agent package with /EnrollToken= enrolls the service in ~10 s and deletes the token, a wrong pin is refused, stopping the service gives late then non_responsive with a Critical alert, starting it resolves the alert, a CLI revoke ends the link at the next beat, re-enrollment works. v0.16.0: (site alerts: alerts tables, schema 6, ListAlerts/AckAlert/ResolveAlert/SilenceAlerts on the Itcmdr service, `itcmdr-cli alerts`, app Alerts page + badge; versions server/keys/app/installer/AgentVersion 0.16.0). v0.15.3: (generic names; history of itcmdr and itcmdr-plugins rewritten once more to scrub vendor/share names, tags re-pointed; re-clone old checkouts). v0.15.2: (site-specific values only in site.json, fault-injection suite, plugin errors for failing checks, app macos/). v0.15.1: Windows restore fix, real-server restore test, local release builds, ADMINSVR decisions 18 and 19. Previous: v0.15.0. Versions: server/keys/app/installer 0.15.1, itcmdr-common 0.15.0, itcmdr-plugin 0.11.0, AgentVersion 0.15.0, schema 5. Releases are built and published locally (`scripts/release-local.sh`, docs/RELEASE-LOCAL.md) while Actions is unavailable; Windows installers are unsigned (no code-signing certificate yet) and there is no macOS app archive until the app has a `macos/` project. v0.15.1: Linux Rust 120, Pester 96/96, Flutter 33 (1 skipped); Windows `restore_app` 10/10, clippy clean. Known: `cutover` test binary crashes at exit ~1% under load (pre-existing). Never run pre-v0.15.1 `restore_app` on a Windows box with itcmdr installed (it stops the real service).
- Decisions (Michael, 2026-11-01): Windows installers and the macOS app stay unsigned until he gets signing; he installs podman on alien-blue himself (once `podman --version` works, test the adminsvr container image in v0.19.2); v0.19.2 scope approved (admin app, push notifications, tested container image). v0.19.1 = passkeys + Diesel (Postgres and SQLite) for adminsvr.
- Before v0.15.0: Latest tag was **v0.14.3** (separate packages per component, the icon set in `branding/`). Previous: v0.14.2. Versions in the tree: server 0.14.3, itcmdr-keys 0.14.3, app 0.14.3, installer 0.14.3, agent `AgentVersion` 0.14.3. v0.14.3 Linux: fmt/clippy clean, Rust 104 passed, Pester 88/88, Flutter 31 passed (1 skipped). VM: all four packages build; upgrade from v0.14.2, fresh installs of each, server+app together, keep-data uninstalls and reinstall, full remove-data on a throwaway data dir all pass; the installed app shows the new icon. Windows app builds need a clean `build\windows` after an icon change (the resource is cached). Host Flutter tests need `cd app/rust && cargo build` first. Run `scripts/fetch-plugins.sh` once after cloning (the installer needs it; since the plugin-mocks change the Pester parity tests use the in-repo mocks in `agent/tests/fixtures/builtin-plugins` instead).
- Linux for v0.12.0: analyzer clean. Widget tests cover the Keys, Schedules and Backup dialogs and the settings search/switches. `app/test/e2e_test.dart` runs init, certifies an app key, then WhoAmI, every registry setting, a schedule, a backup and a restore dry run.
- Windows VM passes: v0.10.1, v0.11.0, v0.12.0 and v0.13.0. App click-through on the VM: passed (Michael). The test VM runs signed-only. See Results.
- **No install is switched.** Every install runs the SYSTEM task as before, plus the plugin versions report-only for parity. Switching is on site: collect a week of parity, `itcmdr-cli cutover status`, then `cutover vm-verified` and `cutover switch` (Michael).
- GitHub release builds are blocked by org billing (see Risks), so there is no installer artifact yet.

## Done (v0.10.0, plugin runtime + scheduler + restore server side)

- `proto/plugin.proto` + `plugin/` crate `itcmdr-plugin` (SDK, host launch/handshake, `log` helpers + validator).
- `server/src/runtime.rs` (run a plugin: Describe/Verify/Execute/Log, timeouts, plugin_runs, audit, share mirror),
  `jobs.rs` (on-demand queue incl. plugins), `plugin_logs.rs` (ingest, rejects, metrics query), `schedule.rs`,
  `restore.rs`; bins `itcmdr-plugin-ps` (PS host) and `itcmdr-restore` (helper). Schema 3.
- `agent/Itcmdr.Plugin.psm1`; rotation zips `plugin-logs/` from finished weeks. Docs: `docs/LOG-STANDARD.md`,
  `docs/BACKUP.md` (app restore), `docs/SETTINGS.md` (34 keys + server.json-only restore commands).
- CLI: `schedule add|list|edit|rm|pause|resume|run-now|history`, `plugins run|runs`.
- Tests: `server/tests/plugin_runtime.rs` (3), `schedule.rs` (4), `restore_app.rs` (1), schedule unit tests,
  `agent/tests/Plugin.Tests.ps1` (6) + rotation plugin-logs test.

## Done (v0.9.1, settings everywhere)

- `server/src/settings.rs`: typed registry (29 keys), `resolve_all`, `set`/`unset` (global or per machine), `export_agent` → `<logs_dir>/agent-settings.json`. Reference: `docs/SETTINGS.md`.
- RPCs `ListSettings` / `UpdateSetting` (admin/owner, audited); CLI `itcmdr-cli config list|get|set|unset [--machine]`.
- Runtime reads settings per use (ingest interval, request timeout, retention, metrics, skew, search, package size).
- Agent `Get-ItcmdrSetting` (site.json > central machine > central global > default), poller `Get-ItcmdrPollSetting`, installer reads `agent.nightlyTime`.
- App: `lib/server_settings.dart` (basic screen in the Settings tab).
- Plugins enabled on install by default (`--disabled`, setting `plugins.enable_on_install`).
- Plan: decision 13 app-triggered restore (StageRestore + helper + rollback) in v0.10.0.
- Tests: `server/tests/settings.rs` (2), CLI config test extended, packages test for enable-on-install, `agent/tests/Settings.Tests.ps1` (6), 2 Flutter tests.

## Done (v0.9.0, packages)

- `server/src/packages.rs`: `read_tar_gz`, `parse_manifest`, `validate`, `install` (quarantine on reject), `list`, `enable`, `disable`, `remove`, `pack`, `list_quarantine`.
- `plugins` table (schema 2). CLI `plugins ...` / `service plugins ...`. Backups include `service_plugins/**` + `quarantine/index.jsonl`.
- Tests: `server/tests/packages.rs` (5).

## Done (v0.8.1, configurable paths)

- Agent: `Get-ItcmdrPath` (site.json `paths.localLogs` / `agentData`), poller `Get-ItcmdrRequestDirectory`, rotation `Get-ItcmdrRotationRoot`; tests in `agent/tests/Paths.Tests.ps1`.
- Server: `server/src/settings.rs` (DB overrides for `backup.*`), RPCs `GetSettings`/`UpdateSettings`, `itcmdr-cli config`. Retention: `backup::block_key`, `keep_blocks`.
- Plans: decisions 14 and 15 in `docs/historical/1.0/SERVER-PLAN.md`.

## Done (v0.8.0, itcmdr-cli + backup & restore)

- `server/src/bin/itcmdr-cli.rs` (clap). `server/src/backup.rs`: `create`, `verify`, `open`, `list`, `prune_plan`/`prune`, `nightly`, `restore`.
- `auth.rs`: `list_keys`, `certificate_of`, `rotate_root`. `db::SCHEMA_VERSION` (user_version). Config `backup` section.
- RPCs `CreateBackup`/`ListBackups`/`VerifyBackup` in `api.rs`. Docs: `docs/BACKUP.md`, `docs/SIGNING.md`.
- Tests: `server/tests/backup.rs` (6, incl. CLI end to end), `server/tests/signing.rs` (2).

## Done (v0.7.0, keys & auth)

- `server/src/keys.rs`: words, `Master::from_words`, `fingerprint`, `certify`/`verify_certificate`, `seal`/`unseal`, `request_message`.
- `server/src/auth.rs`: `init`, `register`, `revoke`, `escrow_db_key`, `recover_db_key`, `load_root_key`, `sign_request`, `verify`, `require`, `NonceCache`, `Role`.
- `server/src/api.rs`: `authz` (mutating) / `read_guard` (reads), `AuthMode`; config `auth_mode` (default `both`).
- `server/src/main.rs`: `init`, `recover-db-key [--force]`. Tests: `server/tests/auth.rs` (4), unit tests in keys.rs/auth.rs.

## Done (v0.6.2)

- `db.rs::finish_migration` (read-back + count check, then `storage::shred_file`), `db::table_counts_of`. Tests: storage.rs (6), storage unit test for shred.

## Done (v0.6.0, storage)

- `server/src/storage.rs`: `resolve_data_dir` (flag > env > config > default), `Layout`, `write_secret`/`read_secret`
  (DPAPI on Windows, 0600 hex elsewhere), `load_or_create_db_key`, `is_plaintext_sqlite`, `key_pragma`.
- `server/src/db.rs`: `init_db` opens SQLCipher with `PRAGMA key` and errors clearly on a wrong key;
  `migrate_plaintext` (sqlcipher_export + row-count check + `*.plaintext-backup`); roles/users/clients/server_meta; `role_id`.
- `server/src/main.rs`: `--data-dir` in every mode. Tests: `server/tests/storage.rs` (5).
- DPAPI code type-checked for Windows in a scratch crate (`cargo check --target x86_64-pc-windows-gnullvm`); not run on Windows.

## Done (v0.5.0)

- Agent: `Invoke-ItcmdrBackupCheck` + `Test-ItcmdrBackupTask` / `Test-ItcmdrBackupDestination`
  (`site.backup.{expectedBackupSources -> BACKUP-SRC-A.., tasks, destinations}`; server role unless an entry lists `roles`).
  `agent/Report-BackupCoverage.ps1` reuses them. Tests: `agent/tests/Backup.Tests.ps1`.
- Poller: `Invoke-ItcmdrPoll -StaleLockMinutes` (default 150, above the task's 2h limit).
- Server: `requests.rs::apply_timeout`, config `request_timeout_secs` (default 10800), status `TIMED_OUT`.
- `server/tests/grpc.rs`: end-to-end over real gRPC (client codegen is now on in `server/build.rs`).

## Done (v0.4.0)

- `server/src/metrics.rs`: `count_flaps`, `flapping()` -> `flaps:<m>:<check>` (>= `FLAP_MIN`=3 changes);
  `repeat_offender:<m>:<check>` (>= `REPEAT_MIN`=3 incidents). Test: `tests/ingest_metrics.rs::flapping_and_repeat_offenders`.
- `server/src/search.rs::search`: machine/category/status exact (case-insensitive), text substring,
  start/end on the record's ts, newest first, `limit` (default 100, max 1000) + `offset`; response `total`, `next_offset`.
- App: `lib/charts.dart` (BarChart, `seriesByBucket`), `lib/logs_page.dart` (filters, date range, paging).

## Done (v0.3.0, checks batch)

- `agent/Invoke-ItcmdrAgent.ps1`: `Test-ItcmdrTcp` (TCP connect with timeout, replaces Test-NetConnection;
  `ITCMDR_TCP_TIMEOUT_MS` overrides). Checks can return `ConvertTo-ItcmdrResult` (ok/detail/skipped);
  `skipped` = no decision possible (e.g. nobody logged on), never repaired, no incident change.
- `Invoke-ItcmdrScannerCheck` (category connectivity): `site.scanners[]` {name,address,ports,roles?} ->
  `SCANNER-<name>-TCP-<port>`, report only. The 3D unit is allowed via an exception rule
  (`checkPattern ^SCANNER-<name>-`, `downgrade:info`), see `config/site.example.json`.
- `Invoke-ItcmdrConfigCheck` (category configuration):
  - `dnsRecords[]` {name,expected[],forbidden[]} -> `DNS-<name>`; repair = local `ipconfig /flushdns` only.
  - `driveMaps[]` {letter,path} -> `DRIVE-<L>`: reads each loaded user hive (HKU\<sid>\Network); report only.
  - `printerQueues[]` {name,server,shared}: server role -> `PRNQ-<name>` (exists, shared, status, stuck jobs;
    repair only removes jobs in error); workstations -> `PRNC-<name>` per-user connection; plus `SVC-Spooler`.
  - ExampleVendor drift -> `EXAMPLEVENDOR-DRIFT-<file>`: files from `examplevendor.configFiles` (env vars + wildcards) searched for
    `stalePatterns`, opened read-only with FileShare.ReadWrite. **No repair, ever.**
- App: `configuration` added to on-demand categories. Tests: `agent/tests/Checks.Tests.ps1`.

## Done (v0.2.0)

- Server (`server/src`): idempotent timestamp-correct ingest incl. nested zips (`ingest.rs::ingest_all`),
  incident dedup (`db.rs::update_incidents_for_check`), ISO-aligned metrics/rollups daily/weekly/
  4-week block/ISO yearly (`metrics.rs`), Windows service + console mode (`main.rs`), on-demand
  status from poller responses (`requests.rs`), exceptions format/validation/export
  (`exceptions.rs`), token auth + audit log + ListAudit (`api.rs`), 400-day retention (`db.rs::apply_retention`).
- Agent (`agent/`): PS 5.1-compatible; exceptions from site.json + server export; `-Categories`,
  `-Repair`; poller with per-machine locks and responses; rotation weekly -> 4-week blocks -> ISO yearly
  with verified manifests (`Rotate-ItcmdrLogs.ps1::Invoke-ItcmdrRotation`).
- App (`app/lib`): Settings (host/port/TLS/token/actor), exceptions add/edit/expire, on-demand,
  audit tab, ranked metrics tables, live refresh.
- Installer (`installer/itcmdr.iss`): service with `--service`, config template, firewall, tasks, uninstall.
- Sample data: `tools/sample-data-gen`.

## Done (v0.11.0, built-in checks as plugins + default schedules)

- `agent/Itcmdr.Checks.ps1` holds every check (moved, unchanged). `plugins/builtin/` had five plugins over it (moved to the itcmdr-plugins repo in v0.14.1) (network, scanners, config, backup-coverage, services); `Build-BuiltinPlugins.ps1` stages/signs them.
- Agents run only the copies installed with the agent (`{app}\agent\plugins`); they never download plugins. Plugin packages are fetched (plain HTTPS) and run by the server only (decision 16).
- `agent/Itcmdr.PluginRunner.ps1`: run plugins as child processes, `Compare-ItcmdrParity`, `schedule-state.json`, `cutover.json`, task enable/disable (never delete), local fallback. `Set-ItcmdrCutover.ps1 -To task|schedule|auto`.
- Nightly: `agent.pluginMode` auto = plugins report-only first, then the built-in checks, then a `parity` line; after the switch the plugins are the run with per-plugin fallback to the built-in function. run-start carries `trigger`, `cutover`, `scheduleStateTs`, `pluginMode`.
- Server: `cutover.rs` (seed `Nightly agent run` paused + `Nightly backup`, reset-to-default, remembered deletes, gate, switch, rollback, `schedule-state.json` each pass); backup loop replaced by the backup schedule; schema 4 (`parity_runs`, `agent_state`); gRPC `ResetScheduleToDefault`, `ScheduleRef.confirm`, `Schedule.builtin_key/action` (Dart protos regenerated).
- CLI: `schedule reset-to-default`, `schedule rm --force --confirm "<name>"`, `cutover status|vm-verified|switch|rollback`.

## Done (v0.12.0, the app signs requests; released as eea653d)

Base v0.11.0 = `c0748ea`. WIP `aa5b408`, released `eea653d` (tag v0.12.0). Linux at the WIP commit: cargo fmt/clippy -D warnings clean, Rust 93 passed,
Pester 82/82, flutter test 15 passed, flutter analyze 4 infos (curly braces in backup_page.dart, schedules_page.dart).
Later the same day: those 4 infos fixed, widget tests added, Dart e2e green, versions set to 0.12.0. Analyzer clean.

**Done (server, tested on Linux)**
- Sealed restore words: `server/src/restore_seal.rs` (X25519 + HKDF (SHA-256) + XChaCha20-Poly1305, one-time 10-min keys bound to the signing key, aad binds key id + backup). `GetRestoreKey` RPC; `StageRestore` refuses plaintext `words`, needs `sealed_words`, `dry_run` first, then `dry_run_id` (30 min, same key + backup). Restore/keys RPCs are signed-only (`signed_only` in `api.rs`); the shared token cannot restore. Audited: restore-dry-run, restore-refused, restore-stage. `itcmdr-cli restore-stage` uses the same path. Tests: `server/tests/restore_app.rs`.
- New RPCs: WhoAmI, ListKeys (admin), RegisterKey/RevokeKey (owner; bundle certified offline), ListPlugins, PreviewSchedule. Tests: `server/tests/app_api.rs`.
- auth_mode: default `signed`. Without a value in server.json, `db::init_db` writes `server_meta.auth_mode_default` once ("both" if the DB already had tables = pre-0.12 install, else "signed"); `auth::settle_auth_mode` applies it in `lib.rs run`. Unknown/empty mode fails secure (Signed). Installer template no longer sets auth_mode (existing installs keep their explicit "both"). `itcmdr-cli auth status|set-mode MODE [--force]` (refuses `signed` without a non-root admin/owner key).
- Vectors: `proto/signing_vectors.json` (+3 new, old ones unchanged), new `proto/app_vectors.json` (Ed25519 sig + sealed words).

**Done (app, Linux)**
- `app/lib/signing.dart`: AppKey, SigningInterceptor, `canonicalBytes` (Dart writes explicit defaults; the vector test caught it), `sealWords`. `app/test/signing_test.dart` matches the Rust vectors byte for byte.
- Pages: `keys_page.dart`, `schedules_page.dart`, `backup_page.dart`, `plugins_page.dart`; `main.dart` now a NavigationRail with 11 entries; `settings.dart` stores the key seed (`auth.key_seed`). The Keys page shows the local fingerprint before the server answers.
- Settings: search box and a switch for each bool (`server_settings.dart`). `app/test/pages_test.dart` checks every key in `docs/SETTINGS.md` plus the dialogs. `app/test/e2e_test.dart` checks the live registry against that doc.

**Half-done / still open**
- Plugin install and enable/disable are CLI-only. The app lists, runs and charts.
- Key seed is stored in shared_preferences (user profile, plaintext); DPAPI later.
- VM: v0.12.0 upgrade is installed (Pester 82/82, service Running, tasks Ready, app binary matches the build). `auth status` is `both` from server.json. The screens were not clicked through.

**Next steps in order**
1. Click through the installed app on the VM (connect, Keys, Schedules, Backup dry run, Settings) with a person at the screen.
2. A real second PC as an agent.
3. On site (Michael): the signed-only checklist in docs/INSTALL.md; a week of parity, then the gated cutover.
4. Done in v0.14.1: plugins in itcmdr-plugins. The itcmdr history was rewritten once to drop `plugins/` (approved by Michael). Re-clone, or `git fetch --force --tags origin && git reset --hard origin/main`.

**Commands**
- Linux: `export PATH=$HOME/Projects/tools/flutter/bin:$HOME/Projects/tools/pwsh:$HOME/Projects/tools/protoc/bin:$HOME/.pub-cache/bin:$PATH`; `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`; `pwsh -NoProfile -c 'Invoke-Pester -Path agent/tests -CI'`; `cd app && flutter analyze && flutter test`.
- Protos: `protoc --dart_out=grpc:app/lib/proto -Iproto proto/itcmdr.proto` (Rust regenerates in build.rs). Vectors: `ITCMDR_REGEN_VECTORS=1 cargo test --test signing` (add only, never change existing).
- VM: see HANDOVER.local.md.

**Gotchas**
- pwsh on Linux crashed after the reboot ("The given assembly name was invalid"): a corrupt `~/.cache/powershell/StartupProfileData-NonInteractive`. Moving the cache aside fixed it.
- One `corrupted size vs. prev_size` abort at exit of the backup test binary, not reproduced in 3 reruns.
- Tests that use the shared token must set `auth_mode: "both"` (default is now signed).

## In progress

v1.1.0 (not tagged): HTTPS default / ACME / trust work is on main. Test flakes under load were fixed in 2026-11-04: wall-clock deadlines (`ITCMDR_TEST_SLOW`, default 3), retries while the API or Pebble starts, and `srv::run` closing its pool on stop (the calllog test binary used to segfault at exit when SQLite workers were still closing). Tests that start a server should stop it and await it before returning. The app e2e test finds the server binaries under `$CARGO_TARGET_DIR/debug` when that is set (or `ITCMDR_E2E_BIN_DIR`). After moving the checkout, rerun `flutter pub get` in both apps (`.dart_tool` keeps absolute paths) and `cargo clean -p itcmdr-keys` if a shared target dir holds test binaries built at the old path (`env!("CARGO_MANIFEST_DIR")` is baked in). 2026-11-04: full suite green, main pushed (no tag). Docs rule from now on: update docs/ in the same commit as the code; after changing a CLI flag rebuild and run `scripts/gen-man.py`, after changing docs run `scripts/gen-help.py` (both have `--check`). Next: Windows VM check, then release.

Nothing in progress. Proposed next: server/agent + adminsvr architecture (docs/historical/1.0/design/SERVER-AGENT.md, docs/historical/1.0/design/ADMINSVR.md incl. the SRE Flutter app; roadmap v0.15.0 to v0.22.0 in PLAN.md), Michael's decisions recorded 2026-10-31 (incl. key-based adminsvr auth and the signed, hash-chained audit of every change) (ADMINSVR.md "Decisions"); next is v0.14.2 (extract `itcmdr-keys` from server/src/keys.rs, auth.rs and restore_seal.rs; app/lib/signing.dart moves to flutter_rust_bridge), then v0.15.0. Passkeys/YubiKeys (second factor) planned as v0.19.1. Key backup defaults decided (on, 24-word lock, COOP/COEP fine); v0.14.2 released (keys crate + bridge; VM pass below). v0.15.0 released (errors + calls). Next: v0.16.0 (alerts). Pending: Mac signing step for iOS/macOS, the adminsvr agent design, rootless Podman install on alien-blue (sudo needs a password: `sudo pacman -S --needed podman passt slirp4netns fuse-overlayfs podman-compose`; subuid/subgid for m3talsmith already set, 100000:65536). v0.14.1 released; itcmdr history rewritten once (plugins/ removed, tags recreated); VM tree re-synced from the new history.

## Backlog (in order)

1. Windows VM test pass: v0.10.1 to v0.14.0 done (results below). App click-through passed (Michael). Still open: a real second PC as agent.
2. ~~Missing agent checks~~ done in v0.3.0. Real values must be added to the site.json on Windows hosts.
3. ~~Flapping/repeat-offender metrics, charts~~ done in v0.4.0.
4. ~~Log search filters and paging~~ done in v0.4.0.
5. ~~Backup coverage~~ done in v0.5.0. Real task names/destinations: see HANDOVER.local.md → Real-site notes.
6. ~~Stale locks, request timeouts~~ done in v0.5.0.
7. ~~gRPC integration test~~ done in v0.5.0.
8. Unblock GitHub Actions (billing) or build the installer on a Windows machine.

## Build and test (Linux dev box)

Tools are under `~/Projects/tools` (flutter, pwsh, protoc); add them to PATH:
`export PATH=$HOME/Projects/tools/flutter/bin:$HOME/Projects/tools/pwsh:$HOME/Projects/tools/protoc/bin:$HOME/.pub-cache/bin:$PATH`

```
scripts/fetch-plugins.sh   # pinned itcmdr-plugins -> third_party (installer and releases; tests use agent/tests/fixtures/builtin-plugins)
cargo build --workspace --all-targets && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
(cd app && flutter pub get && flutter analyze && flutter test)
pwsh -NoProfile -c 'Invoke-ScriptAnalyzer -Path agent -Recurse -Severity Warning,Error'
pwsh -NoProfile -c '$c=New-PesterConfiguration;$c.Run.Path="agent/tests";Invoke-Pester -Configuration $c'
python3 scripts/scan_repo.py        # leak scanner (also the pre-commit hook)
```

- Dart protos: `protoc -I proto --dart_out=grpc:app/lib/proto proto/itcmdr.proto` (protoc_plugin 25.1.0).
- Rust protos regenerate automatically via `server/build.rs`.
- The Windows service code can't be fully cross-compiled here (SQLite needs Windows headers);
  type-check it by copying `main.rs` into a scratch crate and running `cargo check --target x86_64-pc-windows-gnullvm`.

## Release procedure

1. All the checks above are green, and `scan_repo.py` is clean.
2. Grep the staged diff against `config/banned.local.txt`; it must have 0 hits.
3. Commit on `main`. Bump `MyAppVersion` (installer), `version` in `server/Cargo.toml` and `app/pubspec.yaml`, `AgentVersion` when the agent changed; add a `CHANGELOG.md` entry.
4. `git tag -a vX.Y.Z -m "itcmdr vX.Y.Z"`, then `git push origin main`, then `git push origin vX.Y.Z`.
5. Never force-push, and never move or delete tags. Push only tested, tagged commits.
6. The tag triggers `.github/workflows/release.yml`, which builds the Windows installer once Actions works.
7. Until then (v0.15.0 and later): build and publish locally from the tag with `scripts/release-local.sh` (Linux and macOS: `unix`; Windows: the VM step; then `check` and `publish`). See `docs/RELEASE-LOCAL.md`.

## Windows VM test plan

1. Install with `/DAllowSubnet=<lab subnet>`; `sc qc itcmdr-server` shows `"…\itcmdr-server.exe" --service`;
   the service is Running; `%ProgramData%\itcmdr\server-logs\itcmdr-server.txt` exists; it survives a stop/start and a reboot.
2. Set `logs_dir` and `admin_token_hash` (`itcmdr-server.exe hash-token <t>`) in `%ProgramData%\itcmdr\server.json`,
   then restart; the app connects; with no token in the app, a trigger gives the clear error.
3. `powershell.exe -File Invoke-ItcmdrAgent.ps1 -ReportOnly` under **Windows PowerShell 5.1**: the JSONL log
   appears, is copied to the share, and is queued when the share is offline.
4. Start the nightly and poller tasks manually as SYSTEM; the poller repeats every minute and skips overlaps.
5. On-demand: one machine, then all machines; status moves queued -> running -> done.
6. An exception added in the app appears in `<share>\exceptions.json` and is honoured on the next run.
6a. v0.9.1: change `agent.quietMinutes` for one PC in the app; `<share>\agent-settings.json` updates, the PC uses it on the
   next run (and from the agentData cache with the share offline); `itcmdr-cli config list --machine <PC>` agrees.
6b. v0.10.0: `itcmdr-cli plugins run <name>` with a PowerShell plugin under Windows PowerShell 5.1; a schedule
   (`itcmdr-cli schedule add ... --when "every 1h"`) queues runs; app-triggered restore: StageRestore stops and
   restarts the service via sc.exe and rolls back on a failed health check.
7. Rotation with `-Root <copy> -Today 2027-01-04`: the zips open in Explorer, and the server ingests the archive.
8. Uninstall removes the service, the tasks and the firewall rule.
9. v0.11.0: after an upgrade, `itcmdr-cli schedule list` shows the two built-ins; the SYSTEM nightly writes a `parity`
   line with 0 mismatches/errors; `schedule run-now "Nightly agent run"` reaches the agent with `trigger=schedule`;
   `Set-ItcmdrCutover.ps1 -To schedule|task|auto` disables/re-enables the task; an agent reinstall keeps it disabled;
   with the server stopped the poller runs the nightly locally (`trigger=fallback`); `cutover switch` is refused
   without a clean week. Never run `cutover switch` on a real install without the gate.

### Results: first VM pass (v0.10.1, 2026-10-30, Windows 11 Pro, Windows PowerShell 5.1)

Built on the VM (Rust MSVC, Flutter, Inno Setup), installed silently with the full set of components, and used a local folder shared as `DATA` to stand in for the share. Placeholder office values only.

| Item | Result |
|---|---|
| Pester 65/65 and PSScriptAnalyzer 0 findings under PS 5.1 (Pester 6.2) | ✅ after fixing the tests' 3-argument `Join-Path` |
| Installer compiles | ✅ after fixes (brace escapes, quoting, `[Code]` comments, `target\release` path) |
| Silent install: service Running (delayed-auto, under `%ProgramData%\itcmdr`), 3 tasks, firewall rule, files | ✅ |
| Service starts again after a reboot | ✅ |
| `hash-token` + restart; `itcmdr-cli init` (words shown once), `user add`, `keys list` | ✅ |
| Nightly task (SYSTEM): log written, copied to the share | ✅ |
| Ingest of agent logs | ❌ → fixed: PS 5.1 wrote a UTF8 byte-order mark, so the run-start line was lost (fixed in both the agent and the server) |
| On-demand `TriggerRuns` → request file → poller → DONE | ✅ (~60 s) |
| Plugin `pack` / `install` (enabled) / `run` through `itcmdr-plugin-ps` on PS 5.1 (CLI and the SYSTEM service via `RunPlugin`) | ✅ |
| A failed Verify skips Execute; `plugins runs`; v1 log + share mirror; `GetPluginMetrics` | ✅ |
| Schedules: minute cron fired, the concurrency skip worked, pause/rm, next runs with timezones | ✅ |
| Central agent settings per machine → `agent-settings.json` on the share → the agent sees the value | ✅ |
| Backup create, list, verify, verify --deep | ✅ |
| Rotation task: weekly zip including `plugin-logs/` | ✅. The server read those zipped plugin logs as agent logs ❌ → fixed |
| In-place upgrade over a running install | ✅ |
| `restore-stage` (new CLI on the StageRestore path) → sc.exe stop/start → done; wrong words refused | ✅ |
| Forced health failure → rolled back, service running, old value back | ✅ |
| Uninstall removes the service, tasks, firewall rule and program dir, and keeps the data; reinstall picks it up | ✅ |
| Agent repairs on a VM with the example site.json | ⚠️ NET-IP-CORRECT set the example DNS server and cut the VM off. Expected with wrong site values: site.json must match the real network before repairs run |

### Results: second VM pass (v0.11.0, 2026-10-30, Windows 11 Pro, Windows PowerShell 5.1)

Installer built on the VM and installed over v0.10.1 (in-place upgrade). Placeholder office values only; no switch.

| Item | Result |
|---|---|
| Pester 81/81 and PSScriptAnalyzer 0 findings under PS 5.1 (incl. plugin parity and end-to-end runs) | ✅ |
| Upgrade: service Running, new agent files and `agent\plugins\<5>\main.ps1` installed | ✅ |
| Default schedules seeded: `Nightly agent run` (paused, 01:30, repair) and `Nightly backup` (on, 02:00, catch-up); `schedule-state.json` on the share | ✅ |
| SYSTEM nightly: built-in run + plugins report-only first; parity 17 matched, 0 mismatched, 0 errors; itcmdr.log/v1 plugin logs | ✅ |
| `cutover status` shows 7 empty days and the blocking reasons; `cutover switch` refused | ✅ |
| `schedule run-now "Nightly agent run"` → poller → agent with `trigger=schedule`, parity 17/0 | ✅ |
| `Set-ItcmdrCutover.ps1 -To schedule` disables the task; `-To task` re-enables it; `-To auto` follows the server | ✅ |
| Agent reinstall keeps the disabled task disabled | ❌ → fixed: the agent's `$Trigger` parameter collided with the installer script's `$trigger` (re-registration failed); now ✅ |
| Server stopped → schedule state stale → poller ran the nightly locally (`trigger=fallback`, plugins mode, 17 checks) | ✅ (~150 s with serverStaleMinutes=2 for the test) |
| Share offline (SMB share removed): log queued in `spool\outbox`; share back → flushed to the share | ✅ |
| "local" schedule times on Windows across the fall-back day | ❌ → fixed: 02:00 showed 01:00 -08:00 on Nov 1; now uses the IANA zone |
| App UI on Windows; a real second PC | ⏳ app binary installed 2026-10-31; the screens were not clicked through. A second PC is still not done. |

### Results: v0.12.0 VM pass (2026-10-31, Windows 11 Pro, Windows PowerShell 5.1)

In-place upgrade over the v0.11.0 install. The installer is built after `flutter build windows` (the first package that morning was built before the app binary existed, so it still contained the previous app; it was rebuilt and installed again).

| Item | Result |
|---|---|
| Release build of the server | ✅ 1m 55s, exit 0 (one duplicated warning) |
| Installer compiles | ✅ |
| Pester 82/82 and PSScriptAnalyzer 0 findings under PS 5.1 | ✅ |
| `flutter build windows` | ✅ 79s |
| Silent upgrade: exit 0, service Running, nightly / poller / rotation tasks still Ready | ✅ |
| Installed agent `AgentVersion` 0.12.0; installed `itcmdr.exe` timestamp matches the build | ✅ |
| `itcmdr-cli auth status` | both, because server.json still has `"auth_mode": "both"` from the v0.11.0 installer. Upgrades do not replace that file, so this install does not take the kept-both path. The shared token is still configured. One admin key (`Dr Example`) can sign. |
| Kept-both path: `auth_mode` removed from server.json, service restarted | ✅ `auth status` = both ("not set; this install predates v0.12.0"), unsigned token call still works; server.json restored after |
| Installed `itcmdr.exe` starts and stays up (15 s, headless) | ✅ |
| Dart e2e against a real server on Windows (`flutter test test\e2e_test.dart`, release bins via `ITCMDR_E2E_BIN_DIR`) + signing and page tests | ✅ 14 passed |
| Dart e2e on Windows | ❌ → fixed: the test hard-coded `../target/debug/itcmdr-server` (no `.exe`, no bin dir override) |
| App screens clicked through on the VM | ✅ passed (Michael, 2026-10-31) |

### Results: v0.15.0 VM pass (2026-10-31, Windows 11 Pro, Windows PowerShell 5.1)

| Check | Result |
|---|---|
| Release build, Pester, Flutter tests, Dart e2e (release bins) | ✅ 96/96, 33 passed (1 skipped), e2e passed |
| `cargo test --workspace` on Windows (first time run there) | ⚠️ 118 passed, 1 failed: `restore_app` (Access is denied on the open DB; fails identically on v0.14.3) |
| Four 0.15.0 packages build (ISCC) | ✅ |
| In-place upgrade 0.14.3 → 0.15.0 (server, agent, app, cli) | ✅ service Running, keys/config unchanged, signed-only kept, AgentVersion 0.15.0 |
| Signed app calls logged; `itcmdr-cli calls list/show`, `--json` | ✅ principal = key name + fingerprint |
| Plugin run after upgrade | ✅ itcmdr-network ok |

### Results: v0.14.2 VM pass (2026-10-31, Windows 11 Pro, Windows PowerShell 5.1)

| Check | Result |
|---|---|
| Release build of the server bins with `itcmdr-keys` | ✅ |
| `cargo test -p itcmdr-keys` on Windows (vectors) | ✅ 8 passed |
| `flutter build windows` (native assets build the bridge (v0.20.0; cargokit before); `itcmdr_keys_bridge.dll` next to `itcmdr.exe`) | ✅ |
| Installer (ISCC) and upgrade over v0.14.1 | ✅ exit 0, service Running, `AgentVersion` 0.14.2, bridge DLL installed, signed mode kept |
| Pester (Windows PowerShell 5.1) / PSSA | ✅ 88/88, 0 findings |
| flutter test (incl. DPAPI and e2e against the release bins) | ✅ 31 passed, 1 skipped |
| Installed app starts and stays up 15 s | ✅ |
| Signed calls with the DPAPI-stored admin key through the bridge (WhoAmI, ListSchedules, ListPlugins) | ✅ |

Web: the `itcmdr-keys` crate and the bridge build for wasm32 (`flutter_rust_bridge_codegen build-web`), and the crate passes the pinned vectors under WASM (`wasm-pack test --node keys`). The on-site app itself is desktop-only and does not compile for web (its `key_store.dart` imports `dart:ffi` for Windows DPAPI, as before), so no web build of the on-site app ships. A Flutter in-browser test (`flutter test --platform chrome`) hung loading `pkg/` and is a follow-up for the SRE app, which targets web.

### Results: v0.14.1 VM pass (2026-10-31, plugins from itcmdr-plugins)

| Item | Result |
|---|---|
| Build, ISCC with the pinned copy, Pester 88/88 (parity against the pinned plugins), PSScriptAnalyzer 0, `flutter test` 31 + 1 skipped | ✅ |
| Upgrade to v0.14.1: installed `agent\plugins` = the five pinned copies (hash equal), `AgentVersion` 0.14.1, signed mode kept | ✅ |
| itcmdr-plugins Pester on PS 5.1 | ✅ 10/10 after a test fix (v1.0.1) |
| `Install-Plugins.ps1 -Disabled` against the signed-mode server (registered admin key) | ✅ five installed disabled |
| Remove, then `Install-Plugins.ps1` (enable on install) | ✅ five enabled |
| Tampered package via `-Packages` | ✅ refused ("sha256 does not match"), quarantined, exit 1 |
| `-Cli` pointing nowhere | ✅ "itcmdr-cli not found", exit 1 |
| `itcmdr-cli plugins run <check plugin>` | ❌ → fixed: "plugin exited before the handshake" (verbatim `\\?\` script path, so `$PSScriptRoot` was empty under PS 5.1). After the fix all five run (verify ok, execute ok). They ran in repair mode on the test VM. |

### Results: v0.14.0 VM pass (2026-10-31, Windows 11 Pro, Windows PowerShell 5.1)

| Item | Result |
|---|---|
| Build, installer compile, Pester 88/88, PSScriptAnalyzer 0, `flutter test` 31 + 1 skipped | ✅ (3 PSSA findings in the new script, fixed) |
| Back up `%ProgramData%\itcmdr` and `%APPDATA%\itcmdr` (robocopy, service stopped), silent uninstall, data removed | ✅ |
| Clean silent install with `/LogsDir /BackupDir /SiteName /ServerHostname /Share` | ✅ exit 0, service Running, values in server.json and site.json, `setup-check.txt` all OK except "not initialized" (warn), agent check all OK |
| No words in the installer log | ✅ |
| `setup first-run` with redirected output | ✅ refused |
| `setup first-run --app-key-store --admin-name` (headless test flag) | ✅ 24 words, first admin (owner) registered, key created in the DPAPI store |
| The Dart app store reads the key the CLI wrote; signed WhoAmI (owner), ListSchedules, ListPlugins; token refused | ✅ signed from day one |
| Installed app starts; agent poller result 0 | ✅ |
| Uninstall, restore the backup, reinstall v0.13.0, upgrade to v0.14.0 with different site values | ✅ server.json byte-identical, site.json unchanged, `setup apply` not run, tasks Ready, signed calls work, token refused, cutover state unchanged |
| Interactive console (pause, ask 3 words, re-show on mismatch, clear screen) | ✅ on Linux through a pseudo-terminal. On Windows it uses `cls`; the console window itself still needs a person at the screen |

### Results: v0.13.0 VM pass (2026-10-31, Windows 11 Pro, Windows PowerShell 5.1)

In-place upgrade over v0.12.0.

| Item | Result |
|---|---|
| Release build, `flutter build windows`, installer compile | ✅ |
| Pester 82/82, PSScriptAnalyzer 0 findings (PS 5.1) | ✅ |
| `flutter test` on Windows (all files, incl. the DPAPI round trip and tamper check; Dart e2e incl. plugin upload/quarantine against release bins) | ✅ 31 passed, 1 skipped (secret-tool, Linux only) |
| Silent upgrade: exit 0, service Running, tasks Ready, `AgentVersion` 0.13.0 | ✅ |
| App key in the DPAPI store (`%APPDATA%\itcmdr\app-key.dpapi`), no plaintext copy | ✅ (the VM has no words any more, so the registered `Dr Example` admin key was imported into the store) |
| Signed WhoAmI / ListSchedules / ListPlugins in `both` | ✅ |
| `auth set-mode signed` + restart: signed calls work; token and unauthenticated calls refused (`Unauthenticated`, "missing x-itcmdr-key") | ✅ |
| Rollback `auth set-mode both` + restart: token calls work again, signed still work | ✅ |
| Back to `signed`; agent poller run result 0; installed app starts and stays up | ✅ |
| Final state | VM left in `auth_mode = signed` |

## Risks and gotchas

- **GitHub Actions is blocked by billing** on the WeirdBrains org ("payments failed / spending limit").
  The repo is private, so Windows minutes are billed.
- **The Actions allowlist is set at the org level.** Only GitHub-owned actions are allowed, so the
  workflows use no third-party actions (keep it that way, or have an org admin allow them).
- **PowerShell 5.1:** the scheduled tasks run `powershell.exe` (5.1). Don't use `ConvertFrom-Json -AsHashtable`,
  `[System.Globalization.ISOWeek]`, three-argument `Join-Path` (fine in tests only), or other PS7-only features.
  Use `Get-ItcmdrValue` for config lookups; StrictMode throws on missing keys.
- Real site values and instructions: see HANDOVER.local.md → Real-site notes.
- **Never commit real site values** (IPs, hostnames, domains, usernames, MACs).
  `config/site.local.json`, `config/site.json` and `config/banned.local.txt` are gitignored.
- The scanner allows only `0.0.0.0`, `TCP-<port>` and `SCAN-NN` tokens beyond the placeholders; don't widen it further.
- Mutating RPCs need a signed request (v0.12.0 default) or, on an install that still has `auth_mode = both`, the legacy token. Run `itcmdr-server init` once per server; the 24 words are
  printed ONCE and are the only way to certify new keys and recover `db.key`. New installs are `signed`. An upgrade whose server.json does not set `auth_mode` keeps `both` until `itcmdr-cli auth set-mode signed` (needs a non-root admin or owner key first). A value already in server.json wins: the v0.11.0 installer wrote `both`, and upgrades do not replace that file.
- Never change the master derivation (`keys.rs`); the pinned test vector guards it.
- Never edit `proto/signing_vectors.json` to make a test pass; regenerate (ITCMDR_REGEN_VECTORS=1) only for a deliberate protocol change.
- Agent paths come from `Get-ItcmdrPath`; never hard-code `C:\maintenanceRuns` again. In tests use `[IO.Path]::Combine` (Join-Path fails on drive letters / UNC on Linux).
- Use `settings::backup_dir` / `effective_backup` (not `cfg.backup` directly): DB overrides win.
- Package installs do file IO and downloads in `spawn_blocking` (ureq is blocking); keep it that way.
- Never loosen `packages::safe_rel_path` / the regular-file-only rule; restore reuses it for archived plugin paths.
- Bump `db::SCHEMA_VERSION` with every schema change: restore uses it to refuse archives from newer versions.
- Tests that start the server should set `backup.nightly = false` (the loop would otherwise write a backup into the temp dir).
- **SQLCipher build** needs perl + a C compiler (vendored OpenSSL); GitHub Windows runners have Strawberry Perl.
  The first build takes ~40 s on alien-blue.
- **Losing `keys/db.key` means losing the database** (it is random, not derived from the words). Back up the
  data dir's `keys/` with the server backup; on Windows the DPAPI blob only decrypts on the same machine.
- Since v0.6.2 the `*.plaintext-backup` is overwritten and deleted automatically after a verified read-back
  (`db.rs::finish_migration`). If verification fails it is KEPT (unencrypted) and an error is logged: check the
  server log and delete it by hand once resolved.
- Never put `;` inside the SQL comments in `db.rs::create_schema` (the schema is split on `;`).

## Where real config lives (not in git)

- The dev box: `config/site.local.json` and `config/banned.local.txt` in the repo (gitignored), plus `HANDOVER.local.md`.
- Windows hosts: `C:\ProgramData\itcmdr\site.json` and `C:\ProgramData\itcmdr\server.json`.


## Fault-injection suite (v0.15.2)

On a disposable Windows test bed only: `powershell -File scripts\Invoke-ItcmdrFaultSuite.ps1 -IAmATestBed [-Service <harmless service>] [-Root <scratch>]`. It refuses without `-IAmATestBed`, restores the service and removes the scratch folder. Never run it on a production server.
