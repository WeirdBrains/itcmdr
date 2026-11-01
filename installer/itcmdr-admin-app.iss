; itcmdr Admin app package (v0.19.2): the Flutter Windows app for adminsvr (owners, admins, SREs)
; with its bridge (itcmdr_admin_bridge.dll, built by cargokit). The device identity lives in the
; user's app data folder and is kept on uninstall.
; Silent: itcmdr-admin-app-Setup-<ver>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART [/TASKS=desktopicon]
#include "common.iss"

[Setup]
AppId={{{#AdminAppAppId}}
AppName=itcmdr Admin
UninstallDisplayName=itcmdr Admin {#MyAppVersion}
OutputBaseFilename=itcmdr-admin-app-Setup-{#MyAppVersion}
UninstallFilesDir={app}\uninstall\admin-app
UninstallDisplayIcon={app}\admin-app\itcmdr-admin-app.exe
#include "common-setup.iss"

[Files]
#if !FileExists(AddBackslash(SourcePath) + "..\admin-app\build\windows\x64\runner\Release\itcmdr_admin_bridge.dll")
  #error "itcmdr_admin_bridge.dll missing: run flutter build windows in admin-app (cargokit builds the bridge)"
#endif
Source: "..\admin-app\build\windows\x64\runner\Release\*"; DestDir: "{app}\admin-app"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{autoprograms}\itcmdr Admin"; Filename: "{app}\admin-app\itcmdr-admin-app.exe"
Name: "{commondesktop}\itcmdr Admin"; Filename: "{app}\admin-app\itcmdr-admin-app.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; Flags: unchecked

[Code]
#include "common-code.iss"
