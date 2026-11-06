/// Server entrypoint for static pre-rendering.
library;

import 'package:jaspr/server.dart';

import 'app.dart';
import 'main.server.options.dart';

void main() {
  Jaspr.initializeApp(options: defaultServerOptions);
  runApp(Document(
    title: 'itcmdr: self-healing IT for small offices',
    lang: 'en',
    base: null,
    viewport: 'width=device-width, initial-scale=1',
    meta: const {},
    styles: const [],
    head: const [],
    body: const App(),
  ));
}
