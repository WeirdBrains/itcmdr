# Upgrades and watchdogs

The agents and the site server upgrade themselves from owner-signed release packages
(`itcmdr-agent-release`, `itcmdr-server-release`), with a watchdog that rolls back a bad release.

## Update window

`updates.auto` (default true) and `updates.window` (default `daily 01:00-04:00` in `updates.timezone`,
default the server's zone). The window holds the server upgrade, agent offers and ring advances. A range
like `Mon-Fri 22:00-02:00` belongs to the day it starts. A bad window fails closed (nothing installs).
Change it with `itcmdr-cli config set updates.window "Sat,Sun 02:00-05:00"`, the site app's Updates page,
or `itcmdr-admin config set <site> updates.window ...`. `agent.upgrades = false` holds one machine back.

## Agents: rings

    itcmdr-cli plugins install --path itcmdr-agent-release-<ver>.tar.gz
    itcmdr-cli upgrade start <ver> [--now]       # ring 0 first; --now ignores the window
    itcmdr-cli upgrade status | pause | resume | abort

A release is a service-plugin package with a `release.toml` (`version`, `min_from`), signed by an owner
key; it is never pushed as a plugin. Rings come from `agent.ring` (0 = a test PC, 1 = one PC,
2 = everything). When every agent of a ring runs the new version with fresh heartbeats, the ring soaks
for `agent.upgradeSoakMins` and the next ring starts. A failed apply, a watchdog rollback or a new
Critical alert on a ring machine pauses the rollout and raises a Critical alert.

## The site server

    itcmdr-cli upgrade server-start <ver> [--now]
    itcmdr-cli upgrade server-abort              # while it still waits for the window

The server takes a database safety copy and stages the new files; `itcmdr-server-watchdog` swaps
them in and health-checks. On failure it rolls back and restores the database
(`itcmdr-cli upgrade restore-db`) and opens a Critical `server-upgrade` alert. Agent releases that
need a newer server (`min_server`) wait for it.

## Watchdogs

`itcmdr-agent-watchdog` and `itcmdr-server-watchdog` restart their service when its health file is
older than `--stale-secs` (default 300) and perform the swap and rollback. `--once` runs one pass.

## From adminsvr

Approve a release once and roll it out to the sites you choose: `itcmdr-admin release approve|rollout`
([adminsvr](ADMINSVR.md)). The site must allow `release.rollout` (`itcmdr-cli adminsvr allow ...`).
