# itcmdr roadmap

Phases, releases and their status. **Statuses live here only.** The service plugin design (decisions
#1-#11) is in [server/PLAN.md](server/PLAN.md); day-to-day handover notes are in
[docs/HANDOVER.md](docs/HANDOVER.md). Keep this file updated in every commit, like the handover.
Placeholders only (SERVER01, WS-01, 198.51.100.x); real site details go in the gitignored HANDOVER.local.md.

## Status legend

| Status | Meaning |
|---|---|
| ✅ Done | Built, tested on the Linux dev box, tagged and pushed |
| 🔄 In progress | Being worked on now |
| 📋 Planned | Agreed, not started |
| ⛔ Blocked | Can't proceed until something outside the code changes |
| 🧪 Pending test | Built, but still needs testing on real Windows hardware |

Note: every release so far has only been tested on Linux (Rust, Flutter, Pester under pwsh). All of it is
also covered by the Windows VM test pass below.

## Summary

| Version | Title | Status | Tag / commit | Date |
|---|---|---|---|---|
| v0.1.0 | Scaffold: agent, server, app, installer, CI | ✅ Done | `v0.1.0` / `c90fa4b` | 2026-10-29 |
| v0.1.1 | CI without third-party actions | ✅ Done | `v0.1.1` / `adab590` | 2026-10-29 |
| v0.2.0 | Audit fixes batch 1 (real ingest, Windows service, ISO rotation with 4-week blocks, on-demand loop, exceptions, auth, audit) | ✅ Done | `v0.2.0` / `6f3b76b` | 2026-10-29 |
| — | Handover notes + scanner fix (shipped with v0.3.0) | ✅ Done | `c65c6c5` | 2026-10-30 |
| v0.3.0 | Agent checks: scanner, DNS, drive maps, printer queues, Sirona drift | ✅ Done | `v0.3.0` / `db03433` | 2026-10-30 |
| v0.4.0 | Flapping/repeat-offender metrics, charts, log search filters + paging | ✅ Done | `v0.4.0` / `9f9c33f` | 2026-10-30 |
| v0.5.0 | Backup coverage, stale poller locks, request timeouts, gRPC integration test | ✅ Done | `v0.5.0` / `ed548c4` | 2026-10-30 |
| v0.5.1 | Docs: service plugin plan + implementation plan | ✅ Done | `v0.5.1` / `0402dd2` | 2026-10-30 |
| v0.5.2 | Docs: built-in checks as plugins (v0.11.0) | ✅ Done | `v0.5.2` / `b098ff6` | 2026-10-30 |
| v0.5.3 | Docs: plugin logging + structured log standard (decision 11) | ✅ Done | `v0.5.3` / `568b954` | 2026-10-30 |
| v0.6.0 | Storage: data dir, SQLCipher DB, roles/users/clients, migration | ✅ Done | `v0.6.0` / `0ce70c0` | 2026-10-30 |
| v0.6.1 | Docs: this roadmap | ✅ Done | `v0.6.1` / `a4b0eb6` | 2026-10-30 |
| v0.6.2 | Storage follow-up: auto-delete the plaintext backup after a verified migration | ✅ Done | `v0.6.2` / `19c297c` | 2026-10-30 |
| v0.7.0 | Keys & auth: BIP39 master, certified keys, signed requests, roles, db.key escrow | ✅ Done | `v0.7.0` (4afe6c7) | 2026-10-30 |
| v0.8.0 | itcmdr-cli (init, register, users, keys) + backup & restore | ✅ Done | `v0.8.0` (25e232f) | 2026-10-30 |
| v0.8.1 | Configurable paths + backup settings, retention by 4-week blocks | ✅ Done | `v0.8.1` (c1b4388) | 2026-10-30 |
| v0.9.0 | Packages: format, validation, signature, quarantine, plugins install/list/enable/disable/remove, plugins in backups | ✅ Done | `v0.9.0` (073671d) | 2026-10-30 |
| v0.9.1 | Settings everywhere: typed Settings API, central agent settings, `docs/SETTINGS.md`, basic app Settings screen; validated plugins enabled on install | ✅ Done | `v0.9.1` (4e66a71) | 2026-10-30 |
| v0.10.0 | Plugin runtime, log standard, scheduled runs (server, gRPC, CLI), app-triggered restore (server side) | ✅ Done | `v0.10.0` (faa2ae6) | 2026-10-30 |
| v0.10.1 | First Windows VM pass + fixes (BOM in agent logs, installer compile, app exe name, PS 5.1 tests), `itcmdr-cli restore-stage` | ✅ Done | `v0.10.1` (2366819) | 2026-10-30 |
| v0.11.0 | Built-in checks as plugins (parallel report-only, parity); nightly run + backup as built-in schedules; gated cutover, rollback, local fallback | ✅ Done | `v0.11.0` (b249612) | 2026-10-30 |
| v0.12.0 | Flutter: Service Plugins, registration, users/keys, plugin metric charts, Backup & Restore (incl. restore with the words), Schedules, signed requests | ✅ Done | `v0.12.0` (fb007b2) | 2026-10-31 |
| v0.13.0 | App key encrypted at rest (DPAPI / keyring), plugin install/enable/disable/remove from the app, signed-only switch proven on the VM | ✅ Done | `v0.13.0` | 2026-10-31 |
| v0.14.0 | Installer and first-run setup: site values, checks, words shown once, first admin key (signed from day one), upgrade keeps settings | ✅ Done | `v0.14.0` | 2026-10-31 |

## Cross-cutting items

| Item | Status | Notes |
|---|---|---|
| Windows VM test pass | 🔄 Only the second PC is left | v0.10.1, v0.11.0, v0.12.0 and v0.13.0 passes (Pester 82/82, installer, upgrade, task re-registration, both auth paths, Dart tests incl. DPAPI on Windows, signed-only switch and rollback). App click-through on the VM: passed (Michael, 2026-10-31). Still open: a real second PC as agent. Results in docs/HANDOVER.md |
| GitHub Actions installer builds | ⛔ Blocked | Org billing ("payments failed / spending limit"). Workaround in use: the installer builds on the Windows test VM (v0.10.1) |
| Real site values in site.json | 📋 Pending (on-site) | Scanners (incl. the 3D unit + exception), DNS records, drive maps, printer queues, Sirona config paths, stale patterns, backup tasks/destinations. See HANDOVER.local.md |
| Delete the plaintext DB backup after the v0.6 migration | ✅ Done (decided) | Decision 12: deleted automatically after a verified read-back (v0.6.2); kept if verification fails |
| DB key recovery (`keys/db.key`) | ✅ Done (v0.7.0) | Decision 12: `keys/db.key.escrow` sealed for the master key; `itcmdr-server recover-db-key` with the 24 words |
| Backup & restore | ✅ App screen done (v0.12.0) | Decision 13: DB + escrow + config, CLI, RPCs, nightly + retention in v0.8.0; plugins in v0.9.0; app-triggered restore server side in v0.10.0; app Backup & Restore screen (sealed words, dry run first) exercised by `app/test/e2e_test.dart`. Clicked through on the VM (2026-10-31). |
| All hard-coded values as settings | ✅ Done (v0.9.1) | Decision 15 widened: typed settings (`docs/SETTINGS.md`), `ListSettings`/`UpdateSetting`, `itcmdr-cli config`, central `agent-settings.json`. App screen searches the list; bool settings are switches (v0.12.0). |
| Configurable paths | ✅ Done (v0.8.1) | Decision 15: agent `paths.localLogs`/`agentData`, installer LocalLogs, backup settings in the DB via RPC + `itcmdr-cli config` |
| Scheduled runs | ✅ App screen done (v0.12.0) | Decision 14: server scheduler + RPCs + `itcmdr-cli schedule` in v0.10.0; built-in schedules + gated cutover in v0.11.0. App Schedules screen (preview, edit, pause, run now, history) done in v0.12.0. Clicked through on the VM (2026-10-31). |
| Backup retention by 4-week blocks | ✅ Done (v0.8.1) | 7 daily, 4 weekly, 13 four-week blocks (ISO weeks 4N-3..4N) instead of 12 months |
| Plugins enabled on install | ✅ Done (v0.9.1) | A validated package is enabled on install (other versions disabled); `--disabled` opts out; setting `plugins.enable_on_install` (default true); failures still quarantine |
| Signing canonical encoding | ✅ Done (v0.12.0) | `docs/SIGNING.md`, `proto/signing_vectors.json`, `proto/app_vectors.json`. Rust and Dart tests match the same bytes. |
| Retire the shared admin token | 🔄 Default is signed; the VM runs signed-only | New installs use `signed`. An install from before v0.12.0 with no value keeps `both` until `itcmdr-cli auth set-mode signed`. The test VM was switched in v0.13.0; the office server follows the on-site checklist in docs/INSTALL.md. |
| `docs/LOG-STANDARD.md` | ✅ Done (v0.10.0) | itcmdr.log/v1, validator, reject counts, metrics ingest, PS + Rust helpers (decision 11) |

## Phases

### v0.1.0 Scaffold ✅
- [x] PowerShell agent, Rust gRPC server, Flutter app, Inno Setup installer, CI workflows

### v0.1.1 CI without third-party actions ✅
- [x] Workflows rewritten with no third-party actions (rustup, pinned Flutter clone, gh release), because of the org actions allowlist

### v0.2.0 Audit fixes batch 1 ✅
- [x] Idempotent, timestamp-correct ingest incl. nested zips
- [x] Windows service + console mode
- [x] ISO-aligned rotation: weekly → 4-week blocks (B01-B13, week 53 in B13) → ISO yearly, verified manifests
- [x] Metrics/rollups daily, weekly, 4-week block, ISO yearly
- [x] On-demand loop (poller, per-machine locks, responses → request status)
- [x] Exceptions format, validation, export; add/edit/expire in the app
- [x] Token auth for mutating calls + audit log
- [x] PowerShell 5.1 compatibility

### v0.3.0 Agent checks ✅
- [x] Scanner reachability (3D unit allowed via an exception rule)
- [x] DNS records (expected / forbidden addresses)
- [x] Drive maps per logged-on user
- [x] Printer queues + spooler (server and workstation views)
- [x] Sirona drift, detect/report only
- [x] `skipped` check status; `configuration` category in the app

### v0.4.0 Metrics, charts, log search ✅
- [x] Flapping checks and repeat offenders
- [x] Bar charts (incidents, average uptime) in the app
- [x] Log search: category/status/time filters, newest first, paging
- [x] Private handover moved to the gitignored `HANDOVER.local.md`

### v0.5.0 Backup, locks, timeouts, gRPC test ✅
- [x] Backup coverage check (sources, scheduled tasks, destinations), report only
- [x] Stale poller lock cleanup (150 min)
- [x] Request timeouts (`TIMED_OUT`, default 3 h)
- [x] gRPC integration test against a real server

### v0.5.1-v0.5.3 Plugin plan docs ✅
- [x] server/PLAN.md: Michael's plan + agreed implementation plan (decisions 1-9), fixed TOML example (v0.5.1)
- [x] Decision 10: built-in checks as plugins → v0.11.0 (v0.5.2)
- [x] Decision 11: plugin log path, `itcmdr.log/v1`, metrics from logs (v0.5.3)

### v0.6.0 Storage ✅
- [x] Data dir `%ProgramData%\itcmdr` / `~/.itcmdr`, override `--data-dir` > `ITCMDR_DATA_DIR` > `data_dir`
- [x] Layout: `service_plugins/`, `quarantine/`, `keys/`, `plugin-logs/`, `itcmdr.database`
- [x] SQLCipher (bundled, vendored OpenSSL); key in `keys/db.key` (DPAPI on Windows, 0600 on Linux)
- [x] roles / users / clients / server_meta tables (UUID ids, public keys, unique fingerprints)
- [x] Automatic migration of the plaintext DB with row-count verification

### v0.6.1 Roadmap ✅
- [x] This file

### v0.6.2 Storage follow-up ✅
- [x] Read the encrypted DB back after migration; matching row counts → plaintext backup overwritten, deleted, logged
- [x] Backup kept (error logged) when the read-back fails or counts differ

### v0.7.0 Keys & auth ✅
- [x] 24 BIP39 words → master Ed25519 + X25519 (HKDF, pinned test vector); only the master public keys stored; words printed once by `itcmdr-server init`
- [x] Root client keypair at init (role owner, `keys/root`, DPAPI / 0600)
- [x] Client/user keys certified by the master; registration checks the certificate against key, kind, role, name
- [x] Signed requests (fingerprint, ts, nonce, signature), ±5 min window, nonce replay cache, roles owner > admin > user > client
- [x] Revocation (effective immediately)
- [x] db.key escrow under the master key (`keys/db.key.escrow`, X25519 + HKDF + XChaCha20-Poly1305); `itcmdr-server recover-db-key` re-protects it with DPAPI / 0600 (decision 12)
- [~] Retire the shared admin token: `auth_mode = signed` does it; default stays `both` until v0.12.0 (the app can't sign yet)

### v0.8.0 itcmdr-cli + backup & restore ✅
- [x] `itcmdr-cli` (installed next to the server): `init`, `recover-db-key`, `service register --name`, `user add`, `keys certify|new|register|list|revoke|rotate`
- [x] Offline certification from the words (`keys certify` / `keys new` need no data dir; words only on stdin)
- [x] Backup archive: consistent SQLCipher snapshot + db.key escrow + config, encrypted for the master key, signed by the root key, manifest with versions/schema/sha256 (`docs/BACKUP.md`)
- [x] `backup create|list|verify [--deep]|prune`, `restore --from <archive> [--dry-run]` (safety copy, schema check, db.key from the words, new root key)
- [x] RPCs `CreateBackup`, `ListBackups`, `VerifyBackup` (admin/owner, audited); restore is CLI-only (server stopped)
- [x] Nightly backup (server.json `backup`, default 02:00 local) with daily/weekly/monthly retention
- [x] Tests: round trip on a fresh data dir with only the words, tampered/foreign archive rejected, consistent under writes, retention, RPC roles + audit, CLI end to end
- [x] Signing: canonical encoding rules + pinned vectors (`docs/SIGNING.md`); Dart side in v0.12.0

### v0.8.1 Configurable paths ✅
- [x] site.json `paths.localLogs` / `paths.agentData` in every agent script (agent, poller, rotation fallback, backup report); Pester tests
- [x] Installer `/DLocalLogs` (build) and `/LocalLogs` (install) for the local log folder
- [x] `GetSettings` / `UpdateSettings` for backup dir, nightly, hour, retention (admin, audited, folder must exist + be writable); DB overrides beat server.json
- [x] `itcmdr-cli config get|set|unset backup.*`
- [x] Backup retention: 7 daily, 4 weekly, 13 four-week blocks (was 12 months)
- [x] Plans: decisions 14 (scheduled runs, default schedules + cutover) and 15 (configurable paths)

### v0.9.0 Packages ✅
- [x] package.toml (`[[files]]`, `main = true`), exact file list, sha256, admin/owner signature (`package.sig`)
- [x] Safe unpack (no absolute paths, `..`, symlinks, hard links); failures → `quarantine/` + index + log + audit, install denied
- [x] `plugins install --url|--path [--enable]` (v0.9.1: enabled by default, `--disabled`), list, enable, disable, remove, quarantine; install dir `{name}-{version}`; `plugins pack` builds and signs
- [x] Plugin files and the quarantine index in backup archives
- [x] Tests: every rejection case quarantines, management + audit, URL install, backup round trip, CLI end to end

### v0.9.1 Settings everywhere ✅
- [x] Audit every hard-coded value (agent, server, installer, app) → 29 settings with today's defaults (`docs/SETTINGS.md`)
- [x] Typed settings registry (int with range, bool, time, path, text; read-only keys) + `ListSettings` / `UpdateSetting` (validated, admin/owner, audited); `GetSettings` / `UpdateSettings` (backup block) kept
- [x] Runtime wired: ingest interval, request timeout, DB retention, flap/repeat thresholds, clock-skew window, search limits, package size limit
- [x] `itcmdr-cli config list|get|set|unset [--machine]` for every key, audit-logged
- [x] Agent settings served centrally (`agent-settings.json` on the share: global + per machine), site.json as local override, cached in agentData; agent, poller and installer read them
- [x] `docs/SETTINGS.md`: name, default, scope, type/range, when it applies, who can change it
- [x] Basic app Settings screen on the API (server/agent tabs, per-machine view, edit with validation, reset); full section in v0.12.0
- [x] Plugins (Michael, t393u): validated packages enabled on install, `--disabled`, `--enable` kept as a no-op, setting `plugins.enable_on_install`
- [x] Plan: decision 13 gets the app-triggered restore (StageRestore, restore helper, rollback), server side placed in v0.10.0
- [x] Tests: defaults = current behaviour, overrides apply (search, metrics, plugins), validation, RPC roles + audit, agent picks up central settings (Pester)

### v0.10.0 Plugin runtime ✅
- [x] `PluginService` proto (Describe, Verify, Execute, Log), child process + per-launch secret handshake
- [x] `itcmdr-plugin` Rust crate; PowerShell module + `itcmdr-plugin-ps` host (PS 5.1)
- [x] Verify then Execute; failed Verify skips Execute; Log on error; timeouts
- [x] Per-run log path (`ITCMDR_LOG_PATH`, `<data>/plugin-logs/<plugin>/<date>.jsonl`), log/metric helpers
- [x] Hooks into on-demand runs and the audit log
- [x] `itcmdr.log/v1` schema doc, validator, reject counts, generic metrics ingest, `GetPluginMetrics` (decision 11, moved from v0.9.0)
- [x] Scheduled runs (decision 14): schedules table, scheduler queueing via the on-demand path, cron/presets with timezone + DST, clinical guard, max concurrency, missed-run policy, history + next runs
- [x] Schedule RPCs (admin manage, user run-now, audited) and `itcmdr-cli schedule add|list|edit|rm|pause|resume|run-now|history`
- [x] App-triggered restore (decision 13): `StageRestore` (owner, audited, archive verified, words over the authenticated channel, sealing client-side deferred) writes a restore job; `itcmdr-restore` helper stops the service, takes the safety copy, restores, restarts; `GetRestoreStatus` polling; automatic rollback when the health check fails
- [x] `itcmdr-cli plugins run|runs`; plugin logs in the weekly rotation and server backups
- [x] Tests: PS host stages + verify-fail + Log + v1 lines, bin plugins, bad handshake, metrics + rejects, on-demand plugin requests, scheduler (fire, concurrency, missed skip/catch-up, guard, groups, DST), schedule RPC roles + audit + CLI, restore done + forced rollback + roles + wrong words, Pester module + rotation

### v0.11.0 Built-in checks as plugins ✅
Agreed 2026-10-30: agents run only the plugin copies shipped with the agent, never copies pulled from the share or the network; plugin packages are fetched (HTTPS) and run by the server only. A signed share index is deferred.
- [x] Network/connectivity, scanners, config (DNS, drive maps, printers, Sirona drift), backup coverage, services as five plugins over one shared check library
- [x] Pester: plugin versions give the same records as the built-in functions (scanners incl. exception and role filter, Sirona drift, backup coverage, network, services)
- [x] Clinical guard, exceptions, report-only rules unchanged; one itcmdr.log/v1 line per check
- [x] Side-by-side report-only run on every nightly with a `parity` line; switch gated on 7 clean days; built-in functions stay as per-plugin fallback
- [x] Default schedules seeded on every start/upgrade (idempotent); editable/pausable; delete needs owner/admin + `--confirm <name>` and is remembered; reset-to-default
- [x] Safe cutover: task stays source of truth, agent schedule paused; `cutover switch` needs 7 clean parity days + VM flag + agents reading `schedule-state.json`
- [x] After switch: task disabled (kept), `itcmdr-cli cutover rollback` / `Set-ItcmdrCutover.ps1 -To task`; poller runs locally when the server is unreachable
- [x] Tests: Rust (seeding, defaults, delete/reset, backup schedule, gate, switch/rollback, DST) and Pester (parity, cutover, fallback, end to end); Windows VM under PS 5.1
- [ ] Real installs: collect a week of parity, then `cutover vm-verified` + `cutover switch` (on site, by Michael)

### v0.12.0 Flutter ✅
- [x] Service Plugins section: list, run, recent runs, metric charts (install and enable/disable added in v0.13.0)
- [x] Keys: make or import a key, WhoAmI, the offline certify command, owner register and revoke
- [x] Plugin metric charts
- [x] Backup & Restore screen (sealed words, dry run, then restore)
- [x] Schedules screen (preview, edit, pause, run now, history)
- [x] Settings: search, bool switches, client check of every registry key
- [x] App signs requests; default `auth_mode` is `signed` (older installs keep `both`)
- [x] Windows VM: Pester 82/82, installer, in-place upgrade, Dart e2e on Windows (9484fb5). App screens clicked through by Michael (passed).

### v0.13.0 Key storage, plugins in the app, signed-only ✅
- [x] App private key encrypted at rest: DPAPI (Windows), Secret Service keyring or a documented 0600 file fallback (Linux), Keychain (macOS); plaintext key migrated and removed
- [x] Plugins from the app: install from an HTTPS URL or a file upload (optionally disabled), enable, disable, remove, quarantine list; admin, signed, audited
- [x] Signed-only switch on the test VM, with the rollback tested; on-site checklist in docs/INSTALL.md
- [x] App click-through on the VM: passed (Michael)
- [ ] A real second PC as agent (open)

### v0.14.0 Installer and first-run setup ✅
- [x] First-run master setup from the installer, words shown once in a console only (`itcmdr-cli setup first-run`)
- [x] First admin app key registered at first run (DPAPI store on the server PC, or a pasted public key): signed from day one
- [x] Site values (server.json via `setup apply`, site.json via `Initialize-ItcmdrSite.ps1`), server vs agent roles, existence and writability checks (`setup check`)
- [x] Upgrades keep existing settings (VM: server.json byte-identical, site.json unchanged)
- [x] Windows VM: clean install (data backed up and restored), upgrade from v0.13.0
- [x] Windows VM test items for the plugin system are in the handover (v0.10.0 to v0.13.0 results)
