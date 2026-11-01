#!/usr/bin/env bash
# Admin packages for Linux or macOS (Windows: installer/itcmdr-admin.iss, itcmdr-adminsvr.iss):
#   scripts/package-admin.sh
# Output in dist/: itcmdr-admin-<ver>-<os>-<arch>.tar.gz (the admin CLI) and
# itcmdr-adminsvr-<ver>-<os>-<arch>.tar.gz: on Linux the binary, systemd unit, SysV and OpenRC
# init scripts, install.sh/uninstall.sh, settings example, Containerfile and compose file; on
# macOS (v0.19.2) the binary, the LaunchDaemon plist and install-macos.sh/uninstall-macos.sh.
# Each has a .sha256 next to it.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' adminsvr/Cargo.toml | head -1)
triple=$(rustc -vV | sed -n 's/^host: //p')
case "$triple" in *linux*) os=linux ;; *apple-darwin*) os=macos ;; *) echo "unsupported host $triple" >&2; exit 2 ;; esac
arch=${triple%%-*}
pkgs=(-p itcmdr-admin-cli -p itcmdr-adminsvr)
cargo build --release --locked "${pkgs[@]}"
stage=$(mktemp -d); trap 'rm -rf "$stage"' EXIT
mkdir -p dist
sum() { (cd dist && { sha256sum "$1" 2>/dev/null || shasum -a 256 "$1"; } > "$1.sha256"); echo "dist/$1"; }
n="itcmdr-admin-$version-$os-$arch"; mkdir -p "$stage/$n"
cp target/release/itcmdr-admin "$stage/$n/"; strip "$stage/$n/itcmdr-admin" 2>/dev/null || true
cat > "$stage/$n/README.txt" <<TXT
itcmdr-admin $version ($os $arch): the admin CLI for itcmdr-adminsvr.
Install: sudo install -m 0755 itcmdr-admin /usr/local/bin/
Start:   itcmdr-admin enroll <code from your owner/admin>   (shows 24 words once; keep them offline)
Uninstall: delete the binary; your identity is in ~/.config/itcmdr-admin (delete it to forget this device).
Docs: https://github.com/WeirdBrains/itcmdr/blob/main/docs/ADMINSVR.md
TXT
tar -C "$stage" -czf "dist/$n.tar.gz" "$n"; sum "$n.tar.gz"
n="itcmdr-adminsvr-$version-$os-$arch"; mkdir -p "$stage/$n"
cp target/release/itcmdr-adminsvr "$stage/$n/"; strip "$stage/$n/itcmdr-adminsvr" 2>/dev/null || true
cp packaging/adminsvr/adminsvr.env.example "$stage/$n/"
if [[ $os == linux ]]; then
  cp packaging/adminsvr/{install.sh,uninstall.sh,itcmdr-adminsvr.service,itcmdr-adminsvr.init,itcmdr-adminsvr.openrc,Containerfile,compose.yaml} "$stage/$n/"
  cat > "$stage/$n/README.txt" <<TXT
itcmdr-adminsvr $version (linux $arch): the offsite admin server. Uses SQLite in its data folder by default, or Postgres 14+ via ITCMDR_ADMINSVR_DB_URL.
Install:   sudo ./install.sh [--init systemd|openrc|sysv]   (default: what the system runs; then follow what it prints)
Uninstall: sudo ./uninstall.sh [--purge]
Service:   itcmdr-adminsvr.service (systemd), itcmdr-adminsvr.init (SysV, /etc/init.d), itcmdr-adminsvr.openrc
Container: Containerfile and compose.yaml (rootless Podman; see the comments at their top).
Docs: https://github.com/WeirdBrains/itcmdr/blob/main/docs/ADMINSVR.md
TXT
else
  sed 's#^ITCMDR_ADMINSVR_DATA_DIR=.*#ITCMDR_ADMINSVR_DATA_DIR=/usr/local/var/itcmdr-adminsvr#; s#Copy to /etc/itcmdr-adminsvr/adminsvr.env#macOS: /usr/local/etc/itcmdr-adminsvr/adminsvr.env#' packaging/adminsvr/adminsvr.env.example > "$stage/$n/adminsvr.env.example"
  cp packaging/adminsvr/macos/* "$stage/$n/"
  cat > "$stage/$n/README.txt" <<TXT
itcmdr-adminsvr $version (macos $arch): the offsite admin server as a LaunchDaemon. SQLite by default, or Postgres 14+ via ITCMDR_ADMINSVR_DB_URL.
Install:   sudo ./install-macos.sh   (then follow what it prints; unsigned: if macOS blocks the binary, run xattr -d com.apple.quarantine itcmdr-adminsvr first)
Uninstall: sudo ./uninstall-macos.sh [--purge]
Docs: https://github.com/WeirdBrains/itcmdr/blob/main/docs/ADMINSVR.md
TXT
fi
tar -C "$stage" -czf "dist/$n.tar.gz" "$n"; sum "$n.tar.gz"
