#!/bin/sh
# Install itcmdr-adminsvr (v0.19.2) as a service on Linux. Run as root from the unpacked folder.
#   ./install.sh [--init systemd|openrc|sysv]   (default: what this system runs)
# Then edit /etc/itcmdr-adminsvr/adminsvr.env, create the first owner with
#   sudo -u itcmdr-adminsvr itcmdr-adminsvr --env-file /etc/itcmdr-adminsvr/adminsvr.env init --email you@example.com --name "Your Name"
# and start the service (the command is printed at the end).
set -eu
cd "$(dirname "$0")"
init=""
while [ $# -gt 0 ]; do
	case "$1" in
	--init) init=${2:-}; shift 2 ;;
	--init=*) init=${1#--init=}; shift ;;
	*) echo "unknown argument: $1" >&2; exit 2 ;;
	esac
done
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 1; }
if [ -z "$init" ]; then
	if [ -d /run/systemd/system ]; then init=systemd
	elif command -v openrc-run >/dev/null 2>&1 || [ -x /sbin/openrc-run ]; then init=openrc
	else init=sysv
	fi
fi
nologin=/usr/sbin/nologin; [ -x "$nologin" ] || nologin=/sbin/nologin; [ -x "$nologin" ] || nologin=/bin/false
if ! id itcmdr-adminsvr >/dev/null 2>&1; then
	if command -v useradd >/dev/null 2>&1; then
		useradd --system --home /var/lib/itcmdr-adminsvr --shell "$nologin" itcmdr-adminsvr
	else
		addgroup -S itcmdr-adminsvr && adduser -S -D -H -h /var/lib/itcmdr-adminsvr -s "$nologin" -G itcmdr-adminsvr itcmdr-adminsvr
	fi
fi
install -m 0755 itcmdr-adminsvr /usr/local/bin/itcmdr-adminsvr
install -d -m 0700 -o itcmdr-adminsvr -g itcmdr-adminsvr /var/lib/itcmdr-adminsvr
install -d -m 0750 -g itcmdr-adminsvr /etc/itcmdr-adminsvr
[ -f /etc/itcmdr-adminsvr/adminsvr.env ] || install -m 0640 -g itcmdr-adminsvr adminsvr.env.example /etc/itcmdr-adminsvr/adminsvr.env
case "$init" in
systemd)
	install -m 0644 itcmdr-adminsvr.service /etc/systemd/system/itcmdr-adminsvr.service
	systemctl daemon-reload
	go="systemctl enable --now itcmdr-adminsvr"
	;;
openrc)
	install -m 0755 itcmdr-adminsvr.openrc /etc/init.d/itcmdr-adminsvr
	go="rc-update add itcmdr-adminsvr default && rc-service itcmdr-adminsvr start"
	;;
sysv)
	install -m 0755 itcmdr-adminsvr.init /etc/init.d/itcmdr-adminsvr
	if command -v update-rc.d >/dev/null 2>&1; then update-rc.d itcmdr-adminsvr defaults >/dev/null
	elif command -v chkconfig >/dev/null 2>&1; then chkconfig --add itcmdr-adminsvr
	else echo "note: link /etc/init.d/itcmdr-adminsvr into your runlevels yourself"
	fi
	go="/etc/init.d/itcmdr-adminsvr start"
	;;
*) echo "--init: systemd | openrc | sysv" >&2; exit 2 ;;
esac
echo "Installed ($init). Edit /etc/itcmdr-adminsvr/adminsvr.env, run 'itcmdr-adminsvr init' (see the top of install.sh), then: $go"
