# Settings (decision 15)

Every tunable value in itcmdr is a typed setting with today's behaviour as its default. Source of truth: the registry in
`server/src/settings.rs` (unit test `defaults_equal_current_behaviour` pins the defaults to the old hard-coded values).

## Precedence

- **Server settings:** DB override (RPC/app/CLI) > `server.json` > default. Read-only keys come from `server.json` only.
- **Agent settings:** local `site.json` (same dotted name: `agent.quietMinutes` = `{"agent":{"quietMinutes":5}}`) >
  central per-machine override > central global value > default. The server writes the central values to
  `<maintenanceRuns share>\agent-settings.json` (schema `itcmdr-agent-settings/v1`) at startup and after every change;
  agents cache it in `<paths.agentData>` so offline runs still use it.

## Changing settings

- gRPC `ListSettings(machine)` / `UpdateSetting(key, value, machine, unset)`: admin/owner only, validated server-side,
  each change audit-logged as `settings-update` (key, value, machine). Invalid values return `INVALID_ARGUMENT`.
- CLI: `itcmdr-cli config list [--machine PC]`, `config get KEY [--machine PC]`, `config set KEY VALUE [--machine PC]`,
  `config unset KEY [--machine PC]`. `--machine` is accepted for agent settings only. CLI changes are audit-logged too.
- App: Settings tab, *Server & agent settings* (v0.9.1 basic screen; the full section comes in v0.12.0).
- The older `GetSettings` / `UpdateSettings` (backup block) RPCs still work and use the same storage.

## Reference

