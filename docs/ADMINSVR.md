# adminsvr (v0.19.0)

The offsite admin server: one place where owners, admins and SREs see the status and alerts of
many sites and send a few safe commands. Design and decisions: [design/ADMINSVR.md](design/ADMINSVR.md).

## What leaves a site

The site server decides, deny by default (`server/src/sitelink.rs`, `Redactor`):

- opaque site-scoped IDs (`m-…` machines, `a-…` alerts), never names;
- alert codes built from a fixed vocabulary, check ids and plugin names; anything else is hashed;
- severity, state, counts, timestamps and numeric rollups (machines up/late/down, backup age, size, result);
- display names only after the owner opts in on the site: `itcmdr-cli adminsvr share-names on`.

Hostnames, IPs, paths, usernames and message text never leave; tests in `adminsvr/tests/sitelink.rs`
and `server/src/sitelink.rs` check this. Backups never leave the site and there is no restore from
adminsvr.

## Install adminsvr (Linux)

Needs Postgres 14+ and a database for adminsvr.

    tar xzf itcmdr-adminsvr-<ver>-linux-x86_64.tar.gz && cd itcmdr-adminsvr-<ver>-linux-x86_64
    sudo ./install.sh
    sudo editor /etc/itcmdr-adminsvr/adminsvr.env   # DB URL, public host:ports, TLS names
    sudo -u itcmdr-adminsvr sh -c 'set -a; . /etc/itcmdr-adminsvr/adminsvr.env; exec itcmdr-adminsvr init --email you@example.com --name "You"'
    sudo systemctl enable --now itcmdr-adminsvr

`init` prints the first owner's enrollment code. Ports: 8444 admin API, 8443 SiteLink.
`itcmdr-adminsvr recover-owner --email …` (console) makes a new owner code when every owner device
and the words are lost; it is audited. `itcmdr-adminsvr audit-verify` checks the audit chain.
Uninstall: `sudo ./uninstall.sh` (keeps data and settings) or `--purge`; the database is never dropped.
A Containerfile and compose file are included but not tested in this release.

## Admin CLI

    itcmdr-admin enroll <code>        # shows your 24 words once, asks three back; key backup on
    itcmdr-admin whoami
    itcmdr-admin user create <email> --name …      # role sre by default; prints their code
    itcmdr-admin user role <id> admin|sre|owner   # only owners grant or change owner
    itcmdr-admin user approve <user> <fingerprint> # new devices wait for an owner/admin
    itcmdr-admin site create <name>               # prints a one-time site token
    itcmdr-admin alert list | ack | resolve
    itcmdr-admin backup start <site>
    itcmdr-admin audit list | verify
    itcmdr-admin recover <addr> --pin … --email … # new device from the 24 words
    itcmdr-admin key-backup off                   # opt out of the encrypted key backup

Identity: the user config folder (`itcmdr-admin`, mode 0600), or `ITCMDR_ADMIN_HOME`.

## Link a site

On the site server, in person:

    itcmdr-cli adminsvr enroll <site token>
    itcmdr-cli adminsvr status
    itcmdr-cli adminsvr allow backup.start alert.ack alert.resolve   # local command allowlist
    itcmdr-cli adminsvr unlink

Setting `adminsvr.enabled=false` closes the link at once (kill switch); `adminsvr.heartbeatSecs`
overrides the status interval. Every command is checked for both signatures (user and adminsvr
root key), expiry, the never-remote list and the local allowlist, and is written to the site audit.
