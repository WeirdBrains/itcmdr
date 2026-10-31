# Admin server (adminsvr) and the SRE app: design proposal

Status: **proposed** (2026-10-31). Nothing is built yet. Extends [SERVER-AGENT.md](SERVER-AGENT.md). Placeholders only
(SERVER01, WS-01, example.local, 198.51.100.x, adminsvr.example.com).

## 1. Three tiers

```
adminsvr (offsite, container)        <- SRE app (Android, iOS, web, desktop)
   ^  one long-lived gRPC stream per site, dialed by the site
site server (SERVER01, in the office) <- on-site app (desktop)
   ^  one long-lived gRPC stream per agent, dialed by the agent
agents (WS-01 ...)
```

Sites are behind NAT, so **the site server dials adminsvr**. This is the same pattern as agent → server:
`SiteLink.Connect(stream SiteMessage) returns (stream AdminMessage)`.

### Site enrollment and mutual identity

1. An adminsvr owner or admin creates the site in the SRE app and gets a **one-time site enrollment token** (24 h).
   The token holds the adminsvr URL, its TLS certificate fingerprint and a secret (only a hash is stored).
2. On the site server, in person: `itcmdr-cli adminsvr enroll <token>`. The site generates its own Ed25519 **site
   key** (DPAPI LocalMachine, `keys/site.key`) and sends its public key, its master public key fingerprint, version
   and site name.
3. adminsvr stores the site key. In return, the site stores and **pins**:
   - adminsvr's TLS certificate fingerprint
   - adminsvr's **command-signing public keys**: one per adminsvr user device key that may approve commands
     (section 6), delivered and refreshed over the link, each signed by the adminsvr root key that the site
     pinned at enrollment.
4. Both directions carry signed messages:
   - **Site → adminsvr:** every message is signed with the site key and has a monotonic `seq`.
   - **adminsvr → site:** every *command* is signed by the user (or users) who approved it **and** by the adminsvr
     root key. The site verifies both against the pinned keys. TLS alone is never trusted for authorization, so a
     stolen adminsvr TLS key cannot issue commands.
5. A site can **unlink** locally (`itcmdr-cli adminsvr unlink`), and adminsvr can revoke a site. Either side ends the
   link at once.

### What flows each way

| Down (adminsvr → site) | Up (site → adminsvr) |
|---|---|
| Server upgrades (signed release packages) | Alerts (all levels, with state changes: ack/resolve made on site) |
| Agent release packages and plugin packages, forwarded into the site's own validation/quarantine | Heartbeat + site status (version, uptime, auth mode, backup age, cutover state, agent counts by state) |
| Commands from the allowlist (section 6): run schedule now, pause/resume a schedule, set a setting, enable/disable a plugin, start a backup, ack/resolve an alert | Metrics **summaries** (rollups, not raw logs) |
| Config documents (settings, schedules) per site, versioned | Command results (`Applied` ok/failed, output summary) |
| Heartbeat pings | Its own audit and call-log summaries (counts, failures) |

### When adminsvr is unreachable

- The site runs **fully standalone**. Nothing in the site depends on adminsvr for its function: backups, agents,
  schedules, plugins, local alerts and the on-site app all keep working.
- **Local paging still works.** The site's own notifier channels (SMTP / ntfy / webhook, SERVER-AGENT.md section 5)
  are the primary path. adminsvr is an additional one.
- Upstream messages go to a durable outbound queue (`adminsvr-queue.db`, capped by `adminsvr.queue_max_mib`, oldest
  Informational dropped first, and the drop itself is reported). The site reconnects with backoff (1 s → 5 min,
  jitter).
- adminsvr marks the site **late / unreachable** using the same heartbeat state machine as for agents and pages SREs
  (a "site down" alert is Critical by default).

### Site-server self-upgrade via adminsvr

- **Release package:** `itcmdr-server-release`, signed by the release key. It needs owner approval in adminsvr (and
  two-person approval, section 6).
- **On the site:** the package is verified against the master public key, a release signer that is a pinned
  **owner**, and the command signatures.
