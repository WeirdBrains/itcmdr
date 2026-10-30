# Backup and restore

What's in an archive (`itcmdr-YYYYMMDD-HHMMSSmmm-<kind>.itcb`):

| File | Notes |
|---|---|
| `itcmdr.database` | consistent snapshot of the live SQLCipher DB (still encrypted with db.key) |
| `keys/db.key.escrow` | db.key sealed for the master key |
| `server.json` | the config in effect |

Service plugins, plugin config, the quarantine index and plugin logs join the archive in v0.9.0 and
v0.10.0. `keys/db.key` and `keys/root` are never archived: they are bound to the machine.

The whole archive is encrypted for the master key (only the 24 words open it). The header (manifest
with versions, schema version and sha256 per file) is signed by the server's root key, which carries a
master certificate. The signature covers the ciphertext hash.

## Nightly

server.json `backup`: `dir` (default `<data_dir>/backups`; point it at the share), `nightly` (true),
`hour` (2, local), `keep_daily` / `keep_weekly` / `keep_monthly` (7 / 4 / 12). Only `nightly`
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

gRPC: `CreateBackup`, `ListBackups`, `VerifyBackup` (admin or owner, audit-logged). Restore is
CLI-only because it replaces the database under a stopped server.

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
