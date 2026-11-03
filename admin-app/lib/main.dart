// itcmdr Admin (v0.21.0): the adminsvr app for owners, admins and SREs (mobile and desktop).
// Sign-in is the device key from itcmdr-keys onboarding; every request is signed, adminsvr's TLS
// key is pinned, and high-risk actions ask for a passkey touch (desktop security keys).
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import 'backend.dart';
import 'home.dart';
import 'pages/onboarding.dart';
import 'session.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final dir = '${(await getApplicationSupportDirectory()).path}/identity';
  runApp(AdminApp(session: Session(RustBackend(), dir)));
}

class AdminApp extends StatefulWidget {
  const AdminApp({super.key, required this.session});
  final Session session;
  @override
  State<AdminApp> createState() => _AdminAppState();
}

class _AdminAppState extends State<AdminApp> {
  bool loading = true;
  String? error;
  CertReport? cert;
  bool certConfirmed = false;

  @override
  void initState() {
    super.initState();
    _open();
  }

  Future<void> _open() async {
    setState(() {
      loading = true;
      error = null;
    });
    try {
      final m = await widget.session.backend.open(widget.session.dir);
      if (m != null) await widget.session.signedIn(m);
    } catch (e) {
      error = errorText(e);
      // v1.1.0: a refused certificate is shown with its details, never silently accepted.
      try {
        cert = await widget.session.backend.certCheck(widget.session.dir);
      } catch (_) {
        cert = null;
      }
      certConfirmed = false;
    }
    if (mounted) setState(() => loading = false);
  }

  Future<void> _trust(CertReport c) async {
    try {
      await widget.session.backend.trustCert(
        widget.session.dir,
        c.fingerprint,
        c.problem == 'pin-changed',
        c.needsOverride,
      );
      cert = null;
      await _open();
    } catch (e) {
      setState(() => error = errorText(e));
    }
  }

  Widget _certScreen(CertReport c) {
    final changed = c.problem == 'pin-changed';
    final needConfirm = changed || c.needsOverride;
    String day(int ms) => ms <= 0
        ? '?'
        : DateTime.fromMillisecondsSinceEpoch(
            ms,
            isUtc: true,
          ).toIso8601String().substring(0, 10);
    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: ListView(
            shrinkWrap: true,
            padding: const EdgeInsets.all(16),
            children: [
              Text(
                changed
                    ? 'WARNING: the certificate of ${c.server} has changed'
                    : 'Certificate not trusted: ${c.server}',
                key: const Key('cert-title'),
                style: TextStyle(
                  fontSize: 20,
                  color: changed ? Colors.red : null,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                changed
                    ? 'adminsvr presents a different certificate than the one this device '
                          'trusts. Someone may be intercepting the connection. Check the new '
                          'fingerprint with the server (itcmdr-adminsvr tls-status) before '
                          'replacing the trusted one.'
                    : 'This device does not trust the certificate (${c.problem}). Compare the '
                          'fingerprint with the server (itcmdr-adminsvr tls-status) first.',
              ),
              if (changed) Text('Trusted before: ${c.trustedBefore}'),
              if (c.needsOverride)
                Text(
                  'This certificate is ${c.problem}.',
                  key: const Key('cert-override'),
                  style: const TextStyle(color: Colors.red),
                ),
              const SizedBox(height: 8),
              const Text('SHA-256 fingerprint:'),
              SelectableText(
                c.fingerprint,
                key: const Key('cert-fingerprint'),
                style: const TextStyle(fontFamily: 'monospace'),
              ),
              Text('Subject: ${c.subject}'),
              Text('Issuer: ${c.issuer}'),
              Text('Expires: ${day(c.notAfterMs)}'),
              if (needConfirm)
                CheckboxListTile(
                  key: const Key('cert-confirm'),
                  contentPadding: EdgeInsets.zero,
                  value: certConfirmed,
                  onChanged: (v) => setState(() => certConfirmed = v ?? false),
                  title: Text(
                    changed
                        ? 'I checked the new fingerprint with the server'
                        : 'I understand the risk; trust it anyway',
                  ),
                ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  OutlinedButton(
                    key: const Key('cert-cancel'),
                    onPressed: () => setState(() => cert = null),
                    child: const Text('Cancel'),
                  ),
                  FilledButton(
                    key: Key(changed ? 'cert-replace' : 'cert-trust'),
                    onPressed: needConfirm && !certConfirmed
                        ? null
                        : () => _trust(c),
                    child: Text(
                      changed
                          ? 'Replace trusted certificate'
                          : c.needsOverride
                          ? 'Trust anyway'
                          : 'Trust this certificate',
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final s = widget.session;
    return SessionScope(
      session: s,
      child: MaterialApp(
        title: 'itcmdr Admin',
        theme: ThemeData(colorSchemeSeed: Colors.indigo, useMaterial3: true),
        darkTheme: ThemeData(
          colorSchemeSeed: Colors.indigo,
          brightness: Brightness.dark,
          useMaterial3: true,
        ),
        home: ListenableBuilder(
          listenable: s,
          builder: (context, _) {
            if (loading) {
              return const Scaffold(
                body: Center(child: CircularProgressIndicator()),
              );
            }
            if (cert != null && s.me == null) return _certScreen(cert!);
            if (error != null && s.me == null) {
              return Scaffold(
                body: Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Padding(
                        padding: const EdgeInsets.all(16),
                        child: Text(error!, key: const Key('open-error')),
                      ),
                      FilledButton(
                        onPressed: _open,
                        child: const Text('Retry'),
                      ),
                    ],
                  ),
                ),
              );
            }
            return s.me == null ? const OnboardingPage() : const Home();
          },
        ),
      ),
    );
  }
}
