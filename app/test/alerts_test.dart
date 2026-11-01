import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:app/alerts_page.dart';
import 'package:app/proto/itcmdr.pb.dart';

void main() {
  test('badge counts critical and cautious only', () {
    final r = ListAlertsResponse(
      openCritical: Int64(2),
      openCautious: Int64(1),
      openInformational: Int64(9),
    );
    expect(alertBadgeCount(r), 3);
    expect(alertBadgeCount(ListAlertsResponse()), 0);
  });

  test('summary shows level, machine and repeat count', () {
    final a = Alert(
      level: 'critical',
      machine: 'WS-01',
      title: 'x failed',
      count: Int64(3),
    );
    expect(alertSummary(a), 'CRITICAL  WS-01  x failed (x3)');
    expect(
      alertSummary(Alert(level: 'cautious', title: 'y', count: Int64(1))),
      'CAUTIOUS  y',
    );
  });

  testWidgets('table offers ack only on open, nothing on resolved', (t) async {
    final acked = <String>[];
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AlertsTable(
            alerts: [
              Alert(
                id: '1',
                level: 'critical',
                state: 'open',
                title: 'a',
                count: Int64(1),
              ),
              Alert(
                id: '2',
                level: 'cautious',
                state: 'acked',
                title: 'b',
                count: Int64(1),
              ),
              Alert(
                id: '3',
                level: 'cautious',
                state: 'resolved',
                title: 'c',
                count: Int64(1),
              ),
            ],
            onAck: (a) => acked.add(a.id),
            onResolve: (_) {},
          ),
        ),
      ),
    );
    expect(find.text('Ack'), findsOneWidget);
    expect(find.text('Resolve'), findsNWidgets(2));
    await t.tap(find.text('Ack'));
    expect(acked, ['1']);
  });
}
