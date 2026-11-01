# Changelog

## Unreleased

## v0.19.0 (2026-11-01)
adminsvr core: the offsite admin server, the `itcmdr-admin` CLI and the site-side SiteLink, redacted by default. The SRE app and push move to v0.19.2.
- New `itcmdr-adminsvr` (Linux, Postgres): admin API with the site's signed-request auth (`itcmdr-keys`; device keys certified by the user's 24-word master key), roles owner/admin/sre (new users are sre; admins cannot make or change owners), per-SRE site scopes, device approval, single-use enrollment codes, console `init` and `recover-owner`, and a hash-chained audit of every signed change (`audit-verify`, `itcmdr-admin audit verify`).
- Encrypted user key backup, on by default: the device key is wrapped with a key derived from the 24 words (XChaCha20-Poly1305) and stored on adminsvr; `itcmdr-admin recover` restores it on a new device. `key-backup off` opts out.
- SiteLink: the site server dials adminsvr over pinned TLS + mTLS (`itcmdr-cli adminsvr enroll <token>` in person on the site; `status`, `unlink`). Every message is signed with sequence numbers and acks; heartbeats and an unreachable-site alert on adminsvr.
- Redaction on the site, deny by default: only opaque site-scoped IDs, alert codes built from a fixed vocabulary, check ids and plugin names, severity/state, counts and numeric rollups leave the site. No hostnames, IPs, paths, usernames or message text. Display names only with the per-site opt-in `itcmdr-cli adminsvr share-names on`. Property tests prove it.
- Backups never leave the site: adminsvr sees backup age/size/result and can ask for `backup.start`; there is no restore command. Commands (`backup.start`, `alert.ack`, `alert.resolve`) carry the user's signature and the adminsvr root key's, are checked against the site's local allowlist (`itcmdr-cli adminsvr allow`), the never-remote list and expiry, and are audited on the site.
- Site settings `adminsvr.enabled` (kill switch, default true) and `adminsvr.heartbeatSecs`. Settings 51.
- Packages: `itcmdr-adminsvr-<ver>-linux-x86_64.tar.gz` (binary, systemd unit, install.sh/uninstall.sh, settings example, Containerfile and compose file, untested), `itcmdr-admin-<ver>-<os>-<arch>.tar.gz` (Linux, macOS) and `itcmdr-admin-Setup-<ver>.exe` (Windows). Verify with SHA256SUMS; installers are not code-signed yet.
- Deferred v0.18 items (agent cutover switch, app runs UI) not included.

## v0.18.0 (2026-10-31)
The Rust agent runs plugins and schedules (report-only), with run reports, artifacts and parity against the nightly.
- Plugin push: the server sends the plugins selected by the new setting `agent.plugins` (per machine: `all`, `none` or a comma list of names; default `all`) over the link, endorsed by the root key over the package checksum. The agent verifies the endorsement, name, version and every file checksum before installing, and removes plugins no longer selected or disabled. Each install/removal is reported back; a refused one raises `agent-plugin:<machine>:<name>`.
- Schedules: enabled schedules that name plugins (and the builtin nightly run) are pushed to the agent, which fires them in the schedule's time zone, also while the server is unreachable; a missed fire runs once at start when the schedule catches up.
- `itcmdr-cli agent run <machine> <plugin> [--arg k=v]` (RunNow), `agent runs`, `agent plugins`.
- Agent runs are always report-only. Reports (stages, results, log lines, artifacts up to 1 MiB each / 4 MiB per run) travel through the durable queue, so offline runs are delivered later, once. Logs go to `plugin-logs/<plugin>/agents/<machine>.jsonl`, artifacts to `<data>/agent-artifacts/<run>/`. A failing run raises `agent-run:<machine>:<plugin>`.
- Parity: each agent report is compared with the machine's latest nightly checks (36 h) and recorded as `agent:<run>` in the parity tables.
- New `Prepare` RPC: the agent verifies the site CA and root key before enrolling, so a refused root key no longer leaves an enrolled machine on the server (the token is not used).
- New `Check` RPC: a revoked agent learns it is revoked (it only saw handshake resets before), logs "enroll it again" and checks only every 15 min instead of reconnecting.
- Agent package ships `itcmdr-plugin-ps.exe`. Schema 9; settings 49.

