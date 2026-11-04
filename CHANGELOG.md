# Changelog

## Unreleased (v1.1.0)
- **Let's Encrypt / ACME for adminsvr's admin API** (off by default; per-install settings: names, contact, staging/production/URL, `tls-alpn-01` on the API port or `http-01` on port 80, cache dir 0700/0600, renew 30 days before expiry). Hot swap without restart; a failed renewal keeps the current certificate and opens the `tls-acme` alert. The certificate is chosen per handshake by name, so enrolled devices (internal pin) and the SiteLink (pinned + mTLS) are unchanged. `itcmdr-adminsvr tls-status`; `Info` carries the mode, names, expiry, last renewal and a renewal-failing flag (`itcmdr-admin info`). Pure Rust (instant-acme on rustls). Tested against Pebble in podman.
- **HTTPS with certificate checks in every app and CLI**: system trust store first (chain, name, expiry); otherwise the certificate's SHA-256, subject and expiry are shown and the user may trust exactly that certificate per server (trust on first use). A changed trusted certificate is blocked with a loud warning until reviewed and replaced; expired or wrong-name certificates need an explicit "trust anyway". Site app: certificate dialog, Settings → Trusted certificates (remove), HTTPS on by default (a pre-1.1 "off" is switched on once; plain HTTP asks first). `itcmdr-admin trust show|review|set|remove`, `--trust-fingerprint`, `--trust-anyway`, interactive prompt on a terminal. Admin app: certificate dialog and Settings → Certificate.
- **The site server API is HTTPS** (with the server certificate agents already pin). Plain HTTP only from the same machine, or from the network with `api_plain_http: true` (server.json). `itcmdr-cli` uses HTTPS pinned to the local server certificate; `--plain-http` is the explicit opt-in.
- HTTPS and plain HTTP are both supported, HTTPS by default; plain HTTP is an explicit opt-in (site app setting, `itcmdr-cli --plain-http`; server `api_plain_http: true`, also `itcmdr-cli setup apply --api-plain-http true`).
- A certificate valid in the system trust store for the right name is accepted even with an older trusted (pinned) certificate; the pin anchors only certificates the system store refuses (CLIs now match the apps).
- Upgrading: v1.0 site apps talk plain HTTP; a v1.1 server needs `api_plain_http: true` until they are updated.

## v1.0.0 (2026-11-02)
itcmdr 1.0: self-healing IT for small offices, with no file share needed.
- **Fresh install with no share**, tested on the Windows VM: server and agent installed without a share or backup folder, the agent enrolled with only `/EnrollToken=` and came online, a backup and the restore drill passed on the local backup folder, and no share path appears in any config, report or log. docs/INSTALL.md says how.
- **Deep-check reminder**: the site keeps the date and result of the last `backup drill --deep` and shows "Last deep check: N days ago" in the site app (Backup page), warning when it never ran, failed or is older than 90 days. Linked sites send only that date and result to adminsvr (`SiteStatus.deep_check_ms/deep_check_result`); `itcmdr-admin site list` and the admin app (Servers) show it and warn after the organization's limit (`OrgPolicy.deep_check_max_days`, default 90; `itcmdr-admin org policy --deep-check-days N`, owners only, audited). No migration (stored as settings on both databases).
- The agent setup check no longer says that Run now needs the share (the Rust agent takes it over its link).
- README rewritten for 1.0 (what each part is, where to get it, docs, privacy and security).
- Versions 1.0.0 (app and admin app 1.0.0+38).
- Known limits: Windows installers are not Authenticode-signed, the macOS builds are not signed or notarized, and the Android admin app is debug-signed. adminsvr's own backup and restore runbook comes after 1.0 (site backups and the restore drill are covered).

