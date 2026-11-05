# Demo: one Linux box and three Windows machines

A live walk-through of itcmdr with the published release files: **adminsvr**, the admin CLI and the
apps on a Linux box, the **office server** (itcmdr server + agent) on one Windows machine and
**two client PCs** (agents) on two more. `scripts/demo.sh` runs every step with a pause before each
one; this page says what you type and what you see. About 45 minutes live (plus the one-time VM
setup); the unattended dry run (`--yes`) takes about 20.

Nothing is installed system-wide on the Linux box: adminsvr runs as your user with its data in
`DEMO_DIR` (default `~/.cache/itcmdr-demo`). All addresses come from environment variables.

## Setup (once)

- The Windows machines run as [dockur/windows](https://github.com/dockur/windows) VMs in docker
  compose on the Linux box (any Windows PCs with OpenSSH work too). The office server VM already has
  OpenSSH with the box's key in `C:\ProgramData\ssh\administrators_authorized_keys`.
- Client VMs: `DEMO_COMPOSE_DIR=<folder of the compose file> scripts/demo.sh provision-vms` writes
  `<compose folder>-demo/docker-compose.demo.yml` (the main compose file is never edited), layers it
  with `docker compose -f docker-compose.yml -f <override>` and starts `windows-client1`/`windows-client2` (Windows 10 LTSC evaluation, 2 cores,
  3 GB RAM, 40 GB disk, SSH on ports 2223/2224, screen on 8007/8008, on the same compose network as
  the server VM) and starts them. Windows installs itself in 30-60 minutes; a first-boot script turns
  on OpenSSH with the box's key. Then SSH to each once and accept its host key after checking the
  fingerprint (the demo never skips host key checks).
- Environment for the demo:

        export DEMO_ADMIN_HOST=<address the VMs reach this box at>
        export DEMO_SERVER_ADDR=<address the client VMs reach the server VM at>
        # optional: DEMO_SERVER_SSH=2222 DEMO_CLIENT_SSH="2223 2224" DEMO_CLIENT_NAMES="<PC1> <PC2>" (default: their computer names)
        # SSH users: DEMO_SERVER_USER (default: your login) DEMO_CLIENT_USER (default demo)
        # server VM ports not reachable from the clients (dockur publishes only listed ports):
        #   DEMO_TUNNEL_BIND=<compose network gateway> forwards <it>:50051/50052 to the server VM over SSH
        # DEMO_VERSION before 1.3.2: step 6 installs the VC++ runtime on the clients (1.3.2+ need none).
        # live demo: DEMO_INTERACTIVE=1 (words on screen, asked back, never written to a file)

## The steps

`scripts/demo.sh` (all, pausing), `--step N` (resume at N), `--only N`, `--yes` (no pauses),
`list`.

| # | What runs | What you see |
|---|---|---|
| 1 | Downloads the release files, checks SHA256SUMS, reaches every VM | `OK` per file, each VM's name |
| 2 | `itcmdr-adminsvr init` (first owner) and `serve` as your user | "admin API listening", "SiteLink listening", `tls-status`: self-signed with its fingerprint |
| 3 | `itcmdr-admin enroll <code>` | the 24 words once (live) and three asked back; `whoami`: owner; `trust show`: pinned certificate |
| 4 | Server VM: `itcmdr-server-Setup` and `itcmdr-agent-Setup` silent; `setup first-run` in person if new | exit 0, four services Running, `itcmdr-cli 1.3.1`, `setup check` lines |
| 5 | `itcmdr-admin site create "Demo Office"`, on the server `itcmdr-cli adminsvr enroll <token>` | "linked ... (pinned ...)"; `site list`: online, v1.3.1, agents online 1, "deep check never" warning |
| 6 | Per client: `agent enroll-token --machine <PC> --address <server>:50052`, agent installer `/EnrollToken=`; rings (`config set agent.ring 0/1 --machine`) | setup exit 0; `agent list`: three machines online |
| 7 | Admin app and office app bundles from the release; `itcmdr-admin user create` for the app's enrollment code | Admin app: Servers, Deep backup checks, Alerts |
| 8 | `agent list`, `site list`, `site metrics` | the fleet; adminsvr has counts and opaque ids only |
| 9 | Kills the client's agent, then the server process | "back as pid ... after N s"; the server back through its watchdog |
| 10 | Stops a client's agent for ~3 minutes | `alerts list` on the server and `itcmdr-admin alert list`: the machine late / not responding; resolved after it starts again |
| 11 | `backup create/list/drill` on the server; `itcmdr-admin backup start/drill` | archive name and "OK (verified)"; adminsvr: the commands and their result only |
| 12 | `backup drill --deep` on the server (with the words) | "OK (decrypted)"; `site list` and the admin app's Deep backup checks: ok, today |
| 13 | `config get updates.window`, `upgrade status`, `itcmdr-admin release list` | the window, no rollout yet, rings 0/1/2 |
| 14 | `setup check`, `install-logs` on the server and a client | the checks; the newest installer logs (20 kept per part) |

Rollouts (step 13) need an owner-signed `itcmdr-agent-release` package of a newer version
(`itcmdr-cli plugins pack`, [upgrades](UPGRADES.md)); the release files do not include one. With one:
`itcmdr-cli plugins install --path <pkg>`, `itcmdr-cli upgrade start <ver> --now`, and `upgrade status`
shows ring 0 (the first PC), then ring 1, then the rest, each after `agent.upgradeSoakMins`.

## Demo computer names

For recordings, `scripts/demo.sh rename-vms` gives the VMs demo names (`DEMO_RENAME`, default
`OFFICE-SRV FRONT-DESK EXAM-ROOM` for the server VM and the two clients) and restarts them. It keeps their
original names in `DEMO_NAMES_FILE` (`~/.cache/itcmdr-demo-names`, never in the repo); `teardown` (or
`restore-names`) renames them back. Agents enroll under the computer name, so rename before step 4.

## Teardown

`scripts/demo.sh teardown` uninstalls the client agents and revokes them, removes the rings,
unlinks the site from adminsvr, stops adminsvr and deletes `DEMO_DIR` (and the dry run's words file
on the server VM). `teardown --vms` also removes the client VMs and their disks and the override
folder; the main compose file and the server VM stay as they were, and `docker compose ps` lists only
the original VM again. It keeps the Windows ISO in `DEMO_ISO_CACHE` (`~/.cache/itcmdr-demo-iso`) so
the next `provision-vms` skips the download. Running teardown twice is safe. The server VM keeps
itcmdr installed.

See also: [adminsvr](ADMINSVR.md), [deploying adminsvr with Let's Encrypt](DEPLOY-ADMINSVR.md),
[agent](AGENT.md), [backups](BACKUP.md), [upgrades](UPGRADES.md), [installer logs](UPGRADES.md).
