import 'package:app/proto/itcmdr.pb.dart';
import 'package:app/updates_page.dart';
import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

AgentRollout ro(
  String state, {
  int ring = 1,
  int okMs = 0,
  String reason = '',
}) => AgentRollout(
  version: '0.21.0',
  state: state,
  ring: Int64(ring),
  ringOkMs: Int64(okMs),
  reason: reason,
);

void main() {
  test('rollout summary', () {
    final now = DateTime.fromMillisecondsSinceEpoch(20 * 60000);
    expect(
      rolloutSummary(ro('active'), Int64(60), now),
      '0.21.0  active  Ring 1 (pilot)  upgrading',
    );
    expect(
      rolloutSummary(ro('active', okMs: 8 * 60000), Int64(60), now),
      '0.21.0  active  Ring 1 (pilot)  soaking 12/60 min',
    );
    expect(
      rolloutSummary(ro('paused', reason: 'WS-20 failed'), Int64(60), now),
      '0.21.0  paused  Ring 1 (pilot)  (WS-20 failed)',
    );
  });

  test('machine status', () {
    final r = ro('active');
    MachineUpgrade m(
      int ring,
      String v, {
      bool on = true,
      String lv = '',
      bool ok = true,
    }) => MachineUpgrade(
      machine: 'WS-20',
      ring: Int64(ring),
      version: v,
      upgrades: on,
      lastVersion: lv,
      lastOk: ok,
      lastError: 'health',
    );
    expect(machineStatus(m(0, '0.21.0'), r), 'on 0.21.0');
    expect(machineStatus(m(1, '0.20.1'), r), 'upgrading');
    expect(machineStatus(m(2, '0.20.1'), r), 'later ring');
    expect(machineStatus(m(1, '0.20.1', on: false), r), 'upgrades off');
    expect(
      machineStatus(m(1, '0.20.1', lv: '0.21.0', ok: false), r),
      'failed 0.21.0: health',
    );
    expect(machineStatus(m(1, '0.20.1'), null), 'v0.20.1');
  });

  testWidgets('buttons follow role and state', (t) async {
    final calls = <String>[];
    Future<void> pump(AgentUpgrades u, {bool start = true, bool stop = true}) =>
        t.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: UpdatesView(
                u: u,
                now: DateTime.now(),
                canStart: start,
                canStop: stop,
                onStart: (v) => calls.add('start:$v'),
                onPause: () => calls.add('pause'),
                onResume: () => calls.add('resume'),
                onAbort: () => calls.add('abort'),
                onRing: (m, r) => calls.add('ring:${m.machine}:$r'),
              ),
            ),
          ),
        );
    final rel = [
      AgentRelease(version: '0.21.0', ownerSigned: true, signer: 'root'),
      AgentRelease(
        version: '0.20.9',
        ownerSigned: false,
        problem: 'needs the owner signature',
      ),
    ];
    await pump(AgentUpgrades(releases: rel));
    expect(find.text('No agent upgrade in progress.'), findsOneWidget);
    await t.tap(find.text('Roll out').first);
    await t.tap(find.text('Roll out').last); // disabled: not owner-signed
    expect(calls, ['start:0.21.0']);

    calls.clear();
    await pump(
      AgentUpgrades(
        current: ro('active'),
        releases: rel,
        ringHolds: ['WS-20 not on 0.21.0'],
      ),
    );
    expect(find.text('Hold: WS-20 not on 0.21.0'), findsOneWidget);
    await t.tap(find.text('Pause'));
    await t.tap(find.text('Abort'));
    await t.tap(find.text('Roll out').first); // disabled while a rollout runs
    expect(calls, ['pause', 'abort']);

    calls.clear();
    await pump(AgentUpgrades(current: ro('paused')), start: false);
    await t.tap(find.text('Resume')); // owner only
    expect(calls, isEmpty);
    await pump(AgentUpgrades(current: ro('paused')));
    await t.tap(find.text('Resume'));
    expect(calls, ['resume']);
  });
}
