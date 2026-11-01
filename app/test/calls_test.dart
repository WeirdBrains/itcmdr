import 'package:app/calls_page.dart';
import 'package:app/proto/itcmdr.pbgrpc.dart';
import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('calls request carries the filters', () {
    final now = DateTime(2026, 10, 31, 12);
    final r = buildCallsRequest(
      method: ' ListAudit ',
      principal: 'anonymous',
      errorsOnly: true,
      hours: 24,
      now: now,
    );
    expect(r.method, 'ListAudit');
    expect(r.principal, 'anonymous');
    expect(r.errorsOnly, isTrue);
    expect(
      r.sinceMs.toInt(),
      now.subtract(const Duration(hours: 24)).millisecondsSinceEpoch,
    );
    expect(r.hasStatus(), isFalse);
    expect(buildCallsRequest().hasSinceMs(), isFalse);
  });

  testWidgets('calls table shows rows and details', (tester) async {
    final c = CallEntry(
      id: 'c-1',
      ts: Int64(DateTime(2026, 10, 31, 9).millisecondsSinceEpoch),
      service: 'itcmdr.Itcmdr',
      method: 'ListAudit',
      kind: 'unary',
      peer: '127.0.0.1:5000',
      principal: 'anonymous',
      status: 16,
      statusName: 'UNAUTHENTICATED',
      durationMs: Int64(3),
      error: 'missing x-itcmdr-key',
    );
    expect(callSummary(c), 'ListAudit  UNAUTHENTICATED  3 ms  anonymous');
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: CallsTable(calls: [c])),
      ),
    );
    expect(find.textContaining('UNAUTHENTICATED'), findsOneWidget);
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
    await tester.tap(find.textContaining('ListAudit'));
    await tester.pumpAndSettle();
    expect(find.text('c-1'), findsOneWidget);
    expect(find.text('missing x-itcmdr-key'), findsWidgets);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: CallsTable(calls: [])),
      ),
    );
    expect(find.text('No calls'), findsOneWidget);
  });
}
