import 'package:grpc/grpc_or_grpcweb.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'key_store.dart';
import 'proto/itcmdr.pbgrpc.dart';
import 'signing.dart';

/// Server address and the legacy admin token, persisted with shared_preferences. The app's
/// signing key (32-byte seed) lives in the OS key store since v0.13.0 (key_store.dart).
class ServerSettings {
  final String host;
  final int port;
  final bool tls;
  final String token;
  final String actor;

  /// Hex of the app's Ed25519 private seed; empty = no key (unsigned, token only).
  final String keySeed;

  const ServerSettings({
    this.host = 'localhost',
    this.port = 50051,
    this.tls = false,
    this.token = '',
    this.actor = '',
    this.keySeed = '',
  });

  static const _kHost = 'server.host';
  static const _kPort = 'server.port';
  static const _kTls = 'server.tls';
  static const _kToken = 'server.token';
  static const _kActor = 'server.actor';
  static const _kKeySeed = 'auth.key_seed';

  /// Where the key is kept; tests replace it with a MemoryKeyStore.
  static Future<KeyStore> Function() keyStoreFactory = platformKeyStore;
  static KeyStore? _store;
  static Future<KeyStore> keyStore() async =>
      _store ??= await keyStoreFactory();
  static void resetKeyStore() => _store = null;

  /// v0.12.0 stored the seed in shared_preferences (plaintext). Move it into the key store,
  /// check the read-back, then remove the old copy. Returns true when something was moved.
  static Future<bool> migrateLegacyKey(SharedPreferences p, KeyStore ks) async {
    final legacy = (p.getString(_kKeySeed) ?? '').trim();
    if (legacy.isEmpty) {
      if (p.containsKey(_kKeySeed)) await p.remove(_kKeySeed);
      return false;
    }
    final current = await ks.read();
    if (current == null || current.isEmpty) {
      await ks.write(legacy);
      if ((await ks.read()) != legacy) {
        throw StateError('key store read-back failed; the old key was kept');
      }
    }
    // Overwrite before removing, so the rewritten preferences file never holds the old value.
    await p.setString(_kKeySeed, '0' * legacy.length);
    await p.remove(_kKeySeed);
    return true;
  }

  static Future<ServerSettings> load() async {
    final p = await SharedPreferences.getInstance();
    final ks = await keyStore();
    await migrateLegacyKey(p, ks);
    return ServerSettings(
      host: p.getString(_kHost) ?? 'localhost',
      port: p.getInt(_kPort) ?? 50051,
      tls: p.getBool(_kTls) ?? false,
      token: p.getString(_kToken) ?? '',
      actor: p.getString(_kActor) ?? '',
      keySeed: (await ks.read()) ?? '',
    );
  }

  Future<void> save() async {
    final p = await SharedPreferences.getInstance();
    await p.setString(_kHost, host.trim());
    await p.setInt(_kPort, port);
    await p.setBool(_kTls, tls);
    await p.setString(_kToken, token);
    await p.setString(_kActor, actor.trim());
    final ks = await keyStore();
    final want = keySeed.trim();
    if (want.isEmpty) {
      if ((await ks.read()) != null) await ks.delete();
    } else if ((await ks.read()) != want) {
      await ks.write(want);
    }
  }

  ServerSettings copyWith({
    String? host,
    int? port,
    bool? tls,
    String? token,
    String? actor,
    String? keySeed,
  }) => ServerSettings(
    host: host ?? this.host,
    port: port ?? this.port,
    tls: tls ?? this.tls,
    token: token ?? this.token,
    actor: actor ?? this.actor,
    keySeed: keySeed ?? this.keySeed,
  );

  bool get hasKey => keySeed.trim().isNotEmpty;

  /// The signing key, or null when none is set or it is unreadable.
  Future<AppKey?> appKey() async {
    if (!hasKey) return null;
    try {
      return await AppKey.fromSeed(bytesOfHex(keySeed));
    } catch (_) {
      return null;
    }
  }

  /// A client that signs every call when the app has a key.
  ItcmdrClient client(GrpcOrGrpcWebClientChannel ch, AppKey? key) =>
      ItcmdrClient(
        ch,
        interceptors: key == null ? const [] : [SigningInterceptor(key)],
      );

  /// Host without any scheme or path a user may have pasted.
  String get cleanHost {
    var h = host.trim();
    h = h.replaceFirst(RegExp(r'^[a-zA-Z]+://'), '');
    final slash = h.indexOf('/');
    if (slash >= 0) h = h.substring(0, slash);
    final colon = h.lastIndexOf(':');
    if (colon > 0 && !h.contains(']')) h = h.substring(0, colon);
    return h.isEmpty ? 'localhost' : h;
  }

  GrpcOrGrpcWebClientChannel channel() =>
      GrpcOrGrpcWebClientChannel.toSingleEndpoint(
        host: cleanHost,
        port: port,
        transportSecure: tls,
      );

  /// Metadata for mutating calls: bearer token and optional actor name for the audit log.
  CallOptions authOptions() {
    final md = <String, String>{};
    if (token.isNotEmpty) md['authorization'] = 'Bearer $token';
    if (actor.trim().isNotEmpty) md['x-itcmdr-actor'] = actor.trim();
    return CallOptions(metadata: md);
  }
}
