#!/bin/sh
# Fetch the itcmdr-plugins release pinned in plugins.pin.json into third_party/itcmdr-plugins and
# verify that the tag points to the pinned commit. The installer, Push-Install-ItcmdrAgent.ps1 and the
# parity tests use that copy. ITCMDR_PLUGINS_REPO overrides the clone URL (e.g. a local checkout).
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
pin="$root/plugins.pin.json"
get() { sed -n "s/.*\"$1\": *\"\([^\"]*\)\".*/\1/p" "$pin"; }
repo=${ITCMDR_PLUGINS_REPO:-$(get repo)}; tag=$(get tag); commit=$(get commit)
dest="$root/third_party/itcmdr-plugins"
if [ -d "$dest/.git" ] && [ "$(git -C "$dest" rev-parse HEAD)" = "$commit" ]; then
  echo "itcmdr-plugins $tag ($commit) already in third_party"; exit 0
fi
rm -rf "$dest"; mkdir -p "$root/third_party"
git -c advice.detachedHead=false clone -q --depth 1 --branch "$tag" "$repo" "$dest"
got=$(git -C "$dest" rev-parse HEAD)
if [ "$got" != "$commit" ]; then
  rm -rf "$dest"; echo "ERROR: $tag is $got, plugins.pin.json pins $commit" >&2; exit 1
fi
echo "itcmdr-plugins $tag ($commit) -> third_party/itcmdr-plugins"
