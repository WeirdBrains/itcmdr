# Changelog

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
