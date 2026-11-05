#!/usr/bin/env bash
# itcmdr demo on one Linux box (adminsvr, admin CLI and apps) and three Windows machines: the office
# server (itcmdr server + agent) and two client PCs (agents). docs/DEMO.md explains every step.
#
#   scripts/demo.sh                  # run every step, pausing before each one
#   scripts/demo.sh --step 6         # start (or resume) at step 6
#   scripts/demo.sh --only 9         # run one step
#   scripts/demo.sh --yes            # no pauses (dry run)
#   scripts/demo.sh list             # the steps
#   scripts/demo.sh provision-vms    # create the two client VMs (dockur/windows, docker compose)
#   scripts/demo.sh teardown [--vms] # undo the demo (and remove the client VMs)
#
# Everything site-specific comes from the environment (nothing is built in):
#   DEMO_ADMIN_HOST    address the Windows machines use to reach this box (required)
#   DEMO_SERVER_ADDR   address the client PCs use to reach the office server VM (required for steps 6+)
#   DEMO_SSH_HOST      where the VMs' SSH ports are published (default 127.0.0.1)
#   DEMO_TUNNEL_BIND   when the server VM's 50051/50052 are not reachable from the clients (dockur VMs publish
#                      only listed ports): an address of this box the clients reach (the compose network's
#                      gateway); step 6 forwards <it>:50051/50052 to the server VM over its SSH port
#   DEMO_ISO_CACHE     where teardown --vms keeps the Windows ISO for the next provision (~/.cache/itcmdr-demo-iso)
#   DEMO_RENAME        rename-vms: demo computer names for the server VM and the clients, in DEMO_SERVER_SSH,
#                      DEMO_CLIENT_SSH order (default "OFFICE-SRV FRONT-DESK EXAM-ROOM"); the original names are
#                      kept in DEMO_NAMES_FILE (~/.cache/itcmdr-demo-names) and teardown renames them back
#   DEMO_SERVER_USER   SSH user on the server VM (default: your login name)
#   DEMO_CLIENT_USER   SSH user on the client VMs (default demo)
#   DEMO_SERVER_SSH    SSH port of the office server VM (default 2222)
#   DEMO_CLIENT_SSH    SSH ports of the client VMs, space separated (default "2223 2224")
#   DEMO_CLIENT_NAMES  their Windows computer names (default: asked from each VM; agents enroll under it)
#   DEMO_COMPOSE_DIR   folder of the docker-compose.yml that runs the Windows VMs (provision/teardown --vms)
#   DEMO_CLIENT_ROOT   folder for the client VMs' disks and first-boot files (default <compose dir>-demo)
#   DEMO_VERSION       release to demo (default: this checkout's version)
#   DEMO_DIR           working folder (default ~/.cache/itcmdr-demo): release files, adminsvr data,
#                      the demo identity; all removed by teardown
#   DEMO_INTERACTIVE=1 show the 24 words on screen and ask them back (live demo) instead of writing
#                      them to 0600 files in DEMO_DIR / on the server VM (dry runs)
# Needs: bash, curl or gh, ssh/scp with the box's key in the VMs' administrators_authorized_keys,
# docker compose for provision-vms. Nothing is installed system-wide: adminsvr runs as this user.
set -euo pipefail
cd "$(dirname "$0")/.."
REPO=$(pwd)
VER=${DEMO_VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' server/Cargo.toml | head -1)}
D=${DEMO_DIR:-$HOME/.cache/itcmdr-demo}
SSH_HOST=${DEMO_SSH_HOST:-127.0.0.1}
SRV_USER=${DEMO_SERVER_USER:-}
ISO_CACHE=${DEMO_ISO_CACHE:-$HOME/.cache/itcmdr-demo-iso}
CLI_USER=${DEMO_CLIENT_USER:-demo}
SRV_SSH=${DEMO_SERVER_SSH:-2222}
read -r -a CL_SSH <<<"${DEMO_CLIENT_SSH:-2223 2224}"
read -r -a CL_NAME <<<"${DEMO_CLIENT_NAMES:-}"
YES=0; START=1; ONLY=""; CMD=run
while [[ $# -gt 0 ]]; do case "$1" in
  --step) START=$2; shift ;; --only) ONLY=$2; shift ;; --yes|-y) YES=1 ;;
  list|teardown|provision-vms|rename-vms|restore-names) CMD=$1 ;; --vms) TD_VMS=1 ;;
  -h|--help) sed -n '2,30p' "$0"; exit 0 ;; *) echo "unknown argument $1" >&2; exit 2 ;; esac; shift; done

