# adminsvr (v0.21.0)

The offsite admin server: one place where owners, admins and SREs see the status and alerts of
many sites and send a few safe commands. Design and decisions: [design/ADMINSVR.md](design/ADMINSVR.md).

## What leaves a site

The site server decides, deny by default (`server/src/sitelink.rs`, `Redactor`):

- opaque site-scoped IDs (`m-…` machines, `a-…` alerts), never names;
- alert codes built from a fixed vocabulary, check ids and plugin names; anything else is hashed;
- severity, state, counts, timestamps and numeric rollups (machines up/late/down, backup age, size, result);
- display names only after the owner opts in on the site: `itcmdr-cli adminsvr share-names on`.

Hostnames, IPs, paths, usernames and message text never leave; tests in `adminsvr/tests/sitelink.rs`
and `server/src/sitelink.rs` check this. Backups never leave the site and there is no restore from
adminsvr.

## Install adminsvr

Uses SQLite in its data folder by default (`<data dir>/adminsvr.db`); for Postgres 14+, create a
database and set `ITCMDR_ADMINSVR_DB_URL=postgres://…`. Ports: 8444 admin API, 8443 SiteLink.
Settings come from flags, the environment or an env file (`--env-file`, `ITCMDR_ADMINSVR_ENV_FILE`;
values already in the environment win). The service definitions all pass the env file below.
adminsvr stops cleanly on SIGTERM (every service manager) or Ctrl-C.

### Linux: systemd, SysV init or OpenRC

    tar xzf itcmdr-adminsvr-<ver>-linux-x86_64.tar.gz && cd itcmdr-adminsvr-<ver>-linux-x86_64
    sudo ./install.sh                 # or --init systemd | openrc | sysv (default: what the system runs)
    sudo editor /etc/itcmdr-adminsvr/adminsvr.env   # public host:ports, TLS names (and a Postgres URL if wanted)
    sudo -u itcmdr-adminsvr itcmdr-adminsvr --env-file /etc/itcmdr-adminsvr/adminsvr.env init --email you@example.com --name "You"

Then start it with what `install.sh` printed:

| Init | Files | Start / stop / status | Log |
|---|---|---|---|
| systemd | `/etc/systemd/system/itcmdr-adminsvr.service` | `systemctl enable --now itcmdr-adminsvr`, `systemctl stop/status itcmdr-adminsvr` | `journalctl -u itcmdr-adminsvr` |
| SysV | `/etc/init.d/itcmdr-adminsvr` (LSB headers; `update-rc.d` or `chkconfig`), pidfile `/run/itcmdr-adminsvr.pid` | `/etc/init.d/itcmdr-adminsvr start\|stop\|restart\|status` | `/var/log/itcmdr-adminsvr.log` |
| OpenRC | `/etc/init.d/itcmdr-adminsvr` (`openrc-run`) | `rc-update add itcmdr-adminsvr default; rc-service itcmdr-adminsvr start` | `/var/log/itcmdr-adminsvr.log` |

Every variant runs as the `itcmdr-adminsvr` system user with its data in `/var/lib/itcmdr-adminsvr`
(0700). The Linux binary needs the system OpenSSL 3 (`libssl3`, used by the built-in libpq).
Uninstall: `sudo ./uninstall.sh` (keeps data and settings) or `--purge`; a Postgres database is never dropped.

### macOS: launchd

    tar xzf itcmdr-adminsvr-<ver>-macos-aarch64.tar.gz && cd itcmdr-adminsvr-<ver>-macos-aarch64
    sudo ./install-macos.sh
    sudo editor /usr/local/etc/itcmdr-adminsvr/adminsvr.env
    sudo -u _itcmdradminsvr env ITCMDR_ADMINSVR_DATA_DIR=/usr/local/var/itcmdr-adminsvr \
      itcmdr-adminsvr --env-file /usr/local/etc/itcmdr-adminsvr/adminsvr.env init --email you@example.com --name "You"
    sudo launchctl bootstrap system /Library/LaunchDaemons/com.weirdbrains.itcmdr-adminsvr.plist

The LaunchDaemon runs as the hidden `_itcmdradminsvr` account, restarts after a failure
(`KeepAlive`/`SuccessfulExit=false`), logs to `/usr/local/var/log/itcmdr-adminsvr.log` and keeps
its data in `/usr/local/var/itcmdr-adminsvr`. Stop: `sudo launchctl bootout system/com.weirdbrains.itcmdr-adminsvr`.
Uninstall: `sudo ./uninstall-macos.sh [--purge]`. The binary is unsigned: if macOS blocks it, run
`xattr -d com.apple.quarantine itcmdr-adminsvr` before installing.

