# Changelog

## v0.10.1 (2026-10-30)
First Windows VM pass (Windows 11, Windows PowerShell 5.1), with fixes:
- Agent logs: Windows PowerShell 5.1 wrote a UTF8 byte-order mark, so the server could not read the first line
  (run-start) and no machines appeared. The agent and rotation now write UTF8 without a byte-order mark, and the
  server ignores a BOM.
- The server no longer reads `plugin-logs/` entries in weekly zips as agent logs.
- Installer: it compiles now. Literal braces are escaped. The rotation task comes from the new
  `Install-ItcmdrRotationTask.ps1` instead of an inline command. Pascal comments are fixed in `[Code]`.
  Binaries come from the workspace's `target\release` (release.yml copies every exe there).
- App: the Windows executable is `itcmdr.exe` (it was `app.exe`, which did not match the installer
  shortcuts).
- Tests: 3-argument `Join-Path` (PowerShell 7 only) was replaced, so Pester passes on 5.1 (65/65).
- New `itcmdr-cli restore-stage --backup NAME [--wait]` (words on stdin; the StageRestore path signed
  with the root key) and `restore-status`.
- Verified on the VM:
  - service install, boot start, upgrade, uninstall and reinstall (data kept)
  - init and keys
  - the nightly task, on-demand runs through the poller, and ingest
  - a plugin Execute through the PS 5.1 host
  - schedules and central settings
  - backup verify --deep and rotation
  - restore with done and forced-rollback runs
- Docs: log standard stage names for the server lines; Windows VM results in the handover.

## v0.10.0 (2026-10-30)
- Plugin runtime (decisions 6, 11): plugins are separate processes serving gRPC `PluginService`
  (`proto/plugin.proto`: Describe, Verify, Execute, Log) on loopback with a per-launch secret
  handshake. Order: Describe, Verify, Execute; a failed Verify stops the run (`verify-failed`), Log is
  called on any failure, every call times out (`plugins.timeout_secs`, 600). Runs recorded in
  `plugin_runs` and audited (`plugin-run`).
- `itcmdr-plugin` Rust crate (SDK + host side + log helpers + validator); PowerShell module
  `agent/Itcmdr.Plugin.psm1` and host `itcmdr-plugin-ps` (Windows PowerShell 5.1 by default,
  setting `plugins.powershell`).
- Per-run log path `ITCMDR_LOG_PATH` = `<data>/plugin-logs/<plugin>/<date>.jsonl`, mirrored to the share's
  `plugin-logs/`; the weekly rotation zips finished weeks; server backups include `plugin-logs/**`.
- Log standard `itcmdr.log/v1` (`docs/LOG-STANDARD.md`): validator, per-file reject counts, metrics
  ingest into the DB, `GetPluginMetrics` (raw or daily, label filters), `ListPluginRuns`, `RunPlugin`.
- Scheduled runs (decision 14), stored in the encrypted DB: targets all / machines / groups (agent
  roles), check categories and/or plugins, report-only or repair, presets (`hourly`, `every Nh`,
  `daily HH:MM`, `weekdays HH:MM`, `weekly <day> HH:MM`) or `cron <5 fields>`, IANA timezone with DST
  handling, office-hours guard (`schedule.office_hours`; report-only-in-hours by default,
  skip-in-hours, none), one active run per schedule, missed runs recorded (skip or catch-up once,
  `schedule.grace_secs`). Runs are queued through the on-demand path. gRPC Create/Update/Delete/
  List/Pause/Resume/RunNow/ListScheduleRuns (admin manage, user run-now/read, audited) and
  `itcmdr-cli schedule add|list|edit|rm|pause|resume|run-now|history`. `TriggerRuns` accepts plugins.
- `itcmdr-cli plugins run NAME [--report-only] [--arg k=v]` and `plugins runs`.
- App-triggered restore, server side (decision 13): `StageRestore` (owner), `GetRestoreStatus`, and
  the `itcmdr-restore` helper (stop, safety copy, restore, start, health check, automatic rollback).
  server.json `restore.stop_command` / `start_command`; setting `restore.health_timeout_secs`.
- Schema version 3 (plugin_runs, plugin_log_files, plugin_metrics, schedules, schedule_runs; pruned by
  DB retention). Installer ships `itcmdr-plugin-ps.exe`, `itcmdr-restore.exe` and the module.

## v0.9.1 (2026-10-30)
- Settings everywhere (decision 15 widened): every tunable value is a typed setting with today's value as its default,
  documented in `docs/SETTINGS.md` (29 keys: backup, ingest interval, request timeout, DB retention, metric thresholds,
  clock-skew window, search limits, plugin package size, agent guard/thresholds/timeouts/paths/times).
