; Inno Setup script for itcmdr
; Components:
;  - Server (register Windows service itcmdr-server; add firewall rule scoped to subnet)
;  - App (Flutter Windows desktop; Start Menu shortcut)
;  - Agent (PowerShell scripts; scheduled task; local log folder, default C:\maintenanceRuns)
;
; Local log folder (decision 15): build with  ISCC /DLocalLogs="D:\itcmdr\logs" itcmdr.iss,
; or override at install time with  itcmdr-Setup.exe /LocalLogs="D:\itcmdr\logs".
; Set the same folder in site.json "paths": { "localLogs": ... } so the agent scripts use it.

#define MyAppName "itcmdr"
#ifndef MyAppVersion
  #define MyAppVersion "0.11.0"
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
Source: "..\agent\Set-ItcmdrCutover.ps1"; DestDir: "{app}\agent"; Components: agent; Flags: ignoreversion
; Built-in check plugins (v0.11.0): the only plugin copies agents run; never downloaded
Source: "..\plugins\builtin\*"; Excludes: "*.toml,Build-BuiltinPlugins.ps1"; DestDir: "{app}\agent\plugins"; Components: agent; Flags: ignoreversion recursesubdirs
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
Filename: "sc.exe"; Parameters: "start itcmdr-server"; Flags: runhidden; Components: server; StatusMsg: "Starting itcmdr-server..."

; Firewall rule restricted to example subnet; override at install time with /DAllowSubnet=198.51.100.0/24
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""$s = '{code:GetAllowSubnet}'; if ($s) {{ netsh advfirewall firewall delete rule name='itcmdr-server' | Out-Null; netsh advfirewall firewall add rule name='itcmdr-server' dir=in action=allow enable=yes profile=any localport=50051 protocol=TCP remoteip=$s }"""; Flags: runhidden; Components: server; StatusMsg: "Adding firewall rule..."

; Install agent scheduled task
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Install-ItcmdrAgent.ps1"""; Flags: runhidden; Components: agent; StatusMsg: "Installing agent scheduled task..."
; Install log rotation scheduled task (daily, server only)
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Install-ItcmdrRotationTask.ps1"""; Flags: runhidden; Components: server; StatusMsg: "Installing log rotation task..."

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

// Silent install (Agent only):
//   itcmdr-Setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /COMPONENTS=\"agent\"