REL=$D/rel; ASV=$REL/itcmdr-adminsvr-$VER-linux-x86_64/itcmdr-adminsvr
ADM=$REL/itcmdr-admin-$VER-linux-x86_64/itcmdr-admin
export ITCMDR_ADMIN_HOME=$D/admin-home
asv_env() {
  export ITCMDR_ADMINSVR_DATA_DIR=$D/adminsvr ITCMDR_ADMINSVR_BACKUP_HOUR=off
  export ITCMDR_ADMINSVR_PUBLIC_API=${DEMO_ADMIN_HOST:?set DEMO_ADMIN_HOST}:8444
  export ITCMDR_ADMINSVR_PUBLIC_SITE=$DEMO_ADMIN_HOST:8443 ITCMDR_ADMINSVR_TLS_NAMES=$DEMO_ADMIN_HOST,localhost
}
say() { printf '\n\033[1;36m== %s\033[0m\n' "$*"; }
note() { printf '   %s\n' "$*"; }
pause() { [[ $YES == 1 ]] && return 0; read -r -p "   [enter] to continue, ctrl-c to stop " _ </dev/tty || true; }
# PowerShell on a VM over SSH (strict host key checking stays on; the VMs' keys are in known_hosts).
vm() { local port=$1; shift
  local enc; enc=$(printf '%s' "\$ProgressPreference='SilentlyContinue'; $*" | iconv -t utf-16le | base64 -w0)
  ssh -o BatchMode=yes -o ConnectTimeout=10 -p "$port" "$(dest "$port")" "powershell -NoProfile -NonInteractive -EncodedCommand $enc" 2>&1 \
    | grep -v -e 'post-quantum' -e 'upgraded' -e 'store now' || true; }
dest() { if [ "$1" = "${SRV_SSH:-2222}" ]; then u=$SRV_USER; else u=$CLI_USER; fi; echo "${u:+$u@}$SSH_HOST"; }
vmcopy() { scp -q -o BatchMode=yes -P "$1" "$2" "$(dest "$1"):$3"; }
CLI="& 'C:\\Program Files\\itcmdr\\server\\itcmdr-cli.exe'"
site_id() { cat "$D/site-id" 2>/dev/null || true; }

s1() { say "1. Prerequisites and the v$VER release files"
  note "Downloads the published release into $D/rel and checks SHA256SUMS."
  pause
  mkdir -p "$REL"; cd "$REL"
  for f in SHA256SUMS itcmdr-adminsvr-$VER-linux-x86_64.tar.gz itcmdr-admin-$VER-linux-x86_64.tar.gz \
           itcmdr-admin-app-$VER-linux-x86_64.tar.gz itcmdr-app-$VER-linux-x86_64.tar.gz \
           itcmdr-server-Setup-$VER.exe itcmdr-agent-Setup-$VER.exe; do
    [[ -f $f ]] || curl -fsSLO "https://github.com/WeirdBrains/itcmdr/releases/download/v$VER/$f"; done
  sha256sum -c --ignore-missing SHA256SUMS
  for t in itcmdr-*-linux-x86_64.tar.gz; do tar -xzf "$t"; done
  cd "$REPO"
  for p in "$SRV_SSH" "${CL_SSH[@]}"; do printf '   VM on port %s: ' "$p"; vm "$p" '$env:COMPUTERNAME' | tail -1; done; }

