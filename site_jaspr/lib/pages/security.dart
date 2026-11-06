import 'package:jaspr/dom.dart';
import 'package:jaspr/server.dart';

import '../components/site_shell.dart';

class SecurityPage extends StatelessComponent {
  const SecurityPage({super.key});

  @override
  Component build(BuildContext context) {
    return SiteShell(
      pageId: 'security',
      title: """Security & privacy — itcmdr""",
      description: """How itcmdr keeps office data in the office.""",
      body: RawText("""<section class="wrap narrow"><h1>Security &amp; privacy</h1>
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
<h2>Reporting a problem</h2><p>Please report security issues privately through <a href="https://github.com/WeirdBrains/itcmdr/security">GitHub</a>.</p></section>"""),
    );
  }
}
