#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")"/.. && pwd)"
HOOK="$ROOT_DIR/.git/hooks/pre-commit"
cat >"$HOOK" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
python3 scripts/scan_repo.py
EOF
chmod +x "$HOOK"
echo "Installed pre-commit hook -> $HOOK"

