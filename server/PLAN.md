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

### Release breakdown

| Tag | Scope |
|---|---|
| v0.6.0 | storage: data dir layout, SQLCipher DB, roles/users/clients tables, migration of the existing plaintext DB |
| v0.7.0 | keys & auth: BIP39 words, master init (shown once), certified client/user keys, signed requests + roles; the shared token is retired once the new auth works (migration path documented) |
| v0.8.0 | itcmdr-cli: init, service register [--name], user add, key rotate/revoke |
| v0.9.0 | packages: format, validation (exact file list, sha256, signature), quarantine, plugins install --url/--path, list/enable/disable/remove; log standard doc + validator + generic metrics ingest (decision 11) |
| v0.10.0 | plugin runtime: PluginService gRPC, Rust crate, PowerShell module + host, Verify then Execute, Log on error, timeouts, on-demand + audit hooks; per-run log path + log/metric helpers |
| v0.11.0 | built-in checks as plugins: the existing agent checks/repairs rebuilt as service plugins and run through the plugin system (decision 10), logging in itcmdr.log/v1 |
| v0.12.0 | Flutter: Service Plugins section, app registration, user/key management, plugin metric charts |
| v0.13.0 | installer: first-run master setup showing the words once; Windows VM test items in the handover |
