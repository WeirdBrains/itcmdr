# Install guide (admin PowerShell)

This repository contains placeholders only. Real values go in `C:\ProgramData\itcmdr\site.json` (start from `config/site.example.json`) and `C:\ProgramData\itcmdr\server.json`. Never commit real values.

## Packages (v0.14.3)

Each part has its own installer, with its own entry in Apps & features and its own uninstaller.
They all install under `C:\Program Files\itcmdr\<part>` and share the data folder `C:\ProgramData\itcmdr`.

| Package | File | What it installs |
|---|---|---|
| Server | `itcmdr-server-Setup-<ver>.exe` | the `itcmdr-server` service, its own copy of `itcmdr-cli`, `itcmdr-plugin-ps`, `itcmdr-restore`, log rotation, firewall rule |
| Agent | `itcmdr-agent-Setup-<ver>.exe` | the PowerShell agent, the pinned check plugins, the nightly and poller tasks, `Push-Install-ItcmdrAgent.ps1` |
| CLI | `itcmdr-cli-Setup-<ver>.exe`, `itcmdr-cli-<ver>-linux-x86_64.tar.gz`, `itcmdr-cli-<ver>-macos-<arch>.tar.gz` | `itcmdr-cli` on PATH (Windows: `C:\Program Files\itcmdr\cli`) |
| App | `itcmdr-app-Setup-<ver>.exe` (plus Linux and macOS builds from CI) | the desktop app with the `itcmdr_keys_bridge.dll` crypto bridge |

`SHA256SUMS` on the release page lists every file.

