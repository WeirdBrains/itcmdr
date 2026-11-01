#!/bin/sh
# Install itcmdr-adminsvr (v0.19.2) as a macOS LaunchDaemon. Run with sudo from the unpacked folder.
# Then edit /usr/local/etc/itcmdr-adminsvr/adminsvr.env, create the first owner with
#   sudo -u _itcmdradminsvr env ITCMDR_ADMINSVR_DATA_DIR=/usr/local/var/itcmdr-adminsvr \
#     itcmdr-adminsvr --env-file /usr/local/etc/itcmdr-adminsvr/adminsvr.env init --email you@example.com --name "Your Name"
# and start it: sudo launchctl bootstrap system /Library/LaunchDaemons/com.weirdbrains.itcmdr-adminsvr.plist
set -eu
cd "$(dirname "$0")"
[ "$(id -u)" = 0 ] || { echo "run with sudo" >&2; exit 1; }
U=_itcmdradminsvr
LABEL=com.weirdbrains.itcmdr-adminsvr
if ! dscl . -read "/Users/$U" >/dev/null 2>&1; then
	# A hidden system account (no login), with the first free id below 500.
	id=300
	while dscl . -list /Users UniqueID | awk '{print $2}' | grep -qx "$id" || dscl . -list /Groups PrimaryGroupID | awk '{print $2}' | grep -qx "$id"; do id=$((id + 1)); done
	dscl . -create "/Groups/$U"
	dscl . -create "/Groups/$U" PrimaryGroupID "$id"
	dscl . -create "/Groups/$U" Password '*'
	dscl . -create "/Users/$U"
	dscl . -create "/Users/$U" UniqueID "$id"
	dscl . -create "/Users/$U" PrimaryGroupID "$id"
	dscl . -create "/Users/$U" UserShell /usr/bin/false
	dscl . -create "/Users/$U" NFSHomeDirectory /usr/local/var/itcmdr-adminsvr
	dscl . -create "/Users/$U" RealName "itcmdr adminsvr"
	dscl . -create "/Users/$U" Password '*'
	dscl . -create "/Users/$U" IsHidden 1
fi
install -d -m 0755 /usr/local/bin /usr/local/etc /usr/local/var /usr/local/var/log
install -m 0755 itcmdr-adminsvr /usr/local/bin/itcmdr-adminsvr
install -d -m 0700 -o "$U" -g "$U" /usr/local/var/itcmdr-adminsvr
install -d -m 0750 -g "$U" /usr/local/etc/itcmdr-adminsvr
[ -f /usr/local/etc/itcmdr-adminsvr/adminsvr.env ] || install -m 0640 -g "$U" adminsvr.env.example /usr/local/etc/itcmdr-adminsvr/adminsvr.env
touch /usr/local/var/log/itcmdr-adminsvr.log && chown "$U" /usr/local/var/log/itcmdr-adminsvr.log && chmod 0640 /usr/local/var/log/itcmdr-adminsvr.log
install -m 0644 -o root -g wheel "$LABEL.plist" "/Library/LaunchDaemons/$LABEL.plist"
echo "Installed. Edit /usr/local/etc/itcmdr-adminsvr/adminsvr.env, run 'itcmdr-adminsvr init' (see the top of install-macos.sh), then:"
echo "  sudo launchctl bootstrap system /Library/LaunchDaemons/$LABEL.plist"
