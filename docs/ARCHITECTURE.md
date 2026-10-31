# itcmdr architecture

Site-specific values live only in gitignored files (`config/site.json`, `C:\ProgramData\itcmdr\site.json`, `C:\ProgramData\itcmdr\server.json`). Everything in this repo uses placeholders: RFC 5737 ranges (192.0.2.x, 198.51.100.x, 203.0.113.x) and names like `SERVER01`, `WS-01`, `RCU-01`, `PRN-01`.

## Components

| Part | Where it runs | What it does |
|---|---|---|
| Agent (`agent/Invoke-ItcmdrAgent.ps1`) | every PC, nightly as SYSTEM | checks network config, connectivity and services, repairs what it safely can, writes one JSON Lines log per run |
| Poller (`agent/Poll-ItcmdrRequests.ps1`) | every PC, every minute as SYSTEM | runs on-demand requests from the app |
| Rotation (`agent/Rotate-ItcmdrLogs.ps1`) | server, daily | zips logs weekly, into 4-week blocks, and yearly |
| Server (`itcmdr-server`, Rust/tonic, SQLite) | server, Windows service | ingests logs, computes metrics, serves gRPC and gRPC-Web on port 50051 |
| App (Flutter) | Windows desktop or web | Overview, Metrics, Logs, Exceptions, On-demand, Schedules, Plugins, Backup, Audit, Keys, Settings |

The agent and poller run under Windows PowerShell 5.1 (the `powershell.exe` used by the scheduled tasks) and under PowerShell 7.

## Agent runs and logs

- Each run writes `<paths.localLogs>\<COMPUTER>_<yyyyMMdd-HHmmss>_<runId>.log` (site.json; default `C:\maintenanceRuns`): a `run-start` record, one `check` record per check (`checkId`, `category`, `status`, `attemptedRepair`, `result`, `ts`, `excepted`, ...), then `run-complete`.
- The log is copied to `shares.maintenanceRuns`. If the share is unreachable, the log is queued in `<paths.agentData>\spool\outbox` (default `C:\ProgramData\itcmdr`) and sent on the next run.
- Repairs run only as SYSTEM, or with `-Repair`, and never with `-ReportOnly`.
- The clinical guard: if files under `services.rcu.dataPath` changed in the last 10 minutes, the run is report-only unless `-Force` is given.
- Checks that have no safe automatic repair (port and share reachability) are logged as failures without a repair attempt.
- Sirona configuration is never edited by the agent.
- `-Categories network,connectivity,services` limits a run to those categories.

## Exceptions (allowlist)

One JSON shape is used everywhere:

```json
{"id":"…","scope":{"machines":["WS-01"],"role":"Workstation"},"checkId":"NET-SERVER-TCP-445",
 "checkPattern":"^PRN-","action":"ignore|downgrade:warn|downgrade:info","reason":"…","expires":"2099-12-31"}
```

- A rule with no machines and no role applies to every machine.
- `expires` given as a date is valid through that whole day.
- The agent combines the `exceptions` in `site.json` with the server-managed `<maintenanceRuns share>\exceptions.json`. It caches the latter in `C:\ProgramData\itcmdr` for offline runs.
- The server rewrites `exceptions.json` on every add, edit or expire, and at startup.

## On-demand runs

1. The app calls `TriggerRuns`, which requires the token. The server records the request, writes `<logs_dir>\requests\<id>.json` and adds an audit entry.
2. Each poller picks up requests addressed to its computer, or to all computers (an empty `machines` list). It takes a per-machine lock and writes `responses\<id>.<COMPUTER>.json` with `running`.
3. The poller runs the agent with `-ReportOnly`, `-Repair`, `-Force` and `-Categories` taken from the request, then rewrites the response as `done` or `failed`.
4. The server folds the responses into the request status, which is one of:
   - `queued`: no responses yet
   - `running`: a response is still running, or a listed machine hasn't answered
   - `done`: every response is done
   - `failed`: every response finished and at least one failed

   Status is updated on every ingest pass and on each `GetRequestStatus`.

No WinRM or remote PowerShell is needed.

## Server

- **Ingest** runs at startup and then every `ingest_interval_secs` (default 30).
  - It reads `.log`/`.jsonl` files and zips under `logs_dir`, recursively, including zips nested in zips. Archive names don't matter.
  - Files whose size and mtime are unchanged are skipped.
  - Runs are unique by `runId` and checks by `(runId, checkId)`, so re-reading logs from archives never double-counts.
  - Timestamps come from the records (`started`, `ts`, `finished`).
  - Events are applied in time order.
