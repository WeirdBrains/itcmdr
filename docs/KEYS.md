# Keys, roles and signed requests

## The master key and the 24 words

`itcmdr-cli setup first-run` (or `init`) creates the site's master key and shows it **once** as 24
words, in a real console only (redirected output is refused). They are never written to a file or a
log. Write them on paper and keep it off the server. They are needed to certify keys, to restore on a
replacement machine (`recover-db-key`) and to open a backup.

## Keys and roles

Every person, app and service plugin has an Ed25519 key certified with the words. Roles, highest
first: **owner > admin > user > client**.

    itcmdr-cli keys certify --kind user --name "Dr Example" --role owner --public-key <hex> --out app.bundle.json
    itcmdr-cli keys register app.bundle.json      # then delete the bundle file
    itcmdr-cli keys list | revoke <fingerprint> | rotate ...
    itcmdr-cli user add --name "Front Desk" --role user --public-key <hex>
    itcmdr-cli service register ...               # a service plugin's client key

Prefer `--public-key` from the app's Keys page: the private key never leaves the user's device.

## Signed requests

Each call carries `x-itcmdr-key`, `x-itcmdr-ts`, `x-itcmdr-nonce` and `x-itcmdr-sig` over the canonical
request bytes ([signing](SIGNING.md)). The server checks the registered key, a clock window, a nonce
cache and the role. New installs are signed-only (`auth_mode: signed`); older installs may still be in
`both` (the legacy shared token for changes): `itcmdr-cli auth status`, then the switch steps in
[install](INSTALL.md).

## Agents

Agents do not use the words: they enroll with a one-time token and get a client certificate from the
site CA, which is signed by the root key ([agent](AGENT.md)).

## adminsvr

adminsvr devices have their own 24 words per user and device keys; see [adminsvr](ADMINSVR.md) and
[admin CLI](ADMIN-CLI.md).
