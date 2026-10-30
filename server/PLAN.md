## Service plugin system

We need be able to manage and maintain a service plugin system with the server. This will allow us to edit scripts and incoporate binaries as needed.

It will consist of:

- A place for the plugins to be stored and loaded from: `~/.itcmdr/service_plugins`
- An API contract for supported types to implement:
    - Functions for the Powershell
    - FFI crate for the Rust

### Base requirements

We need to have a few things to make this work:

1. An Library for Powershell service plugins to use
1. A crate for rust service plugins to use
1. The service plugin functionality built into the server
1. A cli named `itcmdr-cli` that calls the server over gRPC
1. A new section in the Flutter application: Service Plugins
1. Protobuf and paired functionality in the server to implement this
1. An authentication system to authorize management and maintenance of the server configuration (i.e. service plugins, runtimes, access keys, etc.)
    1. Authentication implies:
        1 database involved:
            1. Sqlite3
            1. Stored at `~/.itcmdr/itcmdr.database`
        1. Encryption at rest for the configuration, database (db), and resources
    1. Authentication involves:
        1. roles:
            1. id: uuid
            1. name: text - owner, admin, user, and client
        1. users:
            1. id: text - a hash of concatinate(name + key)
            1. name: text
            1. email: text - optional
            1. phone: int - optional
            1. created_at: timestampz - Default now()
            1. role_id: uuid
            1. key: text
        1. clients:
            1. id: text - a hash of concatinate(name + key)
            1. name: text
            1. key: text
        1. a master key (which can be taken offline and used to generate service keys)
            1. Generate 24 words for recovery
            1. Generate master key pair from words
            1. Generate client key pair from master private key
            1. Create client record in db with values:
                1. name="root"
                1. key=client private key
            1. Return words and master keys
            1. Store client private key as `~/.itcmdr/keys/root`
            1. Store master keys via a hash of concatinate(master private key + '::' + master public key) as `~/.itcmdr/keys/fizzbuzz`
        1. client keys
            1. Get service name
            1. Generate 24 words for recovery
            1. Generate client key pair from words
            1. Create client record in db with values:
                1. name
                1. key=client private key
            1. Return client public key
        1. user keys
            1. Get name
            1. Get email - optional
            1. Get phone - optional
            1. Generate 24 words for recovery
            1. Generate user key pair from words
            1. Find id for user role
            1. Create user record in db with values:
                1. name
                1. email
                1. phone
                1. role_id=user role id
                1. key=user private key
            1. Return words and user public key

### How this will work

#### The server

When the server is installed, it automatically goes through the master key generation process outlined above. The returned words and master key are made available for the user to print, copy, or save to their file system. After that, the user will not see them again.

When it goes to run a job, it does the following:

1. Break up job by stages:
    1. Verify
    1. Execute
2. Gathers service plugin package information from `~/.itcmdr/service_plugins/**/package.toml`
3. Loads service plugins from `~/.itcmdr/service_plugins/` for each stage
4. Executes plugins for each stage in order:

```rust
let stages = vec![
    ServicePluginStage::Verify,
    ServicePluginStage::Execute,
];

let plugins = load_service_plugins("...").ok(); // hand waving here
let args = vec!["..."]; // more hand waving

let mut error_found = False;
for stage in stages.iter() {
    for (i, plugin) in plugins.iter().enumerate() {
        let result = match plugin.execute(stage, args) {
            Ok(r) => r,
            Err(err) => {
                error_found = True;
                plugin.execute(ServicePluginStage::Log, vec![err].extend[args])
            }
        }
    }
}
```

#### Clients

Clients go through two phases:

- Registration
- Requests

> **Registration**: how a client registers

Registration can be done through two methods:

1. `itcmdr-cli service register` - with optional flag `--name`
2. Through the Flutter application as service registration

> **Requests**: how a client makes requests

Requests outside of registration must include:

1. The public key

#### Service plugins

Service plugins go through two phases:

1. Installation
3. Execution

> **Installation**: how a plugin gets installed

Installation can be done through two methods:

1. `itcmdr-cli service plugins install` - possible flags are:
    1. `--url ...`: [exclusive] a url to the service plugin
    1. `--path ...`: [exclusive] a file path to the service plugin
2. Through the Flutter application as a service plugin import

Installation involves:
    1. Retreiving the service plugin package
        1. If the `--url` flag was used:
            1. Downloading the service plugin package to a tmp directory
        1. If the `--path` flag was used:
            1. Moving the service plugin package to a tmp directory
    1. Decompressing the package
    1. Validating the package
    1. Creating a new directory for it in `~/.itcmdr/service_plugins/{package.name}-{package.version}`
    1. Moving files from the `package.toml` `[files]` section and the `package.toml` file to `~/.itcmdr/service_plugins/{package.name}/`

> **Execution**: how a plugin works with execution

Execution will be called from the server to the service plugin's `execute(stage: ServicePluginStage, args: Vec<&str>) -> Result<(), ServicePluginError>` function.

### Package details

A package will look like the following structure:

> **tar -zxvf something-1.0.6.tar.gz:**

```     
.
├── file1
├── package.toml
└── rand.ps1
```
> **package.toml:**

> _Corrected Oct 30 2026: the original example was not valid TOML (inline tables can't span lines,
> `main` was a string). Same content, valid syntax:_

```toml
name = "something"
version = "1.0.6"
features = ["verify", "execute", "log"]

[[files]]
name = "file1"
type = "bin"
main = true
checksum = "<sha256 hex of file1>"

[[files]]
name = "rand.ps1"
type = "powershell"
checksum = "<sha256 hex of rand.ps1>"
```

A package must meet the following to be considered valid:

1. Have a `package.toml`
1. Contain all, but only all, the `files` listed plus the `package.toml`
1. All files pass sha256 checks against their stated checksum
1. If any of these fail:
    1. The package is moved to `~/.itcmdr/quarantine/`
    1. A log entry is made
    1. The request for installation is denied

---

## Implementation plan (agreed Oct 30 2026)

