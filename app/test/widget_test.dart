import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:app/charts.dart';
import 'package:app/exceptions_form.dart';
import 'package:app/logs_page.dart';
import 'package:app/proto/itcmdr.pbgrpc.dart';
import 'package:fixnum/fixnum.dart';
import 'package:app/main.dart';
import 'package:app/server_settings.dart';
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

  test('seriesByBucket sums incidents and averages uptime per bucket', () {
    MetricPoint p(String k, int ts, double v) =>
        MetricPoint(key: k, ts: Int64(ts), value: v);
    final pts = [
      p('incidents:WS-01', 2, 1),
      p('incidents:WS-02', 2, 3),
      p('incidents:WS-01', 1, 2),
      p('incidents_total:WS-01', 9, 99),
      p('uptime:WS-01', 1, 1),
      p('uptime:WS-02', 1, 0.5),
    ];
    final inc = seriesByBucket(pts, 'incidents');
    expect(inc.map((b) => b.ts), [1, 2]);
    expect(inc.map((b) => b.value), [2.0, 4.0]);
    expect(seriesByBucket(pts, 'uptime', average: true).single.value, 0.75);
  });

  test('buildLogsRequest maps filters, whole-day range and paging', () {
    final r = buildLogsRequest(
      query: ' dns ',
      machine: 'WS-01',
      category: 'network',
      status: 'failed',
      from: DateTime(2026, 10, 1, 15),
      to: DateTime(2026, 10, 2),
      offset: 100,
    );
    expect(r.query, 'dns');
    expect(r.category, 'network');
    expect(r.status, 'failed');
    expect(r.offset, 100);
    expect(r.limit, logPageSize);
    expect(r.startTs.toInt(), DateTime(2026, 10, 1).millisecondsSinceEpoch);
    expect(r.endTs.toInt(), DateTime(2026, 10, 3).millisecondsSinceEpoch - 1);
    expect(buildLogsRequest().hasStartTs(), isFalse);
  });

  testWidgets('BarChart renders bars and an empty state', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              BarChart(
                title: 'A',
                bars: [ChartBar(0, 1), ChartBar(86400000, 3)],
              ),
              BarChart(title: 'B', bars: []),
            ],
          ),
        ),
      ),
    );
    expect(find.text('A'), findsOneWidget);
    expect(find.text('No data'), findsOneWidget);
  });

  test('server settings are validated client-side', () {
    SettingItem it(String type, {int min = 0, int max = 0, bool ro = false}) =>
        SettingItem(
          key: 'k',
          type: type,
          min: Int64(min),
          max: Int64(max),
          readOnly: ro,
        );
    expect(validateSettingValue(it('int', min: 1, max: 24), '3'), isNull);
    expect(validateSettingValue(it('int', min: 1, max: 24), '25'), isNotNull);
    expect(validateSettingValue(it('int', min: 1, max: 24), 'x'), isNotNull);
    expect(validateSettingValue(it('bool'), 'true'), isNull);
    expect(validateSettingValue(it('bool'), 'yes'), isNotNull);
    expect(validateSettingValue(it('time'), '01:30'), isNull);
    expect(validateSettingValue(it('time'), '24:00'), isNotNull);
    expect(validateSettingValue(it('text', ro: true), 'x'), isNotNull);
  });

  testWidgets('settings page shows the server settings section', (
    tester,
  ) async {
    final ch = const ServerSettings(host: '127.0.0.1', port: 1).channel();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SettingsPage(
            settings: const ServerSettings(),
            client: ItcmdrClient(ch),
            onSave: (_) async {},
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.text('Server & agent settings'), findsOneWidget);
    expect(find.text('Agents'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    await ch.shutdown();
  });
}
