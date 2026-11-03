// v1.1.0 certificate trust for the site app: HTTPS validated by the system trust store first.
// When that fails (self-signed, private CA, expired, wrong name) the app shows the certificate
// (SHA-256, subject, expiry) and the user may trust exactly that certificate for that one server
// (trust on first use). A different certificate later is refused until the user reviews it.
import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

String serverKey(String host, int port) => '${host.trim().toLowerCase()}:$port';

String fmtDate(DateTime? d) {
  if (d == null) return '?';
  String two(int n) => n.toString().padLeft(2, '0');
  final u = d.toUtc();
  return '${u.year}-${two(u.month)}-${two(u.day)}';
}

/// What the user sees before trusting a certificate.
class CertInfo {
  final String fingerprint;
  final String subject;
  final String issuer;
  final DateTime? notBefore;
  final DateTime? notAfter;
  const CertInfo({
    required this.fingerprint,
    this.subject = '',
    this.issuer = '',
    this.notBefore,
    this.notAfter,
  });

  factory CertInfo.fromDer(
    List<int> der, {
    String subject = '',
    String issuer = '',
    DateTime? notBefore,
    DateTime? notAfter,
  }) => CertInfo(
    fingerprint: sha256.convert(der).toString(),
    subject: subject,
    issuer: issuer,
    notBefore: notBefore,
    notAfter: notAfter,
  );

  bool expiredAt(DateTime now) => notAfter != null && now.isAfter(notAfter!);
  bool notYetValidAt(DateTime now) =>
      notBefore != null && now.isBefore(notBefore!);

  /// Expired or not yet valid: trusting it needs an explicit "trust anyway".
  bool needsOverrideAt(DateTime now) => expiredAt(now) || notYetValidAt(now);

  /// The fingerprint in groups of four for reading aloud / comparing.
  String get grouped {
    final b = StringBuffer();
    for (var i = 0; i < fingerprint.length; i += 4) {
      if (i > 0) b.write(' ');
      b.write(fingerprint.substring(i, (i + 4).clamp(0, fingerprint.length)));
    }
    return b.toString();
  }
}

class TrustedCert {
  final String server;
  final String fingerprint;
  final String subject;
  final String notAfter;
  final int addedMs;
  const TrustedCert({
    required this.server,
    required this.fingerprint,
    this.subject = '',
    this.notAfter = '',
    this.addedMs = 0,
  });
  Map<String, Object> toJson() => {
    'server': server,
    'fingerprint': fingerprint,
    'subject': subject,
    'notAfter': notAfter,
    'addedMs': addedMs,
  };
  factory TrustedCert.fromJson(Map<String, dynamic> j) => TrustedCert(
    server: j['server'] as String? ?? '',
    fingerprint: j['fingerprint'] as String? ?? '',
    subject: j['subject'] as String? ?? '',
    notAfter: j['notAfter'] as String? ?? '',
    addedMs: (j['addedMs'] as num?)?.toInt() ?? 0,
  );
}

enum CertVerdict { trusted, unknown, changed }

/// A certificate the app refused; shown to the user.
class CertProblem {
  final String server;
  final CertInfo cert;
  final CertVerdict verdict;
  final TrustedCert? previous;
  const CertProblem(this.server, this.cert, this.verdict, [this.previous]);
}

/// The last refused certificate (the home screen shows the warning).
final certProblem = ValueNotifier<CertProblem?>(null);

/// Trusted certificates, one per server (host:port), kept in the app's preferences.
class TrustStore {
  static const prefsKey = 'tls.trusted';
  final Map<String, TrustedCert> _m;
  TrustStore([Map<String, TrustedCert>? m]) : _m = m ?? {};

  static TrustStore instance = TrustStore();

  static Future<TrustStore> load() async {
    final p = await SharedPreferences.getInstance();
    final raw = p.getString(prefsKey);
    final m = <String, TrustedCert>{};
    if (raw != null && raw.isNotEmpty) {
      try {
        for (final e in (jsonDecode(raw) as List)) {
          final t = TrustedCert.fromJson(e as Map<String, dynamic>);
          if (t.server.isNotEmpty && t.fingerprint.length == 64) {
            m[t.server] = t;
          }
        }
      } catch (_) {}
    }
    return instance = TrustStore(m);
  }

  Future<void> save() async {
    final p = await SharedPreferences.getInstance();
    await p.setString(
      prefsKey,
      jsonEncode([for (final t in _m.values) t.toJson()]),
    );
  }

  List<TrustedCert> get all =>
      _m.values.toList()..sort((a, b) => a.server.compareTo(b.server));
  TrustedCert? get(String server) => _m[server];

  /// Called only after the system trust store refused the certificate.
  CertVerdict verdict(String server, CertInfo c) {
    final t = _m[server];
    if (t == null) return CertVerdict.unknown;
    return t.fingerprint == c.fingerprint
        ? CertVerdict.trusted
        : CertVerdict.changed;
  }

  void trust(String server, CertInfo c, {DateTime? now}) {
    _m[server] = TrustedCert(
      server: server,
      fingerprint: c.fingerprint,
      subject: c.subject,
      notAfter: fmtDate(c.notAfter),
      addedMs: (now ?? DateTime.now()).millisecondsSinceEpoch,
    );
  }

  void remove(String server) => _m.remove(server);

  /// The check used by the HTTPS connection: true = accept this certificate.
  bool accept(String server, CertInfo c) {
    final v = verdict(server, c);
    if (v == CertVerdict.trusted) return true;
    certProblem.value = CertProblem(server, c, v, _m[server]);
    return false;
  }
}
