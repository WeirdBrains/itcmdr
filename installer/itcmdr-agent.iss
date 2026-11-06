; itcmdr agent package (v0.14.3): the PowerShell agent (scripts, pinned check plugins, the nightly
; and poller tasks). Push-friendly: silent install with site values and an enroll token, e.g.
;   itcmdr-agent-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SiteName=... /ServerHostname=SERVER01
;     /Share=\\SERVER01\maintenanceRuns [/LocalLogs=C:\maintenanceRuns] [/EnrollToken=<one-time token>]
; v0.17.0: also the Rust agent (itcmdr-agent.exe, Windows service "itcmdr-agent", LocalSystem). The
; enroll token is kept in %ProgramData%\itcmdr\agent-enroll.token (Administrators and SYSTEM only);
; Apply-ItcmdrEnrollToken.ps1 then enrolls immediately as this computer (v1.3.4: roll back on failure, exit 9). The
; PowerShell agent keeps running its tasks unchanged. Push-Install-ItcmdrAgent.ps1 (copy-over-SMB push) still works and is included.
; v0.21.0: the Rust agent runs from {app}\agent\current\ (agent upgrades swap current\ and previous\),
; and the itcmdr-agent-watchdog service ({app}\agent\watchdog\, replaced only by this installer)
; carries out upgrades, rolls a bad one back and restarts a stopped or hung agent.
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
Source: "..\target\release\itcmdr-agent.exe"; DestDir: "{app}\agent\current"; Flags: ignoreversion
Source: "..\target\release\itcmdr-plugin-ps.exe"; DestDir: "{app}\agent\current"; Flags: ignoreversion
Source: "..\target\release\itcmdr-agent-watchdog.exe"; DestDir: "{app}\agent\watchdog"; Flags: ignoreversion
Source: "..\branding\office\windows\app_icon.ico"; DestDir: "{app}\uninstall\agent"; DestName: "itcmdr.ico"; Flags: ignoreversion
Source: "..\agent\Invoke-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Install-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Poll-ItcmdrRequests.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Itcmdr.Checks.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Itcmdr.PluginRunner.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Initialize-ItcmdrSite.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Set-ItcmdrCutover.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Push-Install-ItcmdrAgent.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Apply-ItcmdrEnrollToken.ps1"; DestDir: "{app}\agent"; Flags: ignoreversion
Source: "..\agent\Itcmdr.Plugin.psm1"; DestDir: "{app}\agent"; Flags: ignoreversion sharedfile
#if !FileExists(AddBackslash(SourcePath) + "..\third_party\itcmdr-plugins\plugins\itcmdr-network\main.ps1")
  #error "Pinned plugins missing: run scripts/fetch-plugins.sh (see plugins.pin.json)"
#endif
Source: "..\third_party\itcmdr-plugins\plugins\*"; Excludes: "*.toml"; DestDir: "{app}\agent\plugins"; Flags: ignoreversion recursesubdirs

[InstallDelete]
; v0.21.0: the binaries moved into current\ (the old copies would be stale).
Type: files; Name: "{app}\agent\itcmdr-agent.exe"
Type: files; Name: "{app}\agent\itcmdr-plugin-ps.exe"

[UninstallDelete]
; Written by agent upgrades (not by this installer).
Type: filesandordirs; Name: "{app}\agent\current"
Type: filesandordirs; Name: "{app}\agent\previous"
Type: filesandordirs; Name: "{app}\agent\staged"
Type: filesandordirs; Name: "{app}\agent\failed"

[Dirs]
Name: "{param:LocalLogs|{#LocalLogs}}"; Permissions: users-full; Flags: uninsneveruninstall

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Initialize-ItcmdrSite.ps1"" {code:AgentSiteArgs} -Report ""{commonappdata}\itcmdr\agent-setup-check.txt"""; Flags: runhidden; StatusMsg: "Writing agent site settings..."
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\agent\Install-ItcmdrAgent.ps1"""; Flags: runhidden; StatusMsg: "Installing agent scheduled tasks..."
Filename: "{sys}\sc.exe"; Parameters: "create itcmdr-agent binPath= ""\""{app}\agent\current\itcmdr-agent.exe\"" --service"" start= auto DisplayName= ""itcmdr-agent"""; Flags: runhidden; StatusMsg: "Creating Windows service itcmdr-agent..."; Check: not ServiceExists('itcmdr-agent')
Filename: "{sys}\sc.exe"; Parameters: "config itcmdr-agent binPath= ""\""{app}\agent\current\itcmdr-agent.exe\"" --service"""; Flags: runhidden; Check: ServiceExists('itcmdr-agent')
Filename: "{sys}\sc.exe"; Parameters: "description itcmdr-agent ""itcmdr agent: heartbeat link to the site server"""; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "failure itcmdr-agent reset= 86400 actions= restart/60000/restart/60000/restart/300000"; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "start itcmdr-agent"; Flags: runhidden; StatusMsg: "Starting itcmdr-agent..."
Filename: "{sys}\sc.exe"; Parameters: "create itcmdr-agent-watchdog binPath= ""\""{app}\agent\watchdog\itcmdr-agent-watchdog.exe\"" --service"" start= auto DisplayName= ""itcmdr-agent-watchdog"""; Flags: runhidden; StatusMsg: "Creating Windows service itcmdr-agent-watchdog..."; Check: not ServiceExists('itcmdr-agent-watchdog')
Filename: "{sys}\sc.exe"; Parameters: "config itcmdr-agent-watchdog binPath= ""\""{app}\agent\watchdog\itcmdr-agent-watchdog.exe\"" --service"""; Flags: runhidden; Check: ServiceExists('itcmdr-agent-watchdog')
Filename: "{sys}\sc.exe"; Parameters: "description itcmdr-agent-watchdog ""itcmdr agent watchdog: agent upgrades, rollback and restarts"""; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "failure itcmdr-agent-watchdog reset= 86400 actions= restart/60000/restart/60000/restart/300000"; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "start itcmdr-agent-watchdog"; Flags: runhidden; StatusMsg: "Starting itcmdr-agent-watchdog..."

