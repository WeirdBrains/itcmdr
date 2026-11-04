; itcmdr-adminsvr package (v0.19.2): the offsite admin server as the Windows service
; "itcmdr-adminsvr" (LocalSystem, automatic start, restarts on failure). Settings:
; %ProgramData%\itcmdr-adminsvr\adminsvr.env (kept on upgrade and uninstall); data (root key, TLS,
; SQLite) in %ProgramData%\itcmdr-adminsvr\data, Administrators and SYSTEM only; log in
; %ProgramData%\itcmdr-adminsvr\logs. First owner, in an elevated prompt:
;   "%ProgramFiles%\itcmdr\adminsvr\itcmdr-adminsvr.exe" init --email you@example.com --name "Your Name"
; Silent: itcmdr-adminsvr-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART [/NOSTART=yes]
; Uninstall keeps %ProgramData%\itcmdr-adminsvr; /REMOVEDATA=yes on the uninstaller deletes it.
#include "common.iss"

[Setup]
AppId={{{#AdminsvrAppId}}
AppName=itcmdr-adminsvr
UninstallDisplayName=itcmdr-adminsvr {#MyAppVersion}
OutputBaseFilename=itcmdr-adminsvr-Setup-{#MyAppVersion}
UninstallFilesDir={app}\uninstall\adminsvr
UninstallDisplayIcon={app}\uninstall\adminsvr\itcmdr.ico
#include "common-setup.iss"

[Files]
Source: "..\branding\office\windows\app_icon.ico"; DestDir: "{app}\uninstall\adminsvr"; DestName: "itcmdr.ico"; Flags: ignoreversion
Source: "..\target\release\itcmdr-adminsvr.exe"; DestDir: "{app}\adminsvr"; Flags: ignoreversion
Source: "..\packaging\adminsvr\windows\adminsvr.env.example"; DestDir: "{commonappdata}\itcmdr-adminsvr"; Flags: ignoreversion
Source: "..\packaging\adminsvr\windows\adminsvr.env.example"; DestDir: "{commonappdata}\itcmdr-adminsvr"; DestName: "adminsvr.env"; Flags: onlyifdoesntexist uninsneveruninstall

[Dirs]
Name: "{commonappdata}\itcmdr-adminsvr"; Flags: uninsneveruninstall
Name: "{commonappdata}\itcmdr-adminsvr\data"; Flags: uninsneveruninstall
Name: "{commonappdata}\itcmdr-adminsvr\logs"; Flags: uninsneveruninstall

[Run]
; Only Administrators and SYSTEM may read the settings, keys and database (this folder only).
Filename: "{sys}\icacls.exe"; Parameters: """{commonappdata}\itcmdr-adminsvr"" /inheritance:r /grant:r *S-1-5-32-544:(OI)(CI)F *S-1-5-18:(OI)(CI)F /Q"; Flags: runhidden; StatusMsg: "Securing the adminsvr data folder..."
; ...and everything in it inherits that (no /T above: (OI)(CI) on a file leaves it with no access).
Filename: "{sys}\icacls.exe"; Parameters: """{commonappdata}\itcmdr-adminsvr\*"" /reset /T /C /Q"; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "create itcmdr-adminsvr binPath= ""\""{app}\adminsvr\itcmdr-adminsvr.exe\"" service"" start= auto DisplayName= ""itcmdr-adminsvr"""; Flags: runhidden; StatusMsg: "Creating Windows service itcmdr-adminsvr..."; Check: not ServiceExists('itcmdr-adminsvr')
Filename: "{sys}\sc.exe"; Parameters: "config itcmdr-adminsvr binPath= ""\""{app}\adminsvr\itcmdr-adminsvr.exe\"" service"" start= auto"; Flags: runhidden; Check: ServiceExists('itcmdr-adminsvr')
Filename: "{sys}\sc.exe"; Parameters: "description itcmdr-adminsvr ""itcmdr offsite admin server (admin API and SiteLink)"""; Flags: runhidden
; Recovery: restart after 5 s, 30 s, then every 2 min; the counter resets after a day. failureflag
; also applies it when the service stops with an error (not only on a crash).
Filename: "{sys}\sc.exe"; Parameters: "failure itcmdr-adminsvr reset= 86400 actions= restart/5000/restart/30000/restart/120000"; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "failureflag itcmdr-adminsvr 1"; Flags: runhidden
Filename: "{sys}\sc.exe"; Parameters: "start itcmdr-adminsvr"; Flags: runhidden; StatusMsg: "Starting itcmdr-adminsvr..."; Check: not NoStart

[UninstallRun]
Filename: "{sys}\sc.exe"; Parameters: "stop itcmdr-adminsvr"; Flags: runhidden; RunOnceId: "StopAdminsvrService"
Filename: "powershell.exe"; Parameters: "-NoProfile -Command ""Start-Sleep -Seconds 3"""; Flags: runhidden; RunOnceId: "WaitAdminsvrService"
Filename: "{sys}\sc.exe"; Parameters: "delete itcmdr-adminsvr"; Flags: runhidden; RunOnceId: "DeleteAdminsvrService"

[Code]
#include "common-code.iss"

function NoStart(): Boolean;
begin
  Result := CompareText(ExpandConstant('{param:NOSTART|no}'), 'yes') = 0;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := InstallStopAndProbe();
end;

function InitializeSetup(): Boolean;
begin
  InstallLogBegin('adminsvr', '{#AdminsvrAppId}', ExpandConstant('{commonappdata}\itcmdr-adminsvr\install-logs'),
    'itcmdr-adminsvr', 'adminsvr\itcmdr-adminsvr.exe');
  Result := True;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then InstallFilesDone();
  if (CurStep = ssDone) and not NoStart() then InstallVerifyServices();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usPostUninstall) and
     (CompareText(ExpandConstant('{param:REMOVEDATA|no}'), 'yes') = 0) then
    DelTree(ExpandConstant('{commonappdata}\itcmdr-adminsvr'), True, True, True);
end;