s2() { say "2. adminsvr on this box (HTTPS with its own certificate, as this user)"
  note "itcmdr-adminsvr serve with its data in $D/adminsvr (no root, no service). Let's Encrypt: docs/DEPLOY-ADMINSVR.md."
  pause
  asv_env; mkdir -p "$D"; chmod 700 "$D"
  if [[ ! -f $D/adminsvr/adminsvr.db ]]; then
    (umask 077; "$ASV" init --email owner@example.com --name "Demo Owner" > "$D/owner-code.txt")
    note "first owner created; enrollment code in $D/owner-code.txt (single use, 24 h)"; fi
  if ! pgrep -f "$ASV serve" >/dev/null; then (setsid nohup "$ASV" serve > "$D/adminsvr.log" 2>&1 < /dev/null &); sleep 3; fi
  grep -m2 -E 'listening' "$D/adminsvr.log" | sed 's/^.*INFO //'
  "$ASV" tls-status 2>&1 | head -3; }

s3() { say "3. Enroll the admin CLI (owner device)"
  note "itcmdr-admin enroll <code>: the code pins adminsvr's certificate; the 24 words are shown once."
  pause
  local code; code=$(grep -o 'itcmdr-admin:[A-Za-z0-9_-]*' "$D/owner-code.txt" | head -1)
  if [[ ! -d $ITCMDR_ADMIN_HOME ]]; then
    if [[ ${DEMO_INTERACTIVE:-} == 1 ]]; then "$ADM" enroll "$code"
    else (umask 077; "$ADM" enroll --words-file "$D/owner-words.txt" "$code" </dev/null | grep -v -i words); fi; fi
  "$ADM" whoami | head -1
  "$ADM" trust show 2>&1 | head -3; }

s4() { say "4. Office server VM: install the site server and its agent (release installers)"
  note "Silent installs of itcmdr-server-Setup-$VER.exe and itcmdr-agent-Setup-$VER.exe (an upgrade keeps the data)."
  pause
  vm "$SRV_SSH" "New-Item -ItemType Directory -Force C:\\itcmdr-demo | Out-Null"
  for p in server agent; do vmcopy "$SRV_SSH" "$REL/itcmdr-$p-Setup-$VER.exe" "C:/itcmdr-demo/"
    vm "$SRV_SSH" "\$r = Start-Process C:\\itcmdr-demo\\itcmdr-$p-Setup-$VER.exe -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -Wait -PassThru; 'itcmdr-$p setup exit ' + \$r.ExitCode"; done
  vm "$SRV_SSH" "Get-Service itcmdr* | Format-Table -Auto Name,Status | Out-String -Width 80; $CLI --version"
  if vm "$SRV_SSH" "$CLI setup check" | grep -qi 'not initialized'; then
    if [[ ${DEMO_INTERACTIVE:-} == 1 ]]; then
      note "In person on the server: itcmdr-cli setup first-run (shows the 24 words once)"
      ssh -t -p "$SRV_SSH" "$SSH_HOST" "\"C:\\Program Files\\itcmdr\\server\\itcmdr-cli.exe\" setup first-run --admin-name \"Demo Owner\""
    else
      note "Not initialized: itcmdr-cli init, words into C:\\itcmdr-demo\\words.txt (Administrators only; dry run)"
      vm "$SRV_SSH" "$CLI init > C:\\itcmdr-demo\\words.txt 2>&1; icacls C:\\itcmdr-demo\\words.txt /inheritance:r /grant Administrators:F /grant SYSTEM:F | Out-Null; 'initialized'"
    fi; fi
  vm "$SRV_SSH" "$CLI setup check" | tail -15; }

s5() { say "5. Link the office with adminsvr"
  note "itcmdr-admin site create, then on the server: itcmdr-cli adminsvr enroll <site token>."
  pause
  if [[ -z $(site_id) ]]; then
    local out tok; out=$("$ADM" site create "Demo Office")
    echo "$out" | head -1 | cut -d' ' -f1-3
    echo "$out" | awk '{print $2}' | head -1 > "$D/site-id"
    tok=$(echo "$out" | grep -o '[A-Za-z0-9_:.-]\{80,\}' | tail -1)
    vm "$SRV_SSH" "$CLI adminsvr enroll $tok"; fi
  sleep 10; "$ADM" site list; }

