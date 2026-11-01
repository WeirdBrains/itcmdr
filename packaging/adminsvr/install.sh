#!/bin/sh
# Install itcmdr-adminsvr (v0.19.0) as a systemd service. Run as root from the unpacked folder.
# Then: edit /etc/itcmdr-adminsvr/adminsvr.env, run
#   sudo -u itcmdr-adminsvr sh -c 'set -a; . /etc/itcmdr-adminsvr/adminsvr.env; exec itcmdr-adminsvr init --email you@example.com --name "Your Name"'
# and systemctl enable --now itcmdr-adminsvr.
set -eu
cd "$(dirname "$0")"
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 1; }
id itcmdr-adminsvr >/dev/null 2>&1 || useradd --system --home /var/lib/itcmdr-adminsvr --shell /usr/sbin/nologin itcmdr-adminsvr
install -m 0755 itcmdr-adminsvr /usr/local/bin/itcmdr-adminsvr
install -d -m 0700 -o itcmdr-adminsvr -g itcmdr-adminsvr /var/lib/itcmdr-adminsvr
install -d -m 0750 -g itcmdr-adminsvr /etc/itcmdr-adminsvr
[ -f /etc/itcmdr-adminsvr/adminsvr.env ] || install -m 0640 -g itcmdr-adminsvr adminsvr.env.example /etc/itcmdr-adminsvr/adminsvr.env
install -m 0644 itcmdr-adminsvr.service /etc/systemd/system/itcmdr-adminsvr.service
systemctl daemon-reload
echo "Installed. Edit /etc/itcmdr-adminsvr/adminsvr.env, run 'itcmdr-adminsvr init', then: systemctl enable --now itcmdr-adminsvr"
