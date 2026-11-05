# itcmdr documentation

itcmdr is self-healing IT for small offices: an agent on every PC checks the network, connections and
services and repairs what it safely can; a site server in the office keeps the history, alerts and
encrypted backups; nothing internal leaves the office. These pages are also the in-app help (the Help
page in both apps, `itcmdr-cli help <topic>`, `itcmdr-admin help <topic>`).

## Getting started

- [Install](INSTALL.md): packages, the server, agents, the app, switching to signed-only
- [Deploying a site server](DEPLOY-SITE-SERVER.md): the checklist for one office
- [Architecture](ARCHITECTURE.md): components, runs, incidents, metrics, rollups

## In the office

- [Site server](SERVER.md): ports, configuration, what it does
- [Site app](SITE-APP.md): every page
- [itcmdr-cli](CLI.md): administration on the server
- [Agent](AGENT.md): enrollment, the link, heartbeats, certificates, the offline queue
- [Settings](SETTINGS.md): every runtime setting and where it comes from
- [Alerts](ALERTS.md): levels, sources, dedupe, silences
- [Schedules](SCHEDULES.md): scheduled runs, guards, the cutover from the nightly task
- [Plugins](PLUGINS.md): signed packages, install, quarantine, runs on the agents
- [Log standard](LOG-STANDARD.md): itcmdr.log/v1, plugin errors, the plugin protocol
- [Backup and restore](BACKUP.md): nightly backups, the restore drill, restoring onto a new machine
- [Upgrades and watchdogs](UPGRADES.md): the update window, agent rings, server upgrades, rollback

## Security

- [Keys, roles and signed requests](KEYS.md): the 24 words, certified keys, roles
- [Signing](SIGNING.md): the canonical request encoding, sealed words, where keys are kept
- [HTTPS and certificate trust](CERTIFICATES.md): system trust store, trust on first use, plain HTTP opt-in

## For IT providers (optional)

- [adminsvr](ADMINSVR.md): what leaves a site, install, commands, approvals, releases, passkeys, push, Let's Encrypt
- [Deploying adminsvr with Let's Encrypt](DEPLOY-ADMINSVR.md): DNS, firewall, ACME settings per install
- [Demo](DEMO.md): adminsvr on a Linux box, the office server and two client PCs on Windows, step by step (`scripts/demo.sh`)
- [adminsvr backup and restore](BACKUP-ADMINSVR.md): adminsvr's own encrypted backups, the drill, restoring onto a new machine
- [Admin app](ADMIN-APP.md) and [itcmdr-admin](ADMIN-CLI.md)

## Reference

- [Environment variables](ENVIRONMENT.md): every `ITCMDR_*` variable
- [Man pages](MAN.md): generated from the programs, every flag
- [In-app help](HELP.md): how these pages are built into the apps and CLIs

## Development

- [Local release builds](RELEASE-LOCAL.md) and [the website](DEPLOY-WEBSITE.md)
- [Handover notes](HANDOVER.md): the state of work, how to test, the release procedure
- [Changelog](../CHANGELOG.md) and [roadmap](../PLAN.md)
- [1.0 archive](historical/1.0/): the finished plans, design documents and the changelog through 1.0
