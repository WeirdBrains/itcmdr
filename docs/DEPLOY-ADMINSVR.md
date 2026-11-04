# Deploying adminsvr with Let's Encrypt

adminsvr is optional: it lets an IT provider watch many sites. It only ever sees redacted status and
never holds backups. The domain is set per install; `admin.example.com` below is a placeholder.

1. **A host** with a public address (Linux with systemd is the simplest; macOS, Windows and a
   container also work, see [adminsvr](ADMINSVR.md)).
2. **DNS:** an A/AAAA record for `admin.example.com` pointing at it.
3. **Firewall:** allow TCP 8444 (admin API) and 8443 (SiteLink). For Let's Encrypt also forward
   **443 to 8444** (`tls-alpn-01`, the default) or open **80** (`http-01`).
4. **Install:** `sudo ./install.sh` from `itcmdr-adminsvr-<ver>-linux-x86_64.tar.gz`.
5. **Settings** in `/etc/itcmdr-adminsvr/adminsvr.env`:

        ITCMDR_ADMINSVR_PUBLIC_API=admin.example.com:8444
        ITCMDR_ADMINSVR_PUBLIC_SITE=admin.example.com:8443
        ITCMDR_ADMINSVR_TLS_NAMES=admin.example.com
        ITCMDR_ADMINSVR_ACME_DOMAINS=admin.example.com
        ITCMDR_ADMINSVR_ACME_CONTACT=it@example.com
        ITCMDR_ADMINSVR_ACME_DIRECTORY=staging      # switch to production once staging works
        # ITCMDR_ADMINSVR_ACME_CHALLENGE=http-01    # if you cannot forward 443
        # ITCMDR_ADMINSVR_DB_URL=postgres://...      # default: SQLite in the data folder

6. **First owner:** `sudo -u itcmdr-adminsvr itcmdr-adminsvr --env-file /etc/itcmdr-adminsvr/adminsvr.env init --email you@example.com --name "You"`.
7. **Start:** `systemctl enable --now itcmdr-adminsvr`; then `itcmdr-adminsvr tls-status` shows
   `acme`, the names and the expiry. Switch the directory to `production`, restart, check again.
8. **Enroll your device:** `itcmdr-admin enroll <code>` or the [admin app](ADMIN-APP.md).
9. **Link sites:** `itcmdr-admin site create <name>`, then on the site server
   `itcmdr-cli adminsvr enroll <site token>`.

Renewal is automatic (30 days before expiry, no restart). A failed renewal keeps the current
certificate and opens the `tls-acme` alert. Enrolled devices and sites keep using the pinned
internal certificate, so turning ACME on or off never breaks them. All settings:
[environment](ENVIRONMENT.md).
