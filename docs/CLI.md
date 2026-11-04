# itcmdr-cli: site server administration

`itcmdr-cli` runs on the site server (it comes with the server package; the CLI package puts it on
PATH on other machines). Most commands read the server's data folder directly; some talk to the
running server over HTTPS, pinned to the local server certificate (`--plain-http` is the explicit
opt-in for plain HTTP). Commands that need the 24 words ask for them on the console and never log them.

Global options: `--data-dir <dir>` (beats `ITCMDR_DATA_DIR` and server.json), `--json`, `--plain-http`.
The full list of commands and flags is in `man itcmdr-cli` ([man pages](MAN.md)).

| Command | What |
|---|---|
| `setup check \| apply \| first-run` | install checks, site values into server.json, first-run (24 words, first owner key) |
| `init`, `recover-db-key` | create the master key; restore `keys\db.key` on a replacement machine |
| `keys certify \| new \| register \| list \| revoke \| rotate` | [keys](KEYS.md) |
| `user add` | certify and register a person's key |
| `auth status \| set-mode` | signed-only or the legacy token ([install](INSTALL.md)) |
| `config list \| get \| set \| unset` | runtime [settings](SETTINGS.md) |
| `schedule ...`, `cutover ...` | [schedules](SCHEDULES.md) |
| `plugins ...`, `service register` | [plugins](PLUGINS.md) |
| `agent enroll-token \| list \| revoke \| renew \| run \| runs \| plugins \| maintenance` | [agents](AGENT.md) |
| `alerts list \| ack \| resolve \| silence` | [alerts](ALERTS.md) |
| `backup ...`, `restore`, `restore-stage`, `restore-status` | [backup and restore](BACKUP.md) |
| `upgrade ...` | agent rollouts and server upgrades ([upgrades](UPGRADES.md)) |
| `calls list \| show \| prune` | the call log |
| `adminsvr enroll \| status \| unlink \| share-names \| allow` | the offsite link ([adminsvr](ADMINSVR.md)) |
| `help [topic]` | these guides in the terminal ([in-app help](HELP.md)) |

Exit status: 0 on success, 1 on an error (message on stderr; `setup check` exits 1 when a check
fails), 2 for invalid usage.
