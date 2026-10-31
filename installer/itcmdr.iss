; Inno Setup script for itcmdr
; Components:
;  - Server (register Windows service itcmdr-server; add firewall rule scoped to subnet)
;  - App (Flutter Windows desktop; Start Menu shortcut)
;  - Agent (PowerShell scripts; scheduled task; local log folder, default C:\maintenanceRuns)
;
; Local log folder (decision 15): build with  ISCC /DLocalLogs="D:\itcmdr\logs" itcmdr.iss,
; or override at install time with  itcmdr-Setup.exe /LocalLogs="D:\itcmdr\logs".
; Set the same folder in site.json "paths": { "localLogs": ... } so the agent scripts use it.
;
; First-run setup (v0.14.0). On a fresh install the wizard asks for the site values and writes
; them with `itcmdr-cli setup apply` (server.json) and Initialize-ItcmdrSite.ps1 (site.json).
; Upgrades never change existing values (server.json is never replaced; site.json values are only
; filled when empty). After a fresh server install, "Run first-run setup" opens a console that
; shows the 24 words once and registers the first admin app key (itcmdr-cli setup first-run).
; Silent installs take the values as parameters and never show the words:
;   /LogsDir=  /BackupDir=  /SiteName=  /ServerHostname=  /Share=  /AllowSubnet=
; then run `itcmdr-cli setup first-run` in person at the server. Check reports:
;   %ProgramData%\itcmdr\setup-check.txt (server) and agent-setup-check.txt (agent).

#define MyAppName "itcmdr"
#ifndef MyAppVersion
  #define MyAppVersion "0.14.1"
#endif
#define InstallDirName "itcmdr"
#ifndef LocalLogs
  #define LocalLogs "C:\maintenanceRuns"
#endif

