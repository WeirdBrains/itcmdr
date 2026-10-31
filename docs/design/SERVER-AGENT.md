# Server and agents: design proposal

Status: **proposed** (2026-10-31). Nothing here is built yet. Placeholders only: SERVER01, WS-01, example.local, 198.51.100.x.

The admin server tier (offsite adminsvr, the SRE app, site upgrades from adminsvr) is in [ADMINSVR.md](ADMINSVR.md). Its roadmap and open questions supersede sections 10 and 11 below.

## Goal

One **server** on the main server and one **agent** on every PC that should be watched.

- The server backs up, owns plugins, configs and schedules, and pushes them to the agents. It tracks heartbeats and
  raises alerts (Critical, Cautious, Informational), and it pages admins.
- Agents run plugins on schedule and report back: run results, artifacts, status and errors. They also answer
  heartbeats.
- Everything goes over gRPC, and every call is logged.

Today's design is share-based: agents write logs, read requests and read settings on the `maintenanceRuns` share,
and the server ingests what lands there. This proposal replaces the share with an agent-initiated gRPC stream. It
keeps every current safety rule: signed packages only, agents never download plugins from the share, the clinical
guard, report-only, and the gated cutover.

## 1. Gap table

| Spec item | State | Today (files) | What is missing |
|---|---|---|---|
| Server does all the backups | **Done** | `server/src/backup.rs`, `jobs.rs`, the `Nightly backup` schedule (`schedule.rs`), `itcmdr-cli backup`, the app Backup & Restore screen | Back up the new tables (agents, alerts, call log). Nothing else. |
| Push plugin updates to agents | **Partly** | Server side: packages, signature, quarantine and enable-on-install (`packages.rs`, `InstallPlugin` RPCs, the app). Agents get plugins only through the agent installer (`installer/itcmdr.iss` → `{app}\agent\plugins`, `Push-Install-ItcmdrAgent.ps1`). | A per-agent desired plugin set, delivery over the stream, verification on the agent, and an applied/failed ack. |
| Push config updates | **Partly** | `agent-settings.json` on the share (global + per machine), read by the agent and poller (`Get-ItcmdrSetting`, `Poll-ItcmdrRequests.ps1`); typed settings registry (`settings.rs`, `docs/SETTINGS.md`) | A versioned config document per agent, pushed over the stream, acked, and visible in the app. |
| Push schedules | **Partly** | Server scheduler (`schedule.rs`) runs the server's plugins. It drives the agents only through `schedule-state.json` on the share and the nightly task (`cutover.rs`, `Set-ItcmdrCutover.ps1`). | Agent-targeted schedules (all machines / a list / a tag), delivered to the agent, which runs them locally even when offline. |
| Push heartbeats / agents answer | **New** | Closest today: `agent_state` (from `schedule-state` lines in ingest), the poller's last run, the parity gate's "recently active machines" | Heartbeat ping/pong on the stream, `last_seen`, a missed counter, a state machine. |
| Missed-heartbeat threshold → non-responsive + alert | **New** | none | Settings, state machine, alert rule (section 4). |
| Alert endpoint for paging admins | **New** | none (the app's WatchUpdates stream is the only push) | Alert model, notifier channels, app/CLI screens (section 5). |
| Alert levels Critical / Cautious / Informational | **New** | Check records have `severity`/`status` (`Itcmdr.Checks.ps1`, `ingest.rs` → `checks`, `incidents`); log `level` debug/info/warn/error (`docs/LOG-STANDARD.md`) | `ErrorCriticality`, and a mapping from check severity and log level. |
| Agent errors raised as alerts at the plugin's level | **New** | Plugin errors are only logged (`plugin_logs.rs`, `plugin_runs`), and the agent's errors are only in its local log | The plugin error contract (section 6) and an error → alert rule. |
| Agent listens / runs plugins on schedules | **Partly** | PowerShell agent `Invoke-ItcmdrAgent.ps1` (nightly task), `Itcmdr.PluginRunner.ps1` (runs installed plugin copies), poller task (every few minutes, requests from the share) | A resident agent with a local scheduler and a stream to the server. |
| Agent reports artifacts, status, errors | **Partly** | itcmdr.log/v1 lines written to the share, ingested by `ingest.rs` / `plugin_logs.rs`; zipped logs rotated by `Rotate-ItcmdrLogs.ps1` | Run reports and artifact upload over gRPC, with an offline queue. |
| All comms over gRPC | **Partly** | App ↔ server is gRPC with signed requests (`docs/SIGNING.md`, `auth.rs`); server ↔ plugin is gRPC (`proto/plugin.proto`, `plugin/src/host.rs`, `itcmdr-plugin-ps`) | Agent ↔ server goes over the share today. |
| Every call, its kind and status logged | **Partly** | `audit_log` covers state-changing calls (keys, plugins, settings, restore). Request tracing goes to the server log. | A call log of every RPC (section 7). |
| Upgrade path via a release plugin | **New** | Agents upgrade only by rerunning the installer (`Push-Install-ItcmdrAgent.ps1`) | Section 8. |
| Shared error library `ErrorCriticality` + derive | **New** | `plugin/` crate (SDK + host), `agent/Itcmdr.Plugin.psm1` | Sections 6 and 10. |

## 2. The agent

### Recommendation: a Rust Windows-service agent that hosts the existing PowerShell plugins

Build `itcmdr-agent` as a Rust binary running as a Windows service (LocalSystem; the same `windows-service` crate as
the server). The **plugins stay PowerShell**. The agent starts them through the existing `itcmdr-plugin-ps` host and
`PluginService` (`proto/plugin.proto`), exactly as the server does today (`runtime.rs`). `Itcmdr.Checks.ps1` and the
five check plugins keep running unchanged under Windows PowerShell 5.1.

Why not keep the PowerShell agent:
- A long-lived bidirectional gRPC stream, reconnect with backoff, a durable offline queue and a heartbeat responder
  are all hard to do reliably in PS 5.1. There is no maintained gRPC client for 5.1, and scheduled tasks are not
  resident.
- Self-upgrade needs a process that can stop, replace and restart the thing being upgraded, and roll it back. A Rust
  service plus a tiny watchdog service can do that. A script cannot upgrade the script that runs it.
- The server already has every piece the agent needs, in Rust: plugin hosting, package verification, the
  itcmdr.log/v1 writer (`plugin/src/log.rs`), signing (`auth.rs`, `keys.rs`), DPAPI storage (`storage.rs`). The agent
  reuses them as library code.

What stays in PowerShell: the checks, the plugins, `Itcmdr.Plugin.psm1`, and, during migration, the existing
nightly task as the fallback (section 9).

### Connection

Agents sit behind NAT and firewalls on the office LAN, so **the agent dials the server**. It opens one long-lived
bidirectional stream: `AgentService.Connect(stream AgentMessage) returns (stream ServerMessage)`. "Server pushes"
are messages on that stream. TCP 50051 (or a separate port, see the open questions) already passes the
installer's firewall rule.

- **Transport:** TLS with a server certificate generated by `setup first-run` and **pinned** by fingerprint in the
  enrollment token. There is no CA to run. Plaintext stays available for the VM and tests only (`agent.allow_plaintext`).
- **Reconnect:** exponential backoff with full jitter, 1 s doubling to 5 min (settings). Reset after 60 s connected.
  HTTP/2 keepalive every 30 s.
- **Offline queue:** a local SQLite file in `%ProgramData%\itcmdr\agent\queue.db` holds run reports, errors,
  artifacts (size-capped) and status. It survives restarts. It is sent in order on reconnect, and each item is
  removed only after the server acks its `seq`. It is bounded by `agent.queue_max_mib` (oldest Informational items
  are dropped first, and a drop is itself reported).
- **Offline operation:** the agent keeps the last acked config, schedules and plugin set on disk, so it keeps
  running its schedules while the server is down.

### Enrollment and identity

This ties into the signing work. Each agent gets its **own Ed25519 key**, created on the PC (in a DPAPI
LocalMachine file, `keys/agent.key`) and never copied.

1. An admin creates an enrollment token in the app or CLI: `itcmdr-cli agent enroll-token --machine WS-01
   [--ttl 24h]`. The token holds the server address, the server TLS fingerprint and a one-time secret (only a hash
   is stored). It is valid for one use.
2. The agent installer takes `/EnrollToken=...`, or the admin runs `itcmdr-agent enroll <token>`. The agent
   generates its key and sends `Enroll{token, machine, public_key, os, agent_version}`.
3. The server checks the token and registers the key as `kind=agent`, `role=agent`, **certified by the server's root
   key**. That is delegated certification, so enrolling a PC does not need the 24 words. The `agent` role can
   only use `AgentService`.
4. From then on, the stream opens with a signed `Hello` (same canonical signing as `docs/SIGNING.md`, path
   `/itcmdr.AgentService/Connect`). Each upstream message carries a monotonically increasing `seq`, and the server
   rejects replays.
5. Revocation (`itcmdr-cli agent revoke WS-01`, or the app) closes the stream at once. Re-enrolling needs a new token.

mTLS is not needed on top of per-message signatures. It stays an option (open question 3).

## 3. Proto sketch (`proto/agent.proto`)

```proto
service AgentService {
  rpc Enroll(EnrollRequest) returns (EnrollResponse);                // token, one time
  rpc Connect(stream AgentMessage) returns (stream ServerMessage);   // signed Hello first
  rpc UploadArtifact(stream ArtifactChunk) returns (ArtifactAck);    // large files out of band
}

message AgentMessage {
  uint64 seq = 1;                       // per agent, monotonic; acked by the server
  oneof body {
    Hello hello = 2;                    // machine, agent_version, plugin set hash, config_version, schedule_version, queue depth
    Pong pong = 3;                      // answers Ping.nonce; agent clock, load, last run
    StatusReport status = 4;            // state, versions, disk, uptime, last errors
    RunReport run = 5;                  // schedule_run_id, plugin, stage results, check results, metrics, itcmdr.log/v1 lines
    PluginError error = 6;              // the error contract (section 6)
    Applied applied = 7;                // ack/fail of a pushed config, schedule set, plugin or upgrade, by version
  }
}

message ServerMessage {
  uint64 ack_seq = 1;                   // highest AgentMessage.seq stored
  oneof body {
    Welcome welcome = 2;                // server time, heartbeat interval, settings digest
    Ping ping = 3;                      // nonce, server ts
    ConfigUpdate config = 4;            // version, full agent config document (JSON, typed by the registry)
    ScheduleSet schedules = 5;          // version, full set for this agent (spec, timezone, catch-up)
    PluginUpdate plugin = 6;            // name, version, signed package bytes (or chunk refs), enable flag
    PluginRemove plugin_remove = 7;
    RunNow run_now = 8;                 // schedule_run_id, plugin or schedule, args, report_only
    UpgradeOffer upgrade = 9;           // section 8
  }
}

message PluginError {
  string service_plugin = 1; string plugin_version = 2;
  Criticality criticality = 3; string error = 4; string backtrace = 5;
  string run_id = 6; string check_id = 7; int64 ts_ms = 8; map<string,string> context = 9;
}
enum Criticality { CRITICALITY_UNSPECIFIED = 0; CRITICAL = 1; CAUTIOUS = 2; INFORMATIONAL = 3; }

service AlertService {                                  // app/CLI, signed, admin
  rpc ListAlerts(ListAlertsRequest) returns (ListAlertsResponse);       // filter: level, state, machine, source
  rpc AckAlert(AlertRef) returns (Alert);
  rpc ResolveAlert(AlertRef) returns (Alert);
  rpc SilenceAlerts(SilenceRequest) returns (SilenceResponse);          // machine/source/until
  rpc WatchAlerts(WatchAlertsRequest) returns (stream Alert);
  rpc ListChannels(ListChannelsRequest) returns (ListChannelsResponse); // notifier channels
  rpc SaveChannel(SaveChannelRequest) returns (Channel);
  rpc TestChannel(ChannelRef) returns (TestChannelResponse);
}
// plus AgentAdmin RPCs on Itcmdr: ListAgents, GetAgent, CreateEnrollToken, RevokeAgent, SetAgentTargets
```

Desired state is **versioned and full** (whole config, whole schedule set), not deltas. The agent acks each version.
Pushes are idempotent, and a reconnect resends anything not acked.

## 4. Heartbeats

- **Who:** the server sends `Ping` every `agent.heartbeat_interval_secs` on each open stream. The agent answers with
  `Pong`. If the stream is down there are no pings, and missed beats are counted from `last_seen` against the
  interval. The agent also sends `Hello` on every reconnect.
- **Settings** (new, in the registry, editable in the app and the CLI like every other setting):

| Setting | Default | Meaning |
|---|---|---|
| `agent.heartbeat_interval_secs` | 60 | Ping interval |
| `agent.missed_heartbeats_threshold` | 5 | Missed beats before **non-responsive** (5 min) |
| `agent.late_heartbeats` | 2 | Missed beats before **late** (no alert) |
| `agent.nonresponsive_alert_level` | critical | Level of the non-responsive alert (servers can be set separately: `agent.server_role_level`) |
| `agent.office_hours_only` | false | Alert only in office hours (uses the existing office-hours guard) |
| `agent.reconnect_max_secs` | 300 | Backoff cap |
| `agent.queue_max_mib` | 256 | Offline queue cap |
| `agent.artifact_max_mib` | 50 | Per artifact |
| `alerts.dedupe_window_mins` | 60 | Same key within the window → count++ |
| `alerts.page_min_level` | critical | Lowest level that pages |
| `alerts.repeat_page_mins` | 60 | Re-page an unacked Critical |
| `calllog.retention_days` | 30 | Section 7 |

- **State machine** (per agent, stored in `agents.state`, with changes in the audit log):

```
enrolled --Hello--> online --(late_heartbeats missed)--> late --(threshold missed)--> non_responsive
   ^                  ^  |                                   |                            |
   |                  |  +------- Pong / Hello --------------+---------- Pong / Hello -----+-> online (+ resolve alert)
   |                maintenance (admin; no alerts, e.g. PC switched off for holidays)
revoked (terminal)
```

  Entering `non_responsive` raises an alert (key `agent-nonresponsive:<machine>`). Coming back resolves it
  automatically and sends an Informational "back online" message. Clock skew larger than 5 min in `Pong` raises a
  Cautious alert.

## 5. Alerts

**Model** (table `alerts`):

- id, `level` (critical | cautious | informational), `source` (agent | plugin | server | backup | heartbeat), and
  `machine`.
- `key`, the dedupe key, e.g. `plugin:itcmdr-network:NET-IP-CORRECT:WS-01`.
- `title` and `detail`.
- first_seen, last_seen, `count`.
- `state` (open | acked | resolved | silenced), acked_by/at, resolved_by/at (or `auto`), and links to the run,
  log lines and artifact.

Behavior:
- **Dedupe:** an open alert with the same key gets count++ and a new last_seen. A resolved one reopens.
- **Auto-resolve:** when the same check or plugin reports ok again.
- **Ack:** stops repeat pages.
- **Silence:** per machine or source, until a set time.

**Sources:**
- heartbeat state (section 4)
- `PluginError` at the plugin's criticality
- check results: `fail` on a clinical or Critical check is critical, other fails are cautious, repaired is informational
- server events: a backup failed or overdue, the disk is low, plugin quarantined, a key was revoked, restore,
  cutover changes.

Alerts are also a metric (`alerts_open{level}`, `alerts_raised_total{level,source}`), so they show up in the
existing charts.

**Paging (the alert endpoint).** A `Notifier` trait with channels configured in the app or CLI:
- **Webhook** (HTTPS POST, JSON, HMAC-signed body).
- **Email over SMTP** (STARTTLS, credentials in the DPAPI secret store).
- **ntfy** (a topic URL; self-hosted or ntfy.sh).
- **Pushover** (app + user key).

Each channel has a minimum level and optional office-hours rules. **Recommended default: email (SMTP) for Critical
and Cautious, plus ntfy for Critical phone pushes.** Both work without a vendor account (ntfy can be self-hosted),
and webhooks cover anything else later. Sends are retried with backoff, every send attempt is logged, and the app's
**Test** button sends a sample.

**App:**
- **Alerts** screen: open, acked and resolved; filter by level and machine; ack, resolve and silence.
- A badge on the shell.
- **Agents** screen: state, last seen, versions, queue depth, enroll and revoke.
- Settings: channels.

**CLI:** `itcmdr-cli alerts list|ack|resolve|silence`, `itcmdr-cli alerts channel add|test`,
`itcmdr-cli agent list|enroll-token|revoke|maintenance`.

**Audit:** ack, resolve, silence, channel changes and agent state changes go to `audit_log`. Pages sent go to the
call log (section 7) with status.

## 6. Plugin error contract

New workspace crate **`itcmdr-common`**, with no gRPC and no server dependency, used by the server, the agent, the
plugin SDK and plugins:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorCriticality { Critical, Cautious, Informational }

pub trait ServicePluginError: std::error::Error + Send + Sync + 'static {
    fn criticality(&self) -> ErrorCriticality;
    fn service_plugin(&self) -> &str;
    fn backtrace_text(&self) -> &str;
    fn to_record(&self) -> PluginErrorRecord { /* the proto/log shape */ }
}
```

Proc-macro crate **`itcmdr-common-derive`**, re-exported as `itcmdr_common::ServicePluginError`:

```rust
#[derive(Debug, ServicePluginError)]
#[service_plugin(name = "disk-free")]          // optional; else the field
pub struct CustomServicePluginError {
    error: String,
    criticality: ErrorCriticality,
    backtrace: String,                         // filled by the generated constructor
    service_plugin: String,
}
// generated: impl Display/Error/ServicePluginError, and
// CustomServicePluginError::new(error, criticality) capturing std::backtrace::Backtrace::force_capture()
// into `backtrace` (force_capture so it does not depend on RUST_BACKTRACE).
```

The derive checks at compile time that the four fields exist with the right types. Extra fields go into the record's
`context` map.

Mapping:
- **gRPC:** `StageResponse` (`proto/plugin.proto`) gains `PluginError error`. On the agent → server stream it is the
  `PluginError` message (section 3).
- **itcmdr.log/v1:** a new `event: "plugin-error"` line with `criticality`, `error` and `backtrace` (truncated to
  16 KiB). `level` is derived: critical → error, cautious → warn, informational → info. `docs/LOG-STANDARD.md` gets
  the fields as optional, so old readers keep working.
- **Alerts:** each `plugin-error` becomes an alert at its criticality, deduped by plugin + error kind + machine.
- **PowerShell plugins:** `Itcmdr.Plugin.psm1` gains
  `New-ItcmdrPluginError -Message -Criticality Critical|Cautious|Informational`. The backtrace is
  `$_.ScriptStackTrace` plus `$_.InvocationInfo.PositionMessage`. A thrown `ItcmdrPluginError` (or an object from
  `Write-ItcmdrPluginError`) is serialized by `itcmdr-plugin-ps` into the same `PluginError`. Uncaught exceptions
  become **Cautious** by default; the plugin manifest may set `default_error_criticality`.
- **Check plugins:** a check `fail` keeps today's records. The plugin may also raise a `PluginError` when the check
  itself could not run (e.g. access denied).

## 7. Call logging

Table `call_log`, written by a tonic/tower layer on **every** RPC on all services (Itcmdr, AgentService,
AlertService), and by the plugin host for PluginService calls:

`ts_ms, service, method, kind (unary | server_stream | client_stream | bidi | stream_msg), peer (ip:port),
principal (key fingerprint + name, or "anonymous"), status (gRPC code), duration_ms, req_bytes, resp_bytes,
error (truncated)`.

- For the long-lived agent stream, it logs connect, disconnect (with reason) and one row per **message kind per
  minute** (counts), not every Ping, so it stays small. Paging sends are logged here as `notifier/<channel>`.
- **Retention:** `calllog.retention_days` (30), pruned daily by the existing jobs loop. Also exported daily as
  itcmdr.log/v1 lines (`event: "call"`) into the server log folder, so `Rotate-ItcmdrLogs.ps1` / block zips keep
  long-term history without growing the DB.
- **Relation to `audit_log`:** the audit log stays the record of *who changed what* (state changes, with details).
  The call log is the record of *every call and its outcome*. An audited call has both. The audit row stores the
  call_log id so the two are linked. The app gets a Calls view (filter by method, principal, status) next to Audit.
- Not logged: request bodies, words, keys, package bytes.

## 8. Agent self-upgrade

- **Release plugin:** `itcmdr-agent-release`, published from itcmdr-plugins. A package with `kind = "agent-release"`
  holds the new `itcmdr-agent.exe` (and the PS agent files during migration) plus `release.toml`: version, minimum
  version it upgrades from, sha256 values.
- **Signing:**
  - It is signed like any plugin package (an admin or owner key) and validated and quarantined by the server like
    any plugin.
  - The agent re-verifies the package signature against the master public key and the signer's certificate before
    applying it.
  - Agents only accept releases whose signer has the **owner** role (a stricter rule than for plugins).
- **Delivery:** an admin installs the release on the server and creates an **immediate schedule** "Upgrade agents to
  X.Y.Z" targeting a ring. The server sends `UpgradeOffer` with the package.
- **Staged rollout (rings):** ring 0 is the VM or a test PC, ring 1 is one front-desk PC, ring 2 is everything. Each
  ring waits until all its agents report `Applied(ok)` with healthy heartbeats for `agent.upgrade_soak_mins` (60)
  before the next one starts. This is the same gating idea as the cutover gate. Any failure pauses the rollout and
  raises a Critical alert.
- **Apply on the agent:**
  1. Download into `agent\staged\X.Y.Z` and verify.
  2. Hand off to the **watchdog service**, `itcmdr-agent-watchdog`, a tiny separate service that rarely changes.
  3. The watchdog stops the agent, swaps the `current` and `previous` folders and starts the agent.
  4. The new agent must connect and send `Hello` with the new version within `agent.upgrade_health_secs` (300).
  5. If it doesn't, the watchdog **rolls back** to `previous` and reports the failure.
- **Watchdog of the watchdog:**
  - Both services use SCM recovery actions (restart on failure).
  - The agent checks that the watchdog is running and restarts it.
  - The watchdog checks the agent's local health file every 60 s and restarts it if the file is stale.
  - The server sees an agent's heartbeat stop (non-responsive alert). A missing watchdog is reported in
    `StatusReport` (Cautious alert).
  - The watchdog itself is only upgraded by the MSI/Inno installer, never by a release plugin, so a bad release
    cannot remove the rollback path.

## 9. Migration from the share-based design

Principle: run old and new side by side, compare, then switch with a gate, the same way the v0.11.0 cutover was
done. **The nightly SYSTEM task and the cutover gate stay authoritative until the new path proves itself.**

1. **v0.15–0.16:** the server gains AgentService. The installer adds `itcmdr-agent` next to the PS agent. The new
   agent connects, heartbeats, reports status and runs plugins **report-only**. Nothing changes on the share or in
   the nightly.
2. **v0.17:** the new agent runs the check plugins report-only after each nightly and sends a parity report over
   gRPC. The existing `parity_runs` and `cutover status` include it.
3. **v0.18:** config and schedules come from the stream. `agent-settings.json` is still written to the share (for
   the PS agent) from the same source.
4. **Switch (gated, per site):** `itcmdr-cli cutover switch --to agent` requires 7 days with clean parity, every
   active machine online with heartbeats, and the VM flag. After that, the Rust agent runs the schedules and the
   nightly task is disabled (never deleted). `cutover rollback` re-enables it.
5. **Retired after the switch has held for a release:**
   - the poller task and `requests/` on the share (replaced by `RunNow`)
   - `schedule-state.json` (replaced by heartbeats and `Applied`)
   - `agent-settings.json` (replaced by `ConfigUpdate`)
   - log ingest from the share (replaced by `RunReport`; ingest stays read-only for old files)
   - `Push-Install-ItcmdrAgent.ps1` for upgrades (replaced by release plugins)

   Kept: the share for backups (if `backup.dir` points there), the PS checks and plugins, `Rotate-ItcmdrLogs.ps1` for
   server logs, and the local fallback (the agent's cached schedules replace the nightly fallback).

## 10. Roadmap (proposed)

Every step must be testable on the VM, and the server, agent and app keep working at every step.

| Version | Scope | VM / test plan | Risks |
|---|---|---|---|
| **v0.15.0** | `itcmdr-common` + `itcmdr-common-derive` (ErrorCriticality, ServicePluginError, backtrace capture); `PluginError` in plugin.proto, itcmdr.log/v1 `plugin-error`; PS `New-ItcmdrPluginError`; call log layer + retention + app Calls view | Unit tests for the derive (trybuild compile-fail cases), log round trip; a PS plugin throwing each level on the VM; call log rows for every RPC; prune test | Proc-macro maintenance; call-log volume (mitigated by stream aggregation) |
| **v0.16.0** | Alerts: table, dedupe/ack/resolve/silence, AlertService, app Alerts screen + badge, CLI; sources: plugin errors, server events (backup failed/overdue, quarantine); notifiers: SMTP + ntfy + webhook, Test button | Raise each level on the VM; dedupe, auto-resolve and repeat page; SMTP against a local test mail sink; ntfy against a local ntfy container | Alert noise: defaults conservative, page only Critical |
| **v0.17.0** | `itcmdr-agent` service (Rust): enrollment tokens, per-agent keys (DPAPI), TLS with pinned cert, `Connect` stream, Hello/Ping/Pong/Status, offline queue; heartbeat state machine + non-responsive alerts; app Agents screen; installer component | VM as server + VM agent (and a second agent instance on the same VM, separate data dir); kill the server and check backoff + queue; pull the network and check non-responsive → back online | NAT/firewall; TLS cert handling on renewals; clock skew |
| **v0.18.0** | Agent runs plugins: PluginUpdate (signed, verified on the agent), ScheduleSet + local scheduler, RunNow, RunReport/artifacts, report-only parity vs the nightly | Parity on the VM for 3+ days; push, enable, remove a plugin; offline schedule run then a queued report | Divergence from the PS runner (parity catches it) |
| **v0.19.0** | ConfigUpdate (typed, versioned) replaces agent-settings.json for the new agent; gated `cutover switch --to agent` + rollback | Switch and roll back on the VM; nightly task disabled/enabled | Gate too strict or too loose; tested both ways |
| **v0.20.0** | Self-upgrade: `itcmdr-agent-release` package kind, rings, watchdog service, health-checked swap + rollback | Upgrade the VM agent; a deliberately broken release must roll back; kill the watchdog → restarted | Bricking agents: watchdog never self-upgrades, owner-signed releases only |
| **v0.21.0** | Retire the poller, requests/, schedule-state, share ingest (after one release on the switched path); docs | Fresh install on the VM has no share dependency except backups | Sites not yet switched: keep the old path until switched |

A second real PC (still an open item) should join at v0.17.0 at the latest. Two agents on one VM are a weaker test.

## 11. Open questions (Michael), with recommended defaults

1. **Default paging channel.** Recommended: email over SMTP via the office mailbox for Critical and Cautious, plus
   ntfy (self-hosted or ntfy.sh topic) for Critical pushes to your phone.
2. **Who is paged, and when.** Recommended: Critical at any time; Cautious only in office hours, as an email digest
   every 4 h; Informational never paged (app only).
3. **Agent transport security.** Recommended: TLS with a pinned self-signed server cert plus per-agent signed
   messages; no mTLS in the first version.
4. **Heartbeat defaults.** Recommended: 60 s interval, non-responsive after 5 missed (5 min). PCs that are switched
   off at night use a per-machine "office hours" profile so they don't page every evening.
5. **Port.** Recommended: keep 50051 for both the app and agents (one firewall rule). A separate port is only needed
   if you want different subnets for each.
6. **Release signing.** Recommended: only owner keys may sign agent releases, and rollout rings are VM → one PC → all.
7. **The PowerShell agent's future.** Recommended: keep it as the fallback until v0.21.0, then retire the task-based
   runner and keep only the PS checks/plugins hosted by the Rust agent.
