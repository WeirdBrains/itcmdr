// App state: the backend, who is signed in, the step-up policy, and a guard that shows the
// "touch your security key" prompt while a high-risk call waits for the touch.
import 'package:flutter/material.dart';

import 'backend.dart';

class Session extends ChangeNotifier {
  Session(this.backend, this.dir);
  final Backend backend;
  final String dir;
  Me? me;
  Policy? policy;

  bool get isAdmin => me != null && roleRank(me!.role) >= 2;
  bool get isOwner => me?.role == 'owner';

  Future<void> signedIn(Me m) async {
    me = m;
    try {
      policy = await backend.policy();
    } catch (_) {
      policy = null;
    }
    notifyListeners();
  }

  Future<void> refreshMe() async {
    me = await backend.me();
    notifyListeners();
  }

  Future<void> signOut() async {
    await backend.forget(dir);
    me = null;
    policy = null;
    notifyListeners();
  }

  /// True when `action` will ask this user for a passkey touch.
  bool needsTouch(String action) =>
      (me?.passkeys ?? 0) > 0 &&
      (policy?.stepupActions.contains(action) ?? false);

  /// Run `call`; for step-up actions show the touch prompt until it returns. Errors are shown
  /// in a snack bar and rethrown as null.
  Future<T?> guard<T>(
    BuildContext context,
    String action,
    Future<T> Function() call, {
    String? done,
  }) async {
    final messenger = ScaffoldMessenger.of(context);
    final touch = needsTouch(action);
    if (touch) {
      showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (_) => const AlertDialog(
          key: Key('touch-dialog'),
          icon: Icon(Icons.fingerprint, size: 40),
          title: Text('Touch your security key'),
          content: Text(
            'This action needs a fresh touch of your passkey. '
            'Touch the key when it blinks.',
          ),
        ),
      );
    }
    try {
      final r = await call();
      if (done != null) messenger.showSnackBar(SnackBar(content: Text(done)));
      return r;
    } catch (e) {
      messenger.showSnackBar(SnackBar(content: Text(errorText(e))));
      return null;
    } finally {
      if (touch && context.mounted) {
        Navigator.of(context, rootNavigator: true).pop();
      }
    }
  }
}

String errorText(Object e) {
  final s = e.toString();
  return s.startsWith('AnyhowException(') ? s.substring(16, s.length - 1) : s;
}

String ago(int ms) {
  if (ms <= 0) return 'never';
  final d = DateTime.now().difference(DateTime.fromMillisecondsSinceEpoch(ms));
  if (d.inSeconds < 60) return '${d.inSeconds}s ago';
  if (d.inMinutes < 60) return '${d.inMinutes}m ago';
  if (d.inHours < 48) return '${d.inHours}h ago';
  return '${d.inDays}d ago';
}

Color severityColor(String s, ColorScheme c) => switch (s) {
  'critical' => Colors.red.shade700,
  'cautious' => Colors.orange.shade700,
  _ => c.primary,
};

class SessionScope extends InheritedNotifier<Session> {
  const SessionScope({
    super.key,
    required Session session,
    required super.child,
  }) : super(notifier: session);
  static Session of(BuildContext c) =>
      c.dependOnInheritedWidgetOfExactType<SessionScope>()!.notifier!;
}