[Setup]
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={pf}\{#InstallDirName}
DisableProgramGroupPage=yes
OutputDir=out
OutputBaseFilename=itcmdr-Setup
ArchitecturesInstallIn64BitMode=x64

[Types]
Name: "full"; Description: "Full installation (Server, App, Agent)"
Name: "agentonly"; Description: "Agent only (PowerShell and scheduled tasks)"

[Components]
Name: "server"; Description: "Server (Windows service itcmdr-server)"; Types: full; Flags: checkablealone
Name: "app"; Description: "Desktop App (Windows)"; Types: full; Flags: checkablealone
Name: "agent"; Description: "Agent (PowerShell + scheduled tasks)"; Types: full agentonly; Flags: checkablealone

[Files]
; Server binary (built by CI)
Source: "..\target\release\itcmdr-server.exe"; DestDir: "{app}\server"; Components: server; Flags: ignoreversion
Source: "..\target\release\itcmdr-cli.exe"; DestDir: "{app}\server"; Components: server; Flags: ignoreversion
Source: "..\target\release\itcmdr-plugin-ps.exe"; DestDir: "{app}\server"; Components: server; Flags: ignoreversion
Source: "..\target\release\itcmdr-restore.exe"; DestDir: "{app}\server"; Components: server; Flags: ignoreversion
; App build
Source: "..\app\build\windows\x64\runner\Release\*"; DestDir: "{app}\app"; Components: app; Flags: recursesubdirs ignoreversion
; Agent scripts
Source: "..\agent\Invoke-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
Source: "..\agent\Install-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
Source: "..\agent\Poll-ItcmdrRequests.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
Source: "..\agent\Itcmdr.Checks.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
Source: "..\agent\Itcmdr.PluginRunner.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
Source: "..\agent\Initialize-ItcmdrSite.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
Source: "..\agent\Set-ItcmdrCutover.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
; Check plugins: the only plugin copies agents run; never downloaded by agents. They come from the
; itcmdr-plugins tag pinned in plugins.pin.json (scripts/fetch-plugins.sh puts it in third_party).
#if !FileExists(AddBackslash(SourcePath) + "..\third_party\itcmdr-plugins\plugins\itcmdr-network\main.ps1")
  #error "Pinned plugins missing: run scripts/fetch-plugins.sh (see plugins.pin.json)"
#endif
Source: "..\third_party\itcmdr-plugins\plugins\*"; Excludes: "*.toml"; DestDir: "{app}\agent\plugins"; Components: agent; Flags: ignoreversion recursesubdirs
Source: "..\agent\Report-BackupCoverage.ps1"; DestDir: "{app}\agent"; Components: server; Flags: ignoreversion
Source: "..\agent\Push-Install-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
Source: "..\agent\Rotate-ItcmdrLogs.ps1"; DestDir: "{app}\agent"; Components: server; Flags: ignoreversion
Source: "..\agent\Install-ItcmdrRotationTask.ps1"; DestDir: "{app}\agent"; Components: server; Flags: ignoreversion
; PowerShell plugin module (itcmdr-plugin-ps finds it in ..\agent)
Source: "..\agent\Itcmdr.Plugin.psm1"; DestDir: "{app}\agent"; Components: server agent; Flags: ignoreversion
; Server config template (never overwritten; edit logs_dir and admin_token_hash after install)
Source: "server.windows.json"; DestDir: "{commonappdata}\itcmdr"; DestName: "server.json"; Components: server; Flags: onlyifdoesntexist uninsneveruninstall

[Dirs]
Name: "{param:LocalLogs|{#LocalLogs}}"; Permissions: users-full; Components: agent

[Icons]
Name: "{group}\itcmdr"; Filename: "{app}\app\itcmdr.exe"; Components: app
Name: "{commondesktop}\itcmdr"; Filename: "{app}\app\itcmdr.exe"; Components: app; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; Components: app

[Run]
; Register Windows service (idempotent). --service runs it under the SCM; config is
; read from %ProgramData%\itcmdr\server.json; service log: %ProgramData%\itcmdr\server-logs\.
Filename: "sc.exe"; Parameters: "create itcmdr-server binPath= ""\""{app}\server\itcmdr-server.exe\"" --service"" start= delayed-auto DisplayName= ""itcmdr-server"""; Flags: runhidden; Components: server; StatusMsg: "Creating Windows service itcmdr-server..."; Check: not ServiceExists('itcmdr-server')
Filename: "sc.exe"; Parameters: "config itcmdr-server binPath= ""\""{app}\server\itcmdr-server.exe\"" --service"""; Flags: runhidden; Components: server; Check: ServiceExists('itcmdr-server')
Filename: "sc.exe"; Parameters: "description itcmdr-server ""itcmdr log ingest, metrics and on-demand run server (gRPC)"""; Flags: runhidden; Components: server
Filename: "sc.exe"; Parameters: "failure itcmdr-server reset= 86400 actions= restart/60000/restart/60000/restart/300000"; Flags: runhidden; Components: server
; Site values into server.json on a fresh install only (upgrades keep the existing file as it is).
Filename: "{app}\server\itcmdr-cli.exe"; Parameters: "setup apply --config ""{commonappdata}\itcmdr\server.json"" {code:ServerApplyArgs}"; Flags: runhidden; Components: server; StatusMsg: "Writing site settings..."; Check: FreshServer
Filename: "sc.exe"; Parameters: "start itcmdr-server"; Flags: runhidden; Components: server; StatusMsg: "Starting itcmdr-server..."
; Install checks (folders exist and are writable, initialized, app keys); report kept for support.
Filename: "{app}\server\itcmdr-cli.exe"; Parameters: "setup check --report ""{commonappdata}\itcmdr\setup-check.txt"""; Flags: runhidden; Components: server; StatusMsg: "Checking the server setup..."

; Firewall rule restricted to example subnet; override at install time with /DAllowSubnet=198.51.100.0/24
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""$s = '{code:GetAllowSubnet}'; if ($s) {{ netsh advfirewall firewall delete rule name='itcmdr-server' | Out-Null; netsh advfirewall firewall add rule name='itcmdr-server' dir=in action=allow enable=yes profile=any localport=50051 protocol=TCP remoteip=$s }"""; Flags: runhidden; Components: server; StatusMsg: "Adding firewall rule..."

; Agent site values (fresh: given values; upgrade: only fill empty ones) + checks
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Initialize-ItcmdrSite.ps1"" {code:AgentSiteArgs} -Report ""{commonappdata}\itcmdr\agent-setup-check.txt"""; Flags: runhidden; Components: agent; StatusMsg: "Writing agent site settings..."
; Install agent scheduled task
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Install-ItcmdrAgent.ps1"""; Flags: runhidden; Components: agent; StatusMsg: "Installing agent scheduled task..."
; Install log rotation scheduled task (daily, server only)
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Install-ItcmdrRotationTask.ps1"""; Flags: runhidden; Components: server; StatusMsg: "Installing log rotation task..."

; First-run (fresh server install, interactive only): a console window shows the 24 words once.
Filename: "{app}\server\itcmdr-cli.exe"; Parameters: "setup first-run {code:FirstRunArgs}"; Description: "Run first-run setup now (shows the 24 recovery words once; have paper ready)"; Flags: postinstall skipifsilent runascurrentuser; Components: server; Check: FreshServer

[UninstallRun]
Filename: "sc.exe"; Parameters: "stop itcmdr-server"; Flags: runhidden; Components: server
Filename: "sc.exe"; Parameters: "delete itcmdr-server"; Flags: runhidden; Components: server
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Unregister-ScheduledTask -TaskName 'itcmdr Agent Nightly' -Confirm:$false -ErrorAction SilentlyContinue; Unregister-ScheduledTask -TaskName 'itcmdr Agent Poller' -Confirm:$false -ErrorAction SilentlyContinue"""; Flags: runhidden; Components: agent
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Unregister-ScheduledTask -TaskName 'itcmdr Log Rotation' -Confirm:$false -ErrorAction SilentlyContinue; netsh advfirewall firewall delete rule name='itcmdr-server'"""; Flags: runhidden; Components: server

[Code]
function ServiceExists(Name: string): Boolean;
var
  ResultCode: Integer;
begin
  Result := ShellExec('', ExpandConstant('sc.exe'),
    'query ' + Name, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) and (ResultCode = 0);
end;

// Parameter override via /DAllowSubnet=198.51.100.0/24
function GetAllowSubnet(Param: string): string;
begin
  Result := ExpandConstant('{param:AllowSubnet|198.51.100.0/24}');
end;

var
  SitePage, AdminPage: TInputQueryWizardPage;
  WasFreshServer, WasFreshAgent: Boolean;

function Q(S: string): string;
begin
  StringChangeEx(S, '"', '', True);
  Result := '"' + S + '"';
end;

function InitializeSetup(): Boolean;
begin
  // Decided before [Files] runs (server.json is installed only if it does not exist).
  WasFreshServer := not FileExists(ExpandConstant('{commonappdata}\itcmdr\server.json'));
  WasFreshAgent := not FileExists(ExpandConstant('{commonappdata}\itcmdr\site.json'));
  Result := True;
end;

procedure InitializeWizard();
begin
  SitePage := CreateInputQueryPage(wpSelectComponents, 'Site settings',
    'Values for this office (first install only; upgrades keep what is set).',
    'Leave a field empty to keep the default. All of them can be changed later (itcmdr-cli setup apply, site.json).');
  SitePage.Add('Site name:', False);
  SitePage.Add('Server name (agents connect to it):', False);
  SitePage.Add('maintenanceRuns share (agents write logs here, the server reads them):', False);
  SitePage.Add('Backup folder on the server (a share or second disk):', False);
  SitePage.Values[0] := ExpandConstant('{param:SiteName|}');
  SitePage.Values[1] := ExpandConstant('{param:ServerHostname|}');
  SitePage.Values[2] := ExpandConstant('{param:Share|}');
  SitePage.Values[3] := ExpandConstant('{param:BackupDir|}');
  AdminPage := CreateInputQueryPage(SitePage.ID, 'First admin',
    'The person who manages itcmdr from the app.',
    'First-run setup at the end creates the master key (24 words, shown once) and registers this person''s app key, so the app can sign from day one.');
  AdminPage.Add('Admin name:', False);
  AdminPage.Values[0] := ExpandConstant('{param:AdminName|}');
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := False;
  if PageID = SitePage.ID then
    Result := not ((WasFreshServer and WizardIsComponentSelected('server')) or (WasFreshAgent and WizardIsComponentSelected('agent')));
  if PageID = AdminPage.ID then
    Result := not (WasFreshServer and WizardIsComponentSelected('server'));
end;

function FreshServer(): Boolean;
begin
  Result := WasFreshServer;
end;

// The server reads the logs from the share's local folder or UNC path.
function ServerApplyArgs(Param: string): string;
var LogsDir: string;
begin
  Result := '';
  LogsDir := ExpandConstant('{param:LogsDir|}');
  if LogsDir = '' then LogsDir := SitePage.Values[2];
  if LogsDir <> '' then Result := Result + ' --logs-dir ' + Q(LogsDir);
  if SitePage.Values[3] <> '' then Result := Result + ' --backup-dir ' + Q(SitePage.Values[3]);
end;

function AgentSiteArgs(Param: string): string;
begin
  Result := '-LocalLogs ' + Q(ExpandConstant('{param:LocalLogs|{#LocalLogs}}'));
  if SitePage.Values[0] <> '' then Result := Result + ' -SiteName ' + Q(SitePage.Values[0]);
  if SitePage.Values[1] <> '' then Result := Result + ' -ServerHostname ' + Q(SitePage.Values[1]);
  if SitePage.Values[2] <> '' then Result := Result + ' -MaintenanceRunsShare ' + Q(SitePage.Values[2]);
end;

function FirstRunArgs(Param: string): string;
begin
  Result := '';
  if AdminPage.Values[0] <> '' then Result := '--admin-name ' + Q(AdminPage.Values[0]);
  if WizardIsComponentSelected('app') then Result := Result + ' --app-key-store';
end;

// Silent install (Agent only):
//   itcmdr-Setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /COMPONENTS=\"agent\"

