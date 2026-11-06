#!/usr/bin/env bash
# Copy the published site's static files into web/ for a local jaspr build.
# Prefers ITCMDR_SITE_DIR, then the sibling itcmdr-site worktree, then ../itcmdr-site.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [[ -n "${ITCMDR_SITE_DIR:-}" ]]; then
  SITE="$ITCMDR_SITE_DIR"
elif [[ -d "$HOME/Projects/WeirdBrains/itcmdr-site" ]]; then
  SITE="$HOME/Projects/WeirdBrains/itcmdr-site"
elif [[ -d "$(cd "$ROOT/../itcmdr-site" 2>/dev/null && pwd)" ]]; then
  SITE="$(cd "$ROOT/../itcmdr-site" && pwd)"
else
  echo "set ITCMDR_SITE_DIR to the site-branch checkout (assets+demos)" >&2
  exit 1
fi
mkdir -p "$ROOT/web"
rsync -a "$SITE/assets/" "$ROOT/web/assets/"
rsync -a "$SITE/demos/" "$ROOT/web/demos/"
cp -f "$SITE/CNAME" "$ROOT/web/CNAME"
cp -f "$SITE/.nojekyll" "$ROOT/web/.nojekyll"
echo "synced web assets from $SITE into $ROOT/web"
