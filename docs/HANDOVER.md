# itcmdr handover notes

Keep this file current: update it at the start of each work item and in every commit. Phase statuses live in the root `PLAN.md` (update it in every commit too).
Placeholders only (SERVER01, WS-01, RCU-01, 198.51.100.x). Real site notes go in the
untracked mirror `HANDOVER.local.md` in the repo root (gitignored, never committed).

## Current state (updated 2026-10-30)

- Branch: `main` only (no PRs, no feature branches). Latest tag: **v0.11.0** (built-in checks as plugins, default schedules, gated cutover). Previous: v0.10.1 (`2366819`).
- Windows VM passes: v0.10.1 (installer, service, tasks, agent, plugins, schedules, backup/restore) and v0.11.0 (plugin parity, default schedules, cutover/rollback/fallback, share-offline queue). See Windows VM test plan → Results.
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

- `agent/Itcmdr.Checks.ps1` holds every check (moved, unchanged). `plugins/builtin/` has five plugins over it (network, scanners, config, backup-coverage, services); `Build-BuiltinPlugins.ps1` stages/signs them.
- Agents run only the copies installed with the agent (`{app}\agent\plugins`); they never download plugins. Plugin packages are fetched (plain HTTPS) and run by the server only (decision 16).
- `agent/Itcmdr.PluginRunner.ps1`: run plugins as child processes, `Compare-ItcmdrParity`, `schedule-state.json`, `cutover.json`, task enable/disable (never delete), local fallback. `Set-ItcmdrCutover.ps1 -To task|schedule|auto`.
- Nightly: `agent.pluginMode` auto = plugins report-only first, then the built-in checks, then a `parity` line; after the switch the plugins are the run with per-plugin fallback to the built-in function. run-start carries `trigger`, `cutover`, `scheduleStateTs`, `pluginMode`.
- Server: `cutover.rs` (seed `Nightly agent run` paused + `Nightly backup`, reset-to-default, remembered deletes, gate, switch, rollback, `schedule-state.json` each pass); backup loop replaced by the backup schedule; schema 4 (`parity_runs`, `agent_state`); gRPC `ResetScheduleToDefault`, `ScheduleRef.confirm`, `Schedule.builtin_key/action` (Dart protos regenerated).
- CLI: `schedule reset-to-default`, `schedule rm --force --confirm "<name>"`, `cutover status|vm-verified|switch|rollback`.

## In progress

v0.11.0 released. Next: v0.12.0 (Flutter: plugins, registration, users/keys, metric charts, Backup & Restore, Schedules incl. cutover status, signed requests). Remaining VM items: the app UI on Windows and a real second PC. On site later: a week of parity, then the gated switch.

## Backlog (in order)

1. Windows VM test pass: v0.10.1 and v0.11.0 passes done (results below). Still to do: the app UI on Windows (connect, trigger, settings screen) and a real second PC as agent.
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
| App UI on Windows; a real second PC | ⏳ not done |

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
- Mutating RPCs need the token or (v0.7.0+) a signed request. Run `itcmdr-server init` once per server; the 24 words are
  printed ONCE and are the only way to certify new keys and recover `db.key`. Don't set `auth_mode = signed` until
  the app can sign (v0.12.0), or the app loses access.
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
