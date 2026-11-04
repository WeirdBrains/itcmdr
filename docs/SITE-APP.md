# Site app (itcmdr)

The office app (Flutter; Windows, macOS, Linux) talks to the site server's HTTPS API. Every request
is signed with your app key; the key seed stays in your user profile (Windows DPAPI on Windows).

## App id (v1.2.0)

The app's id is `com.weirdbrains.itcmdr` (Linux application id, macOS bundle id; Windows company
`com.weirdbrains`). Before 1.2.0 it was the Flutter placeholder `com.example.app`. Upgrading keeps
working in place on every platform (there is no Android or iOS build of the office app):

- Windows and Linux: the settings (server, HTTPS, trusted certificates) are copied once from the
  old folder (`%APPDATA%\com.example\itcmdr`, `~/.local/share/com.example.app`) on first start.
- macOS: the settings live in the app's sandbox container, which is per bundle id, so enter the
  server and trust its certificate once more after upgrading. The app key is in the keychain and
  is kept.

## First start

1. **Settings**: the server host and port (default 50051), your name for the audit log, and
   **Use HTTPS** (on by default; plain HTTP asks first). The first connection shows the server's
   certificate fingerprint: compare it with the server and trust it ([certificates](CERTIFICATES.md)).
2. **Keys**: make or import a key and have an owner certify it ([keys](KEYS.md)). With
   *This PC also runs the itcmdr app* ticked in the server installer, first-run setup does this for you.

## Pages

| Page | What it shows or does |
|---|---|
| Overview | machines, open incidents, recent runs; refreshes live |
| Alerts | open alerts worst first (the badge counts them); acknowledge, resolve, silence ([alerts](ALERTS.md)) |
| Agents | enrolled agents and their heartbeat state, enrollment tokens, maintenance, revoke ([agent](AGENT.md)) |
| Metrics | incidents, uptime, repair time and failures over 7, 30, 90 or 365 days, with charts |
| Logs | search runs and check results |
| Exceptions | allowlist rules (ignore or downgrade a check, with an expiry) |
| On-demand | run checks now on some or all machines |
| Schedules | scheduled runs: add, edit, pause, run now, history ([schedules](SCHEDULES.md)) |
| Plugins | installed plugins, enable, disable, quarantine ([plugins](PLUGINS.md)) |
| Updates | the update window and agent/server rollouts ([upgrades](UPGRADES.md)) |
| Backup | backups, start one, restore, the last deep check ([backup](BACKUP.md)) |
| Audit | who changed what, when |
| Calls | the call log, one row per request |
| Keys | your key, WhoAmI, certify and register keys |
| Settings | server, HTTPS, trusted certificates, your name |
| Help | these guides, searchable ([in-app help](HELP.md)) |

Pages you may not use (by role) show the reason instead of data.
