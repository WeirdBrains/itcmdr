import 'package:jaspr/jaspr.dart';
import 'package:jaspr_router/jaspr_router.dart';

import 'pages/demos.dart';
import 'pages/docs.dart';
import 'pages/download.dart';
import 'pages/index.dart';
import 'pages/notfound.dart';
import 'pages/security.dart';

class App extends StatelessComponent {
  const App({super.key});

  @override
  Component build(BuildContext context) {
    return Router(routes: [
      Route(path: '/', title: 'itcmdr: self-healing IT for small offices', builder: (c, s) => const HomePage()),
      Route(path: '/index.html', title: 'itcmdr: self-healing IT for small offices', builder: (c, s) => const HomePage()),
      Route(path: '/demos.html', title: 'Demos — itcmdr', builder: (c, s) => const DemosPage()),
      Route(path: '/download.html', title: 'Download itcmdr', builder: (c, s) => const DownloadPage()),
      Route(path: '/docs.html', title: 'Docs — itcmdr', builder: (c, s) => const DocsPage()),
      Route(path: '/security.html', title: 'Security & privacy — itcmdr', builder: (c, s) => const SecurityPage()),
      Route(path: '/404.html', title: 'Page not found — itcmdr', builder: (c, s) => const NotFoundPage()),
    ]);
  }
}
