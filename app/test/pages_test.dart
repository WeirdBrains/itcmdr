import 'package:app/backup_page.dart';
import 'package:app/keys_page.dart';
import 'package:app/proto/itcmdr.pbgrpc.dart';
import 'package:app/schedules_page.dart';
import 'package:app/server_settings.dart';
import 'package:app/settings.dart';
import 'package:app/signing.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'settings_doc.dart';

void main() {
  test('checkWords wants 24 letter words', () {
    expect(checkWords(''), isNotNull);
    expect(checkWords(List.filled(23, 'abandon').join(' ')), contains('23'));
    expect(
      checkWords('${List.filled(23, 'abandon').join(' ')} ab3'),
      'Words are letters only',
    );
    expect(checkWords('  ${List.filled(24, 'Abandon').join('   ')} '), isNull);
  });

  test('schedule form and delete confirmation', () {
    expect(
      validateScheduleForm(Schedule()..when = 'daily 18:30'),
      'Give the schedule a name',
    );
    expect(
      validateScheduleForm(Schedule()..name = 'Evening'),
      contains('when'),
    );
    expect(
      validateScheduleForm(
        Schedule()
          ..name = 'Evening'
          ..when = 'daily 18:30'
          ..allMachines = false,
      ),
      contains('machines'),
    );
    expect(
      validateScheduleForm(
        Schedule()
          ..name = 'Evening'
          ..when = 'daily 18:30'
          ..allMachines = true,
      ),
      isNull,
    );
    expect(
      validateScheduleForm(
        Schedule()
          ..name = 'Nightly backup'
          ..when = 'daily 02:00'
          ..action = 'backup'
          ..allMachines = false,
      ),
      isNull,
    );
    final builtin = Schedule()
      ..name = 'Nightly backup'
      ..builtin = true;
    expect(deleteAllowed(builtin, 'Nightly backup'), isTrue);
    expect(deleteAllowed(builtin, 'nightly backup'), isFalse);
    expect(deleteAllowed(Schedule()..name = 'Evening', 'nope'), isTrue);
  });

  test('key import and the certify command', () async {
    final k = await AppKey.generate();
    expect(parseKeyInput(hexOf(k.seed)), hexOf(k.seed));
    expect(parseKeyInput('', fileBytes: k.seed), hexOf(k.seed));
    expect(() => parseKeyInput('abcd'), throwsFormatException);
    expect(
      () => parseKeyInput('', fileBytes: [1, 2, 3]),
      throwsFormatException,
    );
    final cmd = certifyCommand('Front desk', 'owner', k.publicHex);
    expect(cmd, contains('--name "Front desk"'));
    expect(cmd, contains('--role owner'));
    expect(cmd, contains(k.publicHex));
  });

  test(
    'every documented setting default is accepted, and search finds them',
    () {
      final all = settingsFromReference();
      expect(all, hasLength(37));
      expect(all.map((s) => s.key).toSet(), hasLength(37));
      for (final s in all) {
        if (s.readOnly) {
          expect(validateSettingValue(s, s.value), isNotNull, reason: s.key);
        } else {
          expect(validateSettingValue(s, s.value), isNull, reason: s.key);
        }
      }
      expect(all.where((s) => s.type == 'bool').map((s) => s.key), [
        'backup.nightly',
        'plugins.enable_on_install',
      ]);
      expect(visibleSettings(all, 'server', 'auth_mode').map((s) => s.key), [
        'server.auth_mode',
      ]);
      expect(visibleSettings(all, 'agent', 'backup').map((s) => s.key), [
        'agent.backupMaxAgeHours',
        'agent.backupMinBytes',
      ]);
      expect(
        visibleSettings(all, 'server', '').length,
        all.where((s) => s.scope == 'server').length,
      );
    },
  );

  testWidgets('schedule dialog blocks an empty name and returns a schedule', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1600, 1400));
    late Future<Schedule?> result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (ctx) => TextButton(
            onPressed: () => result = showDialog<Schedule>(
              context: ctx,
              builder: (_) => const ScheduleDialog(),
            ),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    expect(find.text('New schedule'), findsOneWidget);
    await tester.enterText(find.byKey(const Key('s-name')), '');
    await tester.tap(find.byKey(const Key('s-save')));
    await tester.pump();
    expect(find.text('Give the schedule a name'), findsOneWidget);
    await tester.enterText(find.byKey(const Key('s-name')), 'Evening');
    await tester.tap(find.byKey(const Key('s-save')));
    await tester.pumpAndSettle();
    final s = await result;
    expect(s, isNotNull);
    expect(s!.name, 'Evening');
    expect(s.when, 'daily 18:30');
    expect(s.allMachines, isTrue);
  });

  testWidgets('restore dialog rejects words before calling the server', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1600, 1200));
    final ch = const ServerSettings(host: '127.0.0.1', port: 1).channel();
    addTearDown(ch.shutdown);
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (ctx) => TextButton(
            onPressed: () => showDialog<void>(
              context: ctx,
              builder: (_) => RestoreDialog(
                client: ItcmdrClient(ch),
                backup: BackupInfo(name: 'nightly.itcb'),
              ),
            ),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    expect(find.text('Restore nightly.itcb'), findsOneWidget);
    await tester.enterText(find.byKey(const Key('restore-words')), 'not words');
    await tester.tap(find.byKey(const Key('restore-dry')));
    await tester.pump();
    expect(find.byKey(const Key('restore-error')), findsOneWidget);
    expect(find.text('Dry run'), findsOneWidget);
    expect(find.byKey(const Key('restore-go')), findsNothing);
  });

  testWidgets('keys page shows the fingerprint and confirms a replacement', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1600, 1200));
    final key = await AppKey.generate();
    final ch = const ServerSettings(host: '127.0.0.1', port: 1).channel();
    addTearDown(ch.shutdown);
    ServerSettings? saved;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: KeysPage(
            client: ItcmdrClient(ch),
            settings: ServerSettings(
              host: '127.0.0.1',
              port: 1,
              keySeed: hexOf(key.seed),
            ),
            onSave: (s) async => saved = s,
          ),
        ),
      ),
    );
    await _until(tester, find.byKey(const Key('key-fp')));
    expect(find.textContaining(key.fingerprint), findsWidgets);
    await tester.tap(find.byKey(const Key('key-generate')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(find.text('Replace this key?'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pump();
    expect(saved, isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('settings search and bool switches cover the registry', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1600, 1400));
    final all = settingsFromReference();
    final ch = const ServerSettings(host: '127.0.0.1', port: 1).channel();
    addTearDown(ch.shutdown);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ListView(
            children: [
              ServerSettingsSection(
                client: ItcmdrClient(ch),
                settings: const ServerSettings(),
                initialItems: all,
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.byKey(const Key('settings-search')), findsOneWidget);
    expect(find.byType(Switch), findsNWidgets(2));
    expect(
      find.byKey(const Key('setting-bool-backup.nightly')),
      findsOneWidget,
    );
    expect(tester.widget<Switch>(find.byType(Switch).first).value, isTrue);
    await tester.enterText(
      find.byKey(const Key('settings-search')),
      'auth_mode',
    );
    await tester.pump();
    expect(find.text('server.auth_mode'), findsOneWidget);
    expect(find.byIcon(Icons.lock), findsOneWidget);
    expect(find.byType(Switch), findsNothing);
    expect(find.text('agent.quietMinutes'), findsNothing);
  });
}

Future<void> _until(WidgetTester tester, Finder f) async {
  for (var i = 0; i < 50 && f.evaluate().isEmpty; i++) {
    await tester.pump(const Duration(milliseconds: 50));
  }
  expect(f, findsOneWidget);
}
