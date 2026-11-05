# itcmdr roadmap

Phases, releases and their status. **Statuses live here only.** The service plugin design (decisions
#1-#11) is in [docs/historical/1.0/SERVER-PLAN.md](docs/historical/1.0/SERVER-PLAN.md); day-to-day handover notes are in
[docs/HANDOVER.md](docs/HANDOVER.md). Keep this file updated in every commit, like the handover.
Placeholders only (SERVER01, WS-01, 198.51.100.x); real site details go in the gitignored HANDOVER.local.md.

## Status legend

💡 Proposed (designed, not started).

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
| v0.1.0 | Scaffold: agent, server, app, installer, CI | ✅ Done | `v0.1.0` / `9077a1c` | 2026-10-29 |
| v0.1.1 | CI without third-party actions | ✅ Done | `v0.1.1` / `240c14c` | 2026-10-29 |
| v0.2.0 | Audit fixes batch 1 (real ingest, Windows service, ISO rotation with 4-week blocks, on-demand loop, exceptions, auth, audit) | ✅ Done | `v0.2.0` / `d7fc245` | 2026-10-29 |
| — | Handover notes + scanner fix (shipped with v0.3.0) | ✅ Done | `63cf4f4` | 2026-10-30 |
| v0.3.0 | Agent checks: scanner, DNS, drive maps, printer queues, ExampleVendor drift | ✅ Done | `v0.3.0` / `183495a` | 2026-10-30 |
| v0.4.0 | Flapping/repeat-offender metrics, charts, log search filters + paging | ✅ Done | `v0.4.0` / `e7d3120` | 2026-10-30 |
| v0.5.0 | Backup coverage, stale poller locks, request timeouts, gRPC integration test | ✅ Done | `v0.5.0` / `8ff35c2` | 2026-10-30 |
| v0.5.1 | Docs: service plugin plan + implementation plan | ✅ Done | `v0.5.1` / `9fd0227` | 2026-10-30 |
| v0.5.2 | Docs: built-in checks as plugins (v0.11.0) | ✅ Done | `v0.5.2` / `6e120af` | 2026-10-30 |
| v0.5.3 | Docs: plugin logging + structured log standard (decision 11) | ✅ Done | `v0.5.3` / `297eac3` | 2026-10-30 |
| v0.6.0 | Storage: data dir, SQLCipher DB, roles/users/clients, migration | ✅ Done | `v0.6.0` / `5d88fb7` | 2026-10-30 |
| v0.6.1 | Docs: this roadmap | ✅ Done | `v0.6.1` / `885f2cb` | 2026-10-30 |
| v0.6.2 | Storage follow-up: auto-delete the plaintext backup after a verified migration | ✅ Done | `v0.6.2` / `9f3dc05` | 2026-10-30 |
| v0.7.0 | Keys & auth: BIP39 master, certified keys, signed requests, roles, db.key escrow | ✅ Done | `v0.7.0` (505ec92) | 2026-10-30 |
| v0.8.0 | itcmdr-cli (init, register, users, keys) + backup & restore | ✅ Done | `v0.8.0` (579829a) | 2026-10-30 |
| v0.8.1 | Configurable paths + backup settings, retention by 4-week blocks | ✅ Done | `v0.8.1` (53f3104) | 2026-10-30 |
| v0.9.0 | Packages: format, validation, signature, quarantine, plugins install/list/enable/disable/remove, plugins in backups | ✅ Done | `v0.9.0` (b55592c) | 2026-10-30 |
| v0.9.1 | Settings everywhere: typed Settings API, central agent settings, `docs/SETTINGS.md`, basic app Settings screen; validated plugins enabled on install | ✅ Done | `v0.9.1` (f2f57df) | 2026-10-30 |
| v0.10.0 | Plugin runtime, log standard, scheduled runs (server, gRPC, CLI), app-triggered restore (server side) | ✅ Done | `v0.10.0` (882728e) | 2026-10-30 |
| v0.10.1 | First Windows VM pass + fixes (BOM in agent logs, installer compile, app exe name, PS 5.1 tests), `itcmdr-cli restore-stage` | ✅ Done | `v0.10.1` (333422c) | 2026-10-30 |
| v0.11.0 | Built-in checks as plugins (parallel report-only, parity); nightly run + backup as built-in schedules; gated cutover, rollback, local fallback | ✅ Done | `v0.11.0` (c0748ea) | 2026-10-30 |
| v0.12.0 | Flutter: Service Plugins, registration, users/keys, plugin metric charts, Backup & Restore (incl. restore with the words), Schedules, signed requests | ✅ Done | `v0.12.0` (eea653d) | 2026-10-31 |
| v0.13.0 | App key encrypted at rest (DPAPI / keyring), plugin install/enable/disable/remove from the app, signed-only switch proven on the VM | ✅ Done | `v0.13.0` | 2026-10-31 |
| v0.14.0 | Installer and first-run setup: site values, checks, words shown once, first admin key (signed from day one), upgrade keeps settings | ✅ Done | `v0.14.0` | 2026-10-31 |
| v0.14.1 | Plugins moved to the itcmdr-plugins repo (pinned in `plugins.pin.json`); one-time history rewrite of itcmdr (approved) | ✅ Done | `v0.14.1` | 2026-10-31 |
| v0.14.3 | Separate install packages per component (server, agent, CLI, app), migration from the combined installer, keep-data uninstall, CI per-package release, the icon set | ✅ Done | `v0.14.3` | 2026-10-31 |

### Packaging (v0.14.3 and later)

One package per component, each with its own install, upgrade and uninstall. Uninstall keeps `%ProgramData%\itcmdr` by default; removing data is opt-in (confirmation, or `/REMOVEDATA=yes`), keys only with a second explicit choice (`/REMOVEKEYS=yes`), and backups are never removed. The GitHub release publishes every artifact plus `SHA256SUMS`; Windows packages are Authenticode-signed when the signing secret is set.

| Release | Package | Files |
|---|---|---|
| v0.14.3 | Server (Windows service; reserved slot for the v0.21.0 server watchdog; carries its own `itcmdr-cli` for setup steps so it works standalone) | `itcmdr-server-Setup-<ver>.exe` |
| v0.14.3 | Agent (silent, push-friendly, `/EnrollToken=` stored for v0.17.0; `Push-Install-ItcmdrAgent.ps1` kept) | `itcmdr-agent-Setup-<ver>.exe` |
| v0.14.3 | CLI (on PATH) | `itcmdr-cli-Setup-<ver>.exe`, `itcmdr-cli-<ver>-linux-x86_64.tar.gz`, `itcmdr-cli-<ver>-macos-aarch64.tar.gz` (CI) |
| v0.14.3 | App (with `itcmdr_keys_bridge.dll`) | `itcmdr-app-Setup-<ver>.exe`, `itcmdr-app-<ver>-linux-x86_64.tar.gz`, `itcmdr-app-<ver>-macos-aarch64.zip` (CI, unsigned until the Mac signing step) |
| v0.17.0 | The Rust agent ships in the agent package (same AppId, upgrades the PS agent package) | `itcmdr-agent-Setup-<ver>.exe` |
| v0.19.0 | adminsvr (plain binary package with systemd unit and install/uninstall scripts; Containerfile + compose file, untested), admin CLI (Windows, macOS, Linux) | `itcmdr-adminsvr-<ver>-linux-x86_64.tar.gz`, `itcmdr-admin-<ver>-<os>-<arch>.tar.gz`, `itcmdr-admin-Setup-<ver>.exe` |
| v0.19.2 | admin app (Linux/macOS archives, Windows installer, Android APK; no web build), adminsvr service packages (SysV/OpenRC in the Linux tarball, macOS LaunchDaemon tarball, Windows service installer), tested container image | `itcmdr-admin-app-<ver>-*`, `itcmdr-adminsvr-<ver>-macos-aarch64.tar.gz`, `itcmdr-adminsvr-Setup-<ver>.exe` |
| v0.20.0 | same packages; the Flutter apps build their Rust bridge with flutter_rust_bridge native assets (cargokit removed) and use Swift Package Manager on iOS/macOS (no CocoaPods); Mac builds refuse a Podfile |

Upgrading from the combined v0.14.2 installer: each split package takes over its part; once every component the old installer recorded is installed as a split package, the old uninstall entry is removed (files, data, keys, the service and signed-only mode are untouched).

Branding (v0.14.3): `branding/` holds the SVG masters, `build.py` and the rendered sets. The office icon is wired into the Windows app, the web icons, the Linux bundle (hicolor icons + `itcmdr.desktop`) and every installer. Android `res/` and the iOS `AppIcon.appiconset` (office and admin) are wired when `flutter create --platforms android,ios` runs (the mobile store work); the admin set is for the v0.19.0 SRE app. Tray/notification monochrome icons stay in `branding/` (no tray usage yet).

### Proposed: server, agents and adminsvr (docs/historical/1.0/design/SERVER-AGENT.md, docs/historical/1.0/design/ADMINSVR.md; decisions recorded 2026-10-31; not started)

Key backup on by default (opt-out), locked with the 24 words; COOP/COEP accepted. Pending: Mac build/signing step for iOS/macOS; adminsvr agent design (Michael); rootless Podman on alien-blue (needs sudo).

| Version | Scope | Status |
|---|---|---|
| v0.14.2 | Extract `itcmdr-keys` (key generation, 24 words/master, backup, recovery, signing/verification, vectors) from `server`; the Flutter app uses it via flutter_rust_bridge (native + WASM), replacing the crypto in `app/lib/signing.dart`; parity tests against the existing vectors | ✅ Done (native; web: crate + bridge build for WASM and pass the vectors, on-site app stays desktop-only) |
| v0.15.0 | `itcmdr-common` (ErrorCriticality, ServicePluginError + derive, backtrace), plugin-error in plugin.proto and itcmdr.log/v1, PS `New-ItcmdrPluginError`, call log of every RPC | ✅ Done (`v0.15.0`, 2026-10-31) |
| v0.15.1 | Windows restore fix (wait for the stopped server to release the DB, retry the swap), `restore_app` on a real server process, local release builds (`scripts/release-local.sh`, docs/RELEASE-LOCAL.md), ADMINSVR decisions 18 and 19 | ✅ Done (`v0.15.1`, 2026-10-31) |
| v0.15.2 | Site-specific values moved to site.json (watchedServices, shares, serverPorts, configDrift), structured errors from built-in plugins, fault-injection suite on the VM, ADMINSVR privacy rules aligned, app macos/ project | ✅ |
| v0.15.3 | Generic names in docs, tests, example config and plugin descriptions; legacy site.json read by a generic rule; history of itcmdr and itcmdr-plugins scrubbed of vendor/share names (one-time, approved) | ✅ |
| v0.16.0 | Site alerts (Critical / Cautious / Informational; dedupe, ack, resolve, silence), alert RPCs on the Itcmdr service (ListAlerts/AckAlert/ResolveAlert/SilenceAlerts; no separate AlertService or WatchAlerts yet), app Alerts screen + badge, `itcmdr-cli alerts`, delivered in the on-site app and CLI (no external notifiers) | ✅ |
| v0.17.0 | Rust `itcmdr-agent` service (on every PC and the office server, in the agent package): enrollment with one-time tokens, per-agent Ed25519 keys, pinned TLS + mTLS client certs from a site CA, signed Hello, agent-initiated stream, heartbeats (per machine configurable) + late/non-responsive alerts, maintenance, revoke, `itcmdr-cli agent`, app Agents screen (docs/AGENT.md) | ✅ |
| v0.17.1 | Agent certificate renewal over the link + expiry alerts, revocation by serial in the handshake, site CA certified by the root key (certified by the master key), office-hours profile, offline alert queue (`itcmdr-agent alert`) | ✅ |
| v0.18.0 | Agent runs plugins and schedules, RunNow, run reports and artifacts, report-only parity vs the nightly | ✅ Released |
| v0.19.0 | adminsvr core (released; the SRE app, FCM/APNs push and a tested container image moved to v0.19.2): adminsvr MVP (configurable host, rootless Podman compose, Postgres, mTLS, `itcmdr-admin` CLI with app parity, FCM/APNs push, users/roles, key-based signed-request auth (same as the site), signed + hash-chained audit of every change and push, site enrollment, alerts/status up) + SRE app v1 (Users and Servers categories, alert inbox, sign-in, settings); site backups never leave the site (ADMINSVR.md decision 18: metadata and the allowlisted start-backup command only, no restore from adminsvr; required tests: no backup bytes or key material on the link, a restore from adminsvr is rejected; SiteLink redacted by default (decision 19: opaque site-scoped IDs, codes, counts and rollups only; per-site owner opt-in for display names; site-side deny-by-default redaction layer; required property/fuzz test that no site hostnames, IPs, paths or usernames leave)) | ✅ Released |
| v0.19.1 | adminsvr DB layer on Diesel with Postgres and SQLite (SQLite default for small installs); Passkeys/YubiKeys (WebAuthn) as an optional second factor over the 24-word identity: key unlock, step-up bound to the signed request hash, assertions audited, owner-configurable step-up list, per-role enforcement, recovery with the 24 words plus an owner/admin reset, CLI via libfido2 | ✅ Released (hardware keys untested; software authenticator in tests) |
| v0.19.2 | (scope approved by Michael 2026-11-01; container image tested with podman once he has installed it) SRE app v1 (Users and Servers categories, alert inbox, sign-in, settings; Android/iOS/desktop/web builds), FCM/APNs push, container image tested with rootless Podman | ✅ Released (push tested against a local stub; no Firebase/Apple accounts yet; iOS unsigned build check only) |
| v0.20.0 | Agent config over the stream + gated switch (PS agent retired, no fallback); adminsvr commands down (allowlist, preview, two-person approval, kill switch), metrics summaries up; SRE app Service Plugins category, Servers configs + metrics; adminsvr as plugin source of truth | ✅ Released 2026-11-01 without the agent switch (moved to v0.20.1) |
| v0.20.1 | Agent config over the link (root-key endorsed `ConfigUpdate`), gated `cutover switch --to agent` with rollback, PS agent retired (no fallback), nightly-task self-heal | ✅ Released 2026-11-02 |
| v0.21.0 | Agent + server self-upgrade (rings, watchdogs, rollback), update window with push-now, approve-once releases from adminsvr, two-person approval as an org option (off by default); Updates screen | ✅ Released 2026-11-02 |
| v0.22.0 | Retire the share flows; SRE app Windows/macOS builds; adminsvr restore drill | ✅ Released 2026-11-02 (restore drill on-site only) |
| v1.0.0 | Product website (GitHub Pages from a separate `site` branch, the one allowed branch, site only; custom domain itcmdr.weirdbrains.com; Weird Brains branding), launch announcement (X and the internal Weird Brains Slack, drafted for Michael's review, never posted by us), 1.0 stabilization (nothing published or announced without Michael's review; no office-specific names, IPs or domains on the site) | ✅ Released 2026-11-02 (site branch updated, not live until Pages/DNS are enabled; launch drafts for Michael's review, not posted) |
| v1.1.0 | HTTPS everywhere, Let's Encrypt for adminsvr (TLS-ALPN-01), self-signed trust pins, docs as man pages and in-app help | ✅ Released 2026-11-04 |
| v1.2.0 | Real app ids (office app `com.weirdbrains.itcmdr`), adminsvr's own encrypted backup, restore and restore drill | ✅ Released 2026-11-04 (v1.2.1 installer logs) |
| v1.3.0 | Installer log pruning, admin app Deep backup checks screen, VM end-to-end check script with real breaks | ✅ Released 2026-11-05 |
| v1.3.1 | Release guard: every asset built from the tag (build stamp with commit and help hash, checked before publishing) | ✅ Released 2026-11-05 |
| v1.3.3 | Info, Check and revoked-agent handling; compatibility notes for older Info callers and agents | ✅ Released 2026-11-05 |
| v1.3.2 | Windows parts run without the Visual C++ runtime (static CRT, app-local runtime DLLs for the apps); demo workflow | ✅ Released 2026-11-05 |

## Cross-cutting items

| Item | Status | Notes |
|---|---|---|
| Windows VM test pass | 🔄 Only the second PC is left | v0.10.1, v0.11.0, v0.12.0 and v0.13.0 passes (Pester 82/82, installer, upgrade, task re-registration, both auth paths, Dart tests incl. DPAPI on Windows, signed-only switch and rollback). App click-through on the VM: passed (Michael, 2026-10-31). Still open: a real second PC as agent. Results in docs/HANDOVER.md |
| GitHub Actions installer builds | ⛔ Blocked | Org billing ("payments failed / spending limit"). Workaround in use: the installer builds on the Windows test VM (v0.10.1) |
| Real site values in site.json | 📋 Pending (on-site) | Scanners (incl. the 3D unit + exception), DNS records, drive maps, printer queues, ExampleVendor config paths, stale patterns, backup tasks/destinations. See HANDOVER.local.md |
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

The finished phases v0.1.0 through v1.0.0 (every checklist), the plugin plan and the design documents are
archived in [docs/historical/1.0/](docs/historical/1.0/) (PLAN.md, SERVER-PLAN.md, design/, CHANGELOG.md).

### v1.1.0 HTTPS everywhere, Let's Encrypt for adminsvr ✅
- [x] adminsvr ACME (Let's Encrypt), off by default: per-install domains/contact/challenge (TLS-ALPN-01 on the API port or HTTP-01), staging/production, cache dir 0700/0600, renewal ~30 days before expiry with hot reload, fail-safe (keeps the current certificate, opens alert `tls-acme`), `itcmdr-adminsvr tls-status`; enrolled devices and the SiteLink keep the pinned internal certificate
- [x] Apps and CLIs: HTTPS with system-trust validation; otherwise review and trust exactly that certificate per server (TOFU), changed certificates blocked until reviewed, expired/wrong-name need "trust anyway", trusted certificates listed and removable; no global ignore switch
- [x] Site server API on HTTPS (plain HTTP only from loopback or with `api_plain_http`), `itcmdr-cli --plain-http` opt-in
- [x] Tests hold under heavy load: wall-clock deadlines, API/ACME readiness retries, the server joins its DB workers on stop (no exit-time SIGSEGV)
- [x] Full suite green on 2026-11-04 (fmt, clippy, Rust 200 on SQLite+Postgres, Pester 105/10 skipped, Flutter 50+1 skipped/29); main pushed, not tagged
- [x] Docs to the Muninn standard: a guide per feature with a docs/README.md index, the same docs as in-app help
  (`itcmdr-cli help`, `itcmdr-admin help`, Help in both apps), generated man pages with every flag and env var,
  deploy guides (site server, adminsvr with Let's Encrypt, website), finished plans in docs/historical/1.0/
- [x] Windows VM check (2026-11-04): upgrade over 1.0.0, HTTPS on 50051 with the pinned certificate, plain HTTP from loopback only (network only with `api_plain_http`), adminsvr self-signed trust pins (pinned info/enroll, changed certificate blocked, review + replace, wrong fingerprint refused), real breaks (killed server, watchdog, both, agent: all came back; agent online again); agentlink trust tests on Windows
- [x] Released v1.1.0 (19 files: 7 Windows, 5 Linux, 6 macOS/Android, SHA256SUMS)

### v1.2.0 App ids, adminsvr's own backup ✅

- [x] Office app id `com.weirdbrains.itcmdr` (Linux, macOS, Windows); settings migration on Windows/Linux; admin app ids unchanged (already `com.weirdbrains.*`)
- [x] adminsvr backup: init/create/list/verify/drill/prune/status, restore; encrypted for adminsvr's recovery words; SQLite and Postgres; real-break tests
- [x] Mac build check (macOS, Android; Swift Package Manager), Linux build, Windows VM check (upgrade over 1.1.0, app id + settings migration, adminsvr backup/drill/restore and real breaks), released v1.2.0 (19 files)

### v1.3.0 Installer log pruning, deep-check screen, VM e2e ✅

- [x] Installer keeps the newest 20 logs per part (never `last-<part>.json`, never the current run)
- [x] Admin app Deep backup checks screen (date, ok/failed, overdue warning; widget tests)
- [x] VM end-to-end check (upgrade 1.2.1 -> 1.3.0, pruning on real runs, real breaks heal), full suite, release (19 files)
