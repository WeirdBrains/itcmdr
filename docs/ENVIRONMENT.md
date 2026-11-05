# Environment variables

Every `ITCMDR_*` variable read by itcmdr. The [man pages](MAN.md) are generated from this table, and
a test fails when a variable in the source is missing here. adminsvr flags each have an
`ITCMDR_ADMINSVR_*` variable; values in the environment win over the env file.

## Programs

| Variable | Used by | Meaning |
|---|---|---|
| `ITCMDR_SERVER_CONFIG` | itcmdr-server, itcmdr-cli | path of server.json |
| `ITCMDR_DATA_DIR` | itcmdr-server, itcmdr-cli | data folder (`--data-dir` beats it; it beats server.json) |
| `ITCMDR_SERVER_PROGRAM` | itcmdr-server | the server program folder for self-upgrade (default: the exe's folder) |
| `ITCMDR_RESTORE_BIN` | itcmdr-server | path of itcmdr-restore (default: next to the server) |
| `ITCMDR_PLUGIN_PS_HOST` | itcmdr-server, itcmdr-agent | path of itcmdr-plugin-ps (default: next to the exe) |
| `ITCMDR_POWERSHELL` | itcmdr-plugin-ps, itcmdr-server | PowerShell to run (default powershell.exe on Windows, pwsh elsewhere) |
| `ITCMDR_PLUGIN_SCRIPT` | itcmdr-plugin-ps | the plugin script, instead of `--script` |
| `ITCMDR_AGENT_DIR` | itcmdr-agent, itcmdr-agent-watchdog | agent data folder (default %ProgramData%\itcmdr\agent, /var/lib/itcmdr-agent) |
| `ITCMDR_AGENT_PROGRAM` | itcmdr-agent | the agent program folder (holding `current\`) for self-upgrade |
| `ITCMDR_PLUGIN_START_SECS` | itcmdr-agent | seconds a plugin has to answer the handshake (default 60) |
| `ITCMDR_PS_MODULE` | itcmdr-agent, plugins | path of Itcmdr.Plugin.psm1 for PowerShell plugins |
| `ITCMDR_ADMIN_HOME` | itcmdr-admin | this device's identity folder (default: the user config folder) |
| `ITCMDR_FIDO2_DEVICE` | itcmdr-admin | libfido2 device path for passkeys (default: the first one) |
| `ITCMDR_EXTRA_ROOTS` | itcmdr-admin, itcmdr-cli | extra PEM root files added to the system trust store (PATH-style list) |
| `ITCMDR_ADMINSVR_ENV_FILE` | itcmdr-adminsvr | KEY=VALUE settings file |
| `ITCMDR_ADMINSVR_DB_URL` | itcmdr-adminsvr | `postgres://...` or `sqlite://<path>` (default SQLite in the data folder) |
| `ITCMDR_ADMINSVR_DATA_DIR` | itcmdr-adminsvr | data folder (default /var/lib/itcmdr-adminsvr) |
| `ITCMDR_ADMINSVR_API_ADDR` | itcmdr-adminsvr | admin API listen address (default 0.0.0.0:8444) |
| `ITCMDR_ADMINSVR_SITE_ADDR` | itcmdr-adminsvr | SiteLink listen address (default 0.0.0.0:8443) |
| `ITCMDR_ADMINSVR_PUBLIC_API` | itcmdr-adminsvr | host:port clients use (put into enrollment codes) |
| `ITCMDR_ADMINSVR_PUBLIC_SITE` | itcmdr-adminsvr | host:port sites use (put into site tokens) |
| `ITCMDR_ADMINSVR_TLS_NAMES` | itcmdr-adminsvr | extra internal certificate names, comma separated |
| `ITCMDR_ADMINSVR_HEARTBEAT_SECS` | itcmdr-adminsvr | site status interval (default 60) |
| `ITCMDR_ADMINSVR_SITE_MISSED` | itcmdr-adminsvr | missed heartbeats before a site is down (default 5) |
| `ITCMDR_ADMINSVR_RP_ID` | itcmdr-adminsvr | WebAuthn RP id (default: the host of the public API) |
| `ITCMDR_ADMINSVR_WEB_ORIGINS` | itcmdr-adminsvr | extra WebAuthn origins, comma separated |
| `ITCMDR_ADMINSVR_FCM_SERVICE_ACCOUNT` | itcmdr-adminsvr | Firebase service account JSON (push to Android) |
| `ITCMDR_ADMINSVR_FCM_URL` | itcmdr-adminsvr | FCM endpoint override |
| `ITCMDR_ADMINSVR_APNS_KEY` | itcmdr-adminsvr | APNs .p8 auth key file |
| `ITCMDR_ADMINSVR_APNS_KEY_ID` | itcmdr-adminsvr | APNs key id |
| `ITCMDR_ADMINSVR_APNS_TEAM_ID` | itcmdr-adminsvr | Apple team id |
| `ITCMDR_ADMINSVR_APNS_TOPIC` | itcmdr-adminsvr | the admin app's bundle id |
| `ITCMDR_ADMINSVR_APNS_URL` | itcmdr-adminsvr | APNs endpoint (sandbox for development builds) |
| `ITCMDR_ADMINSVR_PUSH_MIN_SEVERITY` | itcmdr-adminsvr | lowest pushed severity (default cautious) |
| `ITCMDR_ADMINSVR_ACME_DOMAINS` | itcmdr-adminsvr | Let's Encrypt names for the admin API, comma separated (empty = off) |
| `ITCMDR_ADMINSVR_ACME_CONTACT` | itcmdr-adminsvr | ACME account contact e-mail(s) |
| `ITCMDR_ADMINSVR_ACME_DIRECTORY` | itcmdr-adminsvr | production, staging or a directory URL (default production) |
| `ITCMDR_ADMINSVR_ACME_DIRECTORY_ROOT` | itcmdr-adminsvr | PEM root for a test CA's directory |
| `ITCMDR_ADMINSVR_ACME_CHALLENGE` | itcmdr-adminsvr | tls-alpn-01 (default) or http-01 |
| `ITCMDR_ADMINSVR_ACME_HTTP_ADDR` | itcmdr-adminsvr | HTTP-01 listener (default 0.0.0.0:80) |
| `ITCMDR_ADMINSVR_ACME_CACHE_DIR` | itcmdr-adminsvr | account key and certificate (default <data dir>/acme) |
| `ITCMDR_ADMINSVR_ACME_RENEW_DAYS` | itcmdr-adminsvr | renew this many days before expiry (default 30) |
| `ITCMDR_ADMINSVR_ACME_RETRY_SECS` | itcmdr-adminsvr | wait after a failed attempt (default 3600) |
| `ITCMDR_ADMINSVR_BACKUP_DIR` | itcmdr-adminsvr | v1.2.0 folder for adminsvr's own backups (default <data dir>/backups; [adminsvr backup](BACKUP-ADMINSVR.md)) |
| `ITCMDR_ADMINSVR_BACKUP_HOUR` | itcmdr-adminsvr | nightly backup hour, 0-23 local, or `off` (default 3) |
| `ITCMDR_ADMINSVR_BACKUP_KEEP` | itcmdr-adminsvr | nightly backups kept (default 14; manual ones are never pruned) |

## Set for plugins

| Variable | Used by | Meaning |
|---|---|---|
| `ITCMDR_PLUGIN_SECRET` | plugins | per-launch secret for the plugin protocol |
| `ITCMDR_LOG_PATH` | plugins | the run's itcmdr.log/v1 file |
| `ITCMDR_RUN_ID` | plugins | the run id |
| `ITCMDR_MACHINE` | plugins | the machine name |
| `ITCMDR_REPORT_ONLY` | plugins | 1 = do not repair |
| `ITCMDR_ARTIFACT_DIR` | plugins | folder for files the plugin hands back |
| `ITCMDR_SITE_PATH` | plugins | site.json for the PowerShell checks |
| `ITCMDR_CHECKS_LIB` | plugins | path of Itcmdr.Checks.ps1 |
| `ITCMDR_TCP_TIMEOUT_MS` | plugins | TCP check timeout override |
| `ITCMDR_IMPORT_ONLY` | plugins | load the PowerShell agent's functions without running it |

## Development and tests

| Variable | Used by | Meaning |
|---|---|---|
| `ITCMDR_TEST_SLOW` | tests | scale test deadlines (default 3) |
| `ITCMDR_TEST_PG_URL` | tests | use this Postgres instead of starting one |
| `ITCMDR_PG_BIN` | tests | Postgres binaries for tests (default ~/Projects/tools/pgsql/bin) |
| `ITCMDR_REQUIRE_PG` | tests | fail instead of skipping without Postgres |
| `ITCMDR_SKIP_PEBBLE` | tests | skip the Let's Encrypt (Pebble) test |
| `ITCMDR_REGEN_VECTORS` | tests | regenerate signing vectors |
| `ITCMDR_RESTORE_FORCE_HEALTH_FAIL` | tests | make the restore health check fail (rollback test) |
| `ITCMDR_E2E_BIN_DIR` | tests | server binaries for the app's end-to-end test |
| `ITCMDR_KEYS_BRIDGE_LIB` | tests | the app's Rust bridge library for Dart tests |
| `ITCMDR_FAULT_ROOT` | tests | fault-injection suite root |
| `ITCMDR_FAULT_SERVICE` | tests | service name for the fault suite |
| `ITCMDR_FAULT_TESTBED` | tests | enables the fault suite (testbed machines only) |
| `ITCMDR_PLUGINS_REPO` | scripts | plugins repository for fetch-plugins.sh |
| `ITCMDR_LEAK_PATTERNS` | scripts | extra leak patterns for release-local.sh check |
| `ITCMDR_KEEP_BUILD` | scripts | `1`: release-local.sh skips its automatic clean step after unix/android/ios-check (to chain several targets; run `release-local.sh clean` at the end) |
