## Install guide (admin PowerShell)

This repository contains placeholders only. Provide site values via `config/site.json` or `C:\ProgramData\itcmdr\site.json` (see `config/site.example.json`). Do not commit real values.

### Server (Windows)
1. Create `C:\ProgramData\itcmdr\site.json` from the example.
2. Install the Windows release installer (see Releases page).
3. Confirm service `itcmdr-server` is running and firewall rule exists (subnet is configurable via installer parameter `/DAllowSubnet=`).

Silent Agent-only install:
```
itcmdr-Setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /COMPONENTS=\"agent\"
```

### Agent (manual)
1. Copy `agent\\Invoke-ItcmdrAgent.ps1` and `agent\\Install-ItcmdrAgent.ps1` to target machine.
2. In elevated PowerShell:
```
Set-ExecutionPolicy Bypass -Scope Process -Force
.\Install-ItcmdrAgent.ps1
```

### App
- Windows: installed by the Windows installer (Start Menu shortcut).
- Web: run locally with `flutter build web` and host behind an internal web server; configure the server address in the app settings (to be implemented).