- gRPC `ListSettings` / `UpdateSetting` (admin/owner, validated, audit-logged as `settings-update`); per-machine values
  for agent settings. The server applies changes without a restart.
- `itcmdr-cli config list|get|set|unset [--machine PC]` for every key (audit-logged).
- Agent settings are served centrally: the server writes `agent-settings.json` to the maintenanceRuns share (global +
  per machine); the agent, poller and installer use it (cached for offline runs). site.json still wins locally.
- App: Settings tab gains a basic *Server & agent settings* screen (server/agent tabs, per-machine view, edit with
  validation, reset to default).
- Plugins (Michael): a package that passes validation is **enabled on install** (other versions are disabled). New
  `--disabled` flag; `--enable` is still accepted; setting `plugins.enable_on_install` (default true). Failed
  validation still quarantines and denies the install.
- Fix: `itcmdr-cli` closes its database pools before exiting; it could crash intermittently at exit ("double free")
  while SQLCipher connection threads were still closing.
- Plan: decision 13 adds the app-triggered restore (`StageRestore`, restore helper, status polling, health-check
  rollback), server side in v0.10.0, app screen in v0.12.0. v0.9.0 hash recorded.

## v0.9.0 (2026-10-30)
- Service plugin packages (decisions 7, 8): `.tar.gz` with `package.toml` (`[[files]]`, sha256 checksums, one `main`),
  `package.sig` (Ed25519 by an active admin/owner key over the package.toml hash) and exactly the listed files.
- Validation refuses bad gzip/tar/TOML, absolute or `..` paths, symlinks and hard links, missing or extra files, checksum
  mismatches, bad signatures and unknown, revoked or low-role signers. Refused packages go to `quarantine/` with a line in
  `quarantine/index.jsonl`, a server-log warning and a `plugin-quarantine` audit entry; the install is denied.
- `itcmdr-cli plugins install --url|--path [--enable]`, `list`, `enable NAME[@VER]`, `disable`, `remove [--force]`,
  `quarantine`, `pack` (build + sign). Also reachable as `itcmdr-cli service plugins ...`. Installs land disabled by default
  in `service_plugins/{name}-{version}`; state in the new `plugins` table (schema version 2).
- Backup archives now include `service_plugins/**` and the quarantine index; restore copies the current ones aside first.
- Plan: decision 15 widened (every hard-coded value becomes a setting, v0.9.1), audit list recorded; log standard moved to
  v0.10.0; decisions 14 (scheduled runs, default schedules + safe cutover) placed in v0.10.0 / v0.11.0 / v0.12.0.

## v0.8.1 (2026-10-30)
- Backup retention now keeps 7 daily, 4 weekly and **13 four-week blocks** (ISO weeks 4N-3..4N, week 53 in block 13)
  instead of 12 months. server.json key `backup.keep_blocks` (the old `keep_monthly` is still read).
- Configurable paths (decision 15): site.json `paths.localLogs` (default `C:\maintenanceRuns`) and `paths.agentData`
  (default `C:\ProgramData\itcmdr`) are used by the agent, poller, rotation fallback and backup report. Installer:
  `/DLocalLogs=` (build) or `/LocalLogs=` (install).
- Backup settings at runtime: gRPC `GetSettings` / `UpdateSettings` (admin/owner, audit-logged, folder must exist and be
  writable) and `itcmdr-cli config get|set|unset backup.dir|nightly|hour|keep_daily|keep_weekly|keep_blocks`. Stored in the
  DB; they override server.json.
- Plan: decision 14 (scheduled runs; the nightly run and backup become default schedules after a safe cutover) and
  decision 15 (configurable paths).

## v0.8.0 (2026-10-30)
- `itcmdr-cli` (installed next to `itcmdr-server.exe`): `init`, `recover-db-key`, `keys certify|new|register|list|revoke|rotate`,
  `service register`, `user add`. Offline certification from the 24 words (stdin only).
- Backup & restore (decision 13): signed + encrypted `.itcb` archives (consistent DB snapshot, db.key escrow, config,
  manifest with versions/schema/sha256). `backup create|list|verify [--deep]|prune`, `restore --from ... [--dry-run]`.
  Restore takes a safety copy, refuses newer schemas, rebuilds db.key from the words and issues a new root key.
- Nightly backup to `backup.dir` (e.g. the share) with daily/weekly/monthly retention (server.json `backup`).
- gRPC `CreateBackup`, `ListBackups`, `VerifyBackup` (admin/owner, audit-logged).
- Schema version recorded as `PRAGMA user_version` (1).
- `docs/SIGNING.md` + `proto/signing_vectors.json`: canonical protobuf encoding for signed requests, pinned vectors.

