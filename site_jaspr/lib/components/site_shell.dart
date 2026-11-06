import 'package:jaspr/dom.dart';
import 'package:jaspr/server.dart';

/// Shared chrome: skip link, top nav, footer. Matches the previous hand-written site.
class SiteShell extends StatelessComponent {
  const SiteShell({
    required this.pageId,
    required this.title,
    required this.description,
    required this.body,
    super.key,
  });

  /// One of: index, demos, download, docs, security, notfound
  final String pageId;
  final String title;
  final String description;
  final Component body;

  static const _nav = [
    ('index.html', 'Home', 'index'),
    ('demos.html', 'Demos', 'demos'),
    ('download.html', 'Download', 'download'),
    ('docs.html', 'Docs', 'docs'),
    ('security.html', 'Security & privacy', 'security'),
  ];

  @override
  Component build(BuildContext context) {
    return Component.fragment([
      Document.head(
        title: title,
        meta: {
          'description': description,
          'og:title': title,
          'og:description': description,
          'og:image': 'https://itcmdr.weirdbrains.com/assets/og.png',
          'theme-color': '#0B2350',
        },
        children: [
          link(rel: 'icon', type: 'image/svg+xml', href: 'assets/logo-mark.svg'),
          link(rel: 'icon', type: 'image/png', href: 'assets/favicon.png'),
          link(rel: 'apple-touch-icon', href: 'assets/apple-touch-icon.png'),
          link(rel: 'stylesheet', href: 'assets/style.css'),
        ],
      ),
      a(classes: 'skip', href: '#main', [.text('Skip to content')]),
      header(classes: 'top', [
        div(classes: 'wrap bar', [
          a(classes: 'brand', href: 'index.html', [
            img(src: 'assets/logo-mark.svg', alt: '', width: 36, height: 36),
            span([.text('itcmdr')]),
          ]),
          nav(attributes: {'aria-label': 'Main'}, [
            ul([
              for (final (href, label, id) in _nav)
                li([
                  a(
                    href: href,
                    attributes: {
                      if (id == pageId) 'aria-current': 'page',
                    },
                    [.text(label)],
                  ),
                ]),
              li([
                a(href: 'https://github.com/WeirdBrains/itcmdr', [.text('GitHub')]),
              ]),
            ]),
          ]),
        ]),
      ]),
      main_(id: 'main', [body]),
      footer(classes: 'foot', [
        div(classes: 'wrap', [
          p([
            img(src: 'assets/logo-mark.svg', alt: '', width: 24, height: 24),
            .text(' itcmdr is built by '),
            a(href: 'https://weirdbrains.com', [.text('Weird Brains')]),
            .text('.'),
          ]),
          p([
            a(href: 'https://github.com/WeirdBrains/itcmdr', [.text('Source on GitHub')]),
            .text(' · '),
            a(href: 'https://github.com/WeirdBrains/itcmdr/releases', [.text('Releases')]),
            .text(' · '),
            a(href: 'security.html', [.text('Security & privacy')]),
          ]),
        ]),
      ]),
    ]);
  }
}
