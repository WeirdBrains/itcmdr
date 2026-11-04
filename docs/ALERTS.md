# Alerts

The site server raises alerts; there are no outside notifiers. They show in the site app (Alerts
page and badge) and in `itcmdr-cli alerts`, and, when the site is linked, go up to
[adminsvr](ADMINSVR.md) redacted (opaque ids, codes from a fixed vocabulary).

- **Levels:** `critical`, `cautious`, `informational`.
- **Sources:** `plugin` (plugin errors), `check` (agent check results), `backup` (backup outcome),
  `package` (quarantined packages), `server` (for example agent heartbeats, certificate expiry, upgrades).
- **Dedupe:** one row per key while it is open, acknowledged or silenced (the count goes up and the
  level only goes up). A resolved alert raised again within `alerts.dedupe_window_mins` reopens.
- **Auto-resolve:** the same source reporting ok resolves it.
- **Silences:** by machine and/or source for some hours; new matching alerts are stored as silenced.

## Commands

    itcmdr-cli alerts list [--state open|resolved|all|acked|silenced] [--level critical] [--machine WS-01] [--source backup]
    itcmdr-cli alerts ack <id>
    itcmdr-cli alerts resolve <id>
    itcmdr-cli alerts silence --machine WS-01 --hours 4 --reason "moving desks"

An agent (or any local admin script) can queue an alert that is delivered also after an outage:
`itcmdr-agent alert <key> <title> [--level ...] [--detail ...] [--resolve]` ([agent](AGENT.md)).

adminsvr has its own alerts for the sites it watches (`itcmdr-admin alert list|ack|resolve`, push
notifications) and for itself (for example `tls-acme`).

- `installer-server` / `installer-agent` (v1.2.1, Critical): the last Windows installer run of that package failed; the title names the step and the version still installed, the detail has the exit code and the log path. A good run resolves it. See docs/UPGRADES.md, Installer logs.