## v0.7.0 (2026-10-30)
- Keys: 24 BIP39 recovery words → master Ed25519/X25519 keys (HKDF (SHA-256), pinned test vector). The server stores
  only the master public keys. `itcmdr-server init` prints the words once and creates the root client (role owner,
  private key in `keys/root`, DPAPI / 0600).
- Certified keys: client/user public keys carry a master-signed certificate; registration verifies it; revocation.
- Signed requests: `x-itcmdr-key/-ts/-nonce/-sig` (Ed25519 over method + ts + nonce + body hash), ±5 min window,
  replay cache, roles owner > admin > user > client (trigger = user, exceptions = admin, audit = admin).
- `auth_mode`: `token` | `both` (default, migration) | `signed` (token retired; reads need a key).
- db.key escrow sealed for the master key (`keys/db.key.escrow`); `itcmdr-server recover-db-key [--force]`.
- Plan: decision 13 (backup & restore) placed in v0.8.0 / v0.12.0.

## v0.6.2 (2026-10-30)
- After a plaintext->encrypted migration the encrypted DB is read back at its final path; when row counts match, the `*.plaintext-backup` is overwritten with zeros, deleted and logged; otherwise it is kept (decision 12).
- Plan: decision 12 (backup auto-delete; db.key escrow under the master key planned for v0.7.0).

## v0.6.1 (2026-10-30)
- Docs only: root `PLAN.md` roadmap: status legend, every release with status/tag/date, cross-cutting items, per-phase checklists.

## v0.6.0 (2026-10-30)
- Storage (plugin plan, release 1): data dir `%ProgramData%\itcmdr` / `~/.itcmdr`, override with
  `--data-dir`, `ITCMDR_DATA_DIR` or `data_dir` in server.json; creates `service_plugins/`, `quarantine/`,
  `keys/`, `plugin-logs/`.
- Database is now SQLCipher-encrypted (`itcmdr.database`, bundled SQLCipher + vendored OpenSSL, no system lib).
  Key: `keys/db.key`, DPAPI (LocalMachine) on Windows, 0600 hex file on Linux (refused if group/world readable).
- Automatic migration of the pre-0.6 plaintext DB (`itcmdr.sqlite` in the data dir, or a plaintext file at an
  explicit `database_path`): row counts verified, original kept as `*.plaintext-backup`.
- New tables: `roles` (owner/admin/user/client, UUID ids), `users`, `clients` (public key + unique fingerprint,
  certificate, revoked_at), `server_meta`.
- `database_path` is now optional (override only). Code formatted with `cargo fmt`.

## v0.5.3 (2026-10-30)
- Docs only: plan decision 11, plugin log path + structured log standard `itcmdr.log/v1` + metrics from logs, mapped onto v0.9.0-v0.12.0; wording fix in the release table.

## v0.5.2 (2026-10-30)
- Docs only: plan adds v0.11.0 'built-in checks as plugins'; Flutter moves to v0.12.0, installer to v0.13.0.

## v0.5.1 (2026-10-30)
- Docs only: service plugin plan with the agreed implementation plan and release breakdown (`server/PLAN.md`).

## v0.5.0 (2026-10-30)
- Agent 0.5.0: real backup coverage (category `backup`, server role, report only): expected sources
  exist, backup scheduled tasks present/enabled/recent/result 0, newest file at each destination is
  recent and big enough. `Report-BackupCoverage.ps1` runs the same checks on demand (exit 1 on failure).
- Poller: locks older than 150 min (`-StaleLockMinutes`) are treated as left by a crashed poller and reclaimed.
- Server: requests still queued/running after `request_timeout_secs` (default 3h) become `TIMED_OUT`.
- gRPC integration test (`server/tests/grpc.rs`) against a real server on a free port: ingest, machines,
  search, metrics, auth (missing/wrong token), trigger -> response -> done, exceptions + export, audit, shutdown.

## v0.4.0 (2026-10-30)
- Metrics: flapping checks (`flaps:<machine>:<check>`, >= 3 ok/failed changes in the window) and
  repeat offenders (`repeat_offender:<machine>:<check>`, >= 3 incidents in the window).
- App: real bar charts (incidents per bucket, average uptime) and flapping/repeat-offender tables.
- Log search: category/status/time filters, newest-first ordering, paging (`offset`, `total`, `next_offset`).
- Handover: private notes now live in the gitignored `HANDOVER.local.md` in the repo root.

## v0.3.0 (2026-10-30)
- Agent checks: scanner reachability (3D unit via exception), DNS records, drive maps,
  printer queues + spooler, Sirona drift (report only). Skipped check status.

## v0.2.0
- Real ingest, Windows service, ISO-aligned rotation and rollups, on-demand loop, exceptions, auth, audit.
