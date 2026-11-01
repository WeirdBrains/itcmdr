// v0.17.0: Rust agents. List with heartbeat state, enrollment tokens, maintenance, revoke (admin).
import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'proto/itcmdr.pbgrpc.dart';

Color agentStateColor(String s) => switch (s) {
  'online' => Colors.green,
  'late' => Colors.orange,
  'non_responsive' => Colors.red,
  'maintenance' => Colors.blueGrey,
  _ => Colors.grey,
};

String _ts(Int64 ms) {
  if (ms == 0) return 'never';
  final d = DateTime.fromMillisecondsSinceEpoch(ms.toInt());
  String p(int v) => v.toString().padLeft(2, '0');
  return '${d.year}-${p(d.month)}-${p(d.day)} ${p(d.hour)}:${p(d.minute)}';
}

/// One-line summary, e.g. `WS-01  online  workstation  v0.17.0`.
String agentSummary(AgentInfo a) =>
    '${a.machine}  ${a.state}  ${a.role}${a.version.isEmpty ? '' : '  v${a.version}'}';

class AgentsTable extends StatelessWidget {
  final List<AgentInfo> agents;
  final void Function(AgentInfo, bool)? onMaintenance;
  final void Function(AgentInfo)? onRevoke;
  const AgentsTable({
    super.key,
    required this.agents,
    this.onMaintenance,
    this.onRevoke,
  });

  @override
  Widget build(BuildContext context) {
    if (agents.isEmpty) return const Center(child: Text('No agents enrolled'));
    return ListView.builder(
      itemCount: agents.length,
      itemBuilder: (ctx, i) {
        final a = agents[i];
        final maint = a.state == 'maintenance';
        return ListTile(
          dense: true,
          leading: Icon(
            Icons.circle,
            size: 12,
            color: agentStateColor(a.state),
          ),
          title: Text(agentSummary(a)),
          subtitle: Text(
            'last seen ${_ts(a.lastSeenMs)}  certificate until ${_ts(a.certExpiresMs)}',
          ),
          trailing: a.state == 'revoked'
              ? null
              : Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (onMaintenance != null)
                      TextButton(
                        onPressed: () => onMaintenance!(a, !maint),
                        child: Text(maint ? 'End maintenance' : 'Maintenance'),
                      ),
                    if (onRevoke != null)
                      TextButton(
                        onPressed: () => onRevoke!(a),
                        child: const Text('Revoke'),
                      ),
                  ],
                ),
        );
      },
    );
  }
}

class AgentsPage extends StatefulWidget {
  final ItcmdrClient client;
  final String Function(Object) errText;
  const AgentsPage({super.key, required this.client, required this.errText});
  @override
  State<AgentsPage> createState() => _AgentsPageState();
}

class _AgentsPageState extends State<AgentsPage> {
  List<AgentInfo> _agents = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final r = await widget.client.listAgents(ListAgentsRequest());
      if (mounted) {
        setState(() {
          _agents = r.agents;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = widget.errText(e));
    }
  }

  Future<void> _do(Future<Object?> Function() f) async {
    try {
      await f();
      await _load();
    } catch (e) {
      if (mounted) setState(() => _error = widget.errText(e));
    }
  }

  Future<void> _token() async {
    final machine = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Enrollment token'),
        content: TextField(
          controller: machine,
          decoration: const InputDecoration(
            labelText: 'Machine (empty: any one machine)',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Create'),
          ),
        ],
      ),
    );
    if (ok != true) return;
    try {
      final r = await widget.client.createEnrollToken(
        CreateEnrollTokenRequest(machine: machine.text.trim()),
      );
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: const Text('One-time token (24 h)'),
          content: SizedBox(
            width: 520,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SelectableText(r.token),
                const SizedBox(height: 8),
                Text('Agents dial ${r.address}'),
                Text('Server fingerprint ${r.serverFingerprint}'),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Clipboard.setData(ClipboardData(text: r.token)),
              child: const Text('Copy'),
            ),
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('Close'),
            ),
          ],
        ),
      );
    } catch (e) {
      if (mounted) setState(() => _error = widget.errText(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        children: [
          Wrap(
            spacing: 8,
            children: [
              IconButton(
                onPressed: _load,
                icon: const Icon(Icons.refresh),
                tooltip: 'Refresh',
              ),
              OutlinedButton.icon(
                onPressed: _token,
                icon: const Icon(Icons.add_link),
                label: const Text('Enrollment token…'),
              ),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
          const SizedBox(height: 8),
          Expanded(
            child: AgentsTable(
              agents: _agents,
              onMaintenance: (a, on) => _do(
                () => widget.client.setAgentMaintenance(
                  SetAgentMaintenanceRequest(machine: a.machine, on: on),
                ),
              ),
              onRevoke: (a) => _do(
                () => widget.client.revokeAgent(
                  AgentMachineRef(machine: a.machine),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