## v0.22.0 (2026-11-02)
The share is retired after the switch to the Rust agent, and backups get a restore drill.
- **Share flows retired** once a site has run `itcmdr-cli cutover switch --to agent`: the server stops rewriting `schedule-state.json` (written once by the switch) and `agent-settings.json` (the agents get config over the link), and on-demand runs from the app and CLI go to the Rust agents over the link (`RunNow`) instead of `requests\` on the share. The PowerShell poller no longer reads `requests\` in the agent state (it still keeps the nightly task disabled). `itcmdr-cli cutover rollback` brings all of it back. Sites that have not switched are unchanged. The share stays for backups when `backup.dir` points there, the PowerShell checks/plugins and log rotation; old share logs are still read.
- Rust agents from 0.22.0 take `RunNow` with plugin `*` (every installed plugin). **Older agents get one request per plugin they reported as installed**, so Run now keeps working during a mixed upgrade; an older agent with no reported plugins gets nothing (upgrade it).
- **Restore drill**: `itcmdr-cli backup drill [--deep]` copies the newest archive to a temp folder, checks it (signature chain, this server's master key, hash, manifest, schema; `--deep` decrypts with the words and checks every file), deletes the copy and records the result (audit `backup.drill`). New adminsvr command `backup.drill` (`itcmdr-admin backup drill <site>`, admin app Restore drill button): the site runs it on-site and reports only ok/failed; adminsvr never sees an archive. Allowed by default for links still on the default allowlist. Monthly restore-test runbook in docs/BACKUP.md.
- Fix: after a server upgrade was rolled back, the restored database still showed the upgrade as waiting, so the server staged the same broken release again and looped. The rollback result now marks it failed (Critical `server-upgrade` alert) and it is not retried. Found by the v0.22.0 VM test.
- Admin app desktop builds: Windows (`itcmdr-admin-app-Setup-<v>.exe`, built on Windows) and macOS (`itcmdr-admin-app-<v>-macos-aarch64.zip`, Swift Package Manager, no CocoaPods) are part of every release (docs/RELEASE-LOCAL.md).

## v0.21.0 (2026-11-02)
Self-updating sites: agent and server upgrades with watchdogs and rollback, an update window, approve-once releases from adminsvr, and two-person approval as an organization option (off by default).
- **Two-person approval is now an organization setting, off by default** (owners only, audited `org.policy`; `itcmdr-admin org policy --second-approver on|off`, admin app Plugins page). It covers every action that needed a second owner/admin before (plugin install/enable/disable from adminsvr) and release approvals. The setting is carried signed in each command (`two_person`) and checked by the site. Organizations with two or more active owners/admins keep it on through the upgrade (migration). The site setting `adminsvr.requireTwoPerson` now defaults to false and means "always require two here".
- **Releases from adminsvr, approved once**: an owner or admin signs a one-time approval of an exact server or agent release (name, version, checksum; two people with the setting on); adminsvr attaches the approvals to `release.rollout` commands and the site checks them, the checksum and the owner signature before installing and starting the rollout. New RPCs `GetOrgPolicy`, `SetOrgPolicy`, `ListReleases`, `ApproveRelease`; CLI `org policy`, `release list|approve|rollout [--now]`; admin app Releases list with Approve / Roll out. `release.rollout` is off until the site owner allows it. Migration `000005_release_approvals` (Postgres and SQLite).
- **Automatic updates, on by default, in an update window**: `updates.auto`, `updates.window` (`daily 01:00-04:00`; days `Mon-Fri`/`Sat,Sun`, ranges past midnight belong to their start day), `updates.timezone` (IANA or local; DST-safe). Upgrades and ring advances start only inside the window; push-now (`--now`, adminsvr `release rollout --now`) ignores it. Off per site (`updates.auto=false`) or per machine (`agent.upgrades=false`). The window can be set from adminsvr (`setting.set` now allows these two structured text settings).
- **Server self-upgrade**: `itcmdr-cli upgrade server-start <v> [--now]` / `server-abort`; the server takes a database safety copy, stages the new build and hands it to `itcmdr-server-watchdog`, which swaps, health-checks and rolls back (restoring the database with `upgrade restore-db`); failures raise a Critical `server-upgrade` alert. Agent releases can require a server version (`min_server`); the server goes first.
- Fixes: pre-release versions compare piece by piece (`pre2 < pre10 < rc1`); an agent busy with an upgrade ignores new offers instead of refusing (which paused rollouts); the agent reports the watchdog's result as soon as it appears, not only on the next reconnect; `upgrade status` no longer shows a pushed rollout as waiting for the window.
- Agent releases: a service-plugin package named `itcmdr-agent-release` with a `release.toml` (`version`, `min_from`), signed by an **owner** key (an admin-signed release installs but cannot be rolled out). Releases are never pushed to agents as plugins.
- Rollouts through rings (`agent.ring` per machine: 0 = test VM/PC, 1 = one PC, 2 = everything, default 2): `itcmdr-cli upgrade start|status|pause|resume|abort`. Each ring gets an `UpgradeOffer`; once every agent in it runs the new version and was seen in the last 10 minutes, it soaks for `agent.upgradeSoakMins` (60) and the next ring starts. A failed apply or watchdog rollback, or a new Critical alert on a ring machine, pauses the rollout with a Critical alert. `agent.upgrades = false` holds a machine (or every machine) back.
- The agent checks the offer (server root endorsement that the signer is an owner, the owner's own package signature, every checksum, `min_from`, no downgrade), stages it in `agent\staged\<version>\` and hands it to the new `itcmdr-agent-watchdog` service, which swaps `current\`/`previous\`, waits up to `agent.upgradeHealthSecs` (300) for the new agent to link to the server and rolls back otherwise; the result is reported after the next connect. Between upgrades the watchdog restarts a stopped agent or one whose `health.json` is stale; the agent restarts a stopped watchdog and alerts (`watchdog-missing`). The watchdog is only replaced by the installer.
- Agent installer: the Rust agent now runs from `agent\current\`; adds the `itcmdr-agent-watchdog` service.
- Server watchdog: the server installer adds the `itcmdr-server-watchdog` service (delayed start). The server writes `health.json` every 30 s once gRPC answers and the database opens; the watchdog starts a stopped server and restarts one whose health is stale. It also carries the swap/rollback for server upgrades (used once server upgrades land).
- Site app **Updates** screen: the rollout in progress (ring, soak progress, what holds the ring), Pause/Abort (admin) and Roll out/Resume (owner, signed key only), installed agent releases (admin-signed ones flagged), each machine's ring (changeable), version and last upgrade result, and the rollout history. New RPCs `GetAgentUpgrades`, `StartAgentUpgrade`, `PauseAgentUpgrade`, `ResumeAgentUpgrade`, `AbortAgentUpgrade`.
- App: the navigation rail scrolls when the window is shorter than its entries.
- Settings: `agent.ring`, `agent.upgrades`, `agent.upgradeSoakMins`, `agent.upgradeHealthSecs` (docs/SETTINGS.md).

## v0.20.1 (2026-11-02)
The Rust agent takes over: agent config over the link and a gated `cutover switch --to agent`.
- Agent config over the link (`ConfigUpdate`): the cutover state and the machine's agent settings, endorsed by the server root key. The agent stores it in `config.json` only when the endorsement checks and re-checks it on every load; a tampered or broken file is deleted, the agent stays report-only, raises `config-rejected` and the server resends the config. The server records each agent's acknowledged version (`agent_config`).
- `itcmdr-cli cutover switch --to agent`: refused until the existing gate passes and every machine active in the last two days has a non-revoked Rust agent seen in the last 10 minutes that acknowledged the current config and has Rust-agent parity results with no mismatch. After it, the Rust agent runs plugins for real (not report-only) and the PowerShell agent is retired for scheduled runs, with no fallback (on-demand runs still work until v0.22.0).
- Self-heal: in the agent state the Rust agent (and the PowerShell poller) keep the `itcmdr Agent Nightly` task disabled; re-enabling it by hand is undone and alerted (`nightly-task-reenabled`, raised by whichever heals it first; the poller queues it through `itcmdr-agent alert`). `itcmdr-cli cutover rollback` hands the nightly back to the task, the agent re-enables it and resolves that alert. A missing or rejected `config.json` never re-enables the task (only a signed `task` config does). A repeated `cutover switch --to agent` is a no-op.
- PowerShell: `agent` cutover state in `cutover.json`, `Set-ItcmdrCutover.ps1 -To agent`; upgrades keep the task disabled.

## v0.20.0 (2026-11-01)
Commands, configs and plugins from adminsvr with preview and two-person approval; no CocoaPods.
- Commands down from adminsvr: `setting.set` (numbers, switches and times only; never link, auth, path or text settings), `schedule.run`/`pause`/`resume` (opaque schedule ids), `plugin.install`/`enable`/`disable`. Every command can be a dry run: the site answers with before/after and changes nothing. All kinds stay off until the site owner allows them (`itcmdr-cli adminsvr allow`); restore, keys and auth are never remote.
- Two-person approval: plugin commands wait for a second owner or admin, whose key signs the approval; the site checks it itself (`adminsvr.requireTwoPerson`, default on, site-only), so a compromised adminsvr cannot skip it (tested).
- Global freeze (owners): nothing is sent to any site while it is on.
- Plugin catalog: owners/admins add signed packages to adminsvr (it cannot sign one); sites fetch the package over SiteLink (`FetchPackage`, site-signed, only for a sent install command), validate it as for a local install and quarantine a bad one (only a reason code goes up).
- Documents up: each site sends its configuration (values only for numbers, switches and times) and its plugin list when they change; status adds disk space and 24 h run/plugin-error/alert summaries.
- Admin CLI: `config show|set`, `schedule run|pause|resume`, `plugin catalog|upload|status|push|enable|disable`, `command show|approve|reject`, `freeze status|on|off`, `--dry-run`.
- itcmdr Admin: server Config and Plugins tabs (preview, then apply), Plugins page (catalog, approvals, freeze), freeze banner, 24 h summaries.
- No CocoaPods: both Flutter apps build their Rust bridge with flutter_rust_bridge native assets (cargokit removed) and use Swift Package Manager on iOS/macOS.
- Fix: command arguments are encoded in key order (two-argument commands could fail the request signature).
- Not in this release: agent settings over the link and `cutover switch --to agent` (moved to v0.20.1).

## v0.19.2 (2026-11-01)
The itcmdr Admin app, push notifications for alerts, service scripts for every platform and a tested container image.
- itcmdr Admin (Flutter; Linux, macOS, Windows, Android): onboarding with a one-time code and the 24 words, every request signed by the device key over pinned TLS (the CLI's client through a Rust bridge), passkey touch prompt for step-up actions (desktop security keys via libfido2's tools), alert inbox with acknowledge/resolve, servers with status, configuration, per-machine versions and "update available", metrics history (6 h to 30 days), commands and remote backup start, user management per role (create, roles, disable, SRE scopes, device approval/revocation, passkey reset), step-up policy (owners), audit verify. No web build (gRPC over TLS sockets needs gRPC-web in browsers). The Android APK is signed with a debug key; iOS builds without signing and is not published.
- Push notifications: adminsvr pushes newly opened alerts (cautious and critical by default) over FCM (Android) and APNs (iOS/macOS) to the devices of users who may see the site. Payloads carry only the opaque site and alert ids and the severity; every send is audited; gone tokens are removed. Off until configured (`ITCMDR_ADMINSVR_FCM_SERVICE_ACCOUNT`, `ITCMDR_ADMINSVR_APNS_*`, `ITCMDR_ADMINSVR_PUSH_MIN_SEVERITY`); tested against a local stub, not against Google or Apple (no accounts yet). The app shows in-app banners for new alerts; a push-enabled app build needs the Firebase/Apple setup listed in docs/ADMINSVR.md.
- Metrics history: adminsvr keeps the status each site sends (numbers and enums only) for 30 days; `ListSiteMetrics`, `itcmdr-admin site metrics`.
- Services: systemd (kept), SysV init script (LSB headers, pidfile, start/stop/restart/status) and OpenRC script, chosen by `install.sh --init`; macOS LaunchDaemon with install/uninstall scripts in a new `itcmdr-adminsvr-<ver>-macos-aarch64.tar.gz`; Windows service `itcmdr-adminsvr` with restart-on-failure in the new `itcmdr-adminsvr-Setup-<ver>.exe`. New `--env-file` / `ITCMDR_ADMINSVR_ENV_FILE`.
- Container: the Containerfile and compose file (SQLite by default, Postgres profile) are tested with rootless Podman (`scripts/container-smoke.sh`).
- Fixes: adminsvr now stops on SIGTERM (it only handled Ctrl-C, so service managers had to kill it); empty push/env-file values no longer fail flag parsing; log files are plain text (no color codes); the container image includes libssl3 (needed by the built-in libpq); the adminsvr package README says SQLite is the default.

## v0.19.1 (2026-11-01)
adminsvr on Diesel with SQLite or Postgres, and passkeys / security keys as an optional second factor.
- Database: adminsvr's store moved from sqlx (Postgres only) to Diesel (`MultiConnection`) with embedded migrations for both backends. `ITCMDR_ADMINSVR_DB_URL` picks it: `postgres://…` for Postgres, `sqlite://<path>` (or a path) for SQLite; empty = SQLite at `<data dir>/adminsvr.db` (mode 0600), the new default for small installs. libpq and SQLite are built in (no system libraries). Databases made by v0.19.0 are picked up as they are. The adminsvr tests run on both backends.
- Passkeys (WebAuthn; YubiKeys and other FIDO2 keys) as an optional second factor; the 24-word key stays the signing identity. `itcmdr-admin passkey add|list|remove|reset|lock|unlock` and `policy show|set`. adminsvr checks attestation (`none`, `packed` self or x5c), RP id, origin, user presence (and verification when required) and the sign count.
- Step-up: actions on the owner-editable list (default: role and status changes, user and device enrollment and revocation, key backup retrieval and opt-out, site create/revoke, passkey and policy changes) need a fresh touch from users who have a passkey. The challenge is bound to that exact request (`sha256("itcmdr-stepup/v1" || sha256(method, body) || single-use nonce)`), so a touch can't be replayed or used for another request; every assertion, accepted or refused, is in the audit chain. `policy set --required-roles owner` makes a passkey mandatory per role.
- Unlock: `passkey add --unlock` / `passkey lock` encrypts the CLI's device key with the security key's hmac-secret; every use then needs a touch.
- Lost passkey: the user removes it, or an owner/admin resets it (with their own touch; admins not for owners); console `recover-owner` also clears the owner's passkeys. Audited.
- The CLI drives security keys through libfido2's tools (`fido2-token`, `fido2-cred`, `fido2-assert`; device from `ITCMDR_FIDO2_DEVICE` or the first key found). Platform passkeys (phone, Windows Hello) come with the admin app (v0.19.2). Tested with a software authenticator only: the hardware path (a real YubiKey) is untested in this release.
- New adminsvr settings: `ITCMDR_ADMINSVR_RP_ID` (default: the host of the public API address) and `ITCMDR_ADMINSVR_WEB_ORIGINS`.

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
