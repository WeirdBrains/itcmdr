// v0.15.0: the call log (one row per RPC), next to Audit. Admin only on the server.
import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';

import 'proto/itcmdr.pbgrpc.dart';

/// Build a ListCalls request from the filter fields. [hours] 0 = any time.
ListCallsRequest buildCallsRequest({
  String method = '',
  String principal = '',
  bool errorsOnly = false,
  int hours = 0,
  String id = '',
  int limit = 500,
  DateTime? now,
}) {
  final r = ListCallsRequest(
    method: method.trim(),
    principal: principal.trim(),
    errorsOnly: errorsOnly,
    id: id.trim(),
    limit: limit,
  );
  if (hours > 0) {
    final t = (now ?? DateTime.now()).subtract(Duration(hours: hours));
    r.sinceMs = Int64(t.millisecondsSinceEpoch);
  }
  return r;
}

String _ts(Int64 ms) {
  final d = DateTime.fromMillisecondsSinceEpoch(ms.toInt());
  String p(int v) => v.toString().padLeft(2, '0');
  return '${d.year}-${p(d.month)}-${p(d.day)} ${p(d.hour)}:${p(d.minute)}:${p(d.second)}';
}

/// One-line summary of a call, e.g. `ListAudit  UNAUTHENTICATED  3 ms  anonymous`.
String callSummary(CallEntry c) =>
    '${c.method}  ${c.statusName}  ${c.durationMs} ms  ${c.principal}';

/// Table of calls; tap a row for the details.
class CallsTable extends StatelessWidget {
  final List<CallEntry> calls;
  const CallsTable({super.key, required this.calls});

  void _details(BuildContext context, CallEntry c) {
    final rows = <(String, String)>[
      ('Time', _ts(c.ts)),
      ('Method', '${c.service}/${c.method}'),
      ('Kind', c.kind),
      ('Status', '${c.statusName} (${c.status})'),
      ('Principal', c.principal),
      ('Peer', c.peer),
      ('Duration', '${c.durationMs} ms'),
      ('Bytes', '${c.reqBytes} in, ${c.respBytes} out'),
      if (c.error.isNotEmpty) ('Error', c.error),
      ('Call id', c.id),
    ];
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(c.method),
        content: SizedBox(
          width: 520,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              for (final r in rows)
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(width: 90, child: Text(r.$1)),
                    Expanded(child: SelectableText(r.$2)),
                  ],
                ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (calls.isEmpty) return const Center(child: Text('No calls'));
    return ListView.builder(
      itemCount: calls.length,
      itemBuilder: (ctx, i) {
        final c = calls[i];
        final bad = c.status != 0;
        return ListTile(
          dense: true,
          leading: Icon(
            bad ? Icons.error_outline : Icons.check_circle_outline,
            color: bad ? Colors.red : Colors.green,
            size: 18,
          ),
          title: Text('${_ts(c.ts)}  ${callSummary(c)}'),
          subtitle: Text(
            c.error.isEmpty ? '${c.kind}  ${c.peer}' : c.error,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          onTap: () => _details(ctx, c),
        );
      },
    );
  }
}

class CallsPage extends StatefulWidget {
  final ItcmdrClient client;
  final String Function(Object) errText;
  const CallsPage({super.key, required this.client, required this.errText});
  @override
  State<CallsPage> createState() => _CallsPageState();
}

class _CallsPageState extends State<CallsPage> {
  final _method = TextEditingController();
  final _principal = TextEditingController();
  bool _errorsOnly = false;
  int _hours = 24;
  List<CallEntry> _calls = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _method.dispose();
    _principal.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final r = await widget.client.listCalls(
        buildCallsRequest(
          method: _method.text,
          principal: _principal.text,
          errorsOnly: _errorsOnly,
          hours: _hours,
        ),
      );
      if (mounted) {
        setState(() {
          _calls = r.calls;
          _error = null;
        });
      }
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
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SizedBox(
                width: 200,
                child: TextField(
                  key: const Key('calls-method'),
                  controller: _method,
                  decoration: const InputDecoration(labelText: 'Method'),
                  onSubmitted: (_) => _load(),
                ),
              ),
              SizedBox(
                width: 200,
                child: TextField(
                  controller: _principal,
                  decoration: const InputDecoration(labelText: 'Who'),
                  onSubmitted: (_) => _load(),
                ),
              ),
              DropdownButton<int>(
                value: _hours,
                items: const [
                  DropdownMenuItem(value: 1, child: Text('last hour')),
                  DropdownMenuItem(value: 24, child: Text('last 24 hours')),
                  DropdownMenuItem(value: 168, child: Text('last 7 days')),
                  DropdownMenuItem(value: 0, child: Text('any time')),
                ],
                onChanged: (v) => setState(() => _hours = v ?? 24),
              ),
              FilterChip(
                label: const Text('Errors only'),
                selected: _errorsOnly,
                onSelected: (v) => setState(() => _errorsOnly = v),
              ),
              FilledButton.icon(
                onPressed: _load,
                icon: const Icon(Icons.search),
                label: const Text('Search'),
              ),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
          const SizedBox(height: 8),
          Expanded(child: CallsTable(calls: _calls)),
        ],
      ),
    );
  }
}
