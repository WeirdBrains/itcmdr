import 'package:app/agents_page.dart';
import 'package:app/proto/itcmdr.pb.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('summary and state colours', () {
    final a = AgentInfo(
      machine: 'WS-01',
      state: 'online',
      role: 'workstation',
      version: '0.17.0',
    );
    expect(agentSummary(a), 'WS-01  online  workstation  v0.17.0');
    expect(agentStateColor('non_responsive'), Colors.red);
    expect(agentStateColor('online'), Colors.green);
  });

  testWidgets('maintenance and revoke buttons, none on revoked', (t) async {
    final calls = <String>[];
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AgentsTable(
            agents: [
              AgentInfo(machine: 'WS-01', state: 'online'),
              AgentInfo(machine: 'WS-02', state: 'maintenance'),
              AgentInfo(machine: 'WS-03', state: 'revoked'),
            ],
            onMaintenance: (a, on) => calls.add('${a.machine}:$on'),
            onRevoke: (a) => calls.add('revoke:${a.machine}'),
          ),
        ),
      ),
    );
    expect(find.text('Revoke'), findsNWidgets(2));
    await t.tap(find.text('End maintenance'));
    await t.tap(find.text('Maintenance'));
    expect(calls, ['WS-02:false', 'WS-01:true']);
  });
}
