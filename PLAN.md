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
| v0.6.1 | Docs: this roadmap | ✅ Done | `v0.6.1` | 2026-10-30 |
| v0.7.0 | Keys & auth: BIP39 master, certified keys, signed requests, roles | 📋 Planned | — | — |
| v0.8.0 | itcmdr-cli: init, service register, user add, key rotate/revoke | 📋 Planned | — | — |
| v0.9.0 | Packages: format, validation, signatures, quarantine, install/list/enable/disable/remove; log standard + validator + metrics ingest | 📋 Planned | — | — |
| v0.10.0 | Plugin runtime: PluginService gRPC, Rust crate, PowerShell module + host, stages, per-run log path + helpers | 📋 Planned | — | — |
| v0.11.0 | Built-in checks as plugins (same behaviour, no gap in nightly coverage) | 📋 Planned | — | — |
| v0.12.0 | Flutter: Service Plugins, registration, users/keys, plugin metric charts | 📋 Planned | — | — |
| v0.13.0 | Installer: first-run master setup (words shown once); VM test items | 📋 Planned | — | — |

## Cross-cutting items

| Item | Status | Notes |
|---|---|---|
| Windows VM test pass | 🧪 Pending test | Waiting on Michael's VM. Plan in docs/HANDOVER.md → Windows VM test plan; covers every release above |
| GitHub Actions installer builds | ⛔ Blocked | Org billing ("payments failed / spending limit"). Alternative: build the installer on a Windows machine |
| Real site values in site.json | 📋 Pending (on-site) | Scanners (incl. the 3D unit + exception), DNS records, drive maps, printer queues, Sirona config paths, stale patterns, backup tasks/destinations. See HANDOVER.local.md |
| Delete the plaintext DB backup after the v0.6 migration | 📋 Pending (on-site) | `*.plaintext-backup` is left unencrypted next to the database. Decision open: auto-delete or manual |
| DB key recovery (`keys/db.key`) | 📋 Planned (decision open) | Random key, DPAPI machine-bound on Windows. Option: escrow it under the master key in v0.7.0 |
| `docs/LOG-STANDARD.md` | 📋 Planned | Written with v0.9.0/v0.10.0 (decision 11) |

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

### v0.7.0 Keys & auth 📋
- [ ] 24 BIP39 words → master Ed25519 key; only the master public key stored; words shown once
- [ ] Root client keypair at init (`keys/root`, DPAPI / 0600)
- [ ] Client/user keys certified by the master
- [ ] Signed requests (key fingerprint, ts, nonce, signature), replay protection, role-based authorization
- [ ] Retire the shared admin token with a documented migration path

### v0.8.0 itcmdr-cli 📋
- [ ] `init`, `service register [--name]`, `user add`, `key rotate`, `key revoke`
- [ ] Offline master signing / recovery from the words

### v0.9.0 Packages 📋
- [ ] package.toml (`[[files]]`, `main = true`), exact file list, sha256, admin/owner signature
- [ ] Safe unpack (no absolute paths, `..`, symlinks, hard links); failures → `quarantine/` + log, install denied
- [ ] `plugins install --url|--path`, list, enable, disable, remove; install dir `{name}-{version}`
- [ ] `itcmdr.log/v1` schema doc, validator, reject counts, generic metrics ingest, `GetPluginMetrics`

### v0.10.0 Plugin runtime 📋
- [ ] `PluginService` proto (Describe, Verify, Execute, Log), child process + per-launch secret handshake
- [ ] `itcmdr-plugin` Rust crate; PowerShell module + `itcmdr-plugin-ps` host (PS 5.1)
- [ ] Verify then Execute; failed Verify skips Execute; Log on error; timeouts
- [ ] Per-run log path (`ITCMDR_LOG_PATH`, `<data>/plugin-logs/<plugin>/<date>.jsonl`), log/metric helpers
- [ ] Hooks into on-demand runs and the audit log

### v0.11.0 Built-in checks as plugins 📋
- [ ] Network, connectivity, scanner, DNS, drive maps, printers, Sirona drift, backup coverage as signed plugins
- [ ] Existing Pester tests pass against the plugin versions
- [ ] Clinical guard, exceptions, report-only rules unchanged; logging in `itcmdr.log/v1`
- [ ] Side-by-side report-only run, switch-over, one clean week before retiring the old script

### v0.12.0 Flutter 📋
- [ ] Service Plugins section (import, enable/disable, view)
- [ ] App/service registration; user and key management
- [ ] Plugin metric charts

### v0.13.0 Installer 📋
- [ ] First-run master setup in the installer, words shown once
- [ ] Windows VM test items for the plugin system added to the handover