Michael's text above is kept as written (except the TOML example, fixed for syntax). Where it and
this section differ, this section wins. Approved by Michael on Oct 30 2026 ("Exchange gRPC instead of
JSON. The rest sounds good to me").

### Design decisions

1. **Data dir.** `%ProgramData%\itcmdr` on Windows, `~/.itcmdr` on Linux. Override order: `--data-dir` flag,
   then `ITCMDR_DATA_DIR` env, then `data_dir` in server.json, then the default. Layout:
   `service_plugins/`, `quarantine/`, `keys/`, `itcmdr.database`.
2. **Public keys only in the DB.** The `key` column of users/clients holds the *public* key. Private halves
   stay with whoever generated them (the original plan stored private keys; changed for safety).
3. **Signed requests.** Every authenticated gRPC call carries `x-itcmdr-key` (fingerprint), `x-itcmdr-ts`
   (unix ms), `x-itcmdr-nonce` (random) and `x-itcmdr-sig`: Ed25519 over
   `method || ts || nonce || sha256(request bytes)`. The server checks the stored public key, a +/-5 min
   clock window and a nonce cache (replay protection), then authorizes by role:
   owner > admin > user > client.
4. **Offline master key.** At install, 24 BIP39 words are generated and shown once; the master Ed25519 key is
   derived from them. The server stores only the master *public* key. Client/user keys are certified
   (their public key is signed by the master). The words rebuild the master for offline signing and
   recovery (`itcmdr-cli master sign|recover`). A root client keypair is created at init; its private key
   lives in `keys/root` (DPAPI-protected on Windows, mode 0600 on Linux). This replaces `keys/fizzbuzz`:
   nothing derived from the master private key is kept on the server.
5. **Ids.** Records use random UUID v4 ids (instead of hash(name+key)). The key fingerprint
   (sha256 of the public key, hex) has its own column, so a key can rotate without changing the id.
6. **Plugins are child processes that speak gRPC** (not JSON, not in-process FFI). `proto/plugin.proto`
   defines `PluginService { Describe, Verify, Execute, Log }`. Handshake (go-plugin style): the server
   starts the plugin with a per-launch secret in `ITCMDR_PLUGIN_SECRET`. The plugin binds loopback
   (127.0.0.1:0) and prints one line, `ITCMDR-PLUGIN|1|tcp|127.0.0.1:<port>`, to stdout. Every call carries
   the secret. Named pipes or unix sockets can come later.
   - Rust: an `itcmdr-plugin` crate. Implement a `Plugin` trait, then call `itcmdr_plugin::serve(MyPlugin)`.
   - PowerShell: an `Itcmdr.Plugin.psm1` module that defines the contract (`Invoke-ItcmdrPluginVerify`,
     `-Execute`, `-Log`, `Get-ItcmdrPluginInfo`), plus a small Rust host binary,
     `itcmdr-plugin-ps`. The host serves the gRPC side and runs `powershell.exe` (5.1) or `pwsh` on the
     package's main `.ps1` for each call, passing JSON on stdin and reading JSON back. The JSON stays inside the
     host; the server only speaks gRPC.
7. **Signed packages.** `package.toml` + files + `package.sig`, an Ed25519 signature by an admin/owner key
   over the canonical manifest (which includes every file's sha256). Any failure (missing or extra file,
   checksum, signature, unknown signer, bad TOML) moves the package to `quarantine/`, writes an audit and
   log entry, and denies the install.
8. **Fixes to the original flow.** `[[files]]` tables with `main = true` (boolean). Install dir is
   `service_plugins/{name}-{version}` (the original text said both `{name}` and `{name}-{version}`).
   Unpacking rejects absolute paths, `..`, symlinks and hard links. A failed Verify stage skips Execute for
   that job. Log runs whenever a stage errors.
9. **Encryption at rest.** The DB uses SQLCipher, bundled and vendored (no system library). The 32-byte DB key
   is in `keys/db.key`: DPAPI (LocalMachine) on Windows; on Linux it is a 0600 file owned by the service
   user, i.e. protected by file permissions. That is weaker than DPAPI, so use full-disk encryption on Linux hosts.
   Config secrets and plugin resources are encrypted with the same key where feasible (later releases).

10. **Built-in checks become plugins** (Michael, Oct 30 2026: "the current scripts should be rebuilt as
    plugins and used that way"). The checks and repairs in `agent/Invoke-ItcmdrAgent.ps1` are repackaged as
    signed service plugins and run through the plugin system:
    - IP/network, connectivity, scanner reachability, DNS records, drive maps, printer queues + spooler,
      Sirona drift and backup coverage. The on-demand poller and log rotation also become plugins where that fits
      (they may stay scheduled scripts that call the plugin runner, decided during v0.11.0).
    - **Same behaviour:** the existing Pester tests run against the plugin versions and must pass unchanged
      (plus plugin-contract tests).
    - **Same rules:** the clinical guard (no disruptive repair during clinic hours without `force`), exceptions
      (site.json + server export), report-only defaults, and Sirona drift detect/report only (never edited).
      Check ids and the JSON Lines log format stay the same, so ingest, metrics and history continue.
    - **Clean migration, no gap in nightly coverage:** the plugin versions first run side by side with the
      current nightly task in report-only mode and their results are compared. The scheduled task switches
      over only after matching results, and the old script stays as a fallback until one full week is clean.
11. **Plugin logging and a structured log standard** (Michael, Oct 30 2026: "When a plugin logs, I want to
    give it a path to log to so that we can back them up and gather metrics from them... Maybe we should have a
    log standard for structured logs.").
    - **Log path per run.** The server gives every plugin run a log path, both in the Verify/Execute/Log RPC
      context (`log_path`) and as the env var `ITCMDR_LOG_PATH`. Layout:
      `<data>/plugin-logs/<plugin>/<yyyy-mm-dd>.jsonl`. That folder sits inside the area the existing rotation
      (weekly, then 4-week blocks, then ISO yearly) and the backup-coverage check already cover, and it is pushed to
      the share like the agent logs.
    - **Standard `itcmdr.log/v1`** (JSONL), written up in `docs/LOG-STANDARD.md`:
      - Required fields: `schema`, `ts` (RFC 3339 with offset), `level`, `plugin`, `plugin_version`, `run_id`,
        `machine`, `stage`, `event`, `message`.
      - Optional fields: `check_id`, `status` (ok|fail|repaired|skipped|excepted), `duration_ms`, and `metrics`,
        a list of `{name, value, unit, kind: counter|gauge|histogram, labels{}}`.
      - Compatibility: the current agent lines (`type`, `runId`, `computer`, `checkId`, `status`...) keep being
        accepted and are mapped onto v1 at ingest. The agent itself moves to v1 when it becomes plugins (v0.11.0).
    - **Helpers.** The Rust crate has `itcmdr_plugin::log` (`info/warn/error`, `metric(...)`). The PowerShell module has
      `Write-ItcmdrLog` / `Write-ItcmdrMetric`. Both write conforming lines to the given path, so plugins never
      handle the format themselves.
    - **Ingest.** Each line is validated against the schema. Malformed lines are counted as rejects per file and
      shown, never silently dropped. Values in `metrics` are stored generically (name + labels + ts + value)
      next to the incident and uptime metrics. A new gRPC query, `GetPluginMetrics` (filter by name and labels,
      window, rollup), serves them, and the app charts them (v0.12.0).
    - **Releases:** the schema doc, validator and ingest go in v0.9.0, the runtime log path and helpers in v0.10.0, the
      built-in plugins on v1 in v0.11.0, and the app charts in v0.12.0.
    - **Tests:** schema validation (good and bad lines), helpers produce valid lines (Rust + Pester), ingest
      extracts metrics and counts rejects, and plugin logs are included in rotation and backup coverage.
12. **Storage follow-ups** (Michael, Oct 30 2026):
    - **Plaintext backup auto-delete (v0.6.2).** After migrating a pre-0.6 plaintext DB, the server reads the
      encrypted DB back at its final path. When the per-table row counts match the plaintext source, the
      `*.plaintext-backup` is overwritten with zeros, synced, deleted and logged. If verification fails, it is kept
      and an error is logged. Overwriting is best effort: SSDs and copy-on-write filesystems may keep old blocks.
    - **DB key escrow under the master key (v0.7.0).** `keys/db.key` is also stored wrapped for the master key, so
      it can be recovered from the 24 words. Planned scheme: the master seed (from BIP39) derives an X25519 key
      pair. The server stores only the public half. db.key is sealed with ephemeral X25519 + HKDF (SHA-256) +
      XChaCha20-Poly1305 into `keys/db.key.escrow`, which also goes into the backup. Recovery on a replacement
      machine: `itcmdr-cli master recover-db-key` takes the words, unwraps the key and protects it again with
      DPAPI (or 0600 on Linux). The exact scheme is documented and tested in v0.7.0.
13. **Backup and restore** (Michael, Oct 30 2026: "we should have a backup and restore options").
    - **Scope:** the server's whole state: the encrypted DB, `keys/db.key.escrow` (never the raw DPAPI-protected
      `db.key`), server config, installed service plugins and their config, the quarantine index, and optionally
      plugin logs and archives.
    - **Backup:**
      - A consistent snapshot, safe while the server runs (SQLCipher `VACUUM INTO` / online backup API).
      - Packed into one archive with a manifest: itcmdr version, schema version, sha256 per file.
      - Signed by the server's root key and encrypted for the master X25519 key (same scheme as the decision 12
        escrow), so it's safe on the share.
      - Nightly to the share with retention (same weekly / 4-week / yearly idea as the logs), plus on demand.
    - **Restore:**
      - Verify the manifest and signature first.
      - Restore onto a fresh or replacement machine using the 24 words: they open the archive and the db.key
        escrow, then the key is re-protected with DPAPI / 0600.
      - Refuse incompatible schema versions, or run migrations.
      - Always take a safety backup of the current state first.
      - `--dry-run` / verify-only mode.
    - **Interfaces:** `itcmdr-cli backup create|list|verify` and `itcmdr-cli restore --from <archive> [--dry-run]`;
      gRPC RPCs gated to owner/admin and audit-logged; a Backup & Restore screen in the app.
    - **Tests:** backup -> restore round trip on a fresh data dir with only the words; a tampered archive is
      rejected; a backup taken during writes stays consistent; retention rules.
    - **Releases:** core backup/restore (DB, escrow, config) plus CLI and RPCs in v0.8.0. Plugins, plugin config,
      quarantine index and plugin logs join the archive as they appear (v0.9.0/v0.10.0). App screen in v0.12.0.
      No version shift needed.

14. **Scheduled runs** (Michael, Oct 30 2026: "We should have a way to schedule runs in the app and cli").
    - **The server owns schedules**, stored in the encrypted DB: id, name, target (machines, groups or all),
      what to run (categories or specific plugins; report-only or repair), a cron expression or a preset
      (daily, weekly, every N hours), timezone (default: the server's local zone), enabled, clinical-hours
      guard (respect office hours, or block disruptive repairs during them), max concurrency, and a
      missed-run policy (`skip`, or `catch-up` = run once after downtime).
    - **One queue path.** The scheduler queues runs through the existing on-demand request path, so the
      agent poller and locks are reused. Run history is stored with the schedule id; next-run times are exposed.
      A schedule never has two queued/running requests at once (no double-queueing).
    - **gRPC:** `CreateSchedule`, `UpdateSchedule`, `DeleteSchedule`, `ListSchedules` (with next runs),
      `PauseSchedule`, `ResumeSchedule`, `RunScheduleNow`, `ListScheduleRuns`. Admin to manage, user for
      run-now; all audit-logged.
    - **CLI:** `itcmdr-cli schedule add|list|edit|rm|pause|resume|run-now|history`.
    - **App (v0.12.0):** Schedules screen: create/edit with cron presets and a next-run preview, toggle,
      run now, history.
    - **Default schedules replace the current nightly run** (Michael, Oct 30 2026: "change the current one to
      use schedules when it's safe to do so. The current one would be a default schedule on all installs"). v0.11.0:
      - Every install (fresh or upgrade) seeds built-in default schedules, idempotently: the nightly agent run
        (same time, all machines, same categories, same repair/report-only behaviour, clinical guard on) and the
        nightly backup (02:00, replacing the hard-wired loop). Editable and pausable, but not accidentally
        deletable: delete needs owner/admin plus an explicit confirm; `reset-to-default` restores them.
      - Safe cutover: until the switch, the SYSTEM scheduled task stays the source of truth and the default
        agent schedule runs report-only in parallel (or stays disabled). Switch only when (a) a report-only
        comparison matches the task's results for at least one full week (decision 10 rule), (b) the plugin
        runtime and poller are verified on the Windows VM, and (c) the agent can tell schedules are active.
      - After the switch the scheduled task stays as a disabled fallback with a documented one-command rollback.
        If the server is unreachable the agent falls back to its local nightly run, so coverage never gaps.
      - Tests: seeding idempotent on upgrade, defaults match the current task settings, cutover gating,
        fallback when the server is down, rollback.
    - **Tests:** cron parsing and next-run calculation including DST changes (spring-forward gaps run once at
      the next valid time, fall-back repeats run once), missed-run policy, no double-queueing, clinical
      guard, RPC roles and audit.
    - **Release:** server, gRPC and CLI in v0.10.0 (next to the plugin runtime, so "specific plugins" can be
      targeted); app screen in v0.12.0. No version shift.

15. **Configurable paths** (Michael, Oct 30 2026: backup paths must be preferences he can set, not hard-coded).
    - **Agent:** site.json `paths.localLogs` (default `C:\maintenanceRuns`: run logs, local `requests\`) and
      `paths.agentData` (default `C:\ProgramData\itcmdr`: exceptions cache, spool), used by every agent script.
      `shares.maintenanceRuns` is unchanged. Installer: `/DLocalLogs=...` at build time or `/LocalLogs=...` at
      install time for the folder it creates.
    - **Server:** server.json `backup` (dir, nightly, hour, retention) stays the base; runtime overrides are
      stored in the encrypted DB and win. gRPC `GetSettings` / `UpdateSettings` (admin/owner, audit-logged;
      the folder must exist and be writable before it is saved) and `itcmdr-cli config get|set|unset backup.*`.
      Schedule settings join this when decision 14 lands. App settings screen in v0.12.0.
    - **Still fixed, on purpose (bootstrap):** where site.json and server.json are looked up
      (`config\`, `%ProgramData%\itcmdr`, next to the exe, or `ITCMDR_SERVER_CONFIG`), the service log
      `%ProgramData%\itcmdr\server-logs` (written before the config loads) and the push-install staging
      folder `C:\Windows\itcmdr\agent` on remote PCs.
    - **Release:** v0.8.1 (paths, backup settings).
    - **Widened** (Michael, Oct 30 2026: "we need to yank hardcoded values and make them a setting...
      accessible via the app, not just the cli"). Every hard-coded value becomes a setting with today's value
      as the default, documented in `docs/SETTINGS.md` (name, default, scope, who can change it):
      - **One typed Settings API.** gRPC `ListSettings` / `GetSettings` / `UpdateSettings` grow from the backup
        block to a registry of typed keys (int with range, bool, duration, time of day, path, string, list),
        validated server-side, admin/owner only, audit-logged. `itcmdr-cli config list|get|set|unset` and the
        app use the same API. Server keys: DB overrides over server.json over defaults.
      - **Agent keys served centrally**, like `exceptions.json`: the server exports `agent-settings.json` to the
        maintenanceRuns share with global values and per-machine overrides; the agent merges
        built-in defaults < central (global, then its machine) < local site.json (local override/fallback),
        and caches the central file for offline runs.
      - **Audit found (today's defaults):** agent: quiet-minutes guard 10, printer stuck-job minutes 60, backup
        task/destination max age 26 h, min backup bytes 1, TCP probe timeout, poller stale-lock minutes 150,
        local log retention 400 days, install stagger 0 s, rotation time 02:10, paths (v0.8.1). Server: ingest
        interval 30 s, request timeout 3 h, DB retention 400 days, flap threshold 3, repeat-offender threshold
        3, clock skew window 5 min, search page 100 / max 1000, max package size 256 MiB, bind address/port,
        backup block (v0.8.1). App: default port 50051 (a connection preference, stays local). Fixed on
        purpose: config lookup locations, the service log folder, 24 words, schema/format versions.
      - **App:** a basic Settings screen ships with the API (v0.9.1); the full Settings section (server,
        agent, backup, schedules, paths, thresholds) with validation lands in v0.12.0.
      - **Tests:** defaults equal current behaviour, overrides apply, bad values rejected, RPC roles + audit,
        the agent picks up centrally served settings.

### Notes from the v0.9.0 build

- Package = `.tar.gz` with `package.toml`, `package.sig` and exactly the listed files (subfolders allowed).
  `package.sig` is JSON `{v, signer (fingerprint), sig}`; the Ed25519 signature covers
  `itcmdr-package/v1\n<sha256 of package.toml>`, and package.toml carries every file's checksum.
  Signers: active admin/owner keys (users or clients, so the root key works too).
- `type` is one of bin, powershell, data, config; `features` of verify, execute, log; names are
  lowercase `a-z0-9-_`, versions start with a digit.
- Installs land **disabled** unless `--enable` (enabling one version disables the others). Removing an
  enabled version needs `--force`. State in the `plugins` table (schema 2) + `service_plugins/{name}-{version}`.
- Quarantine: the original bytes go to `quarantine/<ts>-<sha12>-<name>`, a JSON line to
  `quarantine/index.jsonl`, a warning to the server log and `plugin-quarantine` to the audit log.
  A failed download or an already installed version is an error, not a quarantine.
- `itcmdr-cli plugins pack` builds and signs a package; `plugins` is also reachable as `service plugins`.
- Backups now carry `service_plugins/**` and `quarantine/index.jsonl`; restore copies the current ones
  aside first. Plugin logs join in v0.10.0. The log standard (decision 11) moved to v0.10.0.

### Auth notes from the v0.7.0 build

- `auth_mode` in server.json: `token` (legacy only), `both` (default: signed requests, or the legacy token for
  mutating calls) or `signed` (token retired; reads need a key too). The shared token is retired by switching to
  `signed`. The default stays `both` until the app signs requests (v0.12.0), or the app would lock itself out.
- The root client is created with role `owner` so it can administer the server locally (CLI in v0.8.0).
- Request signature: Ed25519 over `itcmdr-req/v1\n{grpc path}\n{ts ms}\n{nonce}\n{sha256 hex of the protobuf body}`,
  +/- 5 min clock window, per-key nonce cache.
- Master derivation is pinned by a test vector (`keys.rs::known_vector_is_stable`). Changing it would break
  words already written down.

### Notes from the v0.8.0 build

- `itcmdr-cli` is a second binary in the server crate (shares the library); it works on the local data
  dir, so run it on the server. Words are read from stdin only.
- Archive format `itcmdr-backup/v1` (`server/src/backup.rs`, `docs/BACKUP.md`). Snapshot via
  `ATTACH ... KEY` + `sqlcipher_export` (one read transaction, same db.key). Schema version is
  `PRAGMA user_version` (`db::SCHEMA_VERSION`, now 1).
- Restore is CLI-only (needs the service stopped); there is no restore RPC. It issues a new root key
  because `keys/root` is DPAPI-bound to the old machine, and it doesn't overwrite the active
  server.json (written to `restored-server.json` for review).
- Signed requests: canonical encoding rules and shared vectors in `docs/SIGNING.md`.

### Release breakdown

| Tag | Scope |
|---|---|
| v0.6.0 | storage: data dir layout, SQLCipher DB, roles/users/clients tables, migration of the existing plaintext DB |
| v0.6.2 | storage follow-up: verified migration deletes the plaintext backup (decision 12) |
| v0.7.0 | keys & auth: BIP39 words, master init (shown once), certified client/user keys, signed requests + roles, db.key escrow under the master key + recovery (decision 12); the shared token is retired once the new auth works (migration path documented) |
| v0.8.0 | itcmdr-cli: init, service register [--name], user add, key rotate/revoke; backup create/list/verify + restore [--dry-run], backup RPCs, nightly backup to the share (decision 13) |
| v0.8.1 | configurable paths (decision 15): agent `paths.localLogs`/`agentData`, installer LocalLogs, backup settings RPCs + `itcmdr-cli config`; backup retention by 4-week blocks; plans for decisions 14-15 |
| v0.9.0 | packages: format, validation (exact file list, sha256, signature), quarantine + index, plugins install --url/--path, list/enable/disable/remove, plugin files in backups |
| v0.9.1 | settings everywhere (decision 15 widened): typed Settings registry + `ListSettings`/`GetSettings`/`UpdateSettings`, `itcmdr-cli config list`, central agent settings (global + per machine) with site.json as local override, `docs/SETTINGS.md`, basic app Settings screen |
| v0.10.0 | plugin runtime: PluginService gRPC, Rust crate, PowerShell module + host, Verify then Execute, Log on error, timeouts, on-demand + audit hooks; per-run log path + log/metric helpers; log standard doc + validator + generic metrics ingest (decision 11, moved from v0.9.0); scheduled runs: server scheduler, schedule RPCs, `itcmdr-cli schedule` (decision 14) |
| v0.11.0 | built-in checks as plugins: the existing agent checks/repairs rebuilt as service plugins and run through the plugin system (decision 10), logging in itcmdr.log/v1; nightly agent task and nightly backup as built-in schedules (decision 14, no-gap rule) |
| v0.12.0 | Flutter: Service Plugins section, app registration, user/key management, plugin metric charts, Backup & Restore screen, Schedules screen; app signs requests, default auth_mode -> signed |
| v0.13.0 | installer: first-run master setup showing the words once; Windows VM test items in the handover |
