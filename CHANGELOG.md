# Changelog

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
  printer queues + spooler, Sirona drift (report only). Skipped check status.

## v0.2.0
- Real ingest, Windows service, ISO-aligned rotation and rollups, on-demand loop, exceptions, auth, audit.
