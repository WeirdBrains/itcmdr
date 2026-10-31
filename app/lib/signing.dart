// v0.12.0 signed requests (docs/SIGNING.md) and sealed restore words. Since v0.14.2 all crypto
// runs in the shared Rust crate `itcmdr-keys` through flutter_rust_bridge (app/rust), so the app
// signs with the server's own code (pinned in proto/signing_vectors.json and proto/app_vectors.json).
// Call `RustLib.init()` once before using anything here.
import 'dart:async';
import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:fixnum/fixnum.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64Util;
import 'package:grpc/grpc.dart';
import 'package:protobuf/protobuf.dart';

import 'src/rust/api/keys.dart' as rk;

String hexOf(List<int> b) =>
    b.map((x) => x.toRadixString(16).padLeft(2, '0')).join();

Uint8List bytesOfHex(String h) {
  final s = h.trim();
  if (s.length.isOdd || !RegExp(r'^[0-9a-fA-F]*$').hasMatch(s)) {
    throw const FormatException('not hex');
  }
  return Uint8List.fromList([
    for (var i = 0; i < s.length; i += 2)
      int.parse(s.substring(i, i + 2), radix: 16),
  ]);
}

Uint8List randomBytes(int n) {
  final r = Random.secure();
  return Uint8List.fromList(List.generate(n, (_) => r.nextInt(256)));
}

Future<String> sha256Hex(List<int> b) async =>
    rk.sha256Hex(data: Uint8List.fromList(b));

/// `itcmdr-req/v1\n{method}\n{ts}\n{nonce}\n{sha256 hex of body}`
Future<List<int>> requestMessage(
  String method,
  int tsMs,
  String nonce,
  List<int> body,
) async => rk.requestMessage(
  method: method,
  tsMs: PlatformInt64Util.from(tsMs),
  nonce: nonce,
  body: Uint8List.fromList(body),
);

/// Canonical proto3 bytes (docs/SIGNING.md rule 2): Dart writes a scalar that was set
/// explicitly even when it is the default (e.g. `reportOnly = false`); prost, and the server's
/// re-encoding, omit it. Sign over a copy with such fields cleared. Set sub-messages stay.
List<int> canonicalBytes(GeneratedMessage m) {
  final c = m.deepCopy();
  _strip(c);
  return c.writeToBuffer();
}

bool _isDefault(Object? v) =>
    v == null ||
    v == false ||
    v == 0 ||
    v == 0.0 ||
    v == '' ||
    (v is Int64 && v.isZero) ||
    (v is List<int> && v.isEmpty) ||
    (v is ProtobufEnum && v.value == 0);

void _strip(GeneratedMessage m) {
  for (final fi in m.info_.fieldInfo.values) {
    final tag = fi.tagNumber;
    if (fi.isRepeated || fi.isMapField) {
      final v = m.getField(tag);
      if (v is List) {
        for (final e in v) {
          if (e is GeneratedMessage) _strip(e);
        }
      }
      continue;
    }
    if (!m.hasField(tag)) continue;
    final v = m.getField(tag);
    if (v is GeneratedMessage) {
      _strip(v);
    } else if (_isDefault(v)) {
      m.clearField(tag);
    }
  }
}

/// The app's Ed25519 key. The 32-byte seed is the private key (the same format
/// `itcmdr-cli keys new` writes), stored on this PC only.
class AppKey {
  final Uint8List seed;
  final Uint8List publicKey;
  final String fingerprint;
  AppKey._(this.seed, this.publicKey, this.fingerprint);

  static Future<AppKey> fromSeed(List<int> seed) async {
    if (seed.length != 32) {
      throw const FormatException(
        'a private key is 32 bytes (64 hex characters)',
      );
    }
    final s = Uint8List.fromList(seed);
    final pub = rk.publicKey(seed: s);
    return AppKey._(s, pub, rk.fingerprint(publicKey: pub));
  }

  static Future<AppKey> generate() => fromSeed(rk.randomSeed());

  String get publicHex => hexOf(publicKey);

  Future<String> signHex(List<int> message) async =>
      hexOf(rk.sign(seed: seed, message: Uint8List.fromList(message)));

  /// Metadata for one call. [tsMs]/[nonce] are fixed only in tests.
  Future<Map<String, String>> sign(
    String method,
    List<int> body, {
    int? tsMs,
    String? nonce,
  }) async {
    final ts = tsMs ?? DateTime.now().millisecondsSinceEpoch;
    final n = nonce ?? hexOf(randomBytes(16));
    return {
      'x-itcmdr-key': fingerprint,
      'x-itcmdr-ts': '$ts',
      'x-itcmdr-nonce': n,
      'x-itcmdr-sig': await signHex(await requestMessage(method, ts, n, body)),
    };
  }
}

/// Signs every call. The body is the canonical protobuf encoding of the request (the server
/// hashes its own re-encoding). Streaming calls (WatchUpdates) carry an empty request.
class SigningInterceptor extends ClientInterceptor {
  final AppKey key;
  SigningInterceptor(this.key);

  CallOptions _with(String path, List<int> body, CallOptions options) =>
      options.mergedWith(
        CallOptions(
          providers: [
            (md, uri) async {
              md.addAll(await key.sign(path, body));
            },
          ],
        ),
      );

  @override
  ResponseFuture<R> interceptUnary<Q, R>(
    ClientMethod<Q, R> method,
    Q request,
    CallOptions options,
    ClientUnaryInvoker<Q, R> invoker,
  ) {
    final body = request is GeneratedMessage
        ? canonicalBytes(request)
        : <int>[];
    return invoker(method, request, _with(method.path, body, options));
  }

  @override
  ResponseStream<R> interceptStreaming<Q, R>(
    ClientMethod<Q, R> method,
    Stream<Q> requests,
    CallOptions options,
    ClientStreamingInvoker<Q, R> invoker,
  ) => invoker(method, requests, _with(method.path, const <int>[], options));
}

/// Words sealed for a one-time server key (see server/src/restore_seal.rs).
class SealedWordsParts {
  final String keyId, eph, nonce, ct;
  const SealedWordsParts(this.keyId, this.eph, this.nonce, this.ct);
}

List<int> restoreAad(String keyId, String backup) =>
    utf8.encode('itcmdr-restore-words/v1\n$keyId\n$backup');

/// Seal [words] on this PC so only the server's one-time key opens them. [ephSeed]/[nonce] are
/// random unless a test pins them.
Future<SealedWordsParts> sealWords({
  required String serverPublicHex,
  required String keyId,
  required String backup,
  required String words,
  List<int>? ephSeed,
  List<int>? nonce,
}) async {
  final server = bytesOfHex(serverPublicHex);
  if (server.length != 32) throw const FormatException('bad restore key');
  final out = rk.sealWords(
    serverPublic: server,
    keyId: keyId,
    backup: backup,
    words: words,
    ephSeed: ephSeed == null ? null : Uint8List.fromList(ephSeed),
    nonce: nonce == null ? null : Uint8List.fromList(nonce),
  );
  return SealedWordsParts(out.keyId, out.eph, out.nonce, out.ct);
}
