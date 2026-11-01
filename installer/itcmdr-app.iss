; itcmdr app package (v0.14.3): the Flutter Windows desktop app with the itcmdr-keys bridge
; (itcmdr_keys_bridge.dll, built by the flutter_rust_bridge native-assets hook next to itcmdr.exe). The app key stays in the user's
; DPAPI store (%APPDATA%\itcmdr) and is never removed by the uninstaller.
; Silent: itcmdr-app-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART [/TASKS=desktopicon]
#include "common.iss"

[Setup]
AppId={{{#AppAppId}}
AppName=itcmdr app
UninstallDisplayName=itcmdr app {#MyAppVersion}
OutputBaseFilename=itcmdr-app-Setup-{#MyAppVersion}
UninstallFilesDir={app}\uninstall\app
UninstallDisplayIcon={app}\app\itcmdr.exe
#include "common-setup.iss"

[Files]
#if !FileExists(AddBackslash(SourcePath) + "..\app\build\windows\x64\runner\Release\itcmdr_keys_bridge.dll")
  #error "itcmdr_keys_bridge.dll missing: run flutter build windows (its native-assets hook builds the bridge)"
#endif
Source: "..\app\build\windows\x64\runner\Release\*"; DestDir: "{app}\app"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{autoprograms}\itcmdr"; Filename: "{app}\app\itcmdr.exe"
Name: "{commondesktop}\itcmdr"; Filename: "{app}\app\itcmdr.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; Flags: unchecked

[Code]
#include "common-code.iss"

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then MigrateCombined('app');
end;
