# Site server (`itcmdr-server`)

The site server runs on one machine in the office (a Windows service, or `--console` for
troubleshooting). It keeps the history, alerts, metrics, schedules, plugins, keys and encrypted
backups, and it is the only part that decides what may leave the office.

## Ports

| Port | Setting (server.json) | What | Who connects |
|---|---|---|---|
| 50051 | `bind_addr` | HTTPS API (gRPC and gRPC-Web) for the site app and `itcmdr-cli` | admin PCs in the office |
| 50052 | `agent_bind_addr` | agent link: TLS with client certificates (mTLS), pinned | every enrolled agent |

The Windows installer binds both to all addresses and adds firewall rules limited to
`/AllowSubnet=<lan cidr>`. A server.json without `bind_addr` listens on `127.0.0.1:50051` only.
`agent_bind_addr: ""` turns the agent link off. A client certificate that is not the current
(or, during a renewal overlap, previous) certificate of a live enrollment is refused in the
TLS handshake and again before any message is read (see [agent](AGENT.md)).

Since v1.1.0 the API is HTTPS with the server certificate the agents already pin
(`<data dir>\keys\agent-server.pem`). Plain HTTP is accepted from the same machine only, or from
the network with `api_plain_http: true` (see [certificates](CERTIFICATES.md)).

## Configuration

- `server.json` is read from `ITCMDR_SERVER_CONFIG`, `.\config\server.json`,
  `%ProgramData%\itcmdr\server.json`, or `server.json` next to the exe, in that order. The installer
  writes it once and never overwrites it (`itcmdr-cli setup apply --keep-existing` only fills empty values).
- The data folder (`--data-dir`, `ITCMDR_DATA_DIR`, then `data_dir` in server.json; default
  `%ProgramData%\itcmdr`) holds `itcmdr.database` (encrypted), `keys\`, `service_plugins\`,
  `quarantine\`, `backups\` and `server-logs\`.
- Runtime settings live in the database and override server.json: see [settings](SETTINGS.md).

## Running it

    sc.exe start itcmdr-server                 # the service (itcmdr-server.exe --service)
    itcmdr-server.exe --console                # foreground, same config (stop the service first)
    itcmdr-server.exe init | recover-db-key    # same as itcmdr-cli init / recover-db-key
    itcmdr-server.exe hash-token <token>       # legacy shared token hash (auth mode "both" only)

`itcmdr-server-watchdog` keeps the service running (restart when its health file goes stale) and
swaps in server upgrades with a rollback ([upgrades](UPGRADES.md)). Errors are in
`%ProgramData%\itcmdr\server-logs\itcmdr-server.txt`.

## What it does

- Ingests agent reports (over the agent link) and, for older installs, JSON Lines logs from `logs_dir`;
  tracks incidents per machine and check, computes [metrics](ARCHITECTURE.md) and rollups.
- Raises and dedupes [alerts](ALERTS.md); runs [schedules](SCHEDULES.md) and [plugins](PLUGINS.md).
- Nightly encrypted [backups](BACKUP.md) and the restore drill; restores through `itcmdr-restore`.
- Signed requests and roles ([keys](KEYS.md), [signing](SIGNING.md)); a hash-chained audit log; a call log
  with one row per RPC (`itcmdr-cli calls list|show|prune`, the app's Calls page; `calllog.retention_days`).
- The optional offsite link to [adminsvr](ADMINSVR.md), deny by default.

Administration on the server itself is [itcmdr-cli](CLI.md). Installing: [install](INSTALL.md) and
[deploying a site server](DEPLOY-SITE-SERVER.md).