- **Rings:** across sites (ring 0 = the test VM site, ring 1 = one office, ring 2 = all). Each ring must report a
  healthy heartbeat and `setup check` OK for `adminsvr.upgrade_soak_mins` before the next starts.
- **Apply:**
  1. Take a fresh **pre-upgrade backup**.
  2. Stage the release, then hand off to `itcmdr-server-watchdog`, a small service upgraded only by the installer.
  3. The watchdog stops the service, swaps `current`/`previous`, starts it and health-checks it (gRPC up,
     `setup check` OK, adminsvr link back) within 5 min.
  4. If the health check fails, it **rolls back** automatically to `previous` and restores the pre-upgrade DB if the
     schema changed.
- A site can set `adminsvr.allow_upgrades = false` to refuse upgrades entirely (section 6).

## 2. Data model and RBAC

### Tables (adminsvr)

- `users` (id, email, display name, role, status: active/disabled, created_by, created_at)
- `credentials` (user, kind: password | totp | webauthn, data, created_at, last_used)
- `devices` (user, app device key, platform, push token, approved_at, revoked_at)
- `sessions` (user, device, created, expires, last_seen, ip)
- `sites` (id, name, site key, status, version, last_seen, ring, tags)
- `site_scopes` (user, site): which sites an SRE can act on (empty = none)
- `commands` (id, site, kind, payload, requested_by, approvals, state: pending/approved/sent/applied/failed/expired, results)
- `alerts` (mirrored from sites: site, level, key, state, timestamps)
- `metrics_rollup` (site, machine, metric, window, value)
- `audit_log` and `call_log` (same shape as the site's, SERVER-AGENT.md section 7)

### Roles

| Capability | owner | admin | sre |
|---|---|---|---|
| Receive alerts, ack / resolve / silence (scoped sites) | ✅ all sites | ✅ all sites | ✅ scoped sites |
| View sites, health, metrics, audit and call logs | ✅ | ✅ | ✅ scoped sites (audit: own and scoped-site entries) |
| Push config / run allowlisted commands to a site | ✅ | ✅ | ✅ scoped sites, risky ones need approval |
| Approve a pending risky command | ✅ | ✅ | ❌ |
| Push plugin packages to sites | ✅ | ✅ (needs a second approver) | ❌ request only |
| Push server or agent **upgrades** (code) | ✅ (needs a second owner/admin approver) | request only | ❌ request only |
| Create / disable users; set role sre or admin | ✅ | ✅ (not on owners) | ❌ |
| Grant **owner** | ✅ | ❌ | ❌ |
| Edit, disable or remove an owner | ✅ | ❌ | ❌ |
| Scope SREs to sites | ✅ | ✅ | ❌ |
| Enroll / revoke sites | ✅ | ✅ | ❌ |
| Adminsvr settings, notifier channels, kill switch | ✅ | settings and channels only, not the kill switch | ❌ |

Rules and edge cases (enforced on the server; the app only mirrors them):
- New users default to **sre**, with no site scopes until an owner or admin grants them.
- **Admins can't grant owner**, can't change an owner's role, and can't disable or remove an owner. "Promote to
  admin" is allowed. An admin demoting another admin is allowed. An admin promoting themselves is refused.
- **The last owner** can't be demoted, disabled or removed, by anyone including themselves. Two owners are
  recommended.
- **Self-demotion** is allowed (owner → admin, admin → sre) unless it would remove the last owner. It needs
  re-authentication and is audited.
- Every role change, user change, scope change and approval is audited, with the call log id.
- Bootstrap: `adminsvr init` creates the first owner from the console (an invite link with TOTP and passkey
  enrollment). There is no default password.

### Relation to the site roles

Site roles today are `owner`, `admin`, `user`, `client` (+ the proposed `agent`). **Recommendation: keep them
separate; adminsvr roles do not map down.**
- A site sees adminsvr as **one principal**, the link. The link holds a site-local role (`adminsvr`), capped by the
  site's allowlist (section 6).
- The real user behind each command is recorded in the site's audit log (from the command signature), so the site
  can answer "who did this", but it does not grant that user any site role.
- Keeping them separate means a compromised adminsvr account can never act as a site owner. Site owner actions
  (keys, restore with the words, auth mode) stay local, in person.

### User auth

- **Passkeys (WebAuthn) are the primary sign-in.** Password + **TOTP** is the fallback. Owners and admins must have
  a passkey or TOTP (no password-only accounts). Recovery codes are shown once.
- **App device keys:** each app install creates a device key (Android Keystore / iOS Secure Enclave / Windows DPAPI /
  WebCrypto non-extractable for web). An owner or admin approves the device once. API calls are signed with it, the
  same canonical signing as `docs/SIGNING.md`.
- **Sessions:** short-lived access token (15 min) plus refresh tied to the device key (30 days, revocable).
  Step-up re-auth (passkey/TOTP) for risky actions and approvals.
- **Logged:** sign-in success and failure, 2FA events, device approvals and every call (call log).

## 3. The SRE app (built with the adminsvr, Flutter)

### Platforms and build targets

One Flutter codebase, `adminapp/`:
- **Android** (Play internal testing first, then APK sideload for the office)
- **iOS** (TestFlight)
- **Web** (served by adminsvr itself at `/app`, no third-party hosting)
- **Desktop:** Windows (MSIX or Inno, like the on-site app), macOS and Linux (AppImage), as needed

CI builds Android, web and Linux on Linux, and Windows on the test VM. iOS/macOS need a Mac and an Apple developer
account (open question).

### Shared packages with the on-site app

Turn `app/` into a small mono-repo of Dart packages:
- `packages/itcmdr_proto`: generated protos (site + adminsvr)
- `packages/itcmdr_signing`: canonical request signing, sealed words, test vectors (`proto/*_vectors.json`)
- `packages/itcmdr_keystore`: key storage, DPAPI/keyring/Keychain/Keystore/Secure Enclave/WebCrypto
- `packages/itcmdr_ui`: shared widgets (alert list, metric charts, settings registry editor, confirmation dialogs,
  log viewer)

The on-site app (`app/`) and the SRE app (`adminapp/`) stay **separate apps**: different users, servers, threat
models and store listings. They share the packages, not the shell.

### Screens

1. **Sign-in and 2FA:** server URL (pinned on first use), passkey, password + TOTP fallback, new-device approval
   wait screen, recovery codes. Biometric unlock on mobile.
2. **Alert inbox:** grouped by level (Critical / Cautious / Informational), filter by site, machine and state.
   Ack, resolve and silence (with duration), all synced back to the site. Swipe actions on mobile. A detail view shows
   the source plugin or check, the backtrace, the history and the linked metric.
3. **Sites and servers:** a list with health (online / late / unreachable), version, ring, backup age, auth mode,
   agent counts by state and open alerts. The site detail view lists its agents and their heartbeat state.
4. **Pushes (update and config):**
   - Pick the site(s), then the change: a setting, a schedule, a plugin enable/disable or a package push, an agent
     or server upgrade.
   - **Preview / dry-run:** the site computes the diff and validation (and, for schedules, the next runs) and
     returns it without applying.
   - **Confirm:** type the site name for risky actions, then step-up auth.
   - For upgrades and code pushes, a "needs a second approver" request goes to the inbox of eligible approvers.
   - Track: pending → approved → sent → applied / failed per site, with rollback where available.
5. **Metrics explorer:** pick a site, machine and metric. Time range, rollup (day/week/month), compare sites, and
   alert counts as a metric. Built on the shared chart widgets.
6. **Users and roles** (owner/admin only): invite, role picker, site scopes, disable, sessions/devices, revoke.
   - The role picker **does not offer owner to admins**.
   - Owners' rows are read-only for admins.
   - "Demote" on the last owner is disabled with an explanation.
   - Self-demotion asks for confirmation and step-up auth.
   - Every rule is also enforced server-side, and the app shows the server's refusal text.
7. **Audit and call log viewer:** filter by user, site, action, status and time. Export as CSV. SREs only see their
   own entries and those of their scoped sites.
8. **Settings:** profile, 2FA and passkeys, devices, notification preferences (levels, quiet hours, per-site
   mute), theme. Owners also see the adminsvr settings, notifier channels and the kill switch.

### Push notifications

**Recommendation: a self-hosted ntfy (running next to adminsvr) as the default**, with FCM/APNs as an optional
channel. Privacy-friendly: only alert metadata, never PHI (section 5), and no Google/Apple dependency for the server.
Android gets reliable delivery through the ntfy app's UnifiedPush. iOS needs APNs for reliable background delivery,
so iOS users get APNs through an adminsvr-held key if Michael wants iOS (open question). Payloads carry only an
alert id, level and a short title. The app fetches details over the signed API.

## 4. Hosting adminsvr offsite

- **Deployment:** one container image (`itcmdr-adminsvr`, Rust, static binary, distroless), with Compose for
  adminsvr + Postgres + ntfy + a reverse proxy (Caddy, automatic TLS for the web app). The SiteLink gRPC port uses
  adminsvr's own pinned certificate, separate from the web TLS.
- **DB:** **Postgres** (recommended). It handles concurrent users and many sites, row-level scoping, proper
  backups/PITR, and managed options. SQLite/SQLCipher stays the right choice for site servers (one box, no admin).
  Secrets at rest: application-level encryption of credentials and keys, plus disk encryption on the host.
- **Backups:** nightly `pg_dump`, encrypted (age) with an offline key, copied offsite, plus WAL archiving for PITR.
  A monthly restore test, documented.
- **HA expectations:** single instance is fine to start. Sites are standalone and queue, so an adminsvr outage only
  delays visibility and paging through adminsvr. Target: restore within 4 h, an RPO of 24 h (or 15 min with WAL).
  A health check from outside (e.g. a second ntfy/uptime monitor) pages if adminsvr itself is down.
- **Multi-site / tenancy:** one adminsvr serves many sites of **one organization** (single tenant). Multi-tenant
  (several unrelated practices) later, as separate deployments, not shared tables. Simpler and safer for HIPAA.

## 5. Security

adminsvr can push code to every office PC, so it is the highest-value target. Controls:

- **Least privilege + scoping:** SREs act only on scoped sites. Code-changing actions are never SRE-only.
- **Two-person approval** (recommended): server/agent upgrades and plugin pushes need a requester and a *different*
  owner or admin approver. Upgrades need at least one owner. Both signatures travel with the command and the site
  verifies them. Optional per site: `adminsvr.require_two_person = true` (default true).
- **Site-side allowlist** (`adminsvr.allowed_commands`, edited only locally on the site, by a site owner):
  - Default: alert ack/resolve, run schedule now, pause/resume schedule, set non-security settings, enable/disable
    plugins, start backup.
  - Upgrades only if `adminsvr.allow_upgrades = true`.
  - **Never allowed remotely, whatever the allowlist says:** auth mode, key registration/revocation, restore, word
    entry, cutover switch, unlinking safeguards, and the allowlist itself.
- **Kill switches:**
  - adminsvr owners can freeze all pushes globally (a "freeze" banner, commands refused).
  - Each site can set `adminsvr.enabled = false` locally (link closed, nothing accepted). That is one CLI command,
    and the same works from the on-site app for a site owner.
- **No PHI leaves the site.** It's a dental office, so think HIPAA: only **operational metadata** goes up.
  - Allowed up: machine names (placeholders like WS-01 in docs), check ids and statuses, service names, versions,
    metric values (disk, uptime, counts), alert titles built from templates, error messages and backtraces
    **after** redaction, backup sizes/ages, and call/audit counts.
  - Never up: patient names or ids, file names or paths under clinical data folders (e.g. imaging shares), file
    contents, screenshots, practice-software database contents, free-text log lines from clinical apps, user
    documents.
  - Enforced by an **egress filter** on the site: only typed fields from an allowlisted schema go up. Free text
    passes a redactor that drops paths under configured clinical roots and anything matching patient-id patterns. The
    filter's own decisions are logged locally.
  - Plugins declare `phi_safe = true` for fields they promise are clean. The default is not safe, so the field is
    dropped.
- **Code signing chain:** release and plugin packages keep the existing site signature rules (master-certified
  signer). adminsvr can't sign packages on its own, it only forwards them. A compromised adminsvr can therefore at
  most replay already-valid, site-trusted packages, and the allowlist plus two-person approval cover that.
- **Audit everything** on both sides. Each site keeps its own copy of every command it received, with signatures.

## 6. Roadmap (merged with SERVER-AGENT.md)

The site-side contracts come first, then the agent, then the adminsvr MVP, with the SRE app built **in the same
releases** as the adminsvr features it uses.

| Version | Scope | VM / test plan |
|---|---|---|
| v0.15.0 | Error contract (`itcmdr-common` + derive), call log | as in SERVER-AGENT.md |
| v0.16.0 | Site alerts + paging (SMTP, ntfy, webhook); the alert record is also the shape that will go up to adminsvr | as in SERVER-AGENT.md |
| v0.17.0 | Rust agent, enrollment, heartbeats, offline queue | as in SERVER-AGENT.md |
| v0.18.0 | Agent runs plugins + schedules, report-only parity | as in SERVER-AGENT.md |
| v0.19.0 | **adminsvr MVP + SRE app v1:** adminsvr container (Postgres, ntfy), `adminsvr init`, users/roles/RBAC rules, passkey + TOTP, device keys; SiteLink (site enrollment, pinned TLS, signed messages, up: heartbeats/status/alerts, outbound queue). SRE app: sign-in/2FA, alert inbox (ack/resolve/silence), sites list + health, users and roles, audit/call log viewer, settings; web + Android + Linux desktop builds. Dart packages split out of `app/` | adminsvr in a container on alien-blue; the VM site links to it; alerts raised on the VM reach the app; kill the container and check the site queues and still pages locally; RBAC tests incl. the owner rules |
| v0.20.0 | Config over the agent stream + gated agent switch; **adminsvr down-commands:** allowlist, preview/dry-run, two-person approval, freeze/kill switch; metrics summaries up. SRE app: pushes screen (preview, confirm, approvals), metrics explorer | push a setting and a schedule from the app to the VM site; refused when off the allowlist; two-person flow; egress filter tests (PHI patterns dropped) |
| v0.21.0 | Agent self-upgrade (release plugin, rings, watchdog) + **site-server self-upgrade via adminsvr** (pre-upgrade backup, server watchdog, rollback). SRE app: upgrade flow with rings | broken release rolls back (agent and server); ring gating across two sites (VM + a second test site container) |
| v0.22.0 | Retire share flows; Windows desktop and iOS builds of the SRE app (if approved); adminsvr backup/restore runbook + monthly restore test | fresh install with no share dependency; adminsvr restore drill |

## 7. Open questions (merged list, with recommended defaults)

1. **Paging:** site-local SMTP for Critical and Cautious plus ntfy for Critical; adminsvr adds app push.
2. **Who is paged when:** Critical at any time; Cautious in office hours as a digest; Informational in the app only.
3. **Agent and site transport:** pinned TLS plus signed messages; no mTLS in the first version.
4. **Heartbeats:** 60 s interval; non-responsive after 5 missed; per-machine office-hours profiles for PCs switched
   off at night.
5. **Port:** site 50051 for the app and agents; adminsvr on 443 (web) plus 8443 (SiteLink).
6. **Release signing:** only owner keys; two-person approval; rings test VM → one office → all.
7. **PowerShell agent:** keep it as the fallback until the share flows are retired (v0.22.0).
8. **Where adminsvr runs:** a small VPS you control (EU/US per your preference) with Docker Compose. Start single
   instance.
9. **Mobile platforms:** Android + web + Linux/Windows desktop first. iOS only if you want it (needs an Apple
   developer account and a Mac for builds).
10. **Push for the SRE app:** self-hosted ntfy, with APNs only for iOS.
11. **Roles:** keep adminsvr roles separate from site roles; SREs get no site scopes until granted.
12. **Remote upgrades:** off by default per site (`adminsvr.allow_upgrades = false`) until you turn them on at each
    office.
13. **Test container on alien-blue:** Docker 29.9 and Compose are installed, but this user is not in the `docker`
    group, so the daemon refuses the connection, and Podman is not installed. Recommended: add the user to the
    `docker` group (a system change; needs your OK), or install rootless Podman. Until then the adminsvr can run as
    a plain binary with SQLite for tests.
