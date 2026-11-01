# itcmdr-agent (Rust, v0.17.1)

A Windows service (`itcmdr-agent`, LocalSystem) on every PC **and on the office server**. In v0.17.0 it
enrolls with the site server and keeps a heartbeat link; the PowerShell agent keeps running the checks
and plugins from its scheduled tasks, unchanged. Running plugins, schedules and the offline queue move
into this agent in later point releases (PLAN.md).

Placeholders only: SERVER01, WS-01.

## Enroll

1. On the server (admin): `itcmdr-cli agent enroll-token --machine WS-01` (or the app: Agents >
   Enrollment token). The token is one-time, valid 24 h by default (`--ttl-hours`, max 720) and holds:
   the address agents dial (default: the server's name and the agent link port, `--address host:port`
   to override), the SHA-256 of the server's TLS certificate, and a secret (only its hash is stored).
   The token is never written to the audit log.
2. On the PC: `itcmdr-agent-Setup-<ver>.exe /VERYSILENT /EnrollToken=<token>` (the installer stores it
   for Administrators and SYSTEM only; the service enrolls with it on start and deletes it), or by hand:
   `itcmdr-agent enroll <token> [--machine WS-01] [--role server]`. Use `--role server` for the agent on
   the office server (its non-responsive alert uses `agent.serverNonresponsiveLevel`).
