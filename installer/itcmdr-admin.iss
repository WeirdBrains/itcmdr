; itcmdr-admin package (v0.19.0): the adminsvr admin CLI on PATH ({app}\admin). Its identity lives
; in the user's profile (user config folder) and is kept on uninstall.
; Silent: itcmdr-admin-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
#include "common.iss"

[Setup]
AppId={{{#AdminAppId}}
AppName=itcmdr-admin
UninstallDisplayName=itcmdr-admin {#MyAppVersion}
OutputBaseFilename=itcmdr-admin-Setup-{#MyAppVersion}
UninstallFilesDir={app}\uninstall\admin
UninstallDisplayIcon={app}\uninstall\admin\itcmdr.ico
ChangesEnvironment=yes
#include "common-setup.iss"

[Files]
Source: "..\branding\office\windows\app_icon.ico"; DestDir: "{app}\uninstall\admin"; DestName: "itcmdr.ico"; Flags: ignoreversion
Source: "..\target\release\itcmdr-admin.exe"; DestDir: "{app}\admin"; Flags: ignoreversion

[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\admin"; Check: NeedsPath(ExpandConstant('{app}\admin'))

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
  if CurUninstallStep = usPostUninstall then RemovePath(ExpandConstant('{app}\admin'));
end;
