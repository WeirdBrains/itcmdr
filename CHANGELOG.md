# Changelog

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
  printer queues + spooler, Sirona drift (report only). Skipped check status.

## v0.2.0
- Real ingest, Windows service, ISO-aligned rotation and rollups, on-demand loop, exceptions, auth, audit.
