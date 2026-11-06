import 'package:jaspr/dom.dart';
import 'package:jaspr/server.dart';

import '../components/site_shell.dart';

class DocsPage extends StatelessComponent {
  const DocsPage({super.key});

  @override
  Component build(BuildContext context) {
    return SiteShell(
      pageId: 'docs',
      title: """Docs — itcmdr""",
      description: """Guides for installing and running itcmdr.""",
      body: RawText("""<section class="wrap narrow"><h1>Docs</h1><p class="lead">The documentation lives with the code on GitHub.</p><ul class="doclist"><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/INSTALL.md">Install</a><span>Installing the server, agent, CLI and app, plus first-run setup.</span></li><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/ARCHITECTURE.md">Architecture</a><span>The components, agent runs and logs, exceptions, on-demand runs and the server.</span></li><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/AGENT.md">Agent</a><span>Enrollment, the mutual-TLS link, certificate renewal, revocation and the offline queue.</span></li><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/ADMINSVR.md">Admin server</a><span>What leaves a site, and installing on Linux, macOS, Windows or a container.</span></li><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/BACKUP.md">Backup and restore</a><span>Encrypted backups, retention and on-site restore.</span></li><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/SETTINGS.md">Settings</a><span>Every setting and where it's managed.</span></li><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/SIGNING.md">Signing</a><span>Keys, the 24 words, signed requests and signed plugins.</span></li><li><a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/LOG-STANDARD.md">Log standard</a><span>The structured log format.</span></li></ul><p>See also the <a href="https://github.com/WeirdBrains/itcmdr/blob/main/CHANGELOG.md">changelog</a> and the <a href="https://github.com/WeirdBrains/itcmdr/blob/main/PLAN.md">roadmap</a>.</p></section>"""),
    );
  }
}
