#!/bin/sh
# Remove itcmdr-adminsvr (v0.19.0). Keeps /var/lib/itcmdr-adminsvr (root key, TLS), the settings
# and the Postgres database unless --purge is given (the database itself is never dropped).
set -eu
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 1; }
systemctl disable --now itcmdr-adminsvr 2>/dev/null || true
rm -f /etc/systemd/system/itcmdr-adminsvr.service /usr/local/bin/itcmdr-adminsvr
systemctl daemon-reload
if [ "${1:-}" = --purge ]; then
  rm -rf /var/lib/itcmdr-adminsvr /etc/itcmdr-adminsvr
  userdel itcmdr-adminsvr 2>/dev/null || true
  echo "Purged files; drop the Postgres database yourself if wanted."
else
  echo "Removed. Kept /var/lib/itcmdr-adminsvr and /etc/itcmdr-adminsvr (use --purge to delete)."
fi
