// v0.21.0 Updates: roll a new agent version out through the rings (0 test machine, 1 one PC,
// 2 everything), watch it, pause, resume or abort. Ring per machine is the agent.ring setting.
// Starting and resuming need the owner key; pausing and aborting an admin key (both signed).
import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';

import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';

const ringNames = ['Ring 0 (test)', 'Ring 1 (pilot)', 'Ring 2 (everyone)'];

String ringName(Int64 r) =>
    r >= 0 && r < ringNames.length ? ringNames[r.toInt()] : 'Ring $r';

String _ts(Int64 ms) {
  if (ms == Int64.ZERO) return '-';
  final d = DateTime.fromMillisecondsSinceEpoch(ms.toInt());
  String two(int n) => n.toString().padLeft(2, '0');
  return '${d.month}/${d.day} ${two(d.hour)}:${two(d.minute)}';
}

/// One line for the rollout in progress, e.g. "0.21.0  active  Ring 1 (pilot)  soaking 12/60 min".
String rolloutSummary(AgentRollout r, Int64 soakMins, DateTime now) {
  final b = StringBuffer('${r.version}  ${r.state}  ${ringName(r.ring)}');
  if (r.state == 'active') {
    if (r.ringOkMs == Int64.ZERO) {
      b.write('  upgrading');
    } else {
      final m = now
          .difference(DateTime.fromMillisecondsSinceEpoch(r.ringOkMs.toInt()))
          .inMinutes;
      b.write('  soaking ${m.clamp(0, soakMins.toInt())}/$soakMins min');
    }
  }
  if (r.reason.isNotEmpty) b.write('  (${r.reason})');
  return b.toString();
}

/// Releases that can be rolled out now: owner-signed, no problem, newer than nothing in progress.
List<AgentRelease> startable(AgentUpgrades u) => [
  for (final r in u.releases)
    if (r.ownerSigned && r.problem.isEmpty) r,
];

/// Status of one machine against the rollout in progress.
String machineStatus(MachineUpgrade m, AgentRollout? r) {
  if (!m.upgrades) return 'upgrades off';
  if (m.lastVersion.isNotEmpty &&
      !m.lastOk &&
      (r == null || m.lastVersion == r.version)) {
    return 'failed ${m.lastVersion}: ${m.lastError}';
  }
  if (r == null || r.state != 'active' && r.state != 'paused') {
    return 'v${m.version}';
  }
  if (m.version == r.version) return 'on ${r.version}';
  if (m.ring <= r.ring) {
    return r.state == 'paused' ? 'waiting (paused)' : 'upgrading';
  }
  return 'later ring';
}

class UpdatesView extends StatelessWidget {
  final AgentUpgrades u;
  final bool canStart, canStop;
  final void Function(String version, bool now) onStart;
  final VoidCallback onPause, onResume, onAbort;
  final void Function(MachineUpgrade m, int ring) onRing;
  final DateTime now;
  const UpdatesView({
    super.key,
    required this.u,
    required this.canStart,
    required this.canStop,
    required this.onStart,
    required this.onPause,
    required this.onResume,
    required this.onAbort,
    required this.onRing,
    required this.now,
  });

  @override
  Widget build(BuildContext context) {
    final cur = u.hasCurrent() ? u.current : null;
    final t = Theme.of(context).textTheme;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text('Automatic updates', style: t.titleMedium),
        Text(
          u.auto
              ? 'On: window ${u.window} (${u.timezone})'
              : 'Off for this site: releases wait for "Now" (updates.auto)',
          key: const Key('updates-auto'),
        ),
        Text(
          u.windowHold.isEmpty
              ? 'Upgrades may start now.'
              : 'Waiting: ${u.windowHold}',
          key: const Key('updates-hold'),
        ),
        if (u.serverUpgrade.isNotEmpty)
          Text(
            'Server upgrade: ${u.serverUpgrade}',
            key: const Key('server-upgrade'),
          ),
        const Text(
          'Change updates.auto, updates.window and updates.timezone on the Settings screen. '
          '"Now" ignores the window; machines with agent.upgrades off are held back either way.',
        ),
        const SizedBox(height: 16),
        Text('Rollout', style: t.titleMedium),
        if (cur == null)
          const Text('No agent upgrade in progress.')
        else ...[
          Text(
            rolloutSummary(cur, u.soakMins, now) +
                (cur.push ? ' (pushed: ignores the window)' : ''),
          ),
          for (final h in u.ringHolds)
            Text('Hold: $h', style: const TextStyle(color: Colors.orange)),
          Wrap(
            spacing: 8,
            children: [
              if (cur.state == 'active')
                OutlinedButton(
                  onPressed: canStop ? onPause : null,
                  child: const Text('Pause'),
                ),
              if (cur.state == 'paused')
                FilledButton(
                  onPressed: canStart ? onResume : null,
                  child: const Text('Resume'),
                ),
              OutlinedButton(
                onPressed: canStop ? onAbort : null,
                child: const Text('Abort'),
              ),
            ],
          ),
        ],
        const SizedBox(height: 16),
        Text('Releases', style: t.titleMedium),
        if (u.releases.isEmpty)
          const Text(
            'No agent releases installed. Install an owner-signed itcmdr-agent-release package on the Plugins screen.',
          ),
        for (final r in u.releases)
          ListTile(
            dense: true,
            title: Text(r.version),
            subtitle: Text(
              r.problem.isNotEmpty
                  ? r.problem
                  : 'signed by ${r.signer}${r.minFrom.isEmpty ? '' : ', from ${r.minFrom}'}',
            ),
            trailing: Wrap(
              spacing: 4,
              children: [
                FilledButton(
                  key: Key('rollout-${r.version}'),
                  onPressed:
                      canStart &&
                          cur == null &&
                          r.ownerSigned &&
                          r.problem.isEmpty
                      ? () => onStart(r.version, false)
                      : null,
                  child: const Text('Roll out'),
                ),
                OutlinedButton(
                  key: Key('rollout-now-${r.version}'),
                  onPressed:
                      canStart &&
                          cur == null &&
                          r.ownerSigned &&
                          r.problem.isEmpty
                      ? () => onStart(r.version, true)
                      : null,
                  child: const Text('Now'),
                ),
              ],
            ),
          ),
        const SizedBox(height: 16),
        Text('Machines', style: t.titleMedium),
        for (final m in u.machines)
          ListTile(
            dense: true,
            title: Text(m.machine),
            subtitle: Text(
              '${machineStatus(m, cur)}${m.lastMs == Int64.ZERO ? '' : '  last ${_ts(m.lastMs)}'}',
            ),
            trailing: DropdownButton<int>(
              value: m.ring.toInt().clamp(0, 2),
              items: [
                for (var i = 0; i < ringNames.length; i++)
                  DropdownMenuItem(value: i, child: Text(ringNames[i])),
              ],
              onChanged: canStop
                  ? (v) {
                      if (v != null && v != m.ring.toInt()) onRing(m, v);
                    }
                  : null,
            ),
          ),
        if (u.history.isNotEmpty) ...[
          const SizedBox(height: 16),
          Text('History', style: t.titleMedium),
          for (final h in u.history)
            Text(
              '${_ts(h.startedMs)}  ${h.version}  ${h.state}  ${ringName(h.ring)}'
              '${h.reason.isEmpty ? '' : '  (${h.reason})'}  ${h.actor}',
            ),
        ],
      ],
    );
  }
}

