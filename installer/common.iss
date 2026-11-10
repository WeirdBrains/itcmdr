; Shared by the per-component packages (v0.14.3): itcmdr-server.iss, itcmdr-agent.iss,
; itcmdr-cli.iss, itcmdr-app.iss. Every package installs under {pf}\itcmdr\<part> with its own
; AppId and uninstaller ({app}\uninstall\<part>). Data lives in %ProgramData%\itcmdr and is kept
; on uninstall unless asked for (see RemoveData below).
#ifndef MyAppVersion
  #define MyAppVersion "1.5.0"
#endif
#define InstallDirName "itcmdr"
; AppIds (never change them: they identify the installed package for upgrades and uninstall).
#define ServerAppId "d6ea2f43-c8a2-4d85-9f80-4ae7a2981f46"
#define AgentAppId  "5d38141d-801e-4730-8294-c4bdfe8b3af1"
#define CliAppId    "bf8d258c-96b2-465f-81f0-d95d83d06cc6"
#define AppAppId    "61fbffb5-44e6-4c75-bc85-a397d58b9de8"
#define AdminAppId  "baf17c7c-c912-46da-afd6-0283f3615a97"
#define AdminsvrAppId "2437d20d-62ac-4675-8505-3d3641e3135e"
#define AdminAppAppId "071c85d4-834f-45f1-aa2d-48ea939ef96d"