- **Incidents** are tracked per (machine, check):
  - A failure opens an incident, unless one is already open.
  - The next `ok` closes it.
  - A check that was broken and repaired in the same run counts as an incident with zero repair time.
  - Excepted checks never open incidents.
- **Metrics** (`GetMetrics`, windows of 7, 30, 90 or 365 days):
  - Per bucket: `incidents:<m>`, `uptime:<m>` (share of days with a run), `repair_failures_bucket:<m>`.
  - Over the window: `incidents_total:<m>` (the ranking), `open_incidents:<m>`, `mttr:<m>` (ms), `repair_failures:<m>:<check>`, `missed_runs:<m>`.
- **Rollups** are aligned to ISO weeks:
  - `weekly`: an ISO week, Monday to Sunday.
  - `block4`: block N of an ISO year covers ISO weeks 4N-3 to 4N. There are 13 blocks per year, and in 53-week years week 53 belongs to block 13.
  - `yearly`: an ISO year.
  - `daily` is also available. Calendar months and quarters are not used.
- **Auth**: calls carry an Ed25519 signature (`x-itcmdr-key`, `x-itcmdr-ts`, `x-itcmdr-nonce`, `x-itcmdr-sig`) over the canonical request bytes. See `docs/SIGNING.md`. The server checks the registered public key, a clock window and a nonce cache, then the role (owner > admin > user > client).
  - New installs default to `auth_mode = signed` (a key is required, including for reads). An install from before v0.12.0 with no value in server.json keeps `both`: signed calls, or the legacy token for changes. `itcmdr-cli auth status` shows which. `auth set-mode signed` retires the token.
  - The legacy token is `authorization: Bearer <token>`. The server compares its SHA-256 with `admin_token_hash`. Restore and key registration never accept the token.
  - If no hash is configured and the mode still allows the token, changes are refused with an explanation, and a warning is logged at startup.
  - `allow_insecure_no_auth: true` is the explicit lab-only opt-out.
  - Restrict the listen port with the firewall rule.
- **Audit log**: on-demand triggers and exception add, edit and expire are recorded with actor, time and detail. The actor is the app's `x-itcmdr-actor` name. Read it with `ListAudit`.
- **Live updates**: `WatchUpdates` streams events (`run`, `check`, `incident-open`, `incident-close`, `request`, `exception-*`, `audit`). The app uses them to refresh.
- **Retention**: runs, checks, closed incidents, rollups and finished requests older than `retention_days` (default 400) are deleted at startup and daily. The audit log and exceptions are kept.

## Log rotation (`Rotate-ItcmdrLogs.ps1`, daily on the server)

All periods are ISO-week aligned. The current week is never touched, and a zip is built only after its whole period has ended.

```
<maintenanceRuns>\archive\weekly\itcmdr-<isoYear>-W<ww>.zip   logs of one finished ISO week
<maintenanceRuns>\archive\blocks\itcmdr-<isoYear>-B<NN>.zip   the 4 (block 13 in a 53-week year: 5) weekly zips
<maintenanceRuns>\archive\yearly\itcmdr-<isoYear>.zip          all block zips of the ISO year
```

- **Manifest:** every zip contains `manifest.json` with the kind, ISO year and week or block, the `start`/`end` dates, and the SHA256 of every member. The zip is re-opened and verified before the originals are deleted.
- **Late logs:** logs that arrive for a period that's already zipped go into a numbered supplement, such as `itcmdr-2026-W43-2.zip`.
- **Block zips after yearly:** they're kept unless `retention.keepBlockZipsAfterYearly` is `false`. The legacy `yearlyKeepForever` is honoured.
- **Backup:** the whole `maintenanceRuns` folder, including `archive\`, is what the nightly backup must cover (`backup.expectedBackupSources`).

## Regenerating gRPC code

- Rust: `server/build.rs` regenerates `server/src/generated/itcmdr.rs` with a vendored protoc on every build.
- Dart: `protoc -I proto --dart_out=grpc:app/lib/proto proto/itcmdr.proto` using protoc_plugin 25.1.0 (`dart pub global activate protoc_plugin 25.1.0`), matching protobuf ^6.1 and grpc ^5.2 in `app/pubspec.lock`.

## Demo data

`cargo run -p itcmdr-sample-data-gen -- --out-dir ./sample-maintenanceRuns --days 90` writes agent-format logs for placeholder machines, with varying failure rates, failed repairs and missed nights. Point `logs_dir` at that folder (see `config/server.example.json`).
