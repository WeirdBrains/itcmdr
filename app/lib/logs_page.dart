import 'package:flutter/material.dart';
import 'package:fixnum/fixnum.dart';

import 'proto/itcmdr.pbgrpc.dart';

const logCategories = [
  '',
  'network',
  'connectivity',
  'configuration',
  'backup',
  'services',
];
const logStatuses = ['', 'ok', 'failed', 'skipped'];
const logPageSize = 100;

/// Build a search request. [from]/[to] are local dates (inclusive whole days).
SearchLogsRequest buildLogsRequest({
  String query = '',
  String machine = '',
  String category = '',
  String status = '',
  DateTime? from,
  DateTime? to,
  int offset = 0,
  int limit = logPageSize,
}) {
  final r = SearchLogsRequest(
    query: query.trim(),
    machine: machine.trim(),
    category: category,
    status: status,
    offset: offset,
    limit: limit,
  );
  if (from != null) {
    r.startTs = Int64(
      DateTime(from.year, from.month, from.day).millisecondsSinceEpoch,
    );
  }
  if (to != null) {
    r.endTs = Int64(
      DateTime(to.year, to.month, to.day + 1).millisecondsSinceEpoch - 1,
    );
  }
  return r;
}

class LogsPage extends StatefulWidget {
  final ItcmdrClient client;
  final String Function(Object) errText;
  const LogsPage({super.key, required this.client, required this.errText});
  @override
  State<LogsPage> createState() => _LogsPageState();
}

class _LogsPageState extends State<LogsPage> {
  final _q = TextEditingController();
  final _machine = TextEditingController();
  String _category = '';
  String _status = '';
  DateTime? _from;
  DateTime? _to;
  int _offset = 0;
  int _total = 0;
  int _next = 0;
  List<String> _lines = [];
  String? _error;

  @override
  void dispose() {
    _q.dispose();
    _machine.dispose();
    super.dispose();
  }

  Future<void> _search({int offset = 0}) async {
    try {
      final resp = await widget.client.searchLogs(
        buildLogsRequest(
          query: _q.text,
          machine: _machine.text,
          category: _category,
          status: _status,
          from: _from,
          to: _to,
          offset: offset,
        ),
      );
      if (mounted) {
        setState(() {
          _lines = resp.lines;
          _total = resp.total;
          _next = resp.nextOffset;
          _offset = offset;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = widget.errText(e));
    }
  }

  Future<void> _pick(bool from) async {
    final now = DateTime.now();
    final d = await showDatePicker(
      context: context,
      firstDate: DateTime(now.year - 3),
      lastDate: now,
      initialDate: (from ? _from : _to) ?? now,
    );
    if (mounted) setState(() => from ? _from = d : _to = d);
  }

  String _d(DateTime? d) => d == null
      ? 'any'
      : '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';

  Widget _drop(
    String label,
    String value,
    List<String> opts,
    void Function(String) set,
  ) {
    return DropdownButton<String>(
      value: value,
      items: [
        for (final o in opts)
          DropdownMenuItem(value: o, child: Text(o.isEmpty ? 'any $label' : o)),
      ],
      onChanged: (v) => setState(() => set(v ?? '')),
    );
  }

  @override
  Widget build(BuildContext context) {
    final last = _offset + _lines.length;
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
                width: 220,
                child: TextField(
                  controller: _q,
                  decoration: const InputDecoration(labelText: 'Text'),
                  onSubmitted: (_) => _search(),
                ),
              ),
              SizedBox(
                width: 160,
                child: TextField(
                  controller: _machine,
                  decoration: const InputDecoration(labelText: 'Machine'),
                  onSubmitted: (_) => _search(),
                ),
              ),
              _drop('category', _category, logCategories, (v) => _category = v),
              _drop('status', _status, logStatuses, (v) => _status = v),
              OutlinedButton(
                onPressed: () => _pick(true),
                child: Text('From: ${_d(_from)}'),
              ),
              OutlinedButton(
                onPressed: () => _pick(false),
                child: Text('To: ${_d(_to)}'),
              ),
              FilledButton(onPressed: _search, child: const Text('Search')),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
          const SizedBox(height: 8),
          Row(
            children: [
              Text(
                _total == 0
                    ? 'No results'
                    : '${_offset + 1}-$last of $_total (newest first)',
              ),
              const Spacer(),
              IconButton(
                tooltip: 'Previous page',
                icon: const Icon(Icons.chevron_left),
                onPressed: _offset > 0
                    ? () => _search(
                        offset: (_offset - logPageSize).clamp(0, _offset),
                      )
                    : null,
              ),
              IconButton(
                tooltip: 'Next page',
                icon: const Icon(Icons.chevron_right),
                onPressed: _next > 0 ? () => _search(offset: _next) : null,
              ),
            ],
          ),
          Expanded(
            child: ListView.builder(
              itemCount: _lines.length,
              itemBuilder: (c, i) => SelectableText(
                _lines[i],
                style: const TextStyle(fontFamily: 'monospace'),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
