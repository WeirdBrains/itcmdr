# itcmdr Admin app

The app for owners, admins and SREs who look after many sites through [adminsvr](ADMINSVR.md)
(Linux, macOS, Windows, Android; iOS builds but needs signing). It uses the same signed client as
the [admin CLI](ADMIN-CLI.md).

1. Enroll the device with a one-time code from an owner or admin (`itcmdr-admin user create`). The
   app shows your 24 words once and asks three back.
2. On first contact it checks adminsvr's certificate ([certificates](CERTIFICATES.md)).

| Tab | What |
|---|---|
| Alerts | open, resolved or all; acknowledge and resolve; a banner for new alerts while the app is open |
| Servers | status, configuration, versions per machine ("update available"), metrics history (6 h to 30 days), commands, start a backup, last deep backup check; admins add and revoke servers |
| Plugins | the plugin and release catalog, approvals, the second-approver switch (owners) |
| Users | (owners and admins) create with a code, roles, disable, SRE scopes, approve and revoke devices, reset passkeys |
| Settings | passkeys, the step-up policy (owners), notifications, audit verify, the trusted certificate, forget this device, Help |

Actions on the step-up list show "Touch your security key" while the call waits for the touch. There
is no web build (the client uses gRPC over TLS sockets).