3. The agent creates two keys that never leave the PC (`%ProgramData%\itcmdr\agent\keys\`, DPAPI
   LocalMachine): an Ed25519 key that signs every Hello, and a TLS client key. It sends the public key and
   a CSR; the server burns the token and returns a client certificate from the **site CA**
   (CN = agent id, SAN = machine, 90 days, client auth only). Enrolling a machine again replaces (revokes)
   its previous enrollment.

## The link

- Port `agent_bind_addr` in server.json (default `0.0.0.0:50052`; `""` turns it off). The server
  installer opens it in the firewall for the same subnet as 50051.
- TLS 1.3. The agent **pins** the server certificate's SHA-256 from the token; any other certificate is
  refused before anything is sent ("does not match the pinned"). No public CA is involved.
- **mTLS**: the server verifies the client certificate against the site CA during the handshake and
  then requires it to be the exact certificate issued to that agent, not revoked and not expired.
- The first message is a `Hello` signed with the agent's Ed25519 key over the agent id, machine, time,
  a nonce and the fingerprint of the client certificate on this connection (clock skew within
  `auth.max_skew_secs`; nonces are not reusable).
- Then the server sends `Ping` every `agent.heartbeatSecs` and the agent answers `Pong`. Reconnects use
  full-jitter exponential backoff, 1 s doubling to `agent.reconnectMaxSecs`.

## Heartbeat states and alerts

`enrolled -> online -> late -> non_responsive`, plus `maintenance` and `revoked`; changes are in the
audit log (`agent-state`). Missed beats are counted from the last Pong/Hello:

| Setting (per machine overridable) | Default | |
|---|---|---|
| `agent.heartbeatSecs` | 60 | ping interval |
| `agent.lateHeartbeats` | 2 | late, no alert |
| `agent.missedHeartbeats` | 5 | non-responsive: alert `agent-nonresponsive:<machine>` |
| `agent.nonresponsiveLevel` / `agent.serverNonresponsiveLevel` | critical | its level |
| `agent.reconnectMaxSecs` | 300 | backoff cap |

Coming back resolves the alert and leaves a closed Informational "back online" note. A Pong with the
clock off by more than 5 minutes raises a Cautious `agent-clock:<machine>` alert.

**Office hours (v0.17.1).** `agent.officeHours` (per machine, e.g. `Mon-Fri 07:00-18:00`, server local
time; empty = always) is for PCs that are switched off at night. Outside the hours a missing agent goes
no further than `late` and raises no alert; inside them, missed beats count from the later of the last
beat and the start of today's hours, so a PC switched on at 07:10 is not reported as lost.
`itcmdr-cli config set agent.officeHours "Mon-Fri 07:00-18:00" --machine WS-01`.

## Certificates (v0.17.1)

- **Site CA certified.** The server root key (itcmdr-cli init: itself certified by the master key, docs/SIGNING.md) signs
  the site CA certificate once (`keys/agent-ca.sig`). Enrollment tokens carry the root public key; at
  enrollment the agent checks that the server sent that key and a valid signature over the CA, so a
  token only works with the server that made it. Tokens from a v0.17.0 server (no root) still work.
  (The server has used the token by then: an enrollment the agent refused shows as `enrolled`, never
  seen, in `agent list`; revoke it.)
- **Lifetime and renewal.** Client certificates live `agent.certLifetimeMins` (server setting, default
  129600 = 90 days). At 2/3 of its life the agent sends a new CSR over the link (new TLS key; the
  Ed25519 key stays), saves the certificate and reconnects. The replaced certificate keeps working for
  24 h. `itcmdr-cli agent renew WS-01` asks the agent to renew at its next heartbeat.
- **Expiry alerts.** `agent-cert:<machine>`: Cautious under 14 days left, Critical under 3 days;
  resolved by the renewal.
- **Revocation in the handshake.** The link's TLS layer only accepts client certificates whose serial is
  the current one of a non-revoked agent (or the one just replaced, during the 24 h overlap). A revoked
  or superseded certificate fails the TLS handshake itself (logged as `agent link handshake refused`);
  the agent reports it as refused and stops retrying until it is enrolled again (on Windows the TLS alert
  often arrives as a plain connection reset, so the agent keeps retrying with backoff; every attempt is
  refused). The set is reloaded on
  every heartbeat pass and right after an enroll, renewal or revoke.

## Offline queue (v0.17.1)

`itcmdr-agent alert <key> <title> [--level critical|cautious|informational] [--detail ..] [--resolve]`
(any local script, run as an administrator) drops the alert into `spool\` in the agent's data folder.
The service moves it into `queue\` with a durable sequence number (also while the server is unreachable), sends it when linked and deletes it
when the server acks it; the server raises `agent:<machine>:<key>` (or resolves it with `--resolve`) and
ignores a sequence number it has already seen, so a reconnect never duplicates an alert. Alerts queued
while the server was down are delivered after the reconnect, with the time they were queued. The queue
is capped at `agent.queueMaxMib` (default 256 MiB, sent by the server on connect): the oldest
informational items go first, then the oldest of any level, and the drop is reported as a Cautious
`agent:<machine>:queue-dropped` alert.

## Admin

- `itcmdr-cli agent list [--all]`, `agent revoke WS-01`, `agent renew WS-01` (v0.17.1, CLI only),
  `agent maintenance WS-01 on|off`; the same (except renew) in the
  app's Agents page (`ListAgents`, `CreateEnrollToken`, `RevokeAgent`, `SetAgentMaintenance`, admin only,
  audited).
- Revoking closes an open link at once (from the app) or at the next heartbeat (from `itcmdr-cli`); the
  agent is refused from then on and must be enrolled again with a new token.
- Files on the server: `keys/agent-ca.pem|.key` (site CA), `keys/agent-server.pem|.key` (the pinned
  server certificate). Keys are secret files (DPAPI / 0600). Replacing `agent-server.*` changes the pin:
  every agent then needs a new token.
- The agent's log: `%ProgramData%\itcmdr\agent\logs\agent.log` (rotated at 10 MiB).

## Not yet (later point releases)

Running plugins and schedules in this agent (v0.18.0; the PowerShell tasks still do that), a Renew
button in the app, and queue items from the PowerShell agent.

## Plugins and schedules (v0.18.0)

- `agent.plugins` (per machine; `all`, `none` or `name1,name2`) selects the installed, enabled plugins pushed to the agent. Each push carries the files and a root-key endorsement of `itcmdr-agent-plugin/v1\n<name>\n<version>\n<package sha256>`; the agent verifies it, the name/version and the checksums of every file, then swaps the plugin in under `<state>/plugins/<name>`.
- Enabled schedules with plugins (and the builtin nightly) are pushed as a set with a version; the agent keeps them in `schedules.json` and fires them on its own (time-zone aware; catch-up runs a missed fire once).
- `itcmdr-cli agent run <machine> <plugin> [--arg k=v]` runs a plugin now; `agent runs` and `agent plugins` list reports and installed plugins.
- Runs are report-only (`ITCMDR_REPORT_ONLY=1`). Reports go through the offline queue; artifacts land in `<data>/agent-artifacts/<run>/`, logs in `plugin-logs/<plugin>/agents/<machine>.jsonl`, parity as `agent:<run>`.
- Alerts: `agent-plugin:<machine>:<name>` (install refused), `agent-run:<machine>:<plugin>` (run failed).
- Enrollment calls `Prepare` first (CA and root key verified before the token is used); a disconnected agent calls `Check` and stops when it is revoked.