class UpdatesPage extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  const UpdatesPage({super.key, required this.client, required this.settings});
  @override
  State<UpdatesPage> createState() => _UpdatesPageState();
}

class _UpdatesPageState extends State<UpdatesPage> {
  AgentUpgrades? _u;
  String? _error;
  String _role = '';
  bool _signed = false;

  CallOptions get _opts => widget.settings.authOptions();

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final u = await widget.client.getAgentUpgrades(
        GetAgentUpgradesRequest(),
        options: _opts,
      );
      try {
        final me = await widget.client.whoAmI(WhoAmIRequest(), options: _opts);
        _role = me.role;
        _signed = me.signed;
      } catch (_) {}
      if (!mounted) return;
      setState(() {
        _u = u;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  Future<void> _act(Future<AgentUpgrades> Function() f) async {
    try {
      final u = await f();
      if (!mounted) return;
      setState(() {
        _u = u;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  Future<bool> _confirm(String q) async =>
      await showDialog<bool>(
        context: context,
        builder: (c) => AlertDialog(
          content: Text(q),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(c, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(c, true),
              child: const Text('OK'),
            ),
          ],
        ),
      ) ??
      false;

  @override
  Widget build(BuildContext context) {
    final c = widget.client;
    final owner = _signed && _role == 'owner';
    final admin = _signed && (_role == 'owner' || _role == 'admin');
    return Column(
      children: [
        Row(
          children: [
            const SizedBox(width: 16),
            Text(
              'Agent updates',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const Spacer(),
            IconButton(onPressed: _load, icon: const Icon(Icons.refresh)),
          ],
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.all(8),
            child: Text(_error!, style: const TextStyle(color: Colors.red)),
          ),
        if (_u != null)
          Expanded(
            child: UpdatesView(
              u: _u!,
              now: DateTime.now(),
              canStart: owner,
              canStop: admin,
              onStart: (v, now) async {
                if (await _confirm(
                  'Roll out agent $v, starting with ring 0?',
                )) {
                  _act(
                    () => c.startAgentUpgrade(
                      StartAgentUpgradeRequest(version: v, now: now),
                      options: _opts,
                    ),
                  );
                }
              },
              onPause: () => _act(
                () => c.pauseAgentUpgrade(
                  PauseAgentUpgradeRequest(),
                  options: _opts,
                ),
              ),
              onResume: () => _act(
                () => c.resumeAgentUpgrade(
                  ResumeAgentUpgradeRequest(),
                  options: _opts,
                ),
              ),
              onAbort: () async {
                if (await _confirm(
                  'Abort this rollout? Upgraded agents stay on the new version.',
                )) {
                  _act(
                    () => c.abortAgentUpgrade(
                      AbortAgentUpgradeRequest(),
                      options: _opts,
                    ),
                  );
                }
              },
              onRing: (m, r) => _act(() async {
                await c.updateSetting(
                  UpdateSettingRequest(
                    key: 'agent.ring',
                    value: '$r',
                    machine: m.machine,
                  ),
                  options: _opts,
                );
                return c.getAgentUpgrades(
                  GetAgentUpgradesRequest(),
                  options: _opts,
                );
              }),
            ),
          ),
      ],
    );
  }
}
