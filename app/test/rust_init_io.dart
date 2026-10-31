import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

/// The host build of app/rust (`cargo build` there). Override with ITCMDR_KEYS_BRIDGE_LIB.
ExternalLibrary? hostLibrary() {
  final env = Platform.environment['ITCMDR_KEYS_BRIDGE_LIB'];
  if (env != null && env.isNotEmpty) return ExternalLibrary.open(env);
  final name = Platform.isWindows
      ? 'itcmdr_keys_bridge.dll'
      : Platform.isMacOS
      ? 'libitcmdr_keys_bridge.dylib'
      : 'libitcmdr_keys_bridge.so';
  for (final dir in ['rust/target/release', 'rust/target/debug']) {
    final f = File('$dir/$name');
    if (f.existsSync()) return ExternalLibrary.open(f.absolute.path);
  }
  throw StateError(
    'build the bridge first: (cd app/rust && cargo build) — $name not found',
  );
}
