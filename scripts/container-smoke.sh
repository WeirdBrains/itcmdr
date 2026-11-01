#!/usr/bin/env bash
# v0.19.2 smoke test of the adminsvr container image with rootless Podman (no sudo):
#   scripts/container-smoke.sh [image]   (default itcmdr-adminsvr:test; build it first with
#   podman build -f packaging/adminsvr/Containerfile -t itcmdr-adminsvr:test .)
# Runs the image on loopback ports with SQLite in a volume, creates the owner, enrolls the admin
# CLI from target/ (temporary config dir), creates a site, checks the audit chain, restarts the
# container and checks the data survived; then the SysV init script inside the image
# (install.sh --init sysv, start/status/restart/stop). Cleans up its container and volume.
set -euo pipefail
cd "$(dirname "$0")/.."
img=${1:-itcmdr-adminsvr:test}
n=adm-smoke-$$
P=$((20000 + RANDOM % 20000)); S=$((P + 1))
T=$(mktemp -d)
cleanup() { podman rm -f "$n" "$n-sysv" >/dev/null 2>&1 || true; podman volume rm -f "$n" >/dev/null 2>&1 || true; rm -rf "$T"; }
trap cleanup EXIT
cargo build -q -p itcmdr-admin-cli
A="$PWD/target/debug/itcmdr-admin"
adm() { XDG_CONFIG_HOME="$T" HOME="$T" "$A" "$@"; }
podman run -d --name "$n" -p "127.0.0.1:$P:8444" -p "127.0.0.1:$S:8443" -v "$n:/var/lib/itcmdr-adminsvr" \
  -e "ITCMDR_ADMINSVR_PUBLIC_API=127.0.0.1:$P" -e "ITCMDR_ADMINSVR_PUBLIC_SITE=127.0.0.1:$S" "$img" >/dev/null
for _ in $(seq 30); do podman logs "$n" 2>&1 | grep -q 'admin API listening' && break; sleep 1; done
podman logs "$n" 2>&1 | grep -q 'admin API listening' || { podman logs "$n"; echo "FAIL: not listening"; exit 1; }
echo "ok: running ($(podman exec "$n" id -un), $(podman exec "$n" itcmdr-adminsvr --version))"
code=$(podman exec "$n" itcmdr-adminsvr init --email owner@example.com --name "Owner O" | tail -1)
adm enroll --no-key-backup --words-file "$T/words" "$code" >/dev/null
adm whoami | grep -q owner && echo "ok: CLI enrolled as owner"
adm site create "Smoke Site" >/dev/null
[[ $(adm site list | wc -l) == 1 ]] && echo "ok: site created"
adm audit verify | grep -qi ok && echo "ok: audit chain"
adm push test | grep -q 'providers=$' && echo "ok: push off without config"
[[ $(podman exec "$n" stat -c %a /var/lib/itcmdr-adminsvr) == 700 ]] && echo "ok: data dir 0700"
[[ $(podman exec "$n" stat -c %a /var/lib/itcmdr-adminsvr/adminsvr.db) == 600 ]] && echo "ok: adminsvr.db 0600"
t0=$(date +%s); podman restart -t 15 "$n" >/dev/null
(( $(date +%s) - t0 < 12 )) && echo "ok: container stops on SIGTERM"
for _ in $(seq 30); do adm site list >/dev/null 2>&1 && break; sleep 1; done
adm site list | grep -q "Smoke Site" && echo "ok: data kept across restart"
# SysV init script inside the image (as root in the container; nothing on the host).
podman run -d --init --name "$n-sysv" --user 0 --entrypoint sleep -v "$PWD/packaging/adminsvr:/pkg:ro,Z" "$img" 600 >/dev/null
podman exec "$n-sysv" sh -c 'mkdir /tmp/p && cp /pkg/* /tmp/p/ 2>/dev/null; cp /usr/local/bin/itcmdr-adminsvr /tmp/p/ && /tmp/p/install.sh --init sysv' | tail -1
podman exec "$n-sysv" sh -c 'printf "ITCMDR_ADMINSVR_API_ADDR=127.0.0.1:18444\nITCMDR_ADMINSVR_SITE_ADDR=127.0.0.1:18443\n" >> /etc/itcmdr-adminsvr/adminsvr.env'
podman exec "$n-sysv" /etc/init.d/itcmdr-adminsvr start
podman exec "$n-sysv" /etc/init.d/itcmdr-adminsvr status
podman exec "$n-sysv" sh -c 'sleep 2; ps -o user= -p "$(cat /run/itcmdr-adminsvr.pid)" 2>/dev/null || stat -c %U /proc/"$(cat /run/itcmdr-adminsvr.pid)"' | grep -q itcmdr-adminsvr && echo "ok: SysV runs as itcmdr-adminsvr"
podman exec "$n-sysv" sh -c 'grep -q "admin API listening addr=127.0.0.1:18444" /var/log/itcmdr-adminsvr.log' && echo "ok: SysV log (settings from the env file)" || { podman exec "$n-sysv" tail -5 /var/log/itcmdr-adminsvr.log; echo "FAIL: SysV log"; exit 1; }
podman exec "$n-sysv" /etc/init.d/itcmdr-adminsvr restart
t0=$(date +%s); podman exec "$n-sysv" /etc/init.d/itcmdr-adminsvr stop
(( $(date +%s) - t0 < 10 )) && echo "ok: SysV stop on SIGTERM"
rc=0; podman exec "$n-sysv" /etc/init.d/itcmdr-adminsvr status || rc=$?
[[ $rc == 3 ]] && echo "ok: SysV stopped (status 3)"
podman exec "$n-sysv" sh -c '/tmp/p/uninstall.sh' | tail -1
echo "SMOKE-OK"