- **The server and the CLI:** the server package always carries its own `itcmdr-cli` in `server\`. The installer
  needs it for setup, so the server works without the CLI package. Install the CLI package where you want
  `itcmdr-cli` on PATH (admin PCs, or the server too). Both copies have the same version.
- **Server and app on one PC:** install both packages. On a first install the server asks *This PC also runs the
  itcmdr app*; tick it so first-run puts the admin key into the app's DPAPI store.
- **Upgrading from the combined installer (v0.14.2 and older):** run the new package for each part you had (usually
  server, agent and app) over the existing install. Files, `server.json`, data, keys, the service and signed-only mode
  are kept. When the last of them is installed, the old combined "itcmdr" uninstall entry is removed; its files now
  belong to the new packages. Until then, don't run the old uninstaller.
- **Uninstall** (Apps & features, or `C:\Program Files\itcmdr\uninstall\<part>\unins000.exe`):
  - **Data is kept by default.** The uninstaller asks whether to keep this part's data (Yes is the default).
  - If you choose to remove it, the server uninstaller asks separately about `keys\` (No is the default). Without
    that folder a restore needs the 24 words.
  - Silent: `/VERYSILENT` keeps everything. `/REMOVEDATA=yes` removes the part's data, and for the server
    `/REMOVEKEYS=yes` also removes `keys\`.
  - **Backups (`backups\` and the backup folder) are never removed.** The words are never stored, so there's
    nothing of them to remove.
  - The app's key in `%APPDATA%\itcmdr` and the agent's local log folder (`C:\maintenanceRuns`) are always kept.

## Server (Windows)

1. Run `itcmdr-server-Setup-<ver>.exe` from the Releases page as an admin, at the server. Then run the app and agent packages if this PC needs them.
   The server package adds:
   - the `itcmdr-server` service (`itcmdr-server.exe --service`, delayed auto-start, restart on failure)
   - `C:\ProgramData\itcmdr\server.json`, which is never overwritten
   - an inbound firewall rule for TCP 50051, limited to `/AllowSubnet=<lan cidr>`
   - the `itcmdr Log Rotation` scheduled task (daily, SYSTEM, `rotation.timeLocal`, default 02:10)
2. On a first install the wizard asks for the **site settings**: site name, server name (for example SERVER01), the
   maintenanceRuns share (for example `\\SERVER01\DATA\maintenanceRuns`, or its local folder on the server), and
   the backup folder (a share or a second disk). It also asks for the **first admin** (a person's name). These go into
   `server.json` (`itcmdr-cli setup apply`). The agent package asks for its own values, which go into `site.json` (`Initialize-ItcmdrSite.ps1`). An upgrade keeps every value
   that is already set and only fills empty ones.

   **No file share is needed (v1.0.0).** Leave the maintenanceRuns and backup fields empty and the server keeps
   logs in `C:\ProgramData\itcmdr\maintenanceRuns` and backups in `C:\ProgramData\itcmdr\backups` (set a second
   disk later with `itcmdr-cli setup apply --backup-dir`). Install the agent with only `/EnrollToken=<token>`
   (`itcmdr-cli agent enroll-token --machine <PC> --address <server>:50052`): it reports, takes config and Run now over
   its mutual-TLS link. Tested on the Windows VM in v1.0.0 (fresh install, no share: enrolled, online, backup and
   restore drill OK, no share path anywhere in the config).
3. Have paper ready, leave **Run first-run setup now** ticked, and click Finish. A console window opens
   (`itcmdr-cli setup first-run`):
   - It shows the 24 recovery words once. They are never written to a file or a log. Write them on paper and keep the
     paper off the server (for example in the office safe). They are the only way to certify keys and to open a backup.
   - Press Enter, then type the three words it asks for. On a mismatch it shows the words again.
   - It registers the first admin's app key (role owner). With *This PC also runs the itcmdr app* ticked, the key is created in your Windows
     DPAPI key store, so the itcmdr app on this PC signs from the first start. Signed mode works from day one, and the
     shared token is never needed.
   - The words are only shown in a real console. Redirected output is refused.
4. Skipped it, or a silent install? Run it later, in person, in an admin console:
   `& 'C:\Program Files\itcmdr\server\itcmdr-cli.exe' setup first-run --admin-name "Dr Example" --app-key-store`
   (or `--admin-public-key <hex>` from the Keys screen of an app on another PC).
5. Check that it's working:
   - `itcmdr-cli setup check` (also saved by the installer to `C:\ProgramData\itcmdr\setup-check.txt`). It checks
     that the config, data dir, log share and backup folder exist and are writable, that the server is initialized,
     and that an admin app key exists. Exit code 1 when a check fails.
   - `Get-Service itcmdr-server` shows Running.
   - Errors, if any, are in `C:\ProgramData\itcmdr\server-logs\itcmdr-server.txt`.

Silent install (values as parameters; never shows the words):
`itcmdr-server-Setup-<ver>.exe /VERYSILENT /LogsDir="D:\DATA\maintenanceRuns" /BackupDir="\\NAS01\backup\itcmdr" /AllowSubnet=198.51.100.0/24`, then step 4.

The shared admin token (`admin_token_hash`, `itcmdr-server.exe hash-token`) is only for `auth set-mode both` (older installs).

Console mode, for troubleshooting: stop the service, then run `itcmdr-server.exe --console`. It reads the same config, from `ITCMDR_SERVER_CONFIG`, `.\config\server.json`, `%ProgramData%\itcmdr\server.json`, or `server.json` next to the exe.

## Agents

Silent agent install on a PC (push-friendly; the enroll token is optional and kept for the Rust agent in v0.17.0):

```
itcmdr-agent-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SiteName="Example Office" /ServerHostname=SERVER01 /Share="\\SERVER01\DATA\maintenanceRuns" [/EnrollToken=<token>]
```

With `/EnrollToken=` (v1.3.4) the installer enrolls as this computer's name, even when the PC was already enrolled (it stops the agent services, keeps a copy of `agent.json` so a failed enroll rolls back, restarts the services and checks the link). A bad token or a failed enroll exits 9, writes `install-logs/last-agent.json` and raises the installer alert; it never exits 0 after leaving the old enrollment in place. The token may have leading or trailing whitespace or a stderr line mixed in. v1.3.6: the link check reads `agent.log` with a shared read so a running agent cannot make a good enroll look like a failure.

This registers two tasks, both running as SYSTEM:
- `itcmdr Agent Nightly` at 01:30
- `itcmdr Agent Poller` every minute; a run that's still going makes the next one skip

Each PC also needs `C:\ProgramData\itcmdr\site.json`.

Manual install, or push to several PCs with `agent\Push-Install-ItcmdrAgent.ps1` (uses ADMIN$ and schtasks, no WinRM):

```
Set-ExecutionPolicy Bypass -Scope Process -Force
.\Install-ItcmdrAgent.ps1
```

To do a one-off test run (report-only when not SYSTEM), run `.\Invoke-ItcmdrAgent.ps1 -ReportOnly`. Then look at the newest file in `C:\maintenanceRuns`.

## App

- Windows: `itcmdr-app-Setup-<ver>.exe` (Start menu shortcut; `/TASKS=desktopicon` for a desktop icon).
- Web: not supported for the on-site app (its key store is Windows DPAPI). The web build comes with the adminsvr app.
- Open **Settings** and enter the server host and port (default 50051) and your name for the audit log.
- Open **Keys** and make or import a key. An owner certifies it (see the server steps above). The key seed stays in this Windows profile.
- The shared admin token field is only for an install still in `both` or `token` mode.

## Switch to signed-only (on site, after the app works signed)

Run on SERVER01 in an admin PowerShell. Placeholders: SERVER01 (server), WS-01 (the PC with the app),
`Dr Example` (the key name). Nothing here needs the 24 words except step 3.

1. Check the starting point:
   `& 'C:\Program Files\itcmdr\server\itcmdr-cli.exe' auth status`
   Expect `auth_mode: both`, and note how many admin/owner keys can sign.
2. On WS-01, open the app and go to Keys. Make a key, or keep the existing one. Check that it says
   "Private key stored in: Windows DPAPI (current user)". Copy the certify command it shows.
3. Certify the key offline with the words and register it on SERVER01:
   `itcmdr-cli keys certify --kind user --name "Dr Example" --role owner --public-key <hex> --out C:\Temp\app.bundle.json`
   (type the words when asked), then `itcmdr-cli keys register C:\Temp\app.bundle.json`, then delete the bundle file.
   Alternatively, register the bundle from the Keys screen of an app that already has an owner key.
4. On WS-01, check that the app works signed: Keys shows the name and role (WhoAmI), and Schedules and Plugins load.
5. Make sure at least one admin/owner key can sign besides the root key (`auth status`). Without one,
   `set-mode signed` refuses (`--force` overrides it; don't use it here).
6. Switch: `itcmdr-cli auth set-mode signed`, then `sc.exe stop itcmdr-server`, then `sc.exe start itcmdr-server`.
7. Verify:
   - `itcmdr-cli auth status` shows `auth_mode: signed (set in server.json)`.
   - The app on WS-01 still loads Keys, Schedules and Plugins.
   - A shared-token call is refused with `Unauthenticated` ("missing x-itcmdr-key").
   - The next agent poller run finishes with result 0 (`Get-ScheduledTaskInfo 'itcmdr Agent Poller'`).
8. Rollback, if anything is wrong: `itcmdr-cli auth set-mode both`, then restart the service as in step 6.
   Token calls work again; signed calls keep working.
9. Record the date and the result in the handover.

## Local demo (Linux or Windows dev box)

```
cargo run -p itcmdr-sample-data-gen -- --out-dir ./sample-maintenanceRuns --days 90
cp config/server.example.json config/server.json   # set allow_insecure_no_auth or a token hash
cargo run -p itcmdr-server
cd app && flutter run -d linux   # Settings: localhost : 50051
```

Every Windows installer run writes a log under `%ProgramData%\itcmdr\install-logs` (adminsvr: `%ProgramData%\itcmdr-adminsvr\install-logs`); a failed upgrade exits 7 or 8 and starts the services again. See docs/UPGRADES.md, Installer logs.
