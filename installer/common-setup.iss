; Included inside each package's [Setup] section.
AppVersion={#MyAppVersion}
AppPublisher=itcmdr contributors
AppPublisherURL=https://github.com/WeirdBrains/itcmdr
DefaultDirName={pf}\{#InstallDirName}
DisableDirPage=auto
DisableProgramGroupPage=yes
DirExistsWarning=no
UsePreviousAppDir=yes
OutputDir=out
ArchitecturesInstallIn64BitMode=x64
ArchitecturesAllowed=x64
PrivilegesRequired=admin
CloseApplications=yes
; Optional Authenticode signing: pass /Ssigntool="signtool.exe sign /f $q...pfx$q /p ... /fd sha256 /tr ... /td sha256 $f"
; and /DSign to ISCC (CI does when the signing secret exists). Without it, packages are unsigned
; and the release publishes SHA256SUMS.
#ifdef Sign
SignTool=signtool
SignedUninstaller=yes
#endif
; Office icon from branding/ (v0.14.3) for the setup wizard; packages set UninstallDisplayIcon.
SetupIconFile=..\branding\office\windows\app_icon.ico
WizardStyle=modern
SetupLogging=yes
