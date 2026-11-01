#!/bin/sh
# Remove itcmdr-adminsvr (v0.19.2): the service (systemd, OpenRC or SysV) and the binary. Keeps
# /var/lib/itcmdr-adminsvr (root key, TLS, the SQLite database) and the settings unless --purge
# is given. A Postgres database is never dropped.
set -eu
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 1; }
if [ -f /etc/systemd/system/itcmdr-adminsvr.service ]; then
	systemctl disable --now itcmdr-adminsvr 2>/dev/null || true
	rm -f /etc/systemd/system/itcmdr-adminsvr.service
	systemctl daemon-reload 2>/dev/null || true
fi
if [ -f /etc/init.d/itcmdr-adminsvr ]; then
	if command -v rc-service >/dev/null 2>&1 && head -1 /etc/init.d/itcmdr-adminsvr | grep -q openrc-run; then
		rc-service itcmdr-adminsvr stop 2>/dev/null || true
		rc-update del itcmdr-adminsvr default 2>/dev/null || true
	else
		/etc/init.d/itcmdr-adminsvr stop 2>/dev/null || true
		if command -v update-rc.d >/dev/null 2>&1; then update-rc.d -f itcmdr-adminsvr remove >/dev/null 2>&1 || true
		elif command -v chkconfig >/dev/null 2>&1; then chkconfig --del itcmdr-adminsvr 2>/dev/null || true
		fi
	fi
	rm -f /etc/init.d/itcmdr-adminsvr
fi
rm -f /usr/local/bin/itcmdr-adminsvr /run/itcmdr-adminsvr.pid
if [ "${1:-}" = --purge ]; then
	rm -rf /var/lib/itcmdr-adminsvr /etc/itcmdr-adminsvr /var/log/itcmdr-adminsvr.log
	userdel itcmdr-adminsvr 2>/dev/null || deluser itcmdr-adminsvr 2>/dev/null || true
	echo "Purged files; drop a Postgres database yourself if you used one."
else
	echo "Removed. Kept /var/lib/itcmdr-adminsvr and /etc/itcmdr-adminsvr (use --purge to delete)."
fi
