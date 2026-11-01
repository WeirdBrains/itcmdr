#!/bin/sh
# Remove the itcmdr-adminsvr (v0.19.2) LaunchDaemon and binary. Keeps the data
# (/usr/local/var/itcmdr-adminsvr) and settings unless --purge is given.
set -eu
[ "$(id -u)" = 0 ] || { echo "run with sudo" >&2; exit 1; }
LABEL=com.weirdbrains.itcmdr-adminsvr
launchctl bootout "system/$LABEL" 2>/dev/null || true
rm -f "/Library/LaunchDaemons/$LABEL.plist" /usr/local/bin/itcmdr-adminsvr
if [ "${1:-}" = --purge ]; then
	rm -rf /usr/local/var/itcmdr-adminsvr /usr/local/etc/itcmdr-adminsvr /usr/local/var/log/itcmdr-adminsvr.log
	dscl . -delete /Users/_itcmdradminsvr 2>/dev/null || true
	dscl . -delete /Groups/_itcmdradminsvr 2>/dev/null || true
	echo "Purged."
else
	echo "Removed. Kept /usr/local/var/itcmdr-adminsvr and /usr/local/etc/itcmdr-adminsvr (use --purge to delete)."
fi
