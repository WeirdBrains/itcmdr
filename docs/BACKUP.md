# Backup and restore

What's in an archive (`itcmdr-YYYYMMDD-HHMMSSmmm-<kind>.itcb`):

| File | Notes |
|---|---|
| `itcmdr.database` | consistent snapshot of the live SQLCipher DB (still encrypted with db.key) |
| `keys/db.key.escrow` | db.key sealed for the master key |
| `server.json` | the config in effect |
| `service_plugins/**` | installed plugin packages (v0.9.0) |
| `quarantine/index.jsonl` | refused packages (the quarantined files themselves are not archived) |
| `plugin-logs/**` | per-run plugin logs, `itcmdr.log/v1` (v0.10.0); a restore merges them and keeps existing files |

Plugin config lives in the database (settings). `keys/db.key` and `keys/root` are never archived: they are bound to the machine.

The whole archive is encrypted for the master key (only the 24 words open it). The header (manifest
with versions, schema version and sha256 per file) is signed by the server's root key, which carries a
master certificate. The signature covers the ciphertext hash.

## Nightly

Settings can also be changed at runtime (stored in the DB, they win over server.json):
`itcmdr-cli config get`, `itcmdr-cli config set backup.dir <folder>` (must exist and be writable),
`config unset <key>`, or gRPC `GetSettings` / `UpdateSettings`.


server.json `backup`: `dir` (default `<data_dir>/backups`; point it at the share), `nightly` (true),
`hour` (2, local), `keep_daily` / `keep_weekly` / `keep_blocks` (7 / 4 / 13; a block is ISO weeks 4N-3..4N, 13 per year, week 53 in block 13; the old name `keep_monthly` is still read). Only `nightly`
archives are pruned. Nothing runs until `init` has been done.

## Commands (run on the server, as an admin)

```
itcmdr-cli backup create [--out DIR]
itcmdr-cli backup list [--dir DIR]
itcmdr-cli backup verify ARCHIVE            # signature + hash, pinned to this server's master key
itcmdr-cli backup verify --deep ARCHIVE     # words on stdin: decrypt, check every file
itcmdr-cli backup prune [--dir DIR]
itcmdr-cli restore --from ARCHIVE [--dry-run]   # words on stdin; stop the service first
```

gRPC: `CreateBackup`, `ListBackups`, `VerifyBackup` (admin or owner, audit-logged).

## Restore from the app (v0.10.0, server side)

`StageRestore {backup, words}` (owner only, audited as `restore-stage`; the words are never logged):

1. The archive (a name from `ListBackups`) is fully verified and decrypted with the words; nothing
   changes if that fails.
2. `restore/job.json` is written and `itcmdr-restore` is started (detached on Windows). The words
   go to it on stdin only.
3. The helper runs `restore.stop_command` (server.json, default `sc.exe stop itcmdr-server`), takes
   the `pre-restore-<ts>` safety copy, restores, runs `restore.start_command` and health-checks:
   the DB opens with the right schema, `integrity_check` passes and gRPC answers, within
   `restore.health_timeout_secs`.
4. If any step after the copy fails, the safety copy is put back and the service restarted
   (`rolled-back`, audited `restore-rolled-back`); otherwise `done` (`restore-done`).

`GetRestoreStatus` (admin) returns the state (`staged`, `stopping`, `restoring`, `starting`,
`health-check`, `rolling-back`, `done`, `rolled-back`, `failed`) and every step from
`restore/status.json`. The live server.json is never overwritten.

## Restore onto a replacement machine

1. Install itcmdr, stop the `itcmdr-server` service.
2. `itcmdr-cli restore --from \\SERVER01\share\itcmdr\backups\<archive> --dry-run`, type the words.
3. Same without `--dry-run`. It:
   - copies any existing state to `<data_dir>/pre-restore-<ts>/`,
   - writes the database and escrow, rebuilds `keys/db.key` from the escrow (DPAPI),
   - refuses an archive from a newer schema; an older one is migrated when the DB opens,
   - issues a new root key (the old machine's root key is revoked),
   - writes the archived config to `<data_dir>/restored-server.json` for review (the active
     server.json is left alone; paths may differ on the new machine).
4. Start the service.
