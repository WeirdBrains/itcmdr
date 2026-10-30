import 'package:grpc/grpc_or_grpcweb.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Server address and admin token, persisted with shared_preferences.
class ServerSettings {
  final String host;
  final int port;
  final bool tls;
  final String token;
  final String actor;

  const ServerSettings({
    this.host = 'localhost',
    this.port = 50051,
    this.tls = false,
    this.token = '',
    this.actor = '',
  });

  static const _kHost = 'server.host';
  static const _kPort = 'server.port';
  static const _kTls = 'server.tls';
  static const _kToken = 'server.token';
  static const _kActor = 'server.actor';

  static Future<ServerSettings> load() async {
    final p = await SharedPreferences.getInstance();
    return ServerSettings(
      host: p.getString(_kHost) ?? 'localhost',
      port: p.getInt(_kPort) ?? 50051,
      tls: p.getBool(_kTls) ?? false,
      token: p.getString(_kToken) ?? '',
      actor: p.getString(_kActor) ?? '',
    );
  }

  Future<void> save() async {
    final p = await SharedPreferences.getInstance();
    await p.setString(_kHost, host.trim());
    await p.setInt(_kPort, port);
    await p.setBool(_kTls, tls);
    await p.setString(_kToken, token);
    await p.setString(_kActor, actor.trim());
  }

  ServerSettings copyWith({
    String? host,
    int? port,
    bool? tls,
    String? token,
    String? actor,
  }) => ServerSettings(
    host: host ?? this.host,
    port: port ?? this.port,
    tls: tls ?? this.tls,
    token: token ?? this.token,
    actor: actor ?? this.actor,
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
