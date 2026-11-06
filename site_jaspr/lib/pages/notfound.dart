import 'package:jaspr/dom.dart';
import 'package:jaspr/server.dart';

import '../components/site_shell.dart';

class NotFoundPage extends StatelessComponent {
  const NotFoundPage({super.key});

  @override
  Component build(BuildContext context) {
    return SiteShell(
      pageId: 'notfound',
      title: """Page not found — itcmdr""",
      description: """That page is not on this site.""",
      body: RawText("""<section class="wrap narrow"><h1>Page not found</h1><p><a href="index.html">Back to the home page</a></p></section>"""),
    );
  }
}
