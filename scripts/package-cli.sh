#!/usr/bin/env bash
# Build the standalone itcmdr-cli tarball (v0.14.3) for Linux or macOS:
#   scripts/package-cli.sh [rust-target]        e.g. x86_64-unknown-linux-gnu, aarch64-apple-darwin
# Output: dist/itcmdr-cli-<version>-<os>-<arch>.tar.gz and its .sha256. The build needs perl and a
# C compiler (SQLCipher with vendored OpenSSL), nothing else.
set -euo pipefail
cd "$(dirname "$0")/.."
target="${1:-}"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' server/Cargo.toml | head -1)
args=(build --release --locked -p itcmdr-server --bin itcmdr-cli)
bin=target/release/itcmdr-cli
if [[ -n "$target" ]]; then
  args+=(--target "$target"); bin="target/$target/release/itcmdr-cli"
  triple="$target"
else
  triple=$(rustc -vV | sed -n 's/^host: //p')
fi
case "$triple" in
  *linux*) os=linux ;; *apple-darwin*) os=macos ;; *) echo "unsupported target $triple (Windows uses installer/itcmdr-cli.iss)" >&2; exit 2 ;;
esac
arch=${triple%%-*}
cargo "${args[@]}"
name="itcmdr-cli-$version-$os-$arch"
stage=$(mktemp -d); trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/$name" dist
cp "$bin" "$stage/$name/itcmdr-cli"
strip "$stage/$name/itcmdr-cli" 2>/dev/null || true
cat > "$stage/$name/README.txt" <<TXT
itcmdr-cli $version ($os $arch)
Install: copy itcmdr-cli to a folder on PATH, e.g.  sudo install -m 0755 itcmdr-cli /usr/local/bin/
Uninstall: delete it. It keeps no data of its own; the server's data dir is never touched.
Docs: https://github.com/WeirdBrains/itcmdr/blob/main/docs/INSTALL.md
TXT
tar -C "$stage" -czf "dist/$name.tar.gz" "$name"
(cd dist && sha256sum "$name.tar.gz" 2>/dev/null > "$name.tar.gz.sha256" || shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
echo "dist/$name.tar.gz"
