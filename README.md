# itcmdr

Self-healing IT for small offices, by Weird Brains. An agent on every PC checks the network,
connections and services each night and repairs what it safely can. A site server in the office
keeps the history, alerts, metrics and encrypted backups, and the office app shows it all. Nothing
internal leaves the office.

## What's in the box

| Part | What it is |
|---|---|
| `agent-rs/` | Rust agent (Windows service): signed plugins, schedules, self-upgrade with a watchdog |
| `agent/` | PowerShell checks and plugin runner used by the agent |
| `server/` | Site server (`itcmdr-server`, `itcmdr-cli`): history, alerts, metrics, backups, upgrades |
| `app/` | Office app (Flutter, Windows/macOS/Linux) |
| `adminsvr/`, `admin-cli/`, `admin-app/` | Optional admin server for IT providers: redacted data only, signed commands, two-person approval as an option |
| `installer/` | Windows installers (Inno Setup) |

## Get it

Releases: https://github.com/WeirdBrains/itcmdr/releases (Windows installers, macOS and Linux
builds, the admin app for Android, `SHA256SUMS`). Builds are not code-signed yet, so Windows
SmartScreen and macOS Gatekeeper will warn.

## Docs

Start at [docs/README.md](docs/README.md): a guide per feature (install, the site server and app, agents,
alerts, schedules, plugins, backups, upgrades, keys, certificates, adminsvr), deploy guides for a site
server, adminsvr with Let's Encrypt and the website, every environment variable, and the man pages in
[man/](man/). The same guides are the in-app help (`itcmdr-cli help`, `itcmdr-admin help`, Help in both
apps). Changes: [CHANGELOG.md](CHANGELOG.md) · Roadmap: [PLAN.md](PLAN.md).

## Privacy and security

- The site server decides what leaves the office and denies by default: no hostnames, IPs, paths or usernames go off-site. Backups never leave the site server.
- A 24-word master key (shown once) backs certified keys; every request is signed; agents use mutual TLS; plugins and releases are signed and tampered ones are quarantined; the database is encrypted.

This repository uses neutral placeholders and example ranges (RFC 5737) only. Site-specific values
live in a local, gitignored `config/site.json` or in `C:\ProgramData\itcmdr\site.json` on Windows.
