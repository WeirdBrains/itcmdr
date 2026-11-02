import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:itcmdr_admin/backend.dart';
import 'package:itcmdr_admin/main.dart';
import 'package:itcmdr_admin/session.dart';

import 'fake.dart';

Future<FakeBackend> start(
  WidgetTester t, {
  Me? me,
  Size size = const Size(1200, 900),
  void Function(FakeBackend)? setup,
}) async {
  t.view.physicalSize = size;
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  final b = FakeBackend(me: me);
  setup?.call(b);
  await t.pumpWidget(AdminApp(session: Session(b, '/tmp/x')));
  await t.pumpAndSettle();
  return b;
}

void main() {
  testWidgets('onboarding: enroll, the 24 words once, then home', (t) async {
    final b = await start(t);
    expect(find.text('Enroll this device'), findsOneWidget);
    await t.enterText(find.byKey(const Key('code')), ' itcmdr-admin:abc ');
    await t.tap(find.byKey(const Key('backup')));
    await t.tap(find.byKey(const Key('enroll')));
    await t.pumpAndSettle();
    expect(b.calls, contains('enroll itcmdr-admin:abc false'));
    expect(find.text('Your 24 words'), findsOneWidget);
    expect(find.text('24. w23'), findsOneWidget);
    final cont = t.widget<FilledButton>(find.byKey(const Key('continue')));
    expect(
      cont.onPressed,
      isNull,
      reason: 'continue only after "I wrote them down"',
    );
    await t.tap(find.byKey(const Key('wrote')));
    await t.pump();
    await t.tap(find.byKey(const Key('continue')));
    await t.pumpAndSettle();
    expect(find.textContaining('itcmdr Admin · Alerts'), findsOneWidget);
    expect(find.text('Your 24 words'), findsNothing);
  });

  testWidgets('onboarding: a bad code shows the error', (t) async {
    await start(t, setup: (b) => b.enrollError = Exception('code expired'));
    await t.enterText(find.byKey(const Key('code')), 'itcmdr-admin:old');
    await t.tap(find.byKey(const Key('enroll')));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('error')), findsOneWidget);
    expect(find.textContaining('code expired'), findsOneWidget);
  });

  testWidgets('roles: SREs have no Users tab; owners do', (t) async {
    await start(t, me: me0(role: 'sre'));
    expect(find.text('Users'), findsNothing);
    expect(find.text('Servers'), findsOneWidget);
  });

  testWidgets('owner sees Users and can edit the policy', (t) async {
    await start(t, me: me0());
    expect(find.text('Users'), findsOneWidget);
    await t.tap(find.text('Settings'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('edit-policy')), findsOneWidget);
    expect(find.byKey(const Key('push-note')), findsOneWidget);
    expect(
      find.byKey(const Key('push-enable')),
      findsNothing,
      reason: 'no push SDK in this build',
    );
    await t.tap(find.byKey(const Key('push-test')));
    await t.pumpAndSettle();
    expect(
      find.textContaining('push is not configured on adminsvr'),
      findsOneWidget,
    );
    await t.tap(find.byKey(const Key('verify-audit')));
    await t.pumpAndSettle();
    expect(find.text('ok (1 entries)'), findsOneWidget);
  });

  testWidgets('admin cannot edit the policy', (t) async {
    await start(t, me: me0(role: 'admin'));
    await t.tap(find.text('Settings'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('edit-policy')), findsNothing);
    expect(find.byKey(const Key('policy-actions')), findsOneWidget);
  });

  testWidgets(
    'alerts: ack with a passkey asks for a touch while the call waits',
    (t) async {
      final b = await start(
        t,
        me: me0(passkeys: 1, authenticator: true),
        setup: (b) {
          b.alertList = [
            alert('a1', 'critical'),
            alert('a2', 'cautious', state: 'resolved'),
          ];
        },
      );
      expect(find.byKey(const Key('alert-a1')), findsOneWidget);
      expect(
        find.byKey(const Key('alert-a2')),
        findsNothing,
        reason: 'resolved is not in Open',
      );
      b.gate = Completer();
      await t.tap(find.byKey(const Key('alert-menu-a1')));
      await t.pumpAndSettle();
      await t.tap(find.text('Acknowledge'));
      await t.pump();
      await t.pump();
      expect(find.byKey(const Key('touch-dialog')), findsOneWidget);
      b.gate!.complete();
      await t.pumpAndSettle();
      expect(find.byKey(const Key('touch-dialog')), findsNothing);
      expect(b.calls, contains('ack s1 a1'));
      expect(find.text('Acknowledge sent to the site'), findsOneWidget);
    },
  );

  testWidgets('alerts: no passkey, no touch prompt; resolve', (t) async {
    final b = await start(
      t,
      me: me0(),
      setup: (b) => b.alertList = [alert('a1', 'critical')],
    );
    await t.tap(find.byKey(const Key('alert-menu-a1')));
    await t.pumpAndSettle();
    await t.tap(find.text('Resolve'));
    await t.pump();
    expect(find.byKey(const Key('touch-dialog')), findsNothing);
    await t.pumpAndSettle();
    expect(b.calls, contains('resolve s1 a1'));
  });

  testWidgets('alerts: a new open alert shows a banner', (t) async {
    final b = await start(
      t,
      me: me0(),
      setup: (b) => b.alertList = [alert('a1', 'cautious')],
    );
    b.alertList = [...b.alertList, alert('a9', 'critical')];
    await t.pump(const Duration(seconds: 31));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('new-alerts')), findsOneWidget);
    expect(find.text('1 new alert(s), 1 critical'), findsOneWidget);
  });

  testWidgets(
    'servers: versions, update available, configuration, metrics, backup',
    (t) async {
      final b = await start(
        t,
        me: me0(),
        setup: (b) {
          b.siteList = [
            site(
              's1',
              'Office A',
              '0.19.1',
              machines: [
                const Machine(
                  id: 'm-1',
                  state: 'online',
                  lastSeenMs: 1,
                  version: '0.19.0',
                  displayName: '',
                  updateAvailable: true,
                ),
                const Machine(
                  id: 'm-2',
                  state: 'online',
                  lastSeenMs: 1,
                  version: '0.19.1',
                  displayName: '',
                  updateAvailable: false,
                ),
              ],
            ),
            site('s2', 'Office B', '0.19.2'),
          ];
        },
      );
      await t.tap(find.text('Servers'));
      await t.pumpAndSettle();
      expect(
        find.descendant(
          of: find.byKey(const Key('site-s1')),
          matching: find.text('update available'),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('site-s2')),
          matching: find.text('update available'),
        ),
        findsNothing,
      );
      await t.tap(find.byKey(const Key('site-s1')));
      await t.pumpAndSettle();
      expect(find.byKey(const Key('site-update')), findsOneWidget);
      expect(find.text('signed'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const Key('machine-m-1')),
          matching: find.text('update available'),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('machine-m-2')),
          matching: find.text('update available'),
        ),
        findsNothing,
      );
      await t.tap(find.text('Metrics'));
      await t.pumpAndSettle();
      expect(find.byKey(const Key('chart-agents')), findsOneWidget);
      await t.tap(find.byKey(const Key('backup')));
      await t.pumpAndSettle();
      expect(b.calls, contains('backup s1'));
      await t.tap(find.byKey(const Key('drill')));
      await t.pumpAndSettle();
      expect(b.calls, contains('send backup.drill '));
    },
  );

  testWidgets('servers: admins create a site and get the token', (t) async {
    final b = await start(t, me: me0(role: 'admin'));
    await t.tap(find.text('Servers'));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('add-site')));
    await t.pumpAndSettle();
    await t.enterText(find.byKey(const Key('site-name')), 'Office C');
    await t.tap(find.text('Create'));
    await t.pumpAndSettle();
    expect(b.calls, contains('createSite Office C'));
    expect(find.text('site-token'), findsOneWidget);
  });

  testWidgets('servers: SREs cannot add sites', (t) async {
    await start(t, me: me0(role: 'sre'));
    await t.tap(find.text('Servers'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('add-site')), findsNothing);
  });

  testWidgets(
    'users: role change asks for a touch; approve a pending device; scopes',
    (t) async {
      final b = await start(
        t,
        me: me0(passkeys: 1, authenticator: true),
        setup: (b) {
          b.siteList = [site('s1', 'Office A', '0.19.2')];
          b.userList = [
            user('u1', 'owner@example.com', 'owner'),
            user(
              'u2',
              'sre@example.com',
              'sre',
              devices: [
                const Device(
                  fingerprint: 'abcdef0123456789ff',
                  platform: 'app-android',
                  status: 'pending',
                  createdMs: 1,
                  lastUsedMs: 0,
                ),
              ],
            ),
          ];
        },
      );
      await t.tap(find.text('Users'));
      await t.pumpAndSettle();
      expect(
        find.textContaining('device waiting for approval'),
        findsOneWidget,
      );
      await t.tap(find.byKey(const Key('user-sre@example.com')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('scope-s1')));
      await t.pumpAndSettle();
      expect(b.calls, contains('setScopes u2 s1'));
      b.gate = Completer();
      await t.scrollUntilVisible(
        find.byKey(const Key('approve-abcdef0123456789ff')),
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await t.tap(find.byKey(const Key('approve-abcdef0123456789ff')));
      await t.pump();
      await t.pump();
      expect(find.byKey(const Key('touch-dialog')), findsOneWidget);
      b.gate!.complete();
      await t.pumpAndSettle();
      expect(b.calls, contains('approve u2 abcdef0123456789ff'));
      expect(find.text('Device approved'), findsOneWidget);
      await t.scrollUntilVisible(
        find.byKey(const Key('role')),
        -200,
        scrollable: find.byType(Scrollable).last,
      );
      await t.tap(find.byKey(const Key('role')));
      await t.pumpAndSettle();
      await t.tap(find.text('Admin').last);
      await t.pumpAndSettle();
      expect(b.calls, contains('setRole u2 admin'));
    },
  );

  testWidgets('users: an admin cannot edit an owner', (t) async {
    await start(
      t,
      me: me0(role: 'admin'),
      setup: (b) {
        b.userList = [
          user('u9', 'boss@example.com', 'owner'),
          user('u1', 'owner@example.com', 'admin'),
        ];
      },
    );
    await t.tap(find.text('Users'));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('user-boss@example.com')));
    await t.pumpAndSettle();
    final dd = t.widget<DropdownButton<String>>(find.byKey(const Key('role')));
    expect(dd.onChanged, isNull);
    expect(find.byKey(const Key('disabled')), findsNothing);
    expect(find.byKey(const Key('new-code')), findsNothing);
  });

  testWidgets('phone layout: bottom navigation', (t) async {
    await start(t, me: me0(), size: const Size(400, 800));
    expect(find.byType(NavigationBar), findsOneWidget);
    expect(find.byType(NavigationRail), findsNothing);
  });

  testWidgets('forget this device returns to onboarding', (t) async {
    final b = await start(t, me: me0());
    await t.tap(find.text('Settings'));
    await t.pumpAndSettle();
    await t.scrollUntilVisible(
      find.byKey(const Key('forget')),
      300,
      scrollable: find.byType(Scrollable).last,
    );
    await t.tap(find.byKey(const Key('forget')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('confirm')));
    await t.pumpAndSettle();
    expect(b.calls, contains('forget'));
    expect(find.text('Enroll this device'), findsOneWidget);
  });
}
