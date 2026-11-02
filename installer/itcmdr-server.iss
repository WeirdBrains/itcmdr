; itcmdr server package (v0.14.3): the Windows service itcmdr-server, plus its own private copy
; of itcmdr-cli (the installer runs `setup apply|check|first-run` with it, and an operator can use
; it without the CLI package), itcmdr-plugin-ps, itcmdr-restore and the server-side scripts.
; The CLI package (itcmdr-cli.iss) puts the same itcmdr-cli on PATH; both are the same version.
; v0.21.0: {app}\server-watchdog\itcmdr-server-watchdog.exe (service itcmdr-server-watchdog, only replaced
; by this installer): carries out server upgrades ({app}\server-previous, -staged, -failed) with health
; check and rollback, and restarts a stopped or hung itcmdr-server.
;
; Silent:  itcmdr-server-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
;          [/LogsDir= /BackupDir= /AllowSubnet= /AdminName=]  (first-run is in person: itcmdr-cli setup first-run)
; Uninstall keeps %ProgramData%\itcmdr; /REMOVEDATA=yes [/REMOVEKEYS=yes] to remove (never backups).
#include "common.iss"

[Setup]
AppId={{{#ServerAppId}}
AppName=itcmdr server
UninstallDisplayName=itcmdr server {#MyAppVersion}
OutputBaseFilename=itcmdr-server-Setup-{#MyAppVersion}
UninstallFilesDir={app}\uninstall\server
UninstallDisplayIcon={app}\uninstall\server\itcmdr.ico
#include "common-setup.iss"

[Files]
Source: "..\branding\office\windows\app_icon.ico"; DestDir: "{app}\uninstall\server"; DestName: "itcmdr.ico"; Flags: ignoreversion
Source: "..\target\release\itcmdr-server.exe"; DestDir: "{app}\server"; Flags: ignoreversion
Source: "..\target\release\itcmdr-cli.exe"; DestDir: "{app}\server"; Flags: ignoreversion
Source: "..\target\release\itcmdr-plugin-ps.exe"; DestDir: "{app}\server"; Flags: ignoreversion
Source: "..\target\release\itcmdr-restore.exe"; DestDir: "{app}\server"; Flags: ignoreversion
Source: "..\target\release\itcmdr-server-watchdog.exe"; DestDir: "{app}\server-watchdog"; Flags: ignoreversion
Source: "..\agent\Report-BackupCoverage.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Rotate-ItcmdrLogs.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Install-ItcmdrRotationTask.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
; PowerShell plugin module (itcmdr-plugin-ps finds it in ..\agent); shared with the agent package.
Source: "..\agent\Itcmdr.Plugin.psm1"; DestDir: "{app}\agent"; Flags: ignoreversion sharedfile
; Config template: never overwritten, never uninstalled.
Source: "server.windows.json"; DestDir: "{commonappdata}\itcmdr"; DestName: "server.json"; Flags: onlyifdoesntexist uninsneveruninstall

[UninstallDelete]
; Written by server upgrades (not by this installer).
Type: filesandordirs; Name: "{app}\server-previous"
Type: filesandordirs; Name: "{app}\server-staged"
Type: filesandordirs; Name: "{app}\server-failed"

[Tasks]
Name: "appkey"; Description: "This PC also runs the itcmdr app: store the first admin's key for the app (DPAPI)"; Check: FreshServer

[Run]
Filename: "{sys}\sc.exe"; Parameters: "create itcmdr-server binPath= ""\""{app}\server\itcmdr-server.exe\"" --service"" start= delayed-auto DisplayName= ""itcmdr-server"""; Flags: runhidden; StatusMsg: "Creating Windows service itcmdr-server..."; Check: not ServiceExists('itcmdr-server')
Filename: "{sys}\sc.exe"; Parameters: "config itcmdr-server binPath= ""\""{app}\server\itcmdr-server.exe\"" --service"""; Flags: runhidden; Check: ServiceExists('itcmdr-server')
Filename: "{sys}\sc.exe"; Parameters: "description itcmdr-server ""itcmdr log ingest, metrics and on-demand run server (gRPC)"""; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "failure itcmdr-server reset= 86400 actions= restart/60000/restart/60000/restart/300000"; Flags: runhidden
Filename: "{app}\server\itcmdr-cli.exe"; Parameters: "setup apply --config ""{commonappdata}\itcmdr\server.json"" {code:ServerApplyArgs}"; Flags: runhidden; StatusMsg: "Writing site settings..."; Check: FreshServer
Filename: "{sys}\sc.exe"; Parameters: "start itcmdr-server"; Flags: runhidden; StatusMsg: "Starting itcmdr-server..."
Filename: "{sys}\sc.exe"; Parameters: "create itcmdr-server-watchdog binPath= ""\""{app}\server-watchdog\itcmdr-server-watchdog.exe\"" --service"" start= delayed-auto DisplayName= ""itcmdr-server-watchdog"""; Flags: runhidden; StatusMsg: "Creating Windows service itcmdr-server-watchdog..."; Check: not ServiceExists('itcmdr-server-watchdog')
Filename: "{sys}\sc.exe"; Parameters: "config itcmdr-server-watchdog binPath= ""\""{app}\server-watchdog\itcmdr-server-watchdog.exe\"" --service"""; Flags: runhidden; Check: ServiceExists('itcmdr-server-watchdog')
Filename: "{sys}\sc.exe"; Parameters: "description itcmdr-server-watchdog ""itcmdr server watchdog: server upgrades, rollback and restarts"""; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "failure itcmdr-server-watchdog reset= 86400 actions= restart/60000/restart/60000/restart/300000"; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "start itcmdr-server-watchdog"; Flags: runhidden; StatusMsg: "Starting itcmdr-server-watchdog..."
Filename: "{app}\server\itcmdr-cli.exe"; Parameters: "setup check --report ""{commonappdata}\itcmdr\setup-check.txt"""; Flags: runhidden; StatusMsg: "Checking the server setup..."
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""$s = '{code:GetAllowSubnet}'; if ($s) {{ netsh advfirewall firewall delete rule name='itcmdr-server' | Out-Null; netsh advfirewall firewall add rule name='itcmdr-server' dir=in action=allow enable=yes profile=any localport=50051 protocol=TCP remoteip=$s; netsh advfirewall firewall delete rule name='itcmdr-agent-link' | Out-Null; netsh advfirewall firewall add rule name='itcmdr-agent-link' dir=in action=allow enable=yes profile=any localport=50052 protocol=TCP remoteip=$s }"""; Flags: runhidden; StatusMsg: "Adding firewall rule..."
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Install-ItcmdrRotationTask.ps1"""; Flags: runhidden; StatusMsg: "Installing log rotation task..."
Filename: "{app}\server\itcmdr-cli.exe"; Parameters: "setup first-run {code:FirstRunArgs}"; Description: "Run first-run setup now (shows the 24 recovery words once; have paper ready)"; Flags: postinstall skipifsilent runascurrentuser; Check: FreshServer

[UninstallRun]
Filename: "{sys}\sc.exe"; Parameters: "stop itcmdr-server-watchdog"; Flags: runhidden; RunOnceId: "StopServerWatchdog"
Filename: "{sys}\sc.exe"; Parameters: "stop itcmdr-server"; Flags: runhidden; RunOnceId: "StopService"
Filename: "powershell.exe"; Parameters: "-NoProfile -Command ""Start-Sleep -Seconds 3"""; Flags: runhidden; RunOnceId: "WaitService"
Filename: "{sys}\sc.exe"; Parameters: "delete itcmdr-server"; Flags: runhidden; RunOnceId: "DeleteService"
Filename: "{sys}\sc.exe"; Parameters: "delete itcmdr-server-watchdog"; Flags: runhidden; RunOnceId: "DeleteServerWatchdog"
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Unregister-ScheduledTask -TaskName 'itcmdr Log Rotation' -Confirm:$false -ErrorAction SilentlyContinue; netsh advfirewall firewall delete rule name='itcmdr-server'; netsh advfirewall firewall delete rule name='itcmdr-agent-link'"""; Flags: runhidden; RunOnceId: "RotationFirewall"

[Code]
#include "common-code.iss"

var
  SitePage, AdminPage: TInputQueryWizardPage;
  WasFreshServer: Boolean;

function GetAllowSubnet(Param: string): string;
begin
  Result := ExpandConstant('{param:AllowSubnet|198.51.100.0/24}');
end;

function InitializeSetup(): Boolean;
begin
  WasFreshServer := not FileExists(DataDir() + '\server.json');
  Result := True;
end;

procedure InitializeWizard();
begin
  SitePage := CreateInputQueryPage(wpSelectTasks, 'Site settings',
    'Values for this server (first install only; upgrades keep what is set).',
    'Leave a field empty to keep the default. Change them later with itcmdr-cli setup apply.');
  SitePage.Add('maintenanceRuns share or folder (the server reads the agent logs here):', False);
  SitePage.Add('Backup folder on the server (a share or second disk):', False);
  SitePage.Values[0] := ExpandConstant('{param:LogsDir|}');
  SitePage.Values[1] := ExpandConstant('{param:BackupDir|}');
  AdminPage := CreateInputQueryPage(SitePage.ID, 'First admin',
    'The person who manages itcmdr from the app.',
    'First-run setup at the end creates the master key (24 words, shown once) and registers this person''s app key.');
  AdminPage.Add('Admin name:', False);
  AdminPage.Values[0] := ExpandConstant('{param:AdminName|}');
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := ((PageID = SitePage.ID) or (PageID = AdminPage.ID)) and not WasFreshServer;
end;

function FreshServer(): Boolean;
begin
  Result := WasFreshServer;
end;

function ServerApplyArgs(Param: string): string;
begin
  Result := '';
  if SitePage.Values[0] <> '' then Result := Result + ' --logs-dir ' + Q(SitePage.Values[0]);
  if SitePage.Values[1] <> '' then Result := Result + ' --backup-dir ' + Q(SitePage.Values[1]);
end;

function FirstRunArgs(Param: string): string;
begin
  Result := '';
  if AdminPage.Values[0] <> '' then Result := '--admin-name ' + Q(AdminPage.Values[0]);
  if WizardIsTaskSelected('appkey') then Result := Result + ' --app-key-store';
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  RC: Integer;
begin
  // The watchdog first, so it does not restart the server mid-install.
  if ServiceExists('itcmdr-server-watchdog') then
    Exec(ExpandConstant('{sys}\sc.exe'), 'stop itcmdr-server-watchdog', '', SW_HIDE, ewWaitUntilTerminated, RC);
  Result := '';
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then MigrateCombined('server');
end;

function InitializeUninstall(): Boolean;
begin
  AskRemoveData(True);
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usPostUninstall) and RemoveDataChosen then
  begin
    RemoveItem('itcmdr.database'); RemoveItem('itcmdr.database-wal'); RemoveItem('itcmdr.database-shm');
    RemoveItem('itcmdr.sqlite'); RemoveItem('itcmdr.sqlite-wal'); RemoveItem('itcmdr.sqlite-shm');
    RemoveItem('service_plugins'); RemoveItem('quarantine'); RemoveItem('plugin-logs');
    RemoveItem('server-logs'); RemoveItem('server.json'); RemoveItem('setup-check.txt');
    if RemoveKeysChosen then RemoveItem('keys');
    RemoveDataDirIfEmpty();
  end;
end;