### Windows: a Windows service

`itcmdr-adminsvr-Setup-<ver>.exe` installs `%ProgramFiles%\itcmdr\adminsvr\itcmdr-adminsvr.exe` and
the service **itcmdr-adminsvr** (LocalSystem, automatic start). Recovery: restart after 5 s, 30 s,
then every 2 minutes (also when it stops with an error), reset after a day. Settings:
`%ProgramData%\itcmdr-adminsvr\adminsvr.env` (kept on upgrade and uninstall); data (root key, TLS,
SQLite) in `%ProgramData%\itcmdr-adminsvr\data` and the log in `…\logs`; only Administrators and
SYSTEM can read that folder. First owner, in an elevated prompt:

    "%ProgramFiles%\itcmdr\adminsvr\itcmdr-adminsvr.exe" init --email you@example.com --name "You"

Silent: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART [/NOSTART=yes]`; `sc stop|start itcmdr-adminsvr`.
Uninstalling stops and removes the service and keeps `%ProgramData%\itcmdr-adminsvr` (the
uninstaller's `/REMOVEDATA=yes` deletes it). Allow TCP 8443/8444 in the firewall yourself.

### Container (rootless Podman)

    podman build -f packaging/adminsvr/Containerfile -t itcmdr-adminsvr .
    podman run -d --name itcmdr-adminsvr -p 8443:8443 -p 8444:8444 -v itcmdr-adminsvr:/var/lib/itcmdr-adminsvr \
      -e ITCMDR_ADMINSVR_PUBLIC_API=<host>:8444 -e ITCMDR_ADMINSVR_PUBLIC_SITE=<host>:8443 itcmdr-adminsvr
    podman exec itcmdr-adminsvr itcmdr-adminsvr init --email you@example.com --name "You"

Or `podman compose up -d` with `packaging/adminsvr/compose.yaml` (SQLite; `--profile postgres` for
Postgres). Tested with rootless Podman 6.1: `scripts/container-smoke.sh` runs the image, enrolls
the CLI, creates a site, checks the audit chain and that data survives a restart, then exercises
the SysV script inside the image.

### Console commands

`init` prints the first owner's enrollment code. `itcmdr-adminsvr recover-owner --email …`
(console) makes a new owner code when every owner device and the words are lost; it is audited.
`itcmdr-adminsvr audit-verify` checks the audit chain.

## Admin CLI

    itcmdr-admin enroll <code>        # shows your 24 words once, asks three back; key backup on
    itcmdr-admin whoami
    itcmdr-admin user create <email> --name …      # role sre by default; prints their code
    itcmdr-admin user role <id> admin|sre|owner   # only owners grant or change owner
    itcmdr-admin user approve <user> <fingerprint> # new devices wait for an owner/admin
    itcmdr-admin site create <name>               # prints a one-time site token
    itcmdr-admin alert list | ack | resolve
    itcmdr-admin backup start <site>
    itcmdr-admin org policy --deep-check-days 90  # v1.0.0: warn when a site's last deep backup check is older (owners)
    itcmdr-admin backup drill <site>              # v0.22.0: the site checks its newest backup; you get ok/failed
    itcmdr-admin audit list | verify
    itcmdr-admin recover <addr> --pin … --email … # new device from the 24 words
    itcmdr-admin key-backup off                   # opt out of the encrypted key backup

Identity: the user config folder (`itcmdr-admin`, mode 0600), or `ITCMDR_ADMIN_HOME`.

## Link a site

On the site server, in person:

    itcmdr-cli adminsvr enroll <site token>
    itcmdr-cli adminsvr status
    itcmdr-cli adminsvr allow backup.start backup.drill alert.ack alert.resolve   # local command allowlist
    itcmdr-cli adminsvr unlink

Setting `adminsvr.enabled=false` closes the link at once (kill switch); `adminsvr.heartbeatSecs`
overrides the status interval. Every command is checked for both signatures (user and adminsvr
root key), expiry, the never-remote list and the local allowlist, and is written to the site audit.

## Commands, configs and plugins (v0.20.0)

A site sends its configuration document (setting values only for numbers, switches and times; paths and text
stay on the site; schedules by opaque id, names only with the display-name opt-in) and its plugin list. adminsvr
sends signed commands back; the site checks the user signature, its local allowlist, the never-remote list and,
for plugin kinds, the second person's signed approval when the organization requires one (v0.21.0), then audits.

    itcmdr-admin config show <site> [--all]
    itcmdr-admin config set <site> backup.keep_daily 9 --dry-run     # preview: before/after, nothing changes
    itcmdr-admin schedule pause <site> <schedule-id>
    itcmdr-admin plugin upload disk-watch-1.2.0.tar.gz               # owners/admins; signed package
    itcmdr-admin plugin push <site> <sha256>                          # waits for approval
    itcmdr-admin command approve <id>                                 # another owner/admin
    itcmdr-admin freeze on|off                                        # owners: stop everything

On the site: `itcmdr-cli adminsvr allow setting.set schedule.pause plugin.install ...` (nothing new is allowed by
default). `adminsvr.requireTwoPerson` (default false since v0.21.0) can only be changed on the site: when on,
the site always demands a second approver, whatever the organization's setting says.

## One approver or two: the organization setting (v0.21.0)

Every action that took a second owner or admin in v0.20 (plugin install/enable/disable from adminsvr) and the new
release approvals follow one organization setting, **off by default**: one owner or admin approves.

    itcmdr-admin org policy                                # show it (and who changed it last)
    itcmdr-admin org policy --second-approver on|off       # owners only; audited ("org.policy")

The setting is not only adminsvr's word: each command carries it inside the requester's signature
(`two_person`), adminsvr refuses a command whose flag does not match the setting, and the site checks the
approvals the flag asks for. A compromised adminsvr could send `two_person = false`; a site that wants two
approvers whatever happens sets its own `adminsvr.requireTwoPerson = true`. With the setting off, a device may
approve its own request where the flow still shows an approval step. The admin app's Plugins page has the switch
(owners) and its approval buttons work with one approver.

Upgrade note: an organization that has two or more active owners/admins when adminsvr v0.21.0 first starts keeps
the second approver (the migration turns the setting on for it); turn it off with the command above.

## Releases: approve once, adminsvr rolls out (v0.21.0)

Server and agent releases (`itcmdr-server-release` / `itcmdr-agent-release` packages, signed by an owner key as
before) go into the catalog like plugins. An owner or admin approves **one exact release** once: their device key
signs `itcmdr-admin-release/v1, name, version, sha256`; with the second approver on, a different owner or admin
signs too. adminsvr endorses each approval with its root key and attaches them to every rollout command.

    itcmdr-admin plugin upload itcmdr-agent-release-0.21.0.tar.gz
    itcmdr-admin release list                              # approvals so far, and how many are needed
    itcmdr-admin release approve <sha256>
    itcmdr-admin release rollout <site> <sha256>           # installs in the site's update window
    itcmdr-admin release rollout <site> <sha256> --now     # push: ignores the window

The site (after `itcmdr-cli adminsvr allow ... release.rollout`; it is not allowed by default) checks the
approvals (distinct owner/admin users and keys, endorsed by the root key), fetches the package, checks its
checksum and the owner signature on it, installs it and starts the server upgrade or the agent rollout through
its rings. Rings run within a site; across sites, you choose which sites get it and when.

## Automatic updates and the update window (v0.21.0)

Sites update themselves by default (`updates.auto = true`), but only inside their update window
(`updates.window`, default `daily 01:00-04:00`, in `updates.timezone`, default the server's zone). The window
holds the server upgrade, agent offers and ring advances; a range like `Mon-Fri 22:00-02:00` runs past midnight
and belongs to the day it starts; daylight-saving changes follow the zone's wall clock. A bad window fails closed
(nothing installs) until fixed. Change it on the site (`itcmdr-cli config set updates.window "Sat,Sun 02:00-05:00"`,
the site app's Updates screen) or from adminsvr (`itcmdr-admin config set <site> updates.window ...`).

- Off per site: `updates.auto = false` (an approved release is installed but waits for a push).
- Off per machine: `agent.upgrades = false` holds that machine back, even from a push.
- Push now: `release rollout ... --now` (or `itcmdr-cli upgrade start <v> --now`) ignores the window; the same
  approval rules apply.
- Order: the server upgrades first (database safety copy, staged copy, swap by `itcmdr-server-watchdog`, health
  check, rollback with the database restored on failure, Critical `server-upgrade` alert); agent rollouts that
  need a newer server (`min_server` in release.toml) wait for it.

## Passkeys / security keys (v0.19.1, optional)

The 24-word key signs everything; a passkey is a second factor on top. The CLI uses a USB/NFC
security key through libfido2's tools (`fido2-token`, `fido2-cred`, `fido2-assert`).

    itcmdr-admin passkey add --label yubikey [--unlock]   # --unlock: the device key opens only with a touch
    itcmdr-admin passkey list [user]
    itcmdr-admin passkey remove <credential id>
    itcmdr-admin passkey reset <user> <credential id>     # owner/admin: a user's lost key
    itcmdr-admin passkey lock | unlock
    itcmdr-admin policy show
    itcmdr-admin policy set --stepup SetRole,RevokeDevice,... [--required-roles owner] [--uv]

Once you have a passkey, actions on the step-up list ask for a touch, bound to that exact request.
`ITCMDR_ADMINSVR_RP_ID` sets the WebAuthn RP id (default: the host of the public API address);
`ITCMDR_ADMINSVR_WEB_ORIGINS` adds the web app's origins. A real security key has not been tested
with this release yet (the tests use a software authenticator).

## itcmdr Admin app (v0.19.2)

The app for owners, admins and SREs (Linux, macOS, Windows, Android; iOS builds but needs signing).
It uses the same signed client as the CLI (`admin-app/rust` over `itcmdr-admin-cli`): enroll the
device with a one-time code (the 24 words are shown once), then every request is signed with the
device key and adminsvr's TLS key is pinned. Tabs: Alerts (open/resolved/all, acknowledge and
resolve), Servers (status, configuration, versions per machine with "update available", metrics
history for 6 h to 30 days, commands, start a backup; admins add and revoke servers), Users
(owners/admins: create with a code, roles, disable, SRE scopes, approve and revoke devices, reset
passkeys) and Settings (passkeys, the step-up policy for owners, notifications, audit verify,
forget this device). Actions on the step-up list show "Touch your security key" while the call
waits for the touch; on desktops the app drives the key through libfido2's tools like the CLI.
Changing a server's settings and updating it remotely come with v0.20/v0.21. There is no web
build: the client uses gRPC over TLS sockets, which browsers do not allow (it needs gRPC-web).

## Push notifications (v0.19.2)

adminsvr pushes a newly opened alert (default: cautious and critical,
`ITCMDR_ADMINSVR_PUSH_MIN_SEVERITY`) to the registered devices of every active user who may see
the site (SREs only for their scoped sites; revoked devices and disabled users get nothing). The
payload holds only the opaque site id, the opaque alert id and the severity, with a fixed title
("itcmdr: critical alert"); the app loads the details over the signed API. Every send is audited
(`push.send`), tokens that the provider reports gone are deleted. `itcmdr-admin push register |
unregister | test` (the app does the same), `itcmdr-admin site metrics <site>` for the history.

Push is off until configured. What it needs (accounts the operator creates):
- **Android (FCM)**: a Firebase project with Cloud Messaging; a service account key (JSON) with
  the Firebase Cloud Messaging API role → `ITCMDR_ADMINSVR_FCM_SERVICE_ACCOUNT=/path/sa.json`; and
  the app's `google-services.json` for an app build with the Firebase messaging plugin.
- **iOS/macOS (APNs)**: an Apple Developer team, an APNs auth key (.p8) with its key id and the
  team id, and the app's bundle id with the Push Notifications capability →
  `ITCMDR_ADMINSVR_APNS_KEY`, `_KEY_ID`, `_TEAM_ID`, `_TOPIC` (and
  `ITCMDR_ADMINSVR_APNS_URL=https://api.sandbox.push.apple.com` for development builds).
- The released app does not include a push SDK yet (`admin-app/lib/push.dart`: `NoPush`); it shows
  new alerts in a banner while open (checked every 30 s). The server side is tested against a
  local FCM/APNs stub (`adminsvr/tests/push.rs`).

## Deep backup check reminder (v1.0.0)

Each site status carries `deep_check_ms` and `deep_check_result` (ok | failed | none): the date and
result of the site's last `itcmdr-cli backup drill --deep`, which only runs on the site with the 24
words. Nothing else about backups is sent. `itcmdr-admin site list` and the admin app's Servers page
show "last deep check: N days ago" and a warning when it never ran, failed, or is older than the
organization's limit (`OrgPolicy.deep_check_max_days`, default 90, `itcmdr-admin org policy
--deep-check-days N`, owners only, audited as `org.policy`).
