# adminsvr backup and restore (v1.2.0)

adminsvr backs up **its own** state so a lost or broken adminsvr machine can be rebuilt with the
same identity: enrolled devices and linked sites keep working without re-enrolling or re-linking.

## What is in an archive, and what never is

An archive (`itcmdr-adminsvr-YYYYMMDD-HHMMSSmmm-<kind>.itcab`) holds exactly these files:

| File | Notes |
|---|---|
| `db/adminsvr.sqlite` or `db/postgres.json` | consistent snapshot of adminsvr's database (SQLite `VACUUM INTO`; Postgres: every table as JSON) |
| `keys/root.key` | adminsvr's root key (signs commands and user endorsements; sites pin it) |
| `tls/ca.pem`, `tls/ca.key`, `tls/server.pem`, `tls/server.key` | the CA for site client certificates and the internal server certificate (devices pin it) |
| `acme/**` | the ACME (Let's Encrypt) account and certificate, when used |
| `backup/recovery.json` | the recovery public keys (below) |
| `config/adminsvr.env` | the `ITCMDR_ADMINSVR_*` settings in effect (from the env file and environment; flags given on a command line are not included) |
| `push/*` | the FCM service account and APNs key files, when configured |

Anything else in an archive is refused when it is opened. **Site backups are never included,
fetched, decrypted or inspected**: they stay on each site (see [Backup and restore](BACKUP.md)).
Nothing from offices is in an archive beyond what adminsvr's database already holds (the redacted
status, metrics and alerts the sites send).

## Encryption

Like the site server's backups: each archive is encrypted (XChaCha20-Poly1305) with a fresh
random key, and that key is sealed for a **recovery key** rebuilt from 24 recovery words. adminsvr
stores only the recovery public keys, so it can make backups but cannot open them. The header
(versions, schema version, the size and sha256 of every file) is signed by adminsvr's root key, and
`backup init` certifies the root key with the recovery key, so a restore checks the whole chain.

These are adminsvr's own words, separate from any site's 24 words. Keep them offline, apart from
the backups (an archive and its words together are a full copy of adminsvr's identity).

## Set up

```
itcmdr-adminsvr backup init            # prints 24 recovery words ONCE; write them down
itcmdr-adminsvr backup status          # recovery key, folder, schedule, newest backup, last drill
```

`backup init --replace` issues new words (for example after the root key changed); archives made
before still need the old words.

Settings (flags, `ITCMDR_ADMINSVR_*` in the environment or the env file):

| Setting | Default | |
|---|---|---|
| `--backup-dir` / `ITCMDR_ADMINSVR_BACKUP_DIR` | `<data dir>/backups` | point it at another disk or a mounted share |
| `--backup-hour` / `ITCMDR_ADMINSVR_BACKUP_HOUR` | `3` | nightly at this local hour while `serve` runs; `off` to schedule it yourself |
| `--backup-keep` / `ITCMDR_ADMINSVR_BACKUP_KEEP` | `14` | nightly archives kept; manual ones are never pruned |

Nothing runs until `backup init` has been done. To use your own scheduler instead (cron, a systemd
timer, Task Scheduler), set `--backup-hour off` and run `itcmdr-adminsvr backup create` and
`backup prune` with the same settings as the service.

## Commands (on the adminsvr machine, as the service user)

```
itcmdr-adminsvr backup create [--out DIR]
itcmdr-adminsvr backup list [--dir DIR]
itcmdr-adminsvr backup verify ARCHIVE           # signature, hash, this adminsvr's recovery key
itcmdr-adminsvr backup verify --deep ARCHIVE    # words on stdin: also decrypt and check every file
itcmdr-adminsvr backup drill [--deep]           # restore drill (below)
itcmdr-adminsvr backup prune
itcmdr-adminsvr restore --from ARCHIVE [--dry-run]   # words on stdin; stop the service first
```

The words are read from stdin only (typed, or piped from a file you delete afterwards), never
from a flag, and never logged.

## Restore drill

The drill proves the newest archive can be restored without touching the live install: it copies
the archive to a temp folder, checks the signature chain, the hash, that it was made for this
adminsvr's recovery key and that its schema is not newer, then deletes the copy. `--deep` (words
on stdin) also decrypts it, checks every file, checks the root key and certificates, and restores
the database into a throwaway copy (integrity check and migrations). The result is kept in
`<data dir>/backup/last-drill.json` (`backup status` shows it) and audited (`backup.drill`, the
archive name and ok/failed only).

Runbook: monthly `backup drill`; quarterly (and after any key or setting change) `backup drill
--deep` with the words typed in; once a year a full restore onto a spare machine or VM.

## Restore

1. Install the same or a newer adminsvr. Stop the `itcmdr-adminsvr` service (restore refuses
   while something answers on the admin API port).
2. `itcmdr-adminsvr restore --from <archive> --dry-run`, type the words: everything is opened and
   checked, and the files it would write are listed. Nothing changes.
3. The same without `--dry-run`. It:
   - refuses wrong words, a damaged or truncated archive, an archive for another recovery key
     (on an install that already has one), a newer schema, and a database type that does not
     match `--db-url` (SQLite archive into SQLite, Postgres into Postgres);
   - copies the current state to `<data dir>/pre-restore-<ts>/`;
   - writes the root key, TLS files, ACME files, recovery key and the database, then migrates an
     older schema;
   - puts the archived settings and push keys in `<data dir>/restored/` for review: the active
     configuration is never overwritten (paths may differ on the new machine);
   - puts the safety copy back if any step fails.
4. Review `restored/config/adminsvr.env`, then start the service. Devices and sites reconnect with
   the same certificate and root key.

The audit chain is restored with the database; the restore itself is audited (`backup.restore`).
