# itcmdr-agent (Rust, v0.17.0)

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

## Admin

- `itcmdr-cli agent list [--all]`, `agent revoke WS-01`, `agent maintenance WS-01 on|off`; the same in the
  app's Agents page (`ListAgents`, `CreateEnrollToken`, `RevokeAgent`, `SetAgentMaintenance`, admin only,
  audited).
- Revoking closes an open link at once (from the app) or at the next heartbeat (from `itcmdr-cli`); the
  agent is refused from then on and must be enrolled again with a new token.
- Files on the server: `keys/agent-ca.pem|.key` (site CA), `keys/agent-server.pem|.key` (the pinned
  server certificate). Keys are secret files (DPAPI / 0600). Replacing `agent-server.*` changes the pin:
  every agent then needs a new token.
- The agent's log: `%ProgramData%\itcmdr\agent\logs\agent.log` (rotated at 10 MiB).

## Not yet (later point releases)

Certificate renewal over the link and expiry alerts, a revocation list by serial checked in the
handshake itself, the site CA certified by the master key, the office-hours profile, the offline queue,
and running plugins/schedules in this agent (the PowerShell tasks still do that).
