# HTTPS and certificate trust

Since v1.1.0 every API is HTTPS by default: the site server's API, adminsvr's admin API, and the
SiteLink and agent link (pinned, with client certificates). This page covers how the apps and
CLIs decide whether to trust a server.

## Which certificate each server has

| Server | Certificate | Pinned by |
|---|---|---|
| site server API (50051) and agent link (50052) | `<data dir>\keys\agent-server.pem`, from the site CA | agents (exact pin); the site app and `itcmdr-cli` via trust on first use |
| adminsvr admin API (8444) | internal certificate `<data dir>/tls/server.pem`, plus an optional Let's Encrypt certificate ([deploying adminsvr](DEPLOY-ADMINSVR.md)) | enrolled devices (pin from the enrollment code); browsers and new clients via the system store |
| adminsvr SiteLink (8443) | internal certificate only, mTLS | sites (pin from the site token) |

## Trust in the apps and CLIs

The admin app, the site app, `itcmdr-admin` and `itcmdr-cli` use HTTPS by default and validate
the server against the system trust store (chain, name, expiry). When that fails they do not just
fail and never ignore errors silently: they show the certificate's **SHA-256 fingerprint,
subject and expiry** and offer **Trust this certificate**, which trusts exactly that certificate for
that one server (trust on first use). Compare the fingerprint with the server first
(`itcmdr-adminsvr tls-status`, or the server's `agent-server.pem`).
- A **different** certificate than the trusted one is refused with a loud warning until the user
  reviews it and replaces the trusted one (app: tick "I checked the new fingerprint", then
  **Replace trusted certificate**; CLI: `itcmdr-admin trust review`, then
  `itcmdr-admin trust set <sha256> --replace`).
- An **expired / not yet valid / wrong-name** certificate needs an explicit override on top
  ("Trust anyway" with a confirmation tick; CLI `--trust-anyway`).
- Trusted certificates can be listed and removed: site app Settings → Trusted certificates;
  admin app Settings → Certificate; `itcmdr-admin trust show | review | remove`.
- `itcmdr-admin --trust-fingerprint <sha256>` trusts the certificate non-interactively (it must be
  the one the server presents); on a terminal it asks instead (type `trust`, or `trust anyway`).
- A certificate the system trust store accepts for the right name is accepted even when an older
  certificate is trusted (for example a self-signed one trusted before the server got a Let's
  Encrypt certificate); the trusted one is kept as the fallback for when the system store refuses.
  A changed certificate that the system store does not accept is always blocked.
- **HTTPS and plain HTTP are both supported; HTTPS is the default.** Plain HTTP is an explicit
  opt-in: the site app's "Use HTTPS" switch off, or `itcmdr-cli --plain-http`. The site server
  always serves HTTPS, accepts plain HTTP from the same machine, and from other machines only with
  `api_plain_http: true` in server.json (default false; `itcmdr-cli setup apply --api-plain-http
  true`), for example behind a TLS-terminating proxy.
- **v1.0 apps and a v1.1 server:** v1.0 site apps talk plain HTTP. Until they are updated, set
  `api_plain_http: true` on the server (then back to false after the update).
- Enrolled adminsvr devices keep the pin from their enrollment code (the internal certificate),
  and sites and agents keep their exact pins and mTLS; none of that depends on the system trust store.

## Extra roots

`ITCMDR_EXTRA_ROOTS` adds PEM root certificates (a list of files, separated like `PATH`) to the
system trust store for the CLIs, for example a private CA.
