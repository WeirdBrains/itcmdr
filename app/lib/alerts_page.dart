// v0.16.0: site alerts. List with filters, ack (user), resolve and silence (admin).
import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';

import 'proto/itcmdr.pbgrpc.dart';

/// Colour per alert level.
Color alertColor(String level) => switch (level) {
  'critical' => Colors.red,
  'cautious' => Colors.orange,
  _ => Colors.blueGrey,
};

/// The shell badge: open critical + cautious (informational never nags). 0 = no badge.
int alertBadgeCount(ListAlertsResponse r) =>
    r.openCritical.toInt() + r.openCautious.toInt();

/// One-line summary, e.g. `CRITICAL  WS-01  SVC-x failed (x3)`.
String alertSummary(Alert a) {
  final n = a.count > 1 ? ' (x${a.count})' : '';
  final m = a.machine.isEmpty ? '' : '  ${a.machine}';
  return '${a.level.toUpperCase()}$m  ${a.title}$n';
}

String _ts(Int64 ms) {
  if (ms == 0) return '';
  final d = DateTime.fromMillisecondsSinceEpoch(ms.toInt());
  String p(int v) => v.toString().padLeft(2, '0');
  return '${d.year}-${p(d.month)}-${p(d.day)} ${p(d.hour)}:${p(d.minute)}';
}

/// List of alerts; trailing buttons call [onAck] / [onResolve] when given.
class AlertsTable extends StatelessWidget {
  final List<Alert> alerts;
  final void Function(Alert)? onAck;
  final void Function(Alert)? onResolve;
  const AlertsTable({
    super.key,
    required this.alerts,
    this.onAck,
    this.onResolve,
  });

  @override
  Widget build(BuildContext context) {
    if (alerts.isEmpty) return const Center(child: Text('No alerts'));
    return ListView.builder(
      itemCount: alerts.length,
      itemBuilder: (ctx, i) {
        final a = alerts[i];
        final done = a.state == 'resolved';
        return ListTile(
          dense: true,
          leading: Icon(
            done ? Icons.check_circle_outline : Icons.warning_amber,
            color: done ? Colors.green : alertColor(a.level),
            size: 18,
          ),
          title: Text(alertSummary(a)),
          subtitle: Text(
            '${a.state}  ${a.source}  last ${_ts(a.lastSeenMs)}'
            '${a.ackedBy.isNotEmpty ? '  acked by ${a.ackedBy}' : ''}'
            '${a.detail.isNotEmpty ? '\n${a.detail}' : ''}',
            maxLines: 3,
            overflow: TextOverflow.ellipsis,
          ),
          trailing: done
              ? null
              : Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (a.state == 'open' && onAck != null)
                      TextButton(
                        onPressed: () => onAck!(a),
                        child: const Text('Ack'),
                      ),
                    if (onResolve != null)
                      TextButton(
                        onPressed: () => onResolve!(a),
                        child: const Text('Resolve'),
                      ),
                  ],
                ),
        );
      },
    );
  }
}

class AlertsPage extends StatefulWidget {
  final ItcmdrClient client;
  final String Function(Object) errText;
  final VoidCallback? onChanged;
  const AlertsPage({
    super.key,
    required this.client,
    required this.errText,
    this.onChanged,
  });
  @override
  State<AlertsPage> createState() => _AlertsPageState();
}

class _AlertsPageState extends State<AlertsPage> {
  String _state = '';
  String _level = '';
  List<Alert> _alerts = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final r = await widget.client.listAlerts(
        ListAlertsRequest(state: _state, level: _level),
      );
      if (mounted) {
        setState(() {
          _alerts = r.alerts;
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
      widget.onChanged?.call();
      await _load();
    } catch (e) {
      if (mounted) setState(() => _error = widget.errText(e));
    }
  }

  Future<void> _silence() async {
    final machine = TextEditingController();
    final reason = TextEditingController();
    var hours = 4;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, set) => AlertDialog(
          title: const Text('Silence alerts'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: machine,
                decoration: const InputDecoration(labelText: 'Machine'),
              ),
              TextField(
                controller: reason,
                decoration: const InputDecoration(labelText: 'Reason'),
              ),
              DropdownButton<int>(
                value: hours,
                items: const [
                  DropdownMenuItem(value: 1, child: Text('1 hour')),
                  DropdownMenuItem(value: 4, child: Text('4 hours')),
                  DropdownMenuItem(value: 24, child: Text('1 day')),
                  DropdownMenuItem(value: 168, child: Text('7 days')),
                ],
                onChanged: (v) => set(() => hours = v ?? 4),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('Silence'),
            ),
          ],
        ),
      ),
    );
    if (ok != true || machine.text.trim().isEmpty) return;
    final until = DateTime.now().add(Duration(hours: hours));
    await _do(
      () => widget.client.silenceAlerts(
        SilenceAlertsRequest(
          machine: machine.text.trim(),
          untilMs: Int64(until.millisecondsSinceEpoch),
          reason: reason.text.trim(),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              DropdownButton<String>(
                value: _state,
                items: const [
                  DropdownMenuItem(value: '', child: Text('Unresolved')),
                  DropdownMenuItem(value: 'open', child: Text('Open')),
                  DropdownMenuItem(value: 'acked', child: Text('Acked')),
                  DropdownMenuItem(value: 'silenced', child: Text('Silenced')),
                  DropdownMenuItem(value: 'resolved', child: Text('Resolved')),
                  DropdownMenuItem(value: 'all', child: Text('All')),
                ],
                onChanged: (v) {
                  setState(() => _state = v ?? '');
                  _load();
                },
              ),
              DropdownButton<String>(
                value: _level,
                items: const [
                  DropdownMenuItem(value: '', child: Text('Any level')),
                  DropdownMenuItem(value: 'critical', child: Text('Critical')),
                  DropdownMenuItem(value: 'cautious', child: Text('Cautious')),
                  DropdownMenuItem(
                    value: 'informational',
                    child: Text('Informational'),
                  ),
                ],
                onChanged: (v) {
                  setState(() => _level = v ?? '');
                  _load();
                },
              ),
              IconButton(
                onPressed: _load,
                icon: const Icon(Icons.refresh),
                tooltip: 'Refresh',
              ),
              OutlinedButton.icon(
                onPressed: _silence,
                icon: const Icon(Icons.notifications_off),
                label: const Text('Silence…'),
              ),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
          const SizedBox(height: 8),
          Expanded(
            child: AlertsTable(
              alerts: _alerts,
              onAck: (a) =>
                  _do(() => widget.client.ackAlert(AlertRef(id: a.id))),
              onResolve: (a) =>
                  _do(() => widget.client.resolveAlert(AlertRef(id: a.id))),
            ),
          ),
        ],
      ),
    );
  }
}
