# itcmdr handover notes

Keep this file current: update it at the start of each work item and in every commit. Phase statuses live in the root `PLAN.md` (update it in every commit too).
Placeholders only (SERVER01, WS-01, RCU-01, 198.51.100.x). Real site notes go in the
untracked mirror `HANDOVER.local.md` in the repo root (gitignored, never committed).

## Current state (updated 2026-10-31)

- Branch: `main` only (no PRs, no feature branches). Latest tag: **v0.14.1** (plugins moved to github.com/WeirdBrains/itcmdr-plugins, pinned in `plugins.pin.json`). Previous: v0.14.0. Versions in the tree: server 0.14.1, app 0.14.1+20, installer 0.14.1, agent `AgentVersion` 0.14.1. Run `scripts/fetch-plugins.sh` once after cloning (installer and parity tests need it).
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
- Plans: decisions 14 and 15 in `server/PLAN.md`.

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
  - Sirona drift -> `SIRONA-DRIFT-<file>`: files from `sirona.configFiles` (env vars + wildcards) searched for
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

## Done (v0.12.0, the app signs requests; released as d79a031)

Base v0.11.0 = `1827b06`. WIP `0631198`, released `d79a031` (tag v0.12.0). Linux at the WIP commit: cargo fmt/clippy -D warnings clean, Rust 93 passed,
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

Nothing in progress. v0.14.1 released; itcmdr history rewritten once (plugins/ removed, tags recreated); VM tree re-synced from the new history.

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
scripts/fetch-plugins.sh   # pinned itcmdr-plugins -> third_party (installer + parity tests)
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

Built on the VM (Rust MSVC, Flutter, Inno Setup), installed silently with the full set of components, and used a local folder shared as `PDATA` to stand in for the share. Placeholder office values only.

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
