#!/usr/bin/env bash
# Stage jaspr build output as the flat GitHub Pages layout (*.html at root).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD="$ROOT/build/jaspr"
OUT="${1:-$ROOT/build/site-out}"
rm -rf "$OUT"
mkdir -p "$OUT"
for f in index.html demos.html download.html docs.html security.html 404.html CNAME .nojekyll sitemap.xml main.client.dart.js; do
  [[ -e "$BUILD/$f" ]] && cp -a "$BUILD/$f" "$OUT/$f"
done
# Flatten if jaspr emitted page/index.html instead
for page in demos.html download.html docs.html security.html 404.html index.html; do
  if [[ -f "$OUT/$page/index.html" ]]; then
    mv "$OUT/$page/index.html" "$OUT/$page.tmp"
    rm -rf "$OUT/$page"
    mv "$OUT/$page.tmp" "$OUT/$page"
  fi
done
rsync -a "$BUILD/assets/" "$OUT/assets/"
rsync -a "$BUILD/demos/" "$OUT/demos/"
cp -f "$ROOT/web/CNAME" "$OUT/CNAME"
cp -f "$ROOT/web/.nojekyll" "$OUT/.nojekyll"
echo "staged flat site at $OUT"
ls "$OUT"/*.html
