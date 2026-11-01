#!/usr/bin/env bash
# Local release builds (v0.15.0), used instead of .github/workflows/release.yml while Actions is
# unavailable. Uses no Actions minutes. Steps, run from the repo root at the tagged commit:
#   scripts/release-local.sh unix            # Linux or macOS host: CLI, admin CLI (+ adminsvr on Linux) tarballs + app archive into dist/<ver>/
#   scripts/release-local.sh windows-notes   # print the Windows VM step (see docs/RELEASE-LOCAL.md)
#   scripts/release-local.sh check           # SHA256SUMS + leak check of every artifact (unpacked)
#   scripts/release-local.sh publish         # gh release create vX.Y.Z (tag must exist on origin; never moves it)
# Environment: ITCMDR_LEAK_PATTERNS=file with one extra fixed string per line (e.g. the VM hostname);
# config/banned.local.txt is always used when present. NINJA_DIR is prepended to PATH for Linux builds.
set -euo pipefail
cd "$(dirname "$0")/.."
ver=$(sed -n 's/^version = "\(.*\)"/\1/p' server/Cargo.toml | head -1)
out="dist/$ver"
step="${1:-}"
case "$step" in
unix)
  [[ -z "$(git status --porcelain --untracked-files=no)" ]] || { echo "tree not clean" >&2; exit 1; }
  [[ -n "${NINJA_DIR:-}" ]] && export PATH="$NINJA_DIR:$PATH"
  mkdir -p "$out"
  scripts/package-cli.sh
  mv dist/itcmdr-cli-"$ver"-*.tar.gz "$out"/; rm -f dist/itcmdr-cli-"$ver"-*.sha256
  scripts/package-admin.sh
  mv dist/itcmdr-admin*-"$ver"-*.tar.gz "$out"/; rm -f dist/itcmdr-admin*-"$ver"-*.sha256
  arch=$(uname -m); [[ "$arch" == arm64 ]] && arch=aarch64
  (cd app/rust && cargo build --release)
  cd app; flutter pub get --enforce-lockfile >/dev/null
  if [[ "$(uname)" == Linux ]]; then
    flutter build linux --release
    tar -C build/linux/x64/release -czf "../$out/itcmdr-app-$ver-linux-$arch.tar.gz" bundle
  else
    flutter build macos --release
    (cd build/macos/Build/Products/Release && zip -qry "$OLDPWD/../$out/itcmdr-app-$ver-macos-$arch.zip" ./*.app)
  fi
  ls -l "../$out" ;;
windows-notes) sed -n '/^## Windows VM step/,/^## /p' docs/RELEASE-LOCAL.md ;;
check)
  cd "$out"; rm -f SHA256SUMS
  pats=$(mktemp); trap 'rm -rf "$pats" "$tmp"' EXIT; tmp=$(mktemp -d)
  for f in ../../config/banned.local.txt "${ITCMDR_LEAK_PATTERNS:-}"; do [[ -n "$f" && -f "$f" ]] && grep -v '^\s*\(#\|$\)' "$f" >> "$pats"; done
  [[ -s "$pats" ]] || { echo "no leak patterns (config/banned.local.txt missing)" >&2; exit 1; }
  hits=0
  for a in *; do
    d="$tmp/$a"; mkdir -p "$d"
    case "$a" in *.tar.gz) tar -xzf "$a" -C "$d" ;; *.zip) unzip -q "$a" -d "$d" ;; *) cp "$a" "$d/" ;; esac
    n=$( (grep -raiF -l -f "$pats" "$d" || true) | wc -l); echo "leak check $a: $n file(s) with hits"; hits=$((hits+n))
  done
  [[ $hits -eq 0 ]] || { echo "LEAK: fix before publishing" >&2; exit 1; }
  sha256sum -- * > SHA256SUMS; cat SHA256SUMS ;;
publish)
  tag="v$ver"
  git ls-remote --exit-code origin "refs/tags/$tag" >/dev/null || { echo "$tag not on origin" >&2; exit 1; }
  [[ -f "$out/SHA256SUMS" ]] || { echo "run check first" >&2; exit 1; }
  notes=$(mktemp); trap 'rm -f "$notes"' EXIT
  { awk -v h="## $tag " 'index($0,h)==1{p=1;next} /^## /{p=0} p' CHANGELOG.md
    echo; echo "Built locally (not by Actions). Verify downloads with SHA256SUMS."
    echo "Windows installers are not Authenticode-signed and the macOS app is not signed or notarized yet."; } > "$notes"
  if gh release view "$tag" >/dev/null 2>&1; then gh release upload "$tag" "$out"/* ; else
    gh release create "$tag" --verify-tag --title "itcmdr $tag" --notes-file "$notes" "$out"/*; fi
  gh release view "$tag" --json url,assets --jq '.url, (.assets[].name)' ;;
*) sed -n '2,10p' "$0"; exit 2 ;;
esac
