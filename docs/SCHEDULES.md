# Schedules

Scheduled runs live on the site server and are fired by the agents (also while the server is
offline; reports are delivered later). Two built-in schedules exist: `nightly-agent-run` and
`nightly-backup`.

    itcmdr-cli schedule add --name "weekday morning" --when "weekdays 07:00" --tz local --all --categories network
    itcmdr-cli schedule list
    itcmdr-cli schedule edit <id> --when "daily 01:30"
    itcmdr-cli schedule pause <id> | resume <id> | run-now <id>
    itcmdr-cli schedule history [<id>]
    itcmdr-cli schedule reset-to-default nightly-agent-run
    itcmdr-cli schedule rm <id>                  # built-ins: pause instead (rm needs --force --confirm <name>)

- `--when`: `hourly`, `every 6h`, `daily 01:30`, `weekdays 07:00`, `weekly mon 02:00`, or `cron M H DOM MON DOW`;
  `--tz` an IANA zone or `local`.
- Targets: `--all`, `--machines`, or `--groups` (agent roles: Server, Workstation, RCU). `--plugins` runs
  service plugins on the server.
- Report-only by default; `--repair` allows repairs. `--guard report-only-in-hours` (default),
  `skip-in-hours` or `none` protects office hours (`agent.officeHours`). The agent's clinical guard
  (recent changes to clinical data make the run report-only) always applies, also to `run-now`.
- Missed runs: `--missed skip` (default) or `catch-up`.

## Cutover from the nightly task

Older installs ran the nightly from a Windows scheduled task. `itcmdr-cli cutover status` shows
the parity results and what still blocks the switch; `cutover switch --to agent` switches only when
every gate passes; `cutover rollback` puts the task back in one command.
