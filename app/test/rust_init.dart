// Loads the itcmdr-keys bridge for `flutter test` on the host (native) and in a browser (WASM).
import 'package:app/src/rust/frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import 'rust_init_io.dart' if (dart.library.js_interop) 'rust_init_web.dart';

bool _done = false;

Future<void> initRust() async {
  if (_done) return;
  final ExternalLibrary? lib = hostLibrary();
  await RustLib.init(externalLibrary: lib);
  _done = true;
}