## v0.17.1 (2026-11-01)
Agent certificates renewed over the link, revocation in the handshake, office hours, offline queue.
- Site CA certified: the server root key (certified by the master key) signs the agent site CA (`keys/agent-ca.sig`); enrollment tokens carry the root public key and the agent verifies the server's key and the signature before trusting the CA. v0.17.0 tokens still work.
- Certificate renewal over the link: new setting `agent.certLifetimeMins` (server, default 129600 = 90 days); the agent renews at 2/3 of the lifetime with a new CSR (new TLS key), saves the certificate and reconnects; the replaced certificate is accepted for 24 h. `itcmdr-cli agent renew <machine>` asks for a renewal at the next heartbeat.
- Expiry alerts `agent-cert:<machine>`: Cautious under 14 days, Critical under 3 days, resolved by the renewal.
- Revoked or replaced client certificates are refused in the TLS handshake (own acceptor with a client verifier that checks the serial against the agents table, reloaded every heartbeat pass and on enroll/renew/revoke).
- Office-hours heartbeat profile: `agent.officeHours` (per machine, e.g. `Mon-Fri 07:00-18:00`): no alert outside the hours, misses counted from the start of the hours.
- Offline queue: `itcmdr-agent alert <key> <title> [--level] [--detail] [--resolve]` spools an alert; the service queues it with a durable sequence number and delivers it once (server dedupe + ack), also after an outage; cap `agent.queueMaxMib` (default 256), informational items dropped first and the drop reported.
- agent.proto: `Renew`/`AgentAlert` from the agent, `RenewNow`/`Renewed`/bare acks from the server, `Welcome.queue_max_mib`, `EnrollResponse.ca_signature`/`root_public_key`. Schema 8 (agents: `cert_issued_ms`, `prev_cert_fp`, `prev_serial`, `prev_until_ms`, `renew_requested`, `last_seq`).

