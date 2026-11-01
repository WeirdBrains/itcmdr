// itcmdr Admin (v0.20.1): the adminsvr app for owners, admins and SREs (mobile and desktop).
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
    }
    if (mounted) setState(() => loading = false);
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
