#!/usr/bin/env bash
# Local release builds (v0.15.0), used instead of .github/workflows/release.yml while Actions is
# unavailable. Uses no Actions minutes. Steps, run from the repo root at the tagged commit:
#   scripts/release-local.sh unix            # Linux or macOS host: CLI, admin CLI, adminsvr tarballs + app and admin app archives into dist/<ver>/
#   scripts/release-local.sh android         # (v0.19.2) admin app APK into dist/<ver>/ (Android SDK + NDK; debug-key signed until a release key exists)
#   scripts/release-local.sh ios-check       # (v0.19.2) admin app iOS build without code signing (no artifact: needs an Apple team to sign)
#   scripts/release-local.sh windows-notes   # print the Windows VM step (see docs/RELEASE-LOCAL.md)
#   scripts/release-local.sh check           # SHA256SUMS + leak check of every artifact (unpacked)
#   scripts/release-local.sh publish         # gh release create vX.Y.Z (tag must exist on origin; never moves it)
# Environment: ITCMDR_LEAK_PATTERNS=file with one extra fixed string per line (e.g. the VM hostname);
# config/banned.local.txt is always used when present. NINJA_DIR is prepended to PATH for Linux builds.
# (v0.20.0) No CocoaPods: the Rust bridges build through flutter_rust_bridge native assets (hook/build.dart,
# rust/rust-toolchain.toml via rustup) and iOS/macOS use Swift Package Manager (once per user:
# `flutter config --enable-swift-package-manager`). no_pods stops a build that would call `pod`.
set -euo pipefail
cd "$(dirname "$0")/.."
ver=$(sed -n 's/^version = "\(.*\)"/\1/p' server/Cargo.toml | head -1)
out="dist/$ver"
step="${1:-}"
no_pods() {
  [[ "$(uname)" == Darwin ]] || return 0
  local p; p=$(git ls-files '*Podfile' '*Podfile.lock'; ls -d app/macos/Pods admin-app/ios/Pods admin-app/macos/Pods 2>/dev/null || true)
  [[ -z "$p" ]] || { echo "CocoaPods files present (use Swift Package Manager): $p" >&2; exit 1; }
  flutter config 2>/dev/null | grep -q "enable-swift-package-manager: true" || flutter config --enable-swift-package-manager >/dev/null
}
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
  no_pods
  cd app; flutter pub get --enforce-lockfile >/dev/null
  if [[ "$(uname)" == Linux ]]; then
    flutter build linux --release
    tar -C build/linux/x64/release -czf "../$out/itcmdr-app-$ver-linux-$arch.tar.gz" bundle
  else
    flutter build macos --release
    (cd build/macos/Build/Products/Release && zip -qry "$OLDPWD/../$out/itcmdr-app-$ver-macos-$arch.zip" ./*.app)
  fi
  cd ../admin-app; flutter pub get --enforce-lockfile >/dev/null
  if [[ "$(uname)" == Linux ]]; then
    flutter build linux --release
    rm -rf build/itcmdr-admin-app && cp -r build/linux/x64/release/bundle build/itcmdr-admin-app
    tar -C build -czf "../$out/itcmdr-admin-app-$ver-linux-$arch.tar.gz" itcmdr-admin-app
  else
    flutter build macos --release
    (cd build/macos/Build/Products/Release && zip -qry "$OLDPWD/../$out/itcmdr-admin-app-$ver-macos-$arch.zip" ./*.app)
  fi
  ls -l "../$out" ;;
android)
  mkdir -p "$out"; cd admin-app; flutter pub get --enforce-lockfile >/dev/null
  flutter build apk --release
  cp build/app/outputs/flutter-apk/app-release.apk "../$out/itcmdr-admin-app-$ver-android.apk"; ls -l "../$out" ;;
ios-check)
  no_pods; cd admin-app; flutter pub get --enforce-lockfile >/dev/null
  flutter build ios --release --no-codesign ;;
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