[UninstallRun]
Filename: "{sys}\sc.exe"; Parameters: "stop itcmdr-agent-watchdog"; Flags: runhidden; RunOnceId: "StopAgentWatchdog"
Filename: "{sys}\sc.exe"; Parameters: "stop itcmdr-agent"; Flags: runhidden; RunOnceId: "StopAgentService"
Filename: "powershell.exe"; Parameters: "-NoProfile -Command ""Start-Sleep -Seconds 3"""; Flags: runhidden; RunOnceId: "WaitAgentService"
Filename: "{sys}\sc.exe"; Parameters: "delete itcmdr-agent"; Flags: runhidden; RunOnceId: "DeleteAgentService"
Filename: "{sys}\sc.exe"; Parameters: "delete itcmdr-agent-watchdog"; Flags: runhidden; RunOnceId: "DeleteAgentWatchdog"
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Unregister-ScheduledTask -TaskName 'itcmdr Agent Nightly' -Confirm:$false -ErrorAction SilentlyContinue; Unregister-ScheduledTask -TaskName 'itcmdr Agent Poller' -Confirm:$false -ErrorAction SilentlyContinue"""; Flags: runhidden; RunOnceId: "AgentTasks"

[Code]
#include "common-code.iss"

var
  SitePage: TInputQueryWizardPage;
  WasFreshAgent: Boolean;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  // v1.2.1: the watchdog first, both waited for, then a lock probe (common-code.iss).
  Result := InstallStopAndProbe();
end;

function InitializeSetup(): Boolean;
begin
  WasFreshAgent := not FileExists(DataDir() + '\site.json');
  InstallLogBegin('agent', '{#AgentAppId}', ExpandConstant('{commonappdata}\itcmdr\install-logs'), 'itcmdr-agent-watchdog,itcmdr-agent',
    'agent\current\itcmdr-agent.exe,agent\current\itcmdr-plugin-ps.exe,agent\watchdog\itcmdr-agent-watchdog.exe');
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
  Tok, P, Args, ErrP, Err: string;
  ErrA: AnsiString;
  RC: Integer;
begin
  // v1.3.4: /EnrollToken= on an already-enrolled PC must re-enroll (stop services,
  // enroll as this computer's name, roll back agent.json on failure). Writing the
  // token file and hoping the running service picks it up does nothing once enrolled.
  Tok := Trim(ExpandConstant('{param:EnrollToken|}'));
  if Tok = '' then exit;
  ForceDirectories(DataDir());
  P := DataDir() + '\agent-enroll.token';
  if not SaveStringToFile(P, Tok, False) then
  begin
    IEnrollFail := True;
    IFail('enroll', 'could not write the enrollment token file');
    exit;
  end;
  Exec(ExpandConstant('{sys}\icacls.exe'), Q(P) + ' /inheritance:r /grant:r *S-1-5-32-544:F *S-1-5-18:F', '', SW_HIDE, ewWaitUntilTerminated, RC);
  ILog('applying /EnrollToken= (stop services, enroll as this computer, verify the link)');
  Args := '-NoProfile -ExecutionPolicy Bypass -File ' + Q(ExpandConstant('{app}\agent\Apply-ItcmdrEnrollToken.ps1'))
    + ' -TokenFile ' + Q(P)
    + ' -AgentExe ' + Q(ExpandConstant('{app}\agent\current\itcmdr-agent.exe'))
    + ' -DataDir ' + Q(DataDir() + '\agent')
    + ' -ComputerName ' + Q(GetComputerNameString());
  if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Args, '', SW_HIDE, ewWaitUntilTerminated, RC) then RC := -1;
  if RC <> 0 then
  begin
    IEnrollFail := True;
    Err := 're-enrollment with /EnrollToken= failed (exit ' + IntToStr(RC) + ')';
    ErrP := DataDir() + '\agent\last-enroll-error.txt';
    if LoadStringFromFile(ErrP, ErrA) then Err := Trim(ErrA);
    IFail('enroll', Err);
    ILog('enroll failed: ' + Err);
  end
  else
    ILog('enroll applied; agent linked as ' + GetComputerNameString());
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssDone then InstallVerifyServices();
  if CurStep = ssPostInstall then
  begin
    InstallFilesDone();
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
