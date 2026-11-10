import 'package:jaspr/dom.dart';
import 'package:jaspr/server.dart';

import '../components/site_shell.dart';

class DownloadPage extends StatelessComponent {
  const DownloadPage({super.key});

  @override
  Component build(BuildContext context) {
    return SiteShell(
      pageId: 'download',
      title: """Download itcmdr""",
      description: """Download itcmdr builds from GitHub Releases.""",
      body: RawText("""<section class="wrap narrow"><h1>Download</h1>
<p class="lead">Every build, with its checksums in <code>SHA256SUMS</code>, is on GitHub Releases.</p>
<p><a class="btn" href="https://github.com/WeirdBrains/itcmdr/releases/latest">Latest release</a> <a class="btn ghost" href="https://github.com/WeirdBrains/itcmdr/releases">All releases</a></p>
<div class="note" role="note"><h2>Builds are unsigned for now</h2><p>The Windows installers and the macOS app aren't code-signed yet. Windows SmartScreen will warn about them, and machines where SmartScreen or app control is locked down won't install them at all. On macOS, Gatekeeper may block the app until you allow it. Signed builds are coming. Check each file against <code>SHA256SUMS</code> before you install it.</p></div>
<h2>itcmdr 1.4.0</h2><p>Direct downloads for 1.4.0 (<a href="https://github.com/WeirdBrains/itcmdr/releases/tag/v1.4.0">release notes</a>, <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/SHA256SUMS">SHA256SUMS</a>).</p><div class="table-wrap"><table><thead><tr><th scope="col">Package</th><th scope="col">Downloads</th></tr></thead><tbody>
<tr><td>Site server</td><td><a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-server-Setup-1.4.0.exe">Installer</a></td></tr>
<tr><td>Agent</td><td><a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-agent-Setup-1.4.0.exe">Installer</a></td></tr>
<tr><td>CLI</td><td><a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-cli-Setup-1.4.0.exe">Windows</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-cli-1.4.0-linux-x86_64.tar.gz">Linux</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-cli-1.4.0-macos-aarch64.tar.gz">macOS</a></td></tr>
<tr><td>Office app</td><td><a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-app-Setup-1.4.0.exe">Windows</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-app-1.4.0-linux-x86_64.tar.gz">Linux</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-app-1.4.0-macos-aarch64.zip">macOS</a></td></tr>
<tr><td>Admin server</td><td><a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-adminsvr-Setup-1.4.0.exe">Windows</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-adminsvr-1.4.0-linux-x86_64.tar.gz">Linux</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-adminsvr-1.4.0-macos-aarch64.tar.gz">macOS</a></td></tr>
<tr><td>Admin CLI</td><td><a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-admin-Setup-1.4.0.exe">Windows</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-admin-1.4.0-linux-x86_64.tar.gz">Linux</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-admin-1.4.0-macos-aarch64.tar.gz">macOS</a></td></tr>
<tr><td>Admin app</td><td><a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-admin-app-Setup-1.4.0.exe">Windows</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-admin-app-1.4.0-linux-x86_64.tar.gz">Linux</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-admin-app-1.4.0-macos-aarch64.zip">macOS</a> · <a href="https://github.com/WeirdBrains/itcmdr/releases/download/v1.4.0/itcmdr-admin-app-1.4.0-android.apk">Android</a></td></tr>
</tbody></table></div>
<p>No file share is needed: install the server, then each PC's agent with a one-time enrollment token.</p>
<p>Install steps are in the <a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/INSTALL.md">install guide</a> and the <a href="https://github.com/WeirdBrains/itcmdr/blob/main/docs/ADMINSVR.md">admin server guide</a>.</p></section>"""),
    );
  }
}
