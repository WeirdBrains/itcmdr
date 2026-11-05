#!/usr/bin/env python3
"""Builds the static itcmdr site from the page bodies below. No dependencies: python3 build.py"""
from pathlib import Path
R = "https://github.com/WeirdBrains/itcmdr"
D = R + "/blob/main/docs/"
NAV = [("index.html","Home"),("download.html","Download"),("docs.html","Docs"),("security.html","Security &amp; privacy")]
def page(fn, title, desc, body):
    nav = "".join(f'<li><a href="{h}"{" aria-current=\"page\"" if h==fn else ""}>{t}</a></li>' for h,t in NAV)
    return f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title><meta name="description" content="{desc}">
<meta property="og:title" content="{title}"><meta property="og:description" content="{desc}">
<meta property="og:image" content="https://itcmdr.weirdbrains.com/assets/og.png"><meta name="theme-color" content="#0B2350">
<link rel="icon" type="image/svg+xml" href="assets/logo-mark.svg"><link rel="icon" type="image/png" href="assets/favicon.png">
<link rel="apple-touch-icon" href="assets/apple-touch-icon.png"><link rel="stylesheet" href="assets/style.css"></head>
<body><a class="skip" href="#main">Skip to content</a>
<header class="top"><div class="wrap bar"><a class="brand" href="index.html"><img src="assets/logo-mark.svg" alt="" width="36" height="36"><span>itcmdr</span></a>
<nav aria-label="Main"><ul>{nav}<li><a href="{R}">GitHub</a></li></ul></nav></div></header>
<main id="main">{body}</main>
<footer class="foot"><div class="wrap"><p><img src="assets/logo-mark.svg" alt="" width="24" height="24"> itcmdr is built by <a href="https://weirdbrains.com">Weird Brains</a>.</p>
<p><a href="{R}">Source on GitHub</a> · <a href="{R}/releases">Releases</a> · <a href="security.html">Security &amp; privacy</a></p></div></footer></body></html>
'''
def card(t, p): return f'<div class="card"><h3>{t}</h3><p>{p}</p></div>'
HOME = f'''
<section class="hero"><div class="wrap hero-grid"><div>
<p class="eyebrow">itcmdr 1.2.1</p>
<h1>Self-healing IT for small offices.</h1>
<p class="lead">An agent on every PC checks the network, connections and services each night and repairs what it safely can. A server on site keeps the history, alerts and backups. And nothing internal leaves the office.</p>
<p class="cta"><a class="btn" href="download.html">Download</a> <a class="btn ghost" href="{R}">View on GitHub</a></p></div>
<img class="hero-img" src="assets/logo-mark.svg" alt="itcmdr logo: a navy shield with a teal terminal prompt" width="320" height="320"></div></section>
<section class="wrap" aria-labelledby="f"><h2 id="f">Features</h2><div class="grid">
{card("Nightly checks and repairs","The agent checks network settings, connectivity and services, repairs what it safely can, and logs every check. Checks with no safe repair are reported, not guessed at.")}
{card("Plugins and schedules","Checks are signed plugin packages. Tampered or unsigned packages are quarantined. Schedules run on the agent, even while it's offline.")}
{card("Alerts that make sense","Site alerts come in three severity levels. You can acknowledge, resolve or silence them, and they also show up as push notifications in the admin app.")}
{card("Metrics and log search","Flapping checks, repeat offenders, charts, and log search with filters and paging.")}
{card("Backups that stay home","Backup coverage checks, encrypted backups and on-site restore. Backups never leave the site server.")}
{card("Keys, not passwords","A 24-word master key backs certified keys, and every request is signed. Passkeys and security keys can be added as a second factor on the admin server.")}
</div></section>
<section class="band" aria-labelledby="p"><div class="wrap"><h2 id="p">The privacy promise</h2>
<p class="lead">No internal information leaves the office it's installed in.</p>
<ul class="ticks"><li>The site server decides what leaves, and it denies by default.</li>
<li>Hostnames, IPs, paths, usernames and message text never leave. The admin server sees opaque IDs, alert codes, counts and timestamps.</li>
<li>Display names are only shared if the office owner opts in on that site.</li>
<li>Backups never leave the site server, and the admin server can't restore them.</li></ul>
<p><a href="security.html">How the security and privacy work →</a></p></div></section>
<section class="wrap" aria-labelledby="h"><h2 id="h">How it works</h2><ol class="steps">
<li><strong>Install the site server</strong> on the office server. First-run setup shows your 24 words once and creates the first admin key.</li>
<li><strong>Enroll each PC</strong> with a one-time token. The agent gets its own certificate from the site, and every connection is mutually authenticated.</li>
<li><strong>Let it run.</strong> Checks and repairs run on schedule, results come back to the site server, and problems become alerts.</li>
<li><strong>Optionally connect an admin server</strong> to watch many sites from one place, with redacted data only.</li></ol></section>
<section class="band light" aria-labelledby="c"><div class="wrap"><h2 id="c">Components</h2><div class="grid">
{card("Agent","A Rust service on every PC and on the office server. It enrolls with mutual TLS, keeps a heartbeat, renews its certificate, runs plugins and schedules, and queues alerts while offline.")}
{card("Site server","A Rust gRPC server for the office, with an encrypted database, roles and users, plugins, schedules, alerts, backups and an audit log.")}
{card("Office app","A Flutter desktop app for the office: overview, metrics, logs, schedules, plugins, backups, audit, keys and settings.")}
{card("CLI","<code>itcmdr-cli</code> handles setup, backup and restore, settings and enrollment tokens.")}
{card("Admin server","Optional and off-site, for IT providers. It runs on SQLite or Postgres, uses passkeys and sends push alerts. Changes it sends need preview, two-person approval and can be frozen by the owner. It runs as a service on Linux, macOS or Windows, or as a container.")}
{card('<img src="assets/icon-admin.svg" alt="" width="28" height="28"> Admin app and CLI',"A Flutter app and <code>itcmdr-admin</code> CLI for the people who look after many sites: alert inbox, servers, users and service plugins.")}
</div></div></section>'''
DL = f'''<section class="wrap narrow"><h1>Download</h1>
<p class="lead">Every build, with its checksums in <code>SHA256SUMS</code>, is on GitHub Releases.</p>
<p><a class="btn" href="{R}/releases/latest">Latest release</a> <a class="btn ghost" href="{R}/releases">All releases</a></p>
<div class="note" role="note"><h2>Builds are unsigned for now</h2><p>The Windows installers and the macOS app aren't code-signed yet. Windows SmartScreen will warn about them, and machines where SmartScreen or app control is locked down won't install them at all. On macOS, Gatekeeper may block the app until you allow it. Signed builds are coming. Check each file against <code>SHA256SUMS</code> before you install it.</p></div>
<h2>itcmdr 1.2.1</h2><p>Direct downloads for 1.2.1 (<a href="{R}/releases/tag/v1.2.1">release notes</a>, <a href="{R}/releases/download/v1.2.1/SHA256SUMS">SHA256SUMS</a>).</p><div class="table-wrap"><table><thead><tr><th scope="col">Package</th><th scope="col">Downloads</th></tr></thead><tbody>
<tr><td>Site server</td><td><a href="{R}/releases/download/v1.2.1/itcmdr-server-Setup-1.2.1.exe">Installer</a></td></tr>
<tr><td>Agent</td><td><a href="{R}/releases/download/v1.2.1/itcmdr-agent-Setup-1.2.1.exe">Installer</a></td></tr>
<tr><td>CLI</td><td><a href="{R}/releases/download/v1.2.1/itcmdr-cli-Setup-1.2.1.exe">Windows</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-cli-1.2.1-linux-x86_64.tar.gz">Linux</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-cli-1.2.1-macos-aarch64.tar.gz">macOS</a></td></tr>
<tr><td>Office app</td><td><a href="{R}/releases/download/v1.2.1/itcmdr-app-Setup-1.2.1.exe">Windows</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-app-1.2.1-linux-x86_64.tar.gz">Linux</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-app-1.2.1-macos-aarch64.zip">macOS</a></td></tr>
<tr><td>Admin server</td><td><a href="{R}/releases/download/v1.2.1/itcmdr-adminsvr-Setup-1.2.1.exe">Windows</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-adminsvr-1.2.1-linux-x86_64.tar.gz">Linux</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-adminsvr-1.2.1-macos-aarch64.tar.gz">macOS</a></td></tr>
<tr><td>Admin CLI</td><td><a href="{R}/releases/download/v1.2.1/itcmdr-admin-Setup-1.2.1.exe">Windows</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-admin-1.2.1-linux-x86_64.tar.gz">Linux</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-admin-1.2.1-macos-aarch64.tar.gz">macOS</a></td></tr>
<tr><td>Admin app</td><td><a href="{R}/releases/download/v1.2.1/itcmdr-admin-app-Setup-1.2.1.exe">Windows</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-admin-app-1.2.1-linux-x86_64.tar.gz">Linux</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-admin-app-1.2.1-macos-aarch64.zip">macOS</a> · <a href="{R}/releases/download/v1.2.1/itcmdr-admin-app-1.2.1-android.apk">Android</a></td></tr>
</tbody></table></div>
<p>No file share is needed: install the server, then each PC's agent with a one-time enrollment token.</p>
<p>Install steps are in the <a href="{D}INSTALL.md">install guide</a> and the <a href="{D}ADMINSVR.md">admin server guide</a>.</p></section>'''
docs = [("INSTALL.md","Install","Installing the server, agent, CLI and app, plus first-run setup."),
("ARCHITECTURE.md","Architecture","The components, agent runs and logs, exceptions, on-demand runs and the server."),
("AGENT.md","Agent","Enrollment, the mutual-TLS link, certificate renewal, revocation and the offline queue."),
("ADMINSVR.md","Admin server","What leaves a site, and installing on Linux, macOS, Windows or a container."),
("BACKUP.md","Backup and restore","Encrypted backups, retention and on-site restore."),
("SETTINGS.md","Settings","Every setting and where it's managed."),
("SIGNING.md","Signing","Keys, the 24 words, signed requests and signed plugins."),
("LOG-STANDARD.md","Log standard","The structured log format.")]
DOCS = '<section class="wrap narrow"><h1>Docs</h1><p class="lead">The documentation lives with the code on GitHub.</p><ul class="doclist">' + "".join(f'<li><a href="{D}{f}">{t}</a><span>{d}</span></li>' for f,t,d in docs) + f'</ul><p>See also the <a href="{R}/blob/main/CHANGELOG.md">changelog</a> and the <a href="{R}/blob/main/PLAN.md">roadmap</a>.</p></section>'
SEC = f'''<section class="wrap narrow"><h1>Security &amp; privacy</h1>
<p class="lead">itcmdr is built for offices that handle sensitive data, so privacy is the default, not a setting.</p>
<h2>What stays in the office</h2><ul class="ticks"><li>Everything the agents collect goes to the site server in the office.</li>
<li>Backups stay on the site server. There's no restore from the admin server, and tests check that no backup bytes or key material cross the link.</li>
<li>If you connect an admin server, the site sends only opaque site-scoped IDs, alert codes, check and plugin names, severity, counts, timestamps and numeric rollups. Anything else is hashed. Tests check that no hostnames, IPs, paths or usernames leave.</li>
<li>Display names are shared only after the owner turns it on for that site.</li></ul>
<h2>Keys and identity</h2><ul class="ticks"><li>A 24-word master key, shown once at setup, backs certified keys for every user.</li>
<li>Every request to the server is signed, and every change goes to an audit log.</li>
<li>The database is encrypted and its key is escrowed under the 24 words.</li>
<li>On the admin server, passkeys or security keys can be required as a second factor, and audit entries are signed and hash-chained.</li></ul>
<h2>Agents</h2><ul class="ticks"><li>Agents enroll with one-time tokens and pin the server's certificate.</li>
<li>Every connection uses TLS 1.3 with mutual certificates from the site's own CA. No public CA is involved.</li>
<li>Certificates renew automatically, and revoked agents are refused during the handshake.</li>
<li>Plugins are signed and checked against checksums. Tampered packages are quarantined.</li></ul>
<h2>Changes from the admin server</h2><p>Only allowlisted commands can be sent. They come with a preview and two-person approval, and the site owner can freeze them at any time.</p>
<h2>Reporting a problem</h2><p>Please report security issues privately through <a href="{R}/security">GitHub</a>.</p></section>'''
NF = '<section class="wrap narrow"><h1>Page not found</h1><p><a href="index.html">Back to the home page</a></p></section>'
for fn,t,d,b in [("index.html","itcmdr: self-healing IT for small offices","Self-healing IT for small offices, with privacy by design. By Weird Brains.",HOME),
 ("download.html","Download itcmdr","Download itcmdr builds from GitHub Releases.",DL),("docs.html","itcmdr docs","itcmdr documentation.",DOCS),
 ("security.html","itcmdr security and privacy","How itcmdr keeps office data in the office.",SEC),("404.html","Not found · itcmdr","Page not found.",NF)]:
    Path(fn).write_text(page(fn,t,d,b))
