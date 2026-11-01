#!/usr/bin/env bash
# v0.19.0 admin packages for Linux or macOS (Windows: installer/itcmdr-admin.iss):
#   scripts/package-admin.sh
# Output in dist/: itcmdr-admin-<ver>-<os>-<arch>.tar.gz (the admin CLI, every OS) and, on Linux,
# itcmdr-adminsvr-<ver>-linux-<arch>.tar.gz (binary, systemd unit, install.sh/uninstall.sh,
# settings example, Containerfile and compose file). Each has a .sha256 next to it.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' adminsvr/Cargo.toml | head -1)
triple=$(rustc -vV | sed -n 's/^host: //p')
case "$triple" in *linux*) os=linux ;; *apple-darwin*) os=macos ;; *) echo "unsupported host $triple" >&2; exit 2 ;; esac
arch=${triple%%-*}
pkgs=(-p itcmdr-admin-cli); [[ $os == linux ]] && pkgs+=(-p itcmdr-adminsvr)
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
if [[ $os == linux ]]; then
  n="itcmdr-adminsvr-$version-$os-$arch"; mkdir -p "$stage/$n"
  cp target/release/itcmdr-adminsvr "$stage/$n/"; strip "$stage/$n/itcmdr-adminsvr" 2>/dev/null || true
  cp packaging/adminsvr/* "$stage/$n/"
  cat > "$stage/$n/README.txt" <<TXT
itcmdr-adminsvr $version (linux $arch): the offsite admin server. Uses SQLite in its data folder by default, or Postgres 14+ via ITCMDR_ADMINSVR_DB_URL.
Install:   sudo ./install.sh   (then follow what it prints)
Uninstall: sudo ./uninstall.sh [--purge]
Container: Containerfile and compose.yaml (untested in this release).
Docs: https://github.com/WeirdBrains/itcmdr/blob/main/docs/ADMINSVR.md
TXT
  tar -C "$stage" -czf "dist/$n.tar.gz" "$n"; sum "$n.tar.gz"
fi
