import 'package:app/cert_dialog.dart';
import 'package:app/cert_trust.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

final now = DateTime.utc(2026, 11, 2);
CertInfo cert(int seed, {DateTime? notAfter}) => CertInfo.fromDer(
  [seed, 1, 2, 3],
  subject: 'CN=itcmdr-server',
  issuer: 'CN=itcmdr site CA',
  notBefore: DateTime.utc(2026, 1, 1),
  notAfter: notAfter ?? DateTime.utc(2036, 1, 1),
);

Future<void> host(WidgetTester t, Future<void> Function(BuildContext) f) async {
  await t.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (c) => Scaffold(
          body: TextButton(onPressed: () => f(c), child: const Text('go')),
        ),
      ),
    ),
  );
  await t.tap(find.text('go'));
  await t.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues({}));

  test('trust on first use, exact pin, changed certificate refused', () async {
    final s = TrustStore();
    const srv = 'server.example.com:50051';
    final a = cert(1), b = cert(2);
    expect(a.fingerprint.length, 64);
    expect(s.verdict(srv, a), CertVerdict.unknown);
    expect(s.accept(srv, a), isFalse);
    expect(certProblem.value!.verdict, CertVerdict.unknown);
    s.trust(srv, a, now: now);
    expect(s.accept(srv, a), isTrue);
    expect(s.accept(srv, b), isFalse);
    expect(certProblem.value!.verdict, CertVerdict.changed);
    expect(certProblem.value!.previous!.fingerprint, a.fingerprint);
    // Per server: another server is not covered.
    expect(s.verdict('other.example.com:50051', a), CertVerdict.unknown);
    await s.save();
    final l = await TrustStore.load();
    expect(l.get(srv)!.fingerprint, a.fingerprint);
    l.remove(srv);
    expect(l.verdict(srv, a), CertVerdict.unknown);
    expect(
      cert(1, notAfter: DateTime.utc(2026, 1, 2)).needsOverrideAt(now),
      isTrue,
    );
    expect(a.needsOverrideAt(now), isFalse);
  });

  testWidgets(
    'warning shows the fingerprint and "Trust this certificate" pins it',
    (t) async {
      final s = TrustStore();
      final c = cert(1);
      bool? ok;
      await host(t, (ctx) async {
        ok = await showCertWarning(
          ctx,
          CertProblem('h:1', c, CertVerdict.unknown),
          s,
          now: now,
        );
      });
      expect(find.byKey(const Key('cert-warning')), findsOneWidget);
      expect(find.text(c.grouped), findsOneWidget);
      expect(find.textContaining('CN=itcmdr-server'), findsOneWidget);
      expect(find.textContaining('2036-01-01'), findsOneWidget);
      await t.tap(find.byKey(const Key('cert-trust')));
      await t.pumpAndSettle();
      expect(ok, isTrue);
      expect(s.verdict('h:1', c), CertVerdict.trusted);
    },
  );

  testWidgets('cancel trusts nothing', (t) async {
    final s = TrustStore();
    await host(t, (ctx) async {
      await showCertWarning(
        ctx,
        CertProblem('h:1', cert(1), CertVerdict.unknown),
        s,
        now: now,
      );
    });
    await t.tap(find.byKey(const Key('cert-cancel')));
    await t.pumpAndSettle();
    expect(s.all, isEmpty);
  });

  testWidgets('a changed certificate needs review before replacing', (t) async {
    final s = TrustStore()..trust('h:1', cert(1), now: now);
    final n = cert(2);
    await host(t, (ctx) async {
      await showCertWarning(
        ctx,
        CertProblem('h:1', n, CertVerdict.changed, s.get('h:1')),
        s,
        now: now,
      );
    });
    expect(find.textContaining('WARNING'), findsOneWidget);
    expect(find.textContaining(cert(1).fingerprint), findsOneWidget);
    final btn = find.byKey(const Key('cert-replace'));
    expect(t.widget<FilledButton>(btn).onPressed, isNull);
    await t.tap(btn);
    await t.pumpAndSettle();
    expect(s.verdict('h:1', n), CertVerdict.changed);
    await t.ensureVisible(find.byKey(const Key('cert-confirm')));
    await t.tap(find.byKey(const Key('cert-confirm')));
    await t.pump();
    await t.tap(btn);
    await t.pumpAndSettle();
    expect(s.verdict('h:1', n), CertVerdict.trusted);
  });

  testWidgets('an expired certificate needs an explicit override', (t) async {
    final s = TrustStore();
    final c = cert(3, notAfter: DateTime.utc(2026, 6, 1));
    await host(t, (ctx) async {
      await showCertWarning(
        ctx,
        CertProblem('h:1', c, CertVerdict.unknown),
        s,
        now: now,
      );
    });
    expect(find.byKey(const Key('cert-expired')), findsOneWidget);
    final btn = find.byKey(const Key('cert-trust'));
    expect(find.text('Trust anyway'), findsOneWidget);
    expect(t.widget<FilledButton>(btn).onPressed, isNull);
    await t.tap(find.byKey(const Key('cert-confirm')));
    await t.pump();
    await t.tap(btn);
    await t.pumpAndSettle();
    expect(s.verdict('h:1', c), CertVerdict.trusted);
  });

  testWidgets('trusted certificates are listed and can be removed', (t) async {
    final s = TrustStore()..trust('h:1', cert(1), now: now);
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(body: TrustedCertsSection(store: s)),
      ),
    );
    expect(find.byKey(const Key('cert-h:1')), findsOneWidget);
    await t.tap(find.byKey(const Key('cert-remove-h:1')));
    await t.pumpAndSettle();
    expect(s.all, isEmpty);
    expect(find.byKey(const Key('certs-none')), findsOneWidget);
  });
}
