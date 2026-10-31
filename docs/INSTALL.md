# Install guide (admin PowerShell)

This repository contains placeholders only. Real values go in `C:\ProgramData\itcmdr\site.json` (start from `config/site.example.json`) and `C:\ProgramData\itcmdr\server.json`. Never commit real values.

## Server (Windows)

1. Run the installer from the Releases page as an admin, at the server. Pick **Full** (server, app, agent).
   The installer adds:
   - the `itcmdr-server` service (`itcmdr-server.exe --service`, delayed auto-start, restart on failure)
   - `C:\ProgramData\itcmdr\server.json`, which is never overwritten
   - an inbound firewall rule for TCP 50051, limited to `/AllowSubnet=<lan cidr>`
   - the `itcmdr Log Rotation` scheduled task (daily, SYSTEM, `rotation.timeLocal`, default 02:10)
   - with the Agent component, the agent and poller tasks
2. On a first install the wizard asks for the **site settings**: site name, server name (for example SERVER01), the
   maintenanceRuns share (for example `\\SERVER01\PDATA\maintenanceRuns`, or its local folder on the server), and
   the backup folder (a share or a second disk). It also asks for the **first admin** (a person's name). These go into
   `server.json` (`itcmdr-cli setup apply`) and `site.json` (`Initialize-ItcmdrSite.ps1`). An upgrade keeps every value
   that is already set and only fills empty ones.
3. Have paper ready, leave **Run first-run setup now** ticked, and click Finish. A console window opens
   (`itcmdr-cli setup first-run`):
   - It shows the 24 recovery words once. They are never written to a file or a log. Write them on paper and keep the
     paper off the server (for example in the office safe). They are the only way to certify keys and to open a backup.
   - Press Enter, then type the three words it asks for. On a mismatch it shows the words again.
   - It registers the first admin's app key (role owner). With the App component, the key is created in your Windows
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
`itcmdr-Setup.exe /VERYSILENT /COMPONENTS="server,app,agent" /LogsDir="D:\PDATA\maintenanceRuns" /BackupDir="\\NAS01\backup\itcmdr" /SiteName="Example Office" /ServerHostname=SERVER01 /Share="\\SERVER01\PDATA\maintenanceRuns" /AllowSubnet=198.51.100.0/24`, then step 4.

The shared admin token (`admin_token_hash`, `itcmdr-server.exe hash-token`) is only for `auth set-mode both` (older installs).

Console mode, for troubleshooting: stop the service, then run `itcmdr-server.exe --console`. It reads the same config, from `ITCMDR_SERVER_CONFIG`, `.\config\server.json`, `%ProgramData%\itcmdr\server.json`, or `server.json` next to the exe.

## Agents

Silent agent-only install on a PC:

```
itcmdr-Setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /COMPONENTS="agent"
```

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

- Windows: installed by the installer (Start Menu shortcut).
- Web: `flutter build web`, then host it on an internal web server. The server speaks gRPC-Web on the same port.
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
