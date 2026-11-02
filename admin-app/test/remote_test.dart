// v0.20.0 UI: server Config and Plugins tabs (preview first, then apply), the Plugins page
// (catalog, approvals by a second person), the owners' global freeze and its banner.
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:itcmdr_admin/backend.dart';
import 'package:itcmdr_admin/main.dart';
import 'package:itcmdr_admin/session.dart';

import 'fake.dart';

Future<FakeBackend> start(
  WidgetTester t, {
  Me? me,
  void Function(FakeBackend)? setup,
}) async {
  t.view.physicalSize = const Size(1200, 900);
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  final b = FakeBackend(me: me ?? me0());
  b.siteList = [site('s1', 'Office A', '0.20.0')];
  setup?.call(b);
  await t.pumpWidget(AdminApp(session: Session(b, '/tmp/x')));
  await t.pumpAndSettle();
  return b;
}

Future<void> openServer(WidgetTester t, String tab) async {
  await t.tap(find.text('Servers'));
  await t.pumpAndSettle();
  await t.tap(find.byKey(const Key('site-s1')));
  await t.pumpAndSettle();
  if (tab.isNotEmpty) {
    await t.tap(find.text(tab));
    await t.pumpAndSettle();
  }
}

void main() {
  testWidgets('status shows disk and the last 24 h', (t) async {
    await start(t);
    await openServer(t, '');
    expect(find.byKey(const Key('site-disk')), findsOneWidget);
    expect(find.textContaining('50.0 GB free of 200.0 GB'), findsOneWidget);
    expect(find.textContaining('12 runs, 1 failed'), findsOneWidget);
  });

  testWidgets('config: preview first, then apply; paths stay locked', (
    t,
  ) async {
    final b = await start(t);
    await openServer(t, 'Config');
    expect(find.byKey(const Key('edit-backup.dir')), findsNothing);
    expect(find.textContaining('hidden (stays on the site)'), findsOneWidget);
    await t.tap(find.byKey(const Key('edit-backup.keep_daily')));
    await t.pumpAndSettle();
    await t.enterText(find.byKey(const Key('edit-value')), '9');
    await t.tap(find.byKey(const Key('edit-preview')));
    await t.pumpAndSettle();
    expect(b.calls.last, 'send setting.set key=backup.keep_daily,value=9 dry');
    expect(find.byKey(const Key('preview-dialog')), findsOneWidget);
    expect(find.text('backup.keep_daily: 7 → 9'), findsOneWidget);
    await t.tap(find.byKey(const Key('apply')));
    await t.pumpAndSettle();
    expect(b.calls.last, 'send setting.set key=backup.keep_daily,value=9');
    expect(find.text('applied'), findsOneWidget);
  });

  testWidgets('config: a refused preview sends nothing for real', (t) async {
    final b = await start(t, setup: (b) => b.siteAnswer = (k, dry) => 'failed');
    await openServer(t, 'Config');
    await t.tap(find.byKey(const Key('edit-backup.keep_daily')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('edit-preview')));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('preview-refused')), findsOneWidget);
    expect(find.textContaining('not-allowed'), findsOneWidget);
    expect(b.calls.where((c) => c.startsWith('send')).length, 1);
  });

  testWidgets('schedules: pause by opaque id', (t) async {
    final b = await start(t);
    await openServer(t, 'Config');
    expect(find.byKey(const Key('resume-s-1')), findsNothing);
    await t.tap(find.byKey(const Key('pause-s-1')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('apply')));
    await t.pumpAndSettle();
    expect(b.calls, contains('send schedule.pause schedule=s-1'));
  });

  testWidgets(
    'plugins tab: install from the catalog waits for approval; quarantine',
    (t) async {
      final b = await start(
        t,
        setup: (b) => b.cat = [
          CatalogEntry(
            sha256: 'd' * 64,
            name: 'disk-watch',
            version: '1.3.0',
            size: 2048,
            uploadedBy: 'admin@example.com',
            uploadedMs: 1,
            description: '',
          ),
        ],
      );
      await openServer(t, 'Plugins');
      expect(find.text('bad-signature'), findsOneWidget);
      await t.tap(find.byKey(const Key('push-plugin')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(Key('push-${'d' * 64}')));
      await t.pumpAndSettle();
      expect(
        find.textContaining('second owner or admin must approve'),
        findsOneWidget,
      );
      await t.tap(find.byKey(const Key('apply')));
      await t.pumpAndSettle();
      expect(b.calls.last, 'send plugin.install sha256=${'d' * 64}');
      expect(b.pending, hasLength(1));
      expect(find.textContaining('Waiting for a second owner'), findsOneWidget);
    },
  );

  testWidgets('approvals: the requester cannot approve, another admin can', (
    t,
  ) async {
    CommandRow row(String id, String by) => CommandRow(
      id: id,
      site: 's1',
      kind: 'plugin.enable',
      state: 'awaiting-approval',
      resultCode: '',
      requestedBy: by,
      createdMs: 1,
      dryRun: false,
      preview: const [],
      approvedBy: '',
      args: const ['name=disk-watch'],
    );
    final b = await start(
      t,
      me: me0(role: 'admin'),
      setup: (b) => b.pending = [row('p1', 'u1'), row('p2', 'u7')],
    );
    await t.tap(find.text('Plugins'));
    await t.pumpAndSettle();
    expect(
      t.widget<IconButton>(find.byKey(const Key('approve-p1'))).onPressed,
      isNull,
    );
    await t.tap(find.byKey(const Key('approve-p2')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('confirm')));
    await t.pumpAndSettle();
    expect(b.calls, contains('approve p2'));
    expect(find.byKey(const Key('pending-p2')), findsNothing);
    await t.tap(find.byKey(const Key('reject-p1')));
    await t.pumpAndSettle();
    expect(b.calls, contains('reject p1'));
    // admins cannot freeze
    expect(
      t
          .widget<SwitchListTile>(find.byKey(const Key('freeze-switch')))
          .onChanged,
      isNull,
    );
  });

  testWidgets('SREs see approvals but cannot decide or upload', (t) async {
    await start(
      t,
      me: me0(role: 'sre'),
      setup: (b) =>
          b.pending = [b.sent.isEmpty ? cmd('plugin.enable') : cmd('x')],
    );
    await t.tap(find.text('Plugins'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('approve-c1')), findsNothing);
    expect(find.byKey(const Key('upload-plugin')), findsNothing);
  });

  testWidgets('catalog upload', (t) async {
    final b = await start(t);
    await t.tap(find.text('Plugins'));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('upload-plugin')));
    await t.pumpAndSettle();
    await t.enterText(
      find.byKey(const Key('upload-path')),
      '/tmp/net-watch.tar.gz',
    );
    await t.tap(find.byKey(const Key('upload-ok')));
    await t.pumpAndSettle();
    expect(b.calls, contains('upload /tmp/net-watch.tar.gz'));
    expect(find.text('net-watch v2.0.0'), findsOneWidget);
  });

  testWidgets(
    'v0.21 releases: one approver by default, owner toggles the second',
    (t) async {
      final b = await start(
        t,
        setup: (b) => b.rels = [
          Release(
            sha256: 'a' * 64,
            name: 'itcmdr-agent-release',
            version: '0.21.0',
            approvedBy: const [],
            needed: 1,
            approved: false,
          ),
        ],
      );
      await t.tap(find.text('Plugins'));
      await t.pumpAndSettle();
      expect(find.text('Off: one owner or admin approves'), findsOneWidget);
      await t.tap(find.byKey(Key('approve-release-${'a' * 64}')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('confirm')));
      await t.pumpAndSettle();
      expect(b.calls, contains('approve-release ${'a' * 64}'));
      expect(find.text('1 of 1 approvals (me)'), findsOneWidget);
      expect(find.byKey(Key('rollout-${'a' * 64}')), findsOneWidget);
      await t.tap(find.byKey(const Key('second-approver-switch')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('confirm')));
      await t.pumpAndSettle();
      expect(b.calls, contains('org-policy true'));
      expect(
        find.text('On: two different owners or admins approve'),
        findsOneWidget,
      );
    },
  );

  testWidgets('v0.21 admins cannot change the second-approver setting', (
    t,
  ) async {
    await start(t, me: me0(role: 'admin'));
    await t.tap(find.text('Plugins'));
    await t.pumpAndSettle();
    expect(
      t
          .widget<SwitchListTile>(
            find.byKey(const Key('second-approver-switch')),
          )
          .onChanged,
      isNull,
    );
  });

  testWidgets('freeze: owner turns it on, banner shows, commands refused', (
    t,
  ) async {
    final b = await start(t);
    await t.tap(find.text('Plugins'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('freeze-banner')), findsNothing);
    await t.tap(find.byKey(const Key('freeze-switch')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('confirm')));
    await t.pumpAndSettle();
    expect(b.calls, contains('freeze true'));
    expect(find.byKey(const Key('freeze-banner')), findsOneWidget);
    await openServer(t, 'Config');
    await t.tap(find.byKey(const Key('edit-backup.keep_daily')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('edit-preview')));
    await t.pumpAndSettle();
    expect(find.textContaining('frozen'), findsOneWidget);
    expect(find.byKey(const Key('preview-dialog')), findsNothing);
  });
}
