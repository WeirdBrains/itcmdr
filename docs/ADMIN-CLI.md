# itcmdr-admin: the adminsvr CLI

`itcmdr-admin` is the command line client for [adminsvr](ADMINSVR.md). Each device has its own key,
certified with your 24 words at enrollment; every request is signed. The identity lives in the user
config folder (`itcmdr-admin`, mode 0600) or `ITCMDR_ADMIN_HOME`. Every command and flag:
`man itcmdr-admin` ([man pages](MAN.md)).

| Command | What |
|---|---|
| `info <addr>`, `trust show\|review\|set\|remove` | server version, certificate mode and expiry; [certificate trust](CERTIFICATES.md) |
| `enroll <code>`, `recover <addr> ...`, `whoami` | enroll this device (24 words shown once), a new device from the words |
| `user ...`, `device ...` | people, roles (owner, admin, sre), device approvals |
| `key-backup on\|off` | the encrypted key backup (on by default) |
| `site ...` | create sites (one-time site token), list with status and last deep check, metrics |
| `alert list\|ack\|resolve` | alerts from every site you may see |
| `backup start\|drill <site>` | start a backup; the restore drill (ok or failed only) |
| `command ...`, `freeze on\|off` | signed commands and their approvals; stop everything (owners) |
| `config show\|set`, `schedule ...`, `plugin ...`, `release ...` | a site's settings and schedules, plugins and releases (`--dry-run` previews) |
| `org policy` | second approver on or off, deep-check limit (owners) |
| `passkey ...`, `policy ...` | passkeys and the step-up policy (libfido2's tools; `ITCMDR_FIDO2_DEVICE`) |
| `push register\|unregister\|test` | push notifications for this device |
| `audit list\|verify` | the hash-chained audit log |
| `help [topic]` | these guides in the terminal ([in-app help](HELP.md)) |