## v0.17.0 (2026-11-01)
The Rust agent: enrollment, pinned TLS + client certificates, heartbeats.
- New `itcmdr-agent` (Windows service, also runs in the foreground on Linux/macOS) in the agent package, next to the PowerShell agent, which keeps running the checks. See docs/AGENT.md.
- Enrollment: `itcmdr-cli agent enroll-token` (or the app's Agents page) makes a one-time token with the server address and the pinned server certificate fingerprint; `itcmdr-agent enroll <token>` or the agent installer's `/EnrollToken=`. The agent's Ed25519 key and TLS key stay on the PC (DPAPI); the server signs its CSR with a new site CA (90-day client certificate).
- Agent link on `agent_bind_addr` (server.json, default `0.0.0.0:50052`, firewall rule added by the server installer): TLS with the server certificate pinned by the agent, client certificate required and matched to the agent (mTLS), signed Hello, then Ping/Pong. Backoff with jitter up to `agent.reconnectMaxSecs`.
- Heartbeats: `agent.heartbeatSecs` (60), `agent.lateHeartbeats` (2), `agent.missedHeartbeats` (5), `agent.nonresponsiveLevel` / `agent.serverNonresponsiveLevel` (critical), all per machine overridable. Non-responsive raises `agent-nonresponsive:<machine>`; coming back resolves it. Clock skew over 5 minutes raises a Cautious alert.
- Admin: `ListAgents`, `CreateEnrollToken`, `RevokeAgent`, `SetAgentMaintenance` (admin, audited; the token itself is never logged), `itcmdr-cli agent list|enroll-token|revoke|maintenance`, app Agents page.
- Schema 7 (`agents`, `enroll_tokens`). New crates: `agentlink` (agent.proto, tokens, certificates, pinned TLS) and `agent-rs`.
- Not yet: certificate renewal and expiry alerts, a serial revocation list, the office-hours profile, the offline queue, and plugins/schedules in the Rust agent (v0.17.1 and later).

## v0.16.0 (2026-10-31)
Site alerts.
- New `alerts` and `alert_silences` tables (schema 6). An alert has a level (critical, cautious, informational), a source (check, plugin, backup, package), a machine and a dedupe key. Repeats bump the count and can only raise the level; a resolved alert raised again within `alerts.dedupe_window_mins` (new setting, default 60) reopens instead of adding a row.
- Sources: a failed check raises a cautious alert and a passing one resolves it (a repair that worked is informational; excepted checks raise nothing); plugin errors raise at the plugin's criticality and an ok run of that plugin on that machine resolves them; a failed nightly or manual backup is critical and the next good one resolves it; a quarantined package is cautious.
- RPCs on the Itcmdr service: `ListAlerts` (filters, worst first, open counts for a badge) and `AckAlert` for users; `ResolveAlert` and `SilenceAlerts` (machine and/or source, until a time) for admins. All changes are audited. Silences expire on their own and the alerts come back.
- `itcmdr-cli alerts list|ack|resolve|silence`.
- App: Alerts page (state and level filters, ack, resolve, silence) and a badge on the rail with open critical + cautious alerts.
- Deviation from docs/design/SERVER-AGENT.md: no separate AlertService, no notifier channels and no WatchAlerts yet.

## v0.15.3 (2026-10-31)
Generic names everywhere, and the history scrubbed of vendor and share names.
- History scrub (approved one-time exception): vendor/product names, the share name and the clinic-type site name were replaced in the whole history of itcmdr and itcmdr-plugins (force-pushed, tags re-pointed to the rewritten commits). Existing clones must be re-cloned. The Windows installers of v0.15.0 to v0.15.2 were rebuilt from the rewritten tags and replaced on their releases; short commit ids in the docs were updated to the rewritten commits.
- Current tree generic: docs, tests, example config and the itcmdr-plugins descriptions use neutral names (`DATA`, `ExampleVendor`, `Example Office`).
- Old site.json files keep working without vendor names in code: when `watchedServices` is absent, every object under `services` with a `name` becomes a watched service (with `pluginProcessName` → a Workstation process check `<FIRST-WORD-OF-KEY>-PLUGIN-HEALTH`, otherwise a Server check `SVC-<name>` with its `ports`), and a top-level object with `configFiles` is used as config drift with prefix `<KEY>-DRIFT`. Old check ids are kept. The old vendor-named drift alias is gone; use `Test-ItcmdrConfigDrift`.
- `scripts/scan_repo.py`: `re:<regex>` lines in `config/banned.local.txt` are regular expressions.
- `app/macos/Podfile` committed.

## v0.15.2 (2026-10-31)
No site-specific values in the core, real-break tests, the privacy rules for adminsvr, and a macOS app project.
- Agent checks read every site-specific target from `site.json`; nothing office-specific is built in:
  - `watchedServices[]` (`id`, `name`, `role` Server|Workstation|Any, optional `process`, `ports`, `restart` (default true), `checkId` (default `SVC-<id>`)) replaces the vendor-named service checks. Old `services.*` keys are still read and keep their check ids, but a process name must now come from config.
  - Every entry in `shares` is checked as `SHARE-<KEY>` (before only one fixed share key was checked).
  - `network.serverPorts` (default 445, 53) plus the ports of watched services.
  - `configDrift { files, idPrefix (default DRIFT) }` for vendor config drift (still report-only); the old key keeps its old prefix. `Test-ItcmdrConfigDrift` (old name kept as an alias).
  - `config/site.example.json` uses generic names; the server installer's default `logs_dir` is `C:\ProgramData\itcmdr\maintenanceRuns` (set it with `/LogsDir`).
- Built-in check plugins now send a structured plugin error for each failing check: `RepairFailed` (Critical) when a repair was tried and failed, `CheckFailed` (Cautious) otherwise.
- `ITCMDR_SITE_PATH` pointing to a missing file is now an error instead of a silent fall back to another `site.json`.
- Fault-injection suite: `agent/tests/fault/FaultInjection.Tests.ps1` and `scripts/Invoke-ItcmdrFaultSuite.ps1 -IAmATestBed` (Windows test beds only; generic, configurable targets; restores what it breaks). Covers a stopped and a disabled service, corrupt and missing configs, an unwritable log folder, broken folder permissions, config drift (also a locked file), the agent killed mid-run, and a dropped server link.
- ADMINSVR.md: section 5 and the "what flows" table now match decisions 18 and 19 (no names, paths, message text or backtraces leave the site; command results carry codes, not output text).
- App: `app/macos/` desktop project (shield icon, network client entitlement). Unsigned and not notarized.

## v0.15.1 (2026-10-31)
Windows restore fix, local release builds, and two adminsvr decisions.
- Restore on Windows: the restore now waits until the stopped server has released the database (up to 30 s) and retries the database swap on "access denied" / "sharing violation" (up to 10 s). Before, a service that stopped answering but still held the file made the restore fail with "Access is denied" and no rollback.
- `server/tests/restore_app.rs` runs a real `itcmdr-server` process that the restore helper stops and starts (as with the service), picks up the root key the restore rotates, and always stops its server. The test passes on Windows now; before it only passed on Linux because the in-process server kept using the replaced database. The forced-health-failure test hook also accepts a marker-file path.
- Test safety: before this release `restore_app` used the default restore commands, which on Windows are `sc.exe stop/start itcmdr-server`, so running the test suite on a Windows machine with itcmdr installed stopped and restarted the real service. The test now only stops and starts its own server process.
- Windows clippy: two unnecessary `&mut` in the DPAPI calls (`storage.rs`).
- Local release builds while Actions is unavailable: `scripts/release-local.sh` (`unix`, `check`, `publish`) and `docs/RELEASE-LOCAL.md` (Linux, macOS and Windows VM steps). `check` scans every unpacked artifact for private values and writes `SHA256SUMS`; `publish` uses `gh release` on the existing tag. v0.15.0 was published this way.
- Docs: ADMINSVR.md decisions 18 (site backups never leave the site; adminsvr sees metadata and may start a backup, never restore) and 19 (the SiteLink is redacted by default: opaque site-scoped IDs, codes, counts and rollups; per-site owner opt-in for display names; deny-by-default redaction layer on the site), with required v0.19.0 tests.

## v0.15.0 (2026-10-31)
The plugin error contract and a log of every call.
- New workspace crate `common/` (`itcmdr-common`, no gRPC or server dependency) with `ErrorCriticality` (`critical`, `cautious` (default), `informational`; logged at error, warn and info), `PluginErrorRecord` (error up to 4 KiB, backtrace up to 16 KiB, cut on a character boundary) and the `ServicePluginError` trait. The proc-macro crate `common/derive` (`itcmdr-common-derive`) provides `#[derive(ServicePluginError)]`: the struct needs `error`, `criticality`, `backtrace` and `service_plugin` fields; `#[service_plugin(name = "..", kind = "..")]` sets the plugin and kind; other fields go into the error's context (`#[service_plugin(skip)]` leaves one out). Misuse fails to compile (trybuild tests).
- `proto/plugin.proto`: `PluginError` (service_plugin, kind, criticality, error, backtrace, context) as `StageResponse.error`. The Rust SDK (`itcmdr-plugin` 0.11.0) re-exports the contract and adds `plugin_error(&err)`. Rust plugins that build `StageResponse { .. }` by hand need `..Default::default()`.
- The server logs each plugin error itself as an itcmdr.log/v1 `plugin-error` line with its criticality, and synthesizes one (kind `timeout` or `stage-failed`) when a stage fails without one. Plugins never write that line. Packages can set `default_error_criticality` in `package.toml` (used when a plugin sends none). A package that sets it needs server 0.15.0 or later (older servers refuse unknown manifest keys).
- PowerShell: `New-ItcmdrPluginError` (return it, or throw it, to fail the stage) and `Write-ItcmdrPluginError` (record without stopping; the worst one recorded is sent). An uncaught exception is sent with no criticality, so the package default applies. Existing plugins need no change.
- Call log: every RPC on the itcmdr service is recorded in the new `call_log` table: time, method, kind, peer, verified principal (key name and fingerprint, `token: <actor>`, or `anonymous`), gRPC status, duration, request and response bytes, and the error message. Bodies, words and keys are never logged. Audit rows store the `call_id` of the call that made them. Schema 5.
- Retention: `calllog.retention_days` (30). Each finished day is exported as `event: "call"` itcmdr.log/v1 lines to `<data>/server-logs/calls/calls-<date>.jsonl` before rows are pruned.
- `ListCalls` RPC (admin), `itcmdr-cli calls list|show|prune`, and a Calls page in the app next to Audit. The Audit page shows each row's call id.

## v0.14.3 (2026-10-31)
One package per component, and the approved icon set.
- Installer split into `installer/itcmdr-{server,agent,cli,app}.iss` sharing `common.iss`, `common-setup.iss` and `common-code.iss`. Each package has its own AppId, uninstaller (`{pf}\itcmdr\uninstall\<part>`) and upgrade path. `installer/itcmdr.iss` is gone.
- Packages: `itcmdr-server-Setup-<ver>.exe` (service; keeps a private `itcmdr-cli` for its setup steps so it works without the CLI package; reserved watchdog slot), `itcmdr-agent-Setup-<ver>.exe` (silent, `/SiteName= /ServerHostname= /Share= /EnrollToken=`; the token is stored ACL'd for the v0.17.0 Rust agent; `Push-Install-ItcmdrAgent.ps1` kept), `itcmdr-cli-Setup-<ver>.exe` (on PATH), `itcmdr-app-Setup-<ver>.exe` (with `itcmdr_keys_bridge.dll`). Linux/macOS: `itcmdr-cli-<ver>-<os>-<arch>.tar.gz` (`scripts/package-cli.sh`), `itcmdr-app-<ver>-linux-x86_64.tar.gz`, `itcmdr-app-<ver>-macos-<arch>.zip` (CI, unsigned).
- Upgrade from the combined v0.14.2 installer: each split package takes over its part. Once all recorded components are installed, the old uninstall entry and `unins000.*` are removed; `server.json`, data, keys, the service and signed-only mode are kept. The extra SharedDLLs reference on `Itcmdr.Plugin.psm1` is dropped.
- Uninstall keeps `%ProgramData%\itcmdr` by default. Removal needs a confirmation (or `/REMOVEDATA=yes`); the keys folder needs a second, default-No confirmation (or `/REMOVEKEYS=yes`); backups are never removed.
- CI release: builds and uploads every package plus `SHA256SUMS`; Authenticode signing when the signing secret is set.
- Branding: `branding/` (SVG masters, `build.py`, README, rendered office/admin/shared sets). Office icon wired into the Windows app (`app_icon.ico`), web favicon and Icon-192/512/maskable, the Linux bundle (hicolor icons, `itcmdr.desktop`, window icon name) and every installer (setup icon, uninstall icon). Android/iOS sets and the admin set stay in `branding/` until the mobile projects and v0.19.0. Tray icons stay in `branding/` (no tray usage yet).
- `scripts/scan_repo.py` now scans `.xml`, `.svg` and `.desktop` files; in `.xml`/`.svg` the only URL hosts allowed are the XML namespaces `schemas.android.com` and `www.w3.org` (SVG). Ignored local files (`.idea/`, `testResults.xml`) are skipped.

## v0.14.2 (2026-10-31)
All key code is in one shared crate, `itcmdr-keys`, and the Flutter app now signs and seals with it through flutter_rust_bridge. No behaviour change; the pinned vectors are unchanged.
- New workspace crate `keys/` (`itcmdr-keys`): 24 words and master keys, fingerprints, certificates, escrow, sealed restore words (`itcmdr_keys::restore`), the canonical request message, `sign_request`/`sign_request_at`, `verify_request`, `key_from_seed`, `random_seed`, `sha256_hex`, `now_ms` (also on wasm32). Moved from `server/src/keys.rs` (now a re-export), the crypto half of `server/src/restore_seal.rs` (the one-time key store stays in the server) and request signing/verification in `server/src/auth.rs`. The server, `itcmdr-cli` and `itcmdr-restore` use it.
- Parity tests: `keys/tests/vectors.rs` (request messages, Ed25519 signature, sealed words, pinned master derivation against `proto/*.json`) and `keys/tests/wasm_vectors.rs` (the same under WASM).
- App: `app/rust` (`itcmdr_keys_bridge`, flutter_rust_bridge 2.13.0, built by cargokit via `app/rust_builder`). `app/lib/signing.dart` keeps its API (`AppKey`, `requestMessage`, `sha256Hex`, `sealWords`, `SigningInterceptor`) but all crypto now runs in Rust. The `cryptography` package is gone. `main()` calls `RustLib.init()`. `key_store.dart` (DPAPI and platform storage) is unchanged. Tests load the host build (`cd app/rust && cargo build`, or `ITCMDR_KEYS_BRIDGE_LIB`).
- CI: builds and lints the bridge crate before `flutter test`.
- Web: the `itcmdr-keys` crate and the bridge build for wasm32 (`flutter_rust_bridge_codegen build-web`), and the crate passes the pinned vectors under WASM (`wasm-pack test --node keys`). The on-site app itself is desktop-only and does not compile for web (its `key_store.dart` imports `dart:ffi` for Windows DPAPI, as before), so no web build of the on-site app ships. A Flutter in-browser test (`flutter test --platform chrome`) hung loading `pkg/` and is a follow-up for the SRE app, which targets web.
- Docs (decisions recorded before the build):
  - Key backup defaults: the client-side encrypted key backup on adminsvr is on by default (opt-out), locked with the 24 words. COOP/COEP headers on the adminsvr web host are accepted.
  - Passkeys/YubiKeys (WebAuthn) as an optional second factor, v0.19.1 (no renumbering). The 24-word key stays the signing identity. A passkey unlocks the stored key, and a fresh touch is bound to the signed request's hash for step-up actions (owner-configurable list, optional per-role enforcement). Assertions are audited, recovery is the 24 words plus an owner/admin reset, and the CLI uses a USB key via libfido2.
  - recorded Michael's server/agent/adminsvr decisions in `docs/design/ADMINSVR.md`:
        - configurable adminsvr host
        - rootless Podman for test containers
        - iOS in scope
        - alerts only through the adminsvr app, the CLI (with full parity) or the future adminsvr agent
        - mTLS client certs on every link
        - configurable heartbeats and ports
        - releases owner-signed with an owner/admin second approver
        - no PS agent fallback
        - separate roles
        - remote upgrades on by default through rings
        - Users hidden from SREs
  - Shared `itcmdr-keys` crate (extracted in v0.14.2) for every component, with the Flutter app using it natively and in WASM, so web gets full rights. adminsvr users onboard like site users (24 words and master key, the key stays with the user, optional client-side-encrypted backup on adminsvr with signed retrieval). Owner recovery uses the same 24 words.
  - adminsvr users use the same key-based signed-request auth as the site (reused code, per-device keys, signed-only). Every state-changing action and every push is signed, rejected if unsigned, and logged in a hash-chained audit log with app and CLI views.

## v0.14.1 (2026-10-31)
The check plugins moved to their own repo, github.com/WeirdBrains/itcmdr-plugins (v1.0.1, with history).
- `plugins.pin.json` pins the release (tag + commit). `scripts/fetch-plugins.sh` puts it in `third_party/itcmdr-plugins` (gitignored) and checks the commit.
- The installer, `Push-Install-ItcmdrAgent.ps1`, the agent's source-tree fallback and the parity tests use the pinned copy. ISCC and the tests fail with a clear message when it is missing. Agents still run only the installed copies, and parity/cutover is unchanged.
- Installing on a server: `Install-Plugins.ps1` (PowerShell 5.1) or `install.sh` from itcmdr-plugins (`itcmdr-cli plugins install --path`, `-Disabled`).
- Fix: `itcmdr-plugin-ps` passed a verbatim `\\?\C:\...` script path, so Windows PowerShell 5.1 left `$PSScriptRoot` empty and every check plugin failed at the handshake when the server ran it (`plugins run`). This bug dates from v0.10.0 and was found on the VM. Simple plugins such as disk-free were not affected.
- `agent/Itcmdr.Checks.ps1`, the `plugin/` crate and `Itcmdr.Plugin.psm1` stay here (the agent, the server and the tests need them).

## v0.14.0 (2026-10-31)
Installer and first-run setup. A new office goes from installer to signed-only in one sitting, with no hand-edited JSON.
- `itcmdr-cli setup check|apply|first-run` (`server/src/setup.rs`).
  - `check` verifies the config, the data dir, the log share and the backup folder (they exist and are writable), the initialization, the admin app keys and the auth mode. It saves a report and exits 1 when a check fails.
  - `apply` writes site values into server.json, creating it if needed. With `--keep-existing` it only fills empty values.
  - `first-run` creates the master key and shows the 24 words once, in a real console only. They are never written to a file or log. It asks three words back, then registers the first admin app key (owner), certified while the words are only in memory. On Windows, `--app-key-store` uses or creates the app's DPAPI key (same format as the app). The app signs from the first start.
- Agent role: `agent/Initialize-ItcmdrSite.ps1` writes site name, server name, the maintenanceRuns share and the local log folder into site.json. It keeps existing values unless `-Overwrite`. It checks that the folders exist and are writable and that the server port answers.
- Installer: Site settings and First admin wizard pages, shown on a first install only. Silent parameters `/LogsDir /BackupDir /SiteName /ServerHostname /Share /AdminName`. A post-install "Run first-run setup now" console. Check reports in `%ProgramData%\itcmdr\setup-check.txt` and `agent-setup-check.txt`. Upgrades never replace server.json and only fill empty site.json values.
- VM: clean install (data backed up first, then restored), first-run, signed calls from day one, and an upgrade from v0.13.0 that keeps every setting.

## v0.13.0 (2026-10-31)
App private key encrypted at rest, plugins managed from the app, and the signed-only switch proven on the test VM.
- App key storage (`app/lib/key_store.dart`). Windows: DPAPI for the current user (`%APPDATA%\itcmdr\app-key.dpapi`, own FFI to crypt32, fixed entropy). Linux: the Secret Service keyring via `secret-tool` when it works, else a 0600 file under `$XDG_CONFIG_HOME/itcmdr`. macOS: the login Keychain via `security`. An existing plaintext key in the app preferences is moved into the store on first start, read back, then overwritten and removed. Old plaintext key files are overwritten before they are deleted. The Keys screen shows where the key is stored and warns when the store is not encrypted.
- Plugins from the app: install from an HTTPS URL (the server downloads it; plain http only for loopback) or from a file upload, optionally installed disabled. Enable, disable, remove (an enabled plugin needs a confirmed force), and the quarantine list. New RPCs `InstallPlugin`, `EnablePlugin`, `DisablePlugin`, `RemovePlugin`, `ListQuarantine`. Admin role, signed requests only, audited, same validation and quarantine as `itcmdr-cli plugins install`. The server's maximum message size follows `plugins.max_package_mib`.
- Test VM: switched to `auth_mode = signed` with the app's key in the DPAPI store. Signed calls work, shared-token calls are refused, and the rollback (`auth set-mode both`) was tested. On-site checklist: `docs/INSTALL.md` (Switch to signed-only).
- The installer first-run setup moves to v0.14.0.

## v0.12.0 (2026-10-31)
App screens for keys, schedules, plugins, backup and restore, and signed requests as the default.
Tagged `v0.12.0` (eea653d). The Windows VM pass followed in 2084630.
- App signs every call (`app/lib/signing.dart`). Canonical bytes match `proto/signing_vectors.json`. New installs default to `auth_mode = signed`. An install from before v0.12.0 with no value keeps `both` (`itcmdr-cli auth status` / `auth set-mode`).
- Restore words are sealed on the app for a one-time in-memory server key (`docs/SIGNING.md`). `StageRestore` refuses plaintext words and requires a dry run first. Key and restore calls accept only a signed request.
- Screens: Keys (make, import, certify command, register, revoke), Schedules (preview, edit, pause, run now, history), Service plugins (list, run, metric charts), Backup & Restore (back up, check, dry run, restore), Settings search and bool switches.
- `app/test/e2e_test.dart` runs init, certifies an app key, then WhoAmI, every registry setting, a schedule, a backup and a restore dry run against a local server.

## v0.11.0 (2026-10-30)
Built-in checks as plugins and the nightly run/backup as built-in schedules (decisions 10 and 14). No
install is switched: the SYSTEM task stays the source of truth until `itcmdr-cli cutover switch` passes its gate.
- Check library `agent/Itcmdr.Checks.ps1` (moved out of `Invoke-ItcmdrAgent.ps1`, same code). Five built-in
  check plugins in `plugins/builtin/` call the same check functions, so ids, records, the clinical guard,
  exceptions and report-only rules are unchanged; each check also gets an itcmdr.log/v1 line:
  `itcmdr-network` (IP, server ports, DATA share, printer 9100), `itcmdr-scanners`, `itcmdr-config` (DNS,
  drive maps, printer queues + spooler, ExampleVendor drift report-only), `itcmdr-backup-coverage`, `itcmdr-services`.
  `plugins/builtin/Build-BuiltinPlugins.ps1` stages them (and signs them with `itcmdr-cli plugins pack`).
- **Agents run only the plugin copies installed with the agent** (`{app}\agent\plugins`, also pushed by
  `Push-Install-ItcmdrAgent.ps1`). Agents never download or copy plugins from the share or the network. Plugin
  packages are fetched by the server only (plain HTTPS fetch, `ureq`, then signature + checksum validation).
- Parallel report-only run (`agent.pluginMode` = auto): before the switch every nightly run first runs the plugin
  versions report-only (pre-repair state), then the built-in checks as before, and writes a `parity` line
  (matched, mismatched, errors). After the switch the plugins are the run, and a plugin that can't run falls
  back to its built-in check function (`plugin-fallback` line).
- Default schedules, seeded idempotently on every start/upgrade (and by `itcmdr-cli schedule|cutover`):
  `Nightly agent run` (agent.nightlyTime, all machines, all categories, repair, office-hours guard; paused until
  the switch) and `Nightly backup` (backup.hour, catch-up; replaces the hard-wired backup check). Editable and
  pausable; deleting one needs `--force --confirm "<exact name>"` (gRPC: `force` + `confirm`) and is
  remembered so upgrades don't re-create it; `itcmdr-cli schedule reset-to-default` / `ResetScheduleToDefault`.
  Proto: `Schedule.builtin_key`, `Schedule.action`, `ScheduleRef.confirm`.
- Cutover: `itcmdr-cli cutover status|vm-verified|switch|rollback`. `switch` needs the last 7 full days with
  parity results and no mismatches/errors for every machine that ran, the Windows VM flag, and every recently
  active machine reading `schedule-state.json` (written to the share every scheduler pass). `rollback` is the
  one-command way back. Agents follow the state from the poller: the nightly task is disabled (never deleted)
  after the switch and re-enabled on rollback; an agent reinstall keeps a disabled task disabled.
  `Set-ItcmdrCutover.ps1 -To task|schedule|auto` is the local one-command rollback/pin.
- Local fallback: after the switch, if no nightly ran `agent.fallbackGraceMinutes` (120) after agent.nightlyTime
  and the server is unreachable (`schedule-state.json` older than `agent.serverStaleMinutes`, 15) or nothing
  arrived, the poller runs the nightly locally (`trigger = fallback`). A schedule paused on purpose is respected.
- Ingest: `parity` lines and the agent's cutover view (run-start `trigger`, `cutover`, `scheduleStateTs`,
  `pluginMode`). DB schema 4 (`parity_runs`, `agent_state`).
- Fixes: "local" schedules use the host's IANA zone (a daily 02:00 fired at 01:00 on the fall-back day with
  chrono's Local on Windows); the agent no longer fails off-Windows when checking for SYSTEM;
  `Install-ItcmdrAgent.ps1` no longer reuses the agent's `$Trigger` parameter name (found on the VM).
- Tests: Rust 87 (seeding, defaults vs task settings, delete/confirm/reset, backup schedule, gate, switch,
  rollback, DST); Pester 81 (plugin vs built-in record parity per check family, parity rules, cutover follow/pin,
  server-down fallback, end-to-end runs before/after the switch and with plugins missing).
- Windows VM (PS 5.1): Pester 81/81, upgrade, default schedules, SYSTEM nightly with parity 17/17, schedule
  run-now with trigger=schedule, local switch/rollback, reinstall keeps the task disabled, server-down fallback,
  share-offline queue and flush.

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
  printer queues + spooler, ExampleVendor drift (report only). Skipped check status.

## v0.2.0
- Real ingest, Windows service, ISO-aligned rotation and rollups, on-demand loop, exceptions, auth, audit.
