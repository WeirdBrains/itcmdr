# Deploying a site server

A checklist for one office. Placeholders: SERVER01 (the server), WS-01 (a PC), 198.51.100.0/24 (the LAN).
Details for each step are in [install](INSTALL.md).

1. **Pick the machine.** Windows, always on, on the office LAN. Keep a second disk or NAS folder
   for backups if you have one (not required: v1.0 needs no file share).
2. **Install the server** as an administrator, at the machine:
   `itcmdr-server-Setup-<ver>.exe /AllowSubnet=198.51.100.0/24` (or the wizard). This adds the
   `itcmdr-server` service, the watchdog, firewall rules for 50051 (API) and 50052 (agent link)
   limited to the subnet, and `C:\ProgramData\itcmdr\server.json`.
3. **First-run setup, in person:** the 24 words are shown once. Write them on paper and keep it off
   the server. The first owner's app key is registered.
4. **Check:** `itcmdr-cli setup check` (exit 0), `Get-Service itcmdr-server` Running, and no errors in
   `C:\ProgramData\itcmdr\server-logs\itcmdr-server.txt`.
5. **Backups:** `itcmdr-cli setup apply --backup-dir <second disk>`; `itcmdr-cli backup create`, then
   `backup drill` (and monthly `backup drill --deep` with the words; [backup](BACKUP.md)).
6. **Agents:** for each PC, `itcmdr-cli agent enroll-token --machine WS-01`, then
   `itcmdr-agent-Setup-<ver>.exe /VERYSILENT /EnrollToken=<token>`. `itcmdr-cli agent list` shows it online.
7. **Apps:** install the site app on the admin PCs; Settings: `SERVER01 : 50051`, HTTPS on; compare and
   trust the certificate fingerprint ([certificates](CERTIFICATES.md)); certify the key ([keys](KEYS.md)).
8. **Updates:** set the update window (`itcmdr-cli config set updates.window "daily 01:00-04:00"`;
   [upgrades](UPGRADES.md)).
9. **Optional offsite link:** `itcmdr-cli adminsvr enroll <site token>` and allow only the commands you
   want ([adminsvr](ADMINSVR.md)). Nothing internal leaves the office; backups never do.

Never test break scenarios on a production server; use a test machine or VM.