| Name | Scope | Type / range | Default | Applies | Who can change | Description |
|---|---|---|---|---|---|---|
| `agent.backupMaxAgeHours` | agent | int 1..720 | `26` | next run | admin/owner (RPC, app, CLI) | Default max age of a backup task run / newest backup file (per-item maxAgeHours wins). |
| `agent.backupMinBytes` | agent | int 0..1_000_000_000_000 | `1` | next run | admin/owner (RPC, app, CLI) | Default minimum size of the newest backup file (per-item minBytes wins). |
| `agent.nightlyTime` | agent | time | `01:30` | agent reinstall | admin/owner (RPC, app, CLI) | Nightly maintenance task time (before stagger); also the default time of the built-in nightly-agent-run schedule. |
| `agent.pluginMode` | agent | text: auto, off, parallel, plugins | `auto` | next run | admin/owner (RPC, app, CLI) | Built-in checks (v0.11.0): auto = plugin versions report-only in parallel before the cutover, plugins after it. |
| `agent.fallbackGraceMinutes` | agent | int 15..720 | `120` | next poll | admin/owner (RPC, app, CLI) | After the switch: run the nightly locally when no scheduled run arrived this long after agent.nightlyTime. |
| `agent.serverStaleMinutes` | agent | int 2..1440 | `15` | next poll | admin/owner (RPC, app, CLI) | schedule-state.json older than this means the server is unreachable (local fallback). |
| `agent.printerStuckMinutes` | agent | int 5..10080 | `60` | next run | admin/owner (RPC, app, CLI) | A print job older than this counts as stuck. |
| `agent.quietMinutes` | agent | int 0..240 | `10` | next run | admin/owner (RPC, app, CLI) | Clinical guard: no user input for this many minutes before disruptive repairs. |
| `agent.staleLockMinutes` | agent | int 10..1440 | `150` | next run | admin/owner (RPC, app, CLI) | Poller: a request lock older than this was left by a crashed run. |
| `agent.tcpTimeoutMs` | agent | int 100..60000 | `3000` | next run | admin/owner (RPC, app, CLI) | TCP probe timeout (ms). |
| `auth.max_skew_secs` | server | int 30..3600 | `300` | next pass / next use | admin/owner (RPC, app, CLI) | Allowed clock difference for signed requests (seconds). |
| `backup.dir` | server | path (must exist, writable; empty allowed) | `(empty = <data_dir>/backups)` | next pass / next use | admin/owner (RPC, app, CLI) | Backup archive folder (e.g. the share). Empty = <data_dir>/backups. |
| `backup.hour` | server | int 0..23 | `2` | next pass / next use | admin/owner (RPC, app, CLI) | Default time (HH:00) of the built-in nightly-backup schedule (v0.11.0; afterwards edit the schedule; reset-to-default re-reads it). |
| `backup.keep_blocks` | server | int 0..1300 | `13` | next pass / next use | admin/owner (RPC, app, CLI) | Nightly backups kept: newest per 4-week block (ISO weeks 4N-3..4N). |
| `backup.keep_daily` | server | int 0..3650 | `7` | next pass / next use | admin/owner (RPC, app, CLI) | Nightly backups kept: newest per day, for this many days. |
| `backup.keep_weekly` | server | int 0..520 | `4` | next pass / next use | admin/owner (RPC, app, CLI) | Nightly backups kept: newest per ISO week, for this many weeks. |
| `backup.nightly` | server | bool | `true` | next pass / next use | admin/owner (RPC, app, CLI) | Default of the built-in nightly-backup schedule: on (v0.11.0; afterwards pause/resume the schedule). |
| `metrics.flap_min` | server | int 2..1000 | `3` | next pass / next use | admin/owner (RPC, app, CLI) | ok<->failed changes in the window before a check counts as flapping. |
| `metrics.repeat_min` | server | int 2..1000 | `3` | next pass / next use | admin/owner (RPC, app, CLI) | Incidents in the window before a machine/check is a repeat offender. |
| `paths.agentData` | agent | path (on each PC) | `C:\ProgramData\itcmdr` | next run | admin/owner (RPC, app, CLI) | Agent cache (exceptions, settings) and spool folder. |
| `paths.localLogs` | agent | path (on each PC) | `C:\maintenanceRuns` | next run | admin/owner (RPC, app, CLI) | Local run logs and requests\ folder on each PC. |
| `plugins.enable_on_install` | server | bool | `true` | next pass / next use | admin/owner (RPC, app, CLI) | Enable a package that passed validation when it is installed (other versions are disabled). |
| `plugins.max_package_mib` | server | int 1..2048 | `256` | next pass / next use | admin/owner (RPC, app, CLI) | Largest plugin package accepted (MiB). |
| `plugins.powershell` | server | text | `powershell.exe` (Windows; `pwsh` elsewhere) | next pass / next use | admin/owner (RPC, app, CLI) | PowerShell used by itcmdr-plugin-ps (Windows PowerShell 5.1 by default). |
| `plugins.timeout_secs` | server | int 10..86400 | `600` | next pass / next use | admin/owner (RPC, app, CLI) | Per-stage timeout of a plugin run (seconds). |
| `restore.health_timeout_secs` | server | int 5..3600 | `120` | next pass / next use | admin/owner (RPC, app, CLI) | How long the restore helper waits for the restarted server before rolling back. |
| `retention.agentLocalDays` | agent | int 1..3650 | `400` | next run | admin/owner (RPC, app, CLI) | Days of local run logs kept on each PC. |
| `rotation.timeLocal` | agent | time | `02:10` | server reinstall | admin/owner (RPC, app, CLI) | Log rotation task time on the server. |
| `schedule.grace_secs` | server | int 30..86400 | `300` | next pass / next use | admin/owner (RPC, app, CLI) | A run due longer ago than this counts as missed (server was down). |
| `schedule.office_hours` | server | text | `Mon-Fri 07:00-18:00` | next pass / next use | admin/owner (RPC, app, CLI) | Office hours for the schedule guard, in each schedule's timezone. |
| `search.default_limit` | server | int 1..1000 | `100` | next pass / next use | admin/owner (RPC, app, CLI) | Log search page size when the client asks for none. |
| `search.max_limit` | server | int 100..10000 | `1000` | next pass / next use | admin/owner (RPC, app, CLI) | Largest log search page. |
| `server.auth_mode` | server | text | `both` | restart | nobody at runtime (server.json + restart) | token / both / signed (server.json only: security-critical). |
| `server.bind_addr` | server | text | `127.0.0.1:50051` | restart | nobody at runtime (server.json + restart) | gRPC listen address (server.json only). |
| `server.ingest_interval_secs` | server | int 5..3600 | `30` | next pass / next use | admin/owner (RPC, app, CLI) | Seconds between log ingest / request sync passes. |
| `server.request_timeout_secs` | server | int 60..604800 | `10800` | next pass / next use | admin/owner (RPC, app, CLI) | On-demand requests still queued/running after this many seconds become timed_out. |
| `server.retention_days` | server | int 30..3650 | `400` | next pass / next use | admin/owner (RPC, app, CLI) | Days of run/check data kept in the database. |

## server.json only

- `restore.stop_command` / `restore.start_command`: argv lists the restore helper runs to stop and
  start the service (default `["sc.exe", "stop", "itcmdr-server"]` / `["sc.exe", "start", "itcmdr-server"]` on
  Windows, empty elsewhere). They run commands, so they are not changeable over RPC.

## Not settings (on purpose)

- `keys/`, the DB file name and the data-dir layout: fixed so backup/restore and recovery stay predictable.
- Format constants (`ITCMDRB1`, signature domains, SQLCipher parameters): part of the on-disk formats.
- Where `site.json` and `server.json` are found (bootstrap).
- Per-item values already in site.json (`backup.tasks[].maxAgeHours`, `destinations[].minBytes`, scanner ports, ...):
  they override the matching `agent.*` defaults.
- The installer's build-time `LocalLogs` define and the app's connection settings (stored on the device).
