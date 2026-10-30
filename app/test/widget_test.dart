import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:app/exceptions_form.dart';
import 'package:app/main.dart';
import 'package:app/settings.dart';

void main() {
  setUp(() => SharedPreferences.setMockInitialValues({}));

  testWidgets('app shell builds', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    await tester.pump();
    await tester.pump();
    expect(find.byType(HomeShell), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump(const Duration(seconds: 5));
  });

  test('settings persist and default to port 50051', () async {
    final d = await ServerSettings.load();
    expect(d.port, 50051);
    await const ServerSettings(
      host: 'http://SERVER01:9999/x',
      port: 50052,
      token: 't0k',
      actor: 'me',
    ).save();
    final s = await ServerSettings.load();
    expect(s.port, 50052);
    expect(s.cleanHost, 'SERVER01');
    expect(s.token, 't0k');
  });

  test('token and actor are sent as metadata', () {
    final o = const ServerSettings(token: 'abc', actor: 'tech').authOptions();
    expect(o.metadata['authorization'], 'Bearer abc');
    expect(o.metadata['x-itcmdr-actor'], 'tech');
    expect(
      const ServerSettings().authOptions().metadata.containsKey(
        'authorization',
      ),
      isFalse,
    );
  });

  test('exception form builds and validates rules', () {
    final r = buildRule(
      machines: 'WS-01, WS-02  RCU-01',
      checkId: ' NET-IP-CORRECT ',
      action: 'ignore',
      reason: 'known',
      expires: '2099-12-31',
    );
    expect(r.scope.machines, ['WS-01', 'WS-02', 'RCU-01']);
    expect(r.checkId, 'NET-IP-CORRECT');
    expect(validateRule(r), isNull);
    expect(
      validateRule(buildRule(machines: '', action: 'ignore', reason: 'x')),
      isNotNull,
    );
    expect(
      validateRule(
        buildRule(machines: '', checkId: 'A', action: 'ignore', reason: ''),
      ),
      isNotNull,
    );
    expect(
      validateRule(
        buildRule(
          machines: '',
          checkId: 'A',
          action: 'ignore',
          reason: 'r',
          expires: 'soon',
        ),
      ),
      isNotNull,
    );
  });

  testWidgets('exception dialog returns an edited rule', (tester) async {
    late Future<dynamic> result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (ctx) {
            return TextButton(
              onPressed: () => result = showExceptionDialog(
                ctx,
                existing: buildRule(
                  id: 'x1',
                  machines: 'WS-01',
                  checkId: 'A',
                  action: 'ignore',
                  reason: 'old',
                ),
              ),
              child: const Text('open'),
            );
          },
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    expect(find.text('Edit exception'), findsOneWidget);
    await tester.enterText(
      find.widgetWithText(TextField, 'Reason'),
      'new reason',
    );
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    final r = await result;
    expect(r.id, 'x1');
    expect(r.reason, 'new reason');
  });
}
