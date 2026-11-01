; itcmdr agent package (v0.14.3): the PowerShell agent (scripts, pinned check plugins, the nightly
; and poller tasks). Push-friendly: silent install with site values and an enroll token, e.g.
;   itcmdr-agent-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SiteName=... /ServerHostname=SERVER01
;     /Share=\\SERVER01\maintenanceRuns [/LocalLogs=C:\maintenanceRuns] [/EnrollToken=<one-time token>]
; v0.17.0: also the Rust agent (itcmdr-agent.exe, Windows service "itcmdr-agent", LocalSystem). The
; enroll token is kept in %ProgramData%\itcmdr\agent-enroll.token (Administrators and SYSTEM only);
; the service enrolls with it on start (pinned TLS, client certificate) and deletes it. The
; PowerShell agent keeps running its tasks unchanged. Push-Install-ItcmdrAgent.ps1 (copy-over-SMB push) still works and is included.
; Uninstall keeps %ProgramData%\itcmdr; /REMOVEDATA=yes removes the agent's files there.
#include "common.iss"
#ifndef LocalLogs
  #define LocalLogs "C:\maintenanceRuns"
#endif

[Setup]
AppId={{{#AgentAppId}}
AppName=itcmdr agent
UninstallDisplayName=itcmdr agent {#MyAppVersion}
OutputBaseFilename=itcmdr-agent-Setup-{#MyAppVersion}
UninstallFilesDir={app}\uninstall\agent
UninstallDisplayIcon={app}\uninstall\agent\itcmdr.ico
#include "common-setup.iss"

[Files]
Source: "..\target\release\itcmdr-agent.exe"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\target\release\itcmdr-plugin-ps.exe"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\branding\office\windows\app_icon.ico"; DestDir: "{app}\uninstall\agent"; DestName: "itcmdr.ico"; Flags: ignoreversion
Source: "..\agent\Invoke-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Install-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Poll-ItcmdrRequests.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Itcmdr.Checks.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Itcmdr.PluginRunner.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Initialize-ItcmdrSite.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Set-ItcmdrCutover.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Push-Install-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Itcmdr.Plugin.psm1"; DestDir: "{app}\agent"; Flags: ignoreversion sharedfile
#if !FileExists(AddBackslash(SourcePath) + "..\third_party\itcmdr-plugins\plugins\itcmdr-network\main.ps1")
  #error "Pinned plugins missing: run scripts/fetch-plugins.sh (see plugins.pin.json)"
#endif
Source: "..\third_party\itcmdr-plugins\plugins\*"; Excludes: "*.toml"; DestDir: "{app}\agent\plugins"; Flags: ignoreversion recursesubdirs

[Dirs]
Name: "{param:LocalLogs|{#LocalLogs}}"; Permissions: users-full; Flags: uninsneveruninstall

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Initialize-ItcmdrSite.ps1"" {code:AgentSiteArgs} -Report ""{commonappdata}\itcmdr\agent-setup-check.txt"""; Flags: runhidden; StatusMsg: "Writing agent site settings..."
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Install-ItcmdrAgent.ps1"""; Flags: runhidden; StatusMsg: "Installing agent scheduled tasks..."
Filename: "{sys}\sc.exe"; Parameters: "create itcmdr-agent binPath= ""\""{app}\agent\itcmdr-agent.exe\"" --service"" start= auto DisplayName= ""itcmdr-agent"""; Flags: runhidden; StatusMsg: "Creating Windows service itcmdr-agent..."; Check: not ServiceExists('itcmdr-agent')
Filename: "{sys}\sc.exe"; Parameters: "config itcmdr-agent binPath= ""\""{app}\agent\itcmdr-agent.exe\"" --service"""; Flags: runhidden; Check: ServiceExists('itcmdr-agent')
Filename: "{sys}\sc.exe"; Parameters: "description itcmdr-agent ""itcmdr agent: heartbeat link to the site server"""; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "failure itcmdr-agent reset= 86400 actions= restart/60000/restart/60000/restart/300000"; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "start itcmdr-agent"; Flags: runhidden; StatusMsg: "Starting itcmdr-agent..."

[UninstallRun]
Filename: "{sys}\sc.exe"; Parameters: "stop itcmdr-agent"; Flags: runhidden; RunOnceId: "StopAgentService"
Filename: "powershell.exe"; Parameters: "-NoProfile -Command ""Start-Sleep -Seconds 3"""; Flags: runhidden; RunOnceId: "WaitAgentService"
Filename: "{sys}\sc.exe"; Parameters: "delete itcmdr-agent"; Flags: runhidden; RunOnceId: "DeleteAgentService"
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Unregister-ScheduledTask -TaskName 'itcmdr Agent Nightly' -Confirm:$false -ErrorAction SilentlyContinue; Unregister-ScheduledTask -TaskName 'itcmdr Agent Poller' -Confirm:$false -ErrorAction SilentlyContinue"""; Flags: runhidden; RunOnceId: "AgentTasks"

[Code]
#include "common-code.iss"

var
  SitePage: TInputQueryWizardPage;
  WasFreshAgent: Boolean;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  RC: Integer;
begin
  if ServiceExists('itcmdr-agent') then
  begin
    Exec(ExpandConstant('{sys}\sc.exe'), 'stop itcmdr-agent', '', SW_HIDE, ewWaitUntilTerminated, RC);
    Sleep(3000);
  end;
  Result := '';
end;

function InitializeSetup(): Boolean;
begin
  WasFreshAgent := not FileExists(DataDir() + '\site.json');
  Result := True;
end;

procedure InitializeWizard();
begin
  SitePage := CreateInputQueryPage(wpSelectDir, 'Site settings',
    'Values for this office (first install only; upgrades only fill empty values).',
    'Leave a field empty to keep the default. All of them can be changed later in site.json.');
  SitePage.Add('Site name:', False);
  SitePage.Add('Server name (agents connect to it):', False);
  SitePage.Add('maintenanceRuns share (agents write logs here):', False);
  SitePage.Values[0] := ExpandConstant('{param:SiteName|}');
  SitePage.Values[1] := ExpandConstant('{param:ServerHostname|}');
  SitePage.Values[2] := ExpandConstant('{param:Share|}');
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := (PageID = SitePage.ID) and not WasFreshAgent;
end;

function AgentSiteArgs(Param: string): string;
begin
  Result := '-LocalLogs ' + Q(ExpandConstant('{param:LocalLogs|{#LocalLogs}}'));
  if SitePage.Values[0] <> '' then Result := Result + ' -SiteName ' + Q(SitePage.Values[0]);
  if SitePage.Values[1] <> '' then Result := Result + ' -ServerHostname ' + Q(SitePage.Values[1]);
  if SitePage.Values[2] <> '' then Result := Result + ' -MaintenanceRunsShare ' + Q(SitePage.Values[2]);
end;

procedure SaveEnrollToken();
var
  Tok, P: string;
  RC: Integer;
begin
  Tok := Trim(ExpandConstant('{param:EnrollToken|}'));
  if Tok = '' then exit;
  ForceDirectories(DataDir());
  P := DataDir() + '\agent-enroll.token';
  if SaveStringToFile(P, Tok, False) then
  begin
    // Administrators and SYSTEM only (no inheritance).
    Exec(ExpandConstant('{sys}\icacls.exe'), Q(P) + ' /inheritance:r /grant:r *S-1-5-32-544:F *S-1-5-18:F', '', SW_HIDE, ewWaitUntilTerminated, RC);
    Log('enroll token stored (the itcmdr-agent service enrolls with it)');
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    SaveEnrollToken();
    MigrateCombined('agent');
  end;
end;

function InitializeUninstall(): Boolean;
begin
  AskRemoveData(False);
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usPostUninstall) and RemoveDataChosen then
  begin
    RemoveItem('site.json'); RemoveItem('agent-setup-check.txt'); RemoveItem('agent-enroll.token'); RemoveItem('agent');
    RemoveItem('exceptions.json'); RemoveItem('spool');
    RemoveDataDirIfEmpty();
  end;
end;