s6() { say "6. Client PCs: enroll an agent on each"
  note "On the server: itcmdr-cli agent enroll-token --machine <PC> --address <server>:50052;"
  note "on the PC: itcmdr-agent-Setup-$VER.exe /VERYSILENT /EnrollToken=<token>."
  pause
  local addr=${DEMO_SERVER_ADDR:?set DEMO_SERVER_ADDR}
  if [[ -n ${DEMO_TUNNEL_BIND:-} ]]; then addr=$DEMO_TUNNEL_BIND
    pgrep -f "L $addr:50052:" >/dev/null || ssh -f -N -o BatchMode=yes -o ExitOnForwardFailure=yes \
      -L "$addr:50052:127.0.0.1:50052" -L "$addr:50051:127.0.0.1:50051" -p "$SRV_SSH" "$(dest "$SRV_SSH")"
    note "forwarding $addr:50051/50052 to the server VM"; fi
  local i; for i in "${!CL_SSH[@]}"; do
    local port=${CL_SSH[$i]} name=${CL_NAME[$i]} tok
    tok=$(vm "$SRV_SSH" "$CLI agent enroll-token --machine $name --address $addr:50052" | grep -o 'itcmdr-enroll:[^[:space:]]*' | head -1)
    [[ -n $tok ]] || { echo "no enrollment token for $name" >&2; return 1; }
    vm "$port" "New-Item -ItemType Directory -Force C:\\itcmdr-demo | Out-Null
      if ('$(old_crt)' -eq 'yes' -and -not (Test-Path C:\\Windows\\System32\\vcruntime140.dll)) { Invoke-WebRequest https://aka.ms/vs/17/release/vc_redist.x64.exe -OutFile C:\\itcmdr-demo\\vc_redist.x64.exe -UseBasicParsing
        \$v = Start-Process C:\\itcmdr-demo\\vc_redist.x64.exe -ArgumentList '/install','/quiet','/norestart' -Wait -PassThru; '$name VC++ runtime install exit ' + \$v.ExitCode }"
    vmcopy "$port" "$REL/itcmdr-agent-Setup-$VER.exe" "C:/itcmdr-demo/"
    vm "$port" "\$r = Start-Process C:\\itcmdr-demo\\itcmdr-agent-Setup-$VER.exe -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/EnrollToken=$tok' -Wait -PassThru; '$name agent setup exit ' + \$r.ExitCode"
  done
  note "Upgrade rings: ${CL_NAME[0]} ring 0 (test PC), ${CL_NAME[1]:-} ring 1, the server's own agent stays ring 2."
  vm "$SRV_SSH" "$CLI config set agent.ring 0 --machine ${CL_NAME[0]}; $CLI config set agent.ring 1 --machine ${CL_NAME[1]:-${CL_NAME[0]}}" | tail -2
  note "waiting for the agents to enroll (they retry every minute)"
  local t; for t in $(seq 1 18); do
    (( $(vm "$SRV_SSH" "$CLI agent list" | grep -c ' online ') > ${#CL_SSH[@]} )) && break; sleep 10; done
  vm "$SRV_SSH" "$CLI agent list"; }

s7() { say "7. Open the apps on this box"
  note "Admin app: $REL/itcmdr-admin-app-$VER-linux-x86_64/bundle (enroll it with a code from: itcmdr-admin user create <email> --name <name>)."
  note "Office app: $REL/itcmdr-app-$VER-linux-x86_64/bundle (server address ${DEMO_SERVER_ADDR:-<server>}:50051; pairing: docs/SITE-APP.md)."
  pause
  "$ADM" user create "app@example.com" --name "Demo App" 2>&1 | grep -v -i token | sed 's/^Error: a user with this email exists/(app@example.com already exists: itcmdr-admin user list)/' | head -3 || true
  note "Admin app: Servers shows Demo Office; Servers -> Deep backup checks; Alerts; Plugins; Users."; }

s8() { say "8. Fleet view and health history"
  note "Every PC reports to the office server; adminsvr sees only counts and opaque ids."
  pause
  vm "$SRV_SSH" "$CLI agent list"
  "$ADM" site list
  "$ADM" site metrics "$(site_id)" 2>&1 | tail -5; }

s9() { say "9. Real break on a client PC: kill its agent, watch it come back"
  note "Stop-Process -Force on itcmdr-agent of ${CL_NAME[0]}; the service recovery and the watchdog restart it."
  pause
  vm "${CL_SSH[0]}" "\$p = (Get-CimInstance Win32_Service -Filter \"Name='itcmdr-agent'\").ProcessId; Stop-Process -Id \$p -Force; 'killed pid ' + \$p
    for (\$i = 0; \$i -lt 300; \$i++) { Start-Sleep 1; \$n = (Get-CimInstance Win32_Service -Filter \"Name='itcmdr-agent'\").ProcessId; if (\$n -and \$n -ne \$p) { 'back as pid ' + \$n + ' after ' + \$i + ' s'; break } }"
  note "Server side: the server killed too (its watchdog brings it back)."
  vm "$SRV_SSH" "\$p = (Get-CimInstance Win32_Service -Filter \"Name='itcmdr-server'\").ProcessId; Stop-Process -Id \$p -Force; 'killed itcmdr-server pid ' + \$p
    for (\$i = 0; \$i -lt 360; \$i++) { Start-Sleep 1; \$s = Get-Service itcmdr-server; \$n = (Get-CimInstance Win32_Service -Filter \"Name='itcmdr-server'\").ProcessId; if (\$s.Status -eq 'Running' -and \$n -and \$n -ne \$p) { 'itcmdr-server back after ' + \$i + ' s'; break } }"
  sleep 20; vm "$SRV_SSH" "$CLI agent list"; }

s10() { say "10. Alerts"
  note "Stop a client's agent (a real outage): it goes late, then non-responsive with a critical alert; back online resolves it."
  pause
  local pc2=${CL_NAME[1]:-${CL_NAME[0]}}
  note "For the demo $pc2 beats every 15 s (default 60 s), so 5 missed beats take about 75 s."
  vm "$SRV_SSH" "$CLI config set agent.heartbeatSecs 15 --machine $pc2" | tail -1; sleep 20
  vm "${CL_SSH[1]:-${CL_SSH[0]}}" "Stop-Service itcmdr-agent-watchdog; Stop-Service itcmdr-agent; 'agent stopped on ' + \$env:COMPUTERNAME"
  note "waiting for the heartbeat to go late, then non-responsive (about 2 minutes)"
  sleep 120
  vm "$SRV_SSH" "$CLI agent list"
  vm "$SRV_SSH" "$CLI alerts list" | head -8
  "$ADM" alert list | head -5
  vm "${CL_SSH[1]:-${CL_SSH[0]}}" "Start-Service itcmdr-agent; Start-Service itcmdr-agent-watchdog; 'agent started'"
  sleep 45; vm "$SRV_SSH" "$CLI alerts list" | head -5
  vm "$SRV_SSH" "$CLI config unset agent.heartbeatSecs --machine $pc2" | tail -1; }

s11() { say "11. Backups: on the server, and what adminsvr sees"
  note "itcmdr-cli backup create / list / verify / drill on the server; itcmdr-admin backup start|drill only queue them."
  pause
  vm "$SRV_SSH" "$CLI backup create; $CLI backup list | Select-Object -First 3; $CLI backup drill" | tail -6
  "$ADM" backup start "$(site_id)"; sleep 30
  "$ADM" backup drill "$(site_id)"; sleep 30
  "$ADM" command list "$(site_id)" | head -4
  "$ADM" site list; }

s12() { say "12. Deep backup check (the monthly drill) and the Deep backup checks screen"
  note "itcmdr-cli backup drill --deep decrypts with the 24 words on the server; adminsvr gets only the date and ok/failed."
  pause
  if [[ ${DEMO_INTERACTIVE:-} == 1 ]]; then
    ssh -t -p "$SRV_SSH" "$SSH_HOST" "\"C:\\Program Files\\itcmdr\\server\\itcmdr-cli.exe\" backup drill --deep"
  else
    vm "$SRV_SSH" "\$w = (Get-Content C:\\itcmdr-demo\\words.txt | ForEach-Object { if (\$_ -match '^\\s*\\d+[.):]?\\s+([a-z]+)\\s*\$') { \$Matches[1] } }) -join ' '
      if ((\$w -split ' ').Count -ne 24) { 'no demo words on this server (run it in person: backup drill --deep)' } else { \$w | $CLI backup drill --deep 2>&1 | Select-String 'OK|FAIL|rror' }"
  fi
  sleep 70; "$ADM" site list
  note "Admin app: Servers -> Deep backup checks lists it (ok, today)."; }

s13() { say "13. Updates: window, rings and rollouts"
  note "itcmdr-cli upgrade status; a rollout walks ring 0 -> 1 -> 2 (docs/UPGRADES.md). It needs an owner-signed"
  note "itcmdr-agent-release package of a newer version (itcmdr-cli plugins pack), not part of the release files."
  pause
  vm "$SRV_SSH" "$CLI config get updates.window; $CLI config get updates.auto; $CLI upgrade status" | tail -8
  "$ADM" release list 2>&1 | head -5; }

s14() { say "14. Setup check and installer logs"
  note "setup check on the server; every installer run is logged in %ProgramData%\\itcmdr\\install-logs (newest 20 per part)."
  pause
  vm "$SRV_SSH" "$CLI setup check" | tail -12
  vm "$SRV_SSH" "Get-ChildItem C:\\ProgramData\\itcmdr\\install-logs | Sort-Object LastWriteTime | Select-Object -Last 6 Name | Out-String -Width 100"
  vm "${CL_SSH[0]}" "Get-Content (Get-ChildItem C:\\ProgramData\\itcmdr\\install-logs -Filter 'agent-*.log' | Sort-Object LastWriteTime | Select-Object -Last 1).FullName -TotalCount 8"; }

NAMES_FILE=${DEMO_NAMES_FILE:-$HOME/.cache/itcmdr-demo-names}
# Rename one VM and restart it; wait until SSH answers again and the itcmdr services run.
rename_one() { local port=$1 new=$2 i
  [[ $(vm "$port" '$env:COMPUTERNAME' | tr -d '\r' | tail -1) == "$new" ]] && { note "port $port already $new"; return; }
  vm "$port" "Rename-Computer -NewName '$new' -Force -WarningAction SilentlyContinue; Restart-Computer -Force" >/dev/null || true
  note "port $port: renamed to $new, restarting"; sleep 30
  for i in $(seq 1 60); do [[ $(vm "$port" '$env:COMPUTERNAME' 2>/dev/null | tr -d '\r' | tail -1) == "$new" ]] && break; sleep 10; done
  vm "$port" "'now ' + \$env:COMPUTERNAME; Get-Service itcmdr-* -ErrorAction SilentlyContinue | ForEach-Object { \$_.Name + ' ' + \$_.Status }"; }
rename_vms() { say "Demo computer names"
  local -a ports=("$SRV_SSH" "${CL_SSH[@]}") names; read -r -a names <<<"${DEMO_RENAME:-OFFICE-SRV FRONT-DESK EXAM-ROOM}"
  local i; if [[ ! -s $NAMES_FILE ]]; then (umask 077; for i in "${!ports[@]}"; do
    echo "${ports[$i]} $(vm "${ports[$i]}" '$env:COMPUTERNAME' | tr -d '\r' | tail -1)"; done > "$NAMES_FILE"); fi
  note "original names kept in $NAMES_FILE (not in the repo)"
  for i in "${!ports[@]}"; do [[ -n ${names[$i]:-} ]] && rename_one "${ports[$i]}" "${names[$i]}"; done; }
restore_names() { [[ -s $NAMES_FILE ]] || return 0; say "Original computer names"
  local port name; while read -r port name; do [[ -n $name ]] && rename_one "$port" "$name"; done < "$NAMES_FILE"
  rm -f "$NAMES_FILE"; }

# Before 1.3.2 the Windows binaries needed the VC++ runtime, which a fresh Windows does not have.
old_crt() { [[ $(printf '%s\n' "$VER" 1.3.2 | sort -V | head -1) == 1.3.2 ]] && echo no || echo yes; }

client_names() { [[ -n ${DEMO_CLIENT_NAMES:-} ]] && return; local i
  for i in "${!CL_SSH[@]}"; do CL_NAME[$i]=$(vm "${CL_SSH[$i]}" '$env:COMPUTERNAME' | tr -d '\r' | tail -1); done; }

STEPS=(s1 s2 s3 s4 s5 s6 s7 s8 s9 s10 s11 s12 s13 s14)

provision_vms() { say "Provision the two client VMs (dockur/windows, Windows 10 Pro without activation, 2 cores, 3 GB, 40 GB)"
  local C=${DEMO_COMPOSE_DIR:?set DEMO_COMPOSE_DIR}; local R=${DEMO_CLIENT_ROOT:-$C-demo}
  mkdir -p "$R/oem"; chmod 700 "$R"
  [[ -f $R/.env-demo ]] || (umask 077; echo "PASSWORD=$(head -c 18 /dev/urandom | base64 | tr -d '/+=')" > "$R/.env-demo")
  cp "${DEMO_SSH_PUBKEY:-$HOME/.ssh/id_ed25519.pub}" "$R/oem/host.pub"
  cat > "$R/oem/install.bat" <<'BAT'
@echo off
rem itcmdr demo client first boot: OpenSSH server, key login for the host's key.
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-WindowsCapability -Online -Name 'OpenSSH.Server*' | Add-WindowsCapability -Online; Set-Service sshd -StartupType Automatic; Start-Service sshd; New-NetFirewallRule -Name OpenSSH-Server-In-TCP -DisplayName 'OpenSSH Server (sshd)' -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22 -ErrorAction SilentlyContinue; Copy-Item C:\OEM\host.pub C:\ProgramData\ssh\administrators_authorized_keys -Force; icacls C:\ProgramData\ssh\administrators_authorized_keys /inheritance:r /grant 'Administrators:F' /grant 'SYSTEM:F'"
BAT
  # The clients live in an override file next to their disks; the main compose file is never edited.
  local O="$R/docker-compose.demo.yml" n
  { echo "# itcmdr demo client VMs, layered on $C/docker-compose.yml with -f; removed by: scripts/demo.sh teardown --vms"
    echo "services:"
    for n in 1 2; do mkdir -p "$R/client$n/storage"
      # reuse the Windows ISO a past teardown kept (dockur skips the download when it is in /storage)
      local iso; for iso in "$ISO_CACHE"/*.iso; do [[ -f $iso ]] && cp -n "$iso" "$R/client$n/storage/"; done
      cat <<YML
  windows-client$n:
    # itcmdr demo client $n (agent only)
    image: dockurr/windows
    container_name: itcmdr-demo-client$n
    environment:
      VERSION: "10"
      RAM_SIZE: "3G"
      CPU_CORES: "2"
      DISK_SIZE: "40G"
      USERNAME: "demo"
    env_file: $R/.env-demo
    devices:
      - /dev/kvm
      - /dev/net/tun
    cap_add:
      - NET_ADMIN
    ports:
      - 127.0.0.1:$((8006+n)):8006
      - 127.0.0.1:$((3389+n)):3389/tcp
      - 127.0.0.1:$((2222+n)):22
    volumes:
      - $R/client$n/storage:/storage
      - $R/oem:/oem
    restart: "no"
    stop_grace_period: 2m
YML
    done; } > "$O"
  dc() { (cd "$C" && docker compose -f docker-compose.yml -f "$O" "$@"); }
  dc up -d --no-deps windows-client1 windows-client2
  note "Windows installs itself (30-60 min). Watch: http://127.0.0.1:8007 and :8008. Then add their SSH host keys:"
  note "ssh -p 2223 demo@$SSH_HOST (and 2224) once, check and accept the fingerprints shown in the VMs."; }

teardown() { say "Teardown"
  local sid; sid=$(site_id)
  for i in "${!CL_SSH[@]}"; do
    vm "${CL_SSH[$i]}" "if (Test-Path 'C:\\Program Files\\itcmdr\\uninstall\\agent\\unins000.exe') { Start-Process 'C:\\Program Files\\itcmdr\\uninstall\\agent\\unins000.exe' -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -Wait }; Remove-Item -Recurse -Force C:\\itcmdr-demo -ErrorAction SilentlyContinue; 'agent removed from ' + \$env:COMPUTERNAME"
    vm "$SRV_SSH" "$CLI agent revoke ${CL_NAME[$i]}" | tail -1
  done
  vm "$SRV_SSH" "$CLI config unset agent.ring --machine ${CL_NAME[0]}; $CLI config unset agent.ring --machine ${CL_NAME[1]:-${CL_NAME[0]}}" | tail -1
  [[ -n $sid ]] && { vm "$SRV_SSH" "$CLI adminsvr unlink" | tail -1; "$ADM" site revoke "$sid" 2>/dev/null | tail -1 || true; }
  vm "$SRV_SSH" "Remove-Item -Recurse -Force C:\\itcmdr-demo -ErrorAction SilentlyContinue; 'demo files removed from the server VM'"
  pkill -f "$ASV serve" 2>/dev/null || true
  [[ -n ${DEMO_TUNNEL_BIND:-} ]] && { pkill -f "L $DEMO_TUNNEL_BIND:50052:" 2>/dev/null || true; }
  rm -rf "$D"; note "removed $D (adminsvr data, demo identity, release files)"
  restore_names
  if [[ ${TD_VMS:-} == 1 ]]; then
    local C=${DEMO_COMPOSE_DIR:?set DEMO_COMPOSE_DIR}; local R=${DEMO_CLIENT_ROOT:-$C-demo}
    if [[ -f $R/docker-compose.demo.yml ]]; then
      (cd "$C" && docker compose -f docker-compose.yml -f "$R/docker-compose.demo.yml" rm -sf windows-client1 windows-client2)
    fi
    local iso; iso=$(ls "$R"/client*/storage/*.iso 2>/dev/null | head -1)
    [[ -n $iso ]] && { mkdir -p "$ISO_CACHE"; mv -n "$iso" "$ISO_CACHE/"; note "kept the Windows ISO in $ISO_CACHE"; }
    docker rm -f itcmdr-demo-client1 itcmdr-demo-client2 >/dev/null 2>&1 || true
    rm -rf "$R"; note "client VMs, their disks and $R removed (dockur's Windows ISO cache, if any, is kept)"
    (cd "$C" && docker compose ps --format '{{.Name}}  {{.Status}}')
  fi
  note "The office server VM keeps itcmdr installed and initialized (its backups stay in its backup folder)."; }

case $CMD in
  list) for i in "${!STEPS[@]}"; do grep -m1 -o "say \"$((i+1))\.[^\"]*" "$0" | sed 's/say "//'; done ;;
  provision-vms) provision_vms ;;
  rename-vms) rename_vms ;;
  restore-names) restore_names ;;
  teardown) client_names; teardown ;;
  run) if (( ${ONLY:-$START} >= 6 )); then client_names; fi
       if [[ -n $ONLY ]]; then "${STEPS[$((ONLY - 1))]}"; else
         for ((i = START; i <= ${#STEPS[@]}; i++)); do echo "$i" > "$D.step" 2>/dev/null || true; "${STEPS[$((i-1))]}"; done
         say "Demo done. Teardown: scripts/demo.sh teardown [--vms]"; fi ;;
esac
