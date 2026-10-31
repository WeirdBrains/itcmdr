import 'package:grpc/grpc_or_grpcweb.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'proto/itcmdr.pbgrpc.dart';
import 'signing.dart';

/// Server address, the app's signing key (v0.12.0) and the legacy admin token, persisted with
/// shared_preferences. The key's 32-byte private seed is stored on this PC only (user profile).
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

  static Future<ServerSettings> load() async {
    final p = await SharedPreferences.getInstance();
    return ServerSettings(
      host: p.getString(_kHost) ?? 'localhost',
      port: p.getInt(_kPort) ?? 50051,
      tls: p.getBool(_kTls) ?? false,
      token: p.getString(_kToken) ?? '',
      actor: p.getString(_kActor) ?? '',
      keySeed: p.getString(_kKeySeed) ?? '',
    );
  }

  Future<void> save() async {
    final p = await SharedPreferences.getInstance();
    await p.setString(_kHost, host.trim());
    await p.setInt(_kPort, port);
    await p.setBool(_kTls, tls);
    await p.setString(_kToken, token);
    await p.setString(_kActor, actor.trim());
    await p.setString(_kKeySeed, keySeed.trim());
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
