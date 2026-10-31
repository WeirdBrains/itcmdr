; itcmdr-cli package (v0.14.3): the command line tool on PATH ({app}\cli), for admins and scripts on
; any PC. The server package carries its own copy in {app}\server for its setup steps, so the server
; works without this package; install this one where you want `itcmdr-cli` on PATH.
; Silent: itcmdr-cli-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
#include "common.iss"

[Setup]
AppId={{{#CliAppId}}
AppName=itcmdr-cli
UninstallDisplayName=itcmdr-cli {#MyAppVersion}
OutputBaseFilename=itcmdr-cli-Setup-{#MyAppVersion}
UninstallFilesDir={app}\uninstall\cli
UninstallDisplayIcon={app}\uninstall\cli\itcmdr.ico
ChangesEnvironment=yes
#include "common-setup.iss"

[Files]
Source: "..\branding\office\windows\app_icon.ico"; DestDir: "{app}\uninstall\cli"; DestName: "itcmdr.ico"; Flags: ignoreversion
Source: "..\target\release\itcmdr-cli.exe"; DestDir: "{app}\cli"; Flags: ignoreversion

[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\cli"; Check: NeedsPath(ExpandConstant('{app}\cli'))

[Code]
#include "common-code.iss"

function NeedsPath(Dir: string): Boolean;
var
  P: string;
begin
  if not RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', P) then
    P := '';
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(P) + ';') = 0;
end;

procedure RemovePath(Dir: string);
var
  P: string;
  I: Integer;
begin
  if not RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', P) then exit;
  P := ';' + P + ';';
  I := Pos(';' + Uppercase(Dir) + ';', Uppercase(P));
  if I = 0 then exit;
  Delete(P, I, Length(Dir) + 1);
  P := Copy(P, 2, Length(P) - 2);
  RegWriteExpandStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', P);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then RemovePath(ExpandConstant('{app}\cli'));
end;
