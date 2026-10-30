import 'dart:async';

import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';

import 'exceptions_form.dart';
import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'itcmdr',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
      ),
      home: const HomeShell(),
    );
  }
}

String fmtTs(Int64 ms) {
  if (ms == Int64.ZERO) return '-';
  final d = DateTime.fromMillisecondsSinceEpoch(ms.toInt());
  String two(int n) => n.toString().padLeft(2, '0');
  return '${d.year}-${two(d.month)}-${two(d.day)} ${two(d.hour)}:${two(d.minute)}';
}

String errText(Object e) =>
    e is GrpcError ? (e.message ?? e.toString()) : e.toString();

class HomeShell extends StatefulWidget {
  const HomeShell({super.key});
  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _index = 0;
  int _generation = 0;
  ServerSettings? _settings;
  GrpcOrGrpcWebClientChannel? _channel;
  ItcmdrClient? _client;
  StreamSubscription<Update>? _updates;
  final _updatesCtl = StreamController<Update>.broadcast();
  bool _live = false;

  @override
  void initState() {
    super.initState();
    ServerSettings.load().then(_apply);
  }

  Future<void> _apply(ServerSettings s) async {
    await _updates?.cancel();
    await _channel?.shutdown();
    final ch = s.channel();
    final client = ItcmdrClient(ch);
    if (!mounted) {
      await ch.shutdown();
      return;
    }
    setState(() {
      _settings = s;
      _channel = ch;
      _client = client;
      _generation++;
      _live = false;
    });
    _watch(client);
  }

  void _watch(ItcmdrClient client) {
    _updates = client
        .watchUpdates(WatchUpdatesRequest())
        .listen(
          (u) {
            if (!_live && mounted) setState(() => _live = true);
            _updatesCtl.add(u);
          },
          onError: (_) {
            if (mounted) setState(() => _live = false);
          },
          cancelOnError: true,
        );
  }

  @override
  void dispose() {
    _updates?.cancel();
    _channel?.shutdown();
    _updatesCtl.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final s = _settings;
    final c = _client;
    if (s == null || c == null) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    final key = ValueKey(_generation);
    final updates = _updatesCtl.stream;
    final pages = <Widget>[
      OverviewPage(key: key, client: c, updates: updates),
      MetricsPage(key: key, client: c),
      LogsPage(key: key, client: c),
      ExceptionsPage(key: key, client: c, settings: s, updates: updates),
      OnDemandPage(key: key, client: c, settings: s, updates: updates),
      AuditPage(key: key, client: c, updates: updates),
      SettingsPage(
        key: key,
        settings: s,
        onSave: (n) async {
          await n.save();
          await _apply(n);
        },
      ),
    ];
    return Scaffold(
      appBar: AppBar(
        title: const Text('itcmdr'),
        actions: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Row(
              children: [
                Icon(
                  Icons.circle,
                  size: 10,
                  color: _live ? Colors.green : Colors.grey,
                ),
                const SizedBox(width: 6),
                Text('${s.cleanHost}:${s.port}'),
              ],
            ),
          ),
        ],
      ),
      body: pages[_index],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _index,
        destinations: const [
          NavigationDestination(icon: Icon(Icons.grid_view), label: 'Overview'),
          NavigationDestination(
            icon: Icon(Icons.query_stats),
            label: 'Metrics',
          ),
          NavigationDestination(icon: Icon(Icons.description), label: 'Logs'),
          NavigationDestination(icon: Icon(Icons.rule), label: 'Exceptions'),
          NavigationDestination(
            icon: Icon(Icons.playlist_add_check),
            label: 'On-demand',
          ),
          NavigationDestination(icon: Icon(Icons.history), label: 'Audit'),
          NavigationDestination(icon: Icon(Icons.settings), label: 'Settings'),
        ],
        onDestinationSelected: (i) => setState(() => _index = i),
      ),
    );
  }
}

/// Refreshes `reload` when a live update of one of `kinds` arrives (debounced).
mixin LiveRefresh<T extends StatefulWidget> on State<T> {
  StreamSubscription<Update>? _sub;
  Timer? _debounce;

  void listenUpdates(
    Stream<Update>? s,
    Set<String> kinds,
    VoidCallback reload,
  ) {
    _sub = s?.listen((u) {
      if (kinds.isNotEmpty && !kinds.any((k) => u.kind.startsWith(k))) return;
      _debounce?.cancel();
      _debounce = Timer(const Duration(milliseconds: 500), () {
        if (mounted) reload();
      });
    });
  }

  @override
  void dispose() {
    _sub?.cancel();
    _debounce?.cancel();
    super.dispose();
  }
}

class OverviewPage extends StatefulWidget {
  final ItcmdrClient client;
  final Stream<Update>? updates;
  const OverviewPage({super.key, required this.client, this.updates});
  @override
  State<OverviewPage> createState() => _OverviewPageState();
}

class _OverviewPageState extends State<OverviewPage> with LiveRefresh {
  List<Machine> _machines = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _refresh();
    listenUpdates(widget.updates, {'run', 'incident'}, _refresh);
  }

  Future<void> _refresh() async {
    try {
      final resp = await widget.client.listMachines(ListMachinesRequest());
      if (!mounted) return;
      setState(() {
        _machines = resp.machines;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return Center(
        child: Text(
          'Server unavailable: $_error\nCheck Settings.',
          textAlign: TextAlign.center,
        ),
      );
    }
    final open = _machines.fold<int>(0, (a, m) => a + m.openIncidents);
    return RefreshIndicator(
      onRefresh: _refresh,
      child: ListView(
        children: [
          ListTile(
            title: Text('${_machines.length} machines'),
            subtitle: Text('$open open incidents'),
            trailing: IconButton(
              icon: const Icon(Icons.refresh),
              onPressed: _refresh,
            ),
          ),
          for (final m in _machines)
            ListTile(
              leading: Icon(
                Icons.desktop_windows,
                color: m.openIncidents > 0 ? Colors.orange : Colors.green,
              ),
              title: Text(m.name),
              subtitle: Text(
                'Open incidents: ${m.openIncidents}   Last run: ${fmtTs(m.lastRunTs)}',
              ),
              trailing: Text(m.status),
            ),
        ],
      ),
    );
  }
}

const rollupLabels = {
  Rollup.ROLLUP_DAILY: 'Daily',
  Rollup.ROLLUP_WEEKLY: 'Weekly (ISO)',
  Rollup.ROLLUP_BLOCK4: '4-week block',
  Rollup.ROLLUP_YEARLY: 'Yearly (ISO)',
};

class MetricsPage extends StatefulWidget {
  final ItcmdrClient client;
  const MetricsPage({super.key, required this.client});
  @override
  State<MetricsPage> createState() => _MetricsPageState();
}

class _MetricsPageState extends State<MetricsPage> {
  int _window = 30;
  Rollup _rollup = Rollup.ROLLUP_WEEKLY;
  List<MetricPoint> _points = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final resp = await widget.client.getMetrics(
        GetMetricsRequest(windowDays: _window, rollup: _rollup),
      );
      if (!mounted) return;
      setState(() {
        _points = resp.points;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  Iterable<MetricPoint> _with(String prefix) =>
      _points.where((p) => p.key.startsWith('$prefix:'));
  String _rest(MetricPoint p) => p.key.substring(p.key.indexOf(':') + 1);

  Widget _table(String title, List<String> cols, List<List<String>> rows) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            if (rows.isEmpty) const Text('No data'),
            if (rows.isNotEmpty)
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: DataTable(
                  columns: [for (final c in cols) DataColumn(label: Text(c))],
                  rows: [
                    for (final r in rows)
                      DataRow(cells: [for (final v in r) DataCell(Text(v))]),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final ranked = _with('incidents_total').toList()
      ..sort((a, b) => b.value.compareTo(a.value));
    double metric(String k, String m) => _points
        .firstWhere((p) => p.key == '$k:$m', orElse: () => MetricPoint())
        .value;
    final repairs = _with('repair_failures').toList()
      ..sort((a, b) => b.value.compareTo(a.value));
    final buckets = _with('incidents').toList()
      ..sort((a, b) => a.ts.compareTo(b.ts));
    final maxInc = buckets.fold<double>(1, (a, p) => p.value > a ? p.value : a);
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        Wrap(
          spacing: 12,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            DropdownButton<int>(
              value: _window,
              items: const [7, 30, 90, 365]
                  .map(
                    (d) => DropdownMenuItem(value: d, child: Text('$d days')),
                  )
                  .toList(),
              onChanged: (v) => setState(() => _window = v ?? 30),
            ),
            DropdownButton<Rollup>(
              value: _rollup,
              items: rollupLabels.entries
                  .map(
                    (e) => DropdownMenuItem(value: e.key, child: Text(e.value)),
                  )
                  .toList(),
              onChanged: (v) =>
                  setState(() => _rollup = v ?? Rollup.ROLLUP_WEEKLY),
            ),
            FilledButton(onPressed: _load, child: const Text('Load')),
          ],
        ),
        if (_error != null) Text('Error: $_error'),
        _table(
          'Machines ranked by incidents',
          ['Machine', 'Incidents', 'Open', 'MTTR (h)', 'Missed runs'],
          [
            for (final p in ranked)
              [
                _rest(p),
                p.value.toStringAsFixed(0),
                metric('open_incidents', _rest(p)).toStringAsFixed(0),
                (metric('mttr', _rest(p)) / 3600000).toStringAsFixed(1),
                metric('missed_runs', _rest(p)).toStringAsFixed(0),
              ],
          ],
        ),
        _table(
          'Where repairs failed',
          ['Machine', 'Check', 'Failures'],
          [
            for (final p in repairs)
              [
                _rest(p).split(':').first,
                _rest(p).split(':').skip(1).join(':'),
                p.value.toStringAsFixed(0),
              ],
          ],
        ),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Incidents per ${rollupLabels[_rollup]} bucket',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                for (final p in buckets)
                  Row(
                    children: [
                      SizedBox(
                        width: 220,
                        child: Text(
                          '${fmtTs(p.ts).split(' ').first}  ${_rest(p)}',
                        ),
                      ),
                      Expanded(
                        child: LinearProgressIndicator(
                          value: p.value / maxInc,
                          minHeight: 10,
                        ),
                      ),
                      SizedBox(
                        width: 48,
                        child: Text(
                          p.value.toStringAsFixed(0),
                          textAlign: TextAlign.right,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class ExceptionsPage extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  final Stream<Update>? updates;
  const ExceptionsPage({
    super.key,
    required this.client,
    required this.settings,
    this.updates,
  });
  @override
  State<ExceptionsPage> createState() => _ExceptionsPageState();
}

class _ExceptionsPageState extends State<ExceptionsPage> with LiveRefresh {
  List<ExceptionRule> _rules = [];
  bool _showExpired = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
    listenUpdates(widget.updates, {'exception'}, _load);
  }

  Future<void> _load() async {
    try {
      final resp = await widget.client.listExceptions(
        ListExceptionsRequest(includeInactive: _showExpired),
      );
      if (!mounted) return;
      setState(() {
        _rules = resp.rules;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  Future<void> _edit([ExceptionRule? existing]) async {
    final rule = await showExceptionDialog(context, existing: existing);
    if (rule == null) return;
    try {
      await widget.client.saveException(
        SaveExceptionRequest(rule: rule),
        options: widget.settings.authOptions(),
      );
      await _load();
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  Future<void> _expire(ExceptionRule r) async {
    try {
      await widget.client.expireException(
        ExpireExceptionRequest(id: r.id),
        options: widget.settings.authOptions(),
      );
      await _load();
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        children: [
          Row(
            children: [
              FilledButton.icon(
                onPressed: () => _edit(),
                icon: const Icon(Icons.add),
                label: const Text('Add'),
              ),
              const SizedBox(width: 12),
              OutlinedButton(onPressed: _load, child: const Text('Refresh')),
              const Spacer(),
              const Text('Show expired'),
              Switch(
                value: _showExpired,
                onChanged: (v) {
                  setState(() => _showExpired = v);
                  _load();
                },
              ),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
          Expanded(
            child: ListView(
              children: [
                for (final r in _rules)
                  ListTile(
                    enabled: r.active,
                    title: Text(
                      r.checkId.isNotEmpty ? r.checkId : '/${r.checkPattern}/',
                    ),
                    subtitle: Text(
                      [
                        r.scope.machines.isEmpty && r.scope.role.isEmpty
                            ? 'all machines'
                            : [
                                ...r.scope.machines,
                                if (r.scope.role.isNotEmpty)
                                  'role ${r.scope.role}',
                              ].join(', '),
                        r.action,
                        r.reason,
                        if (r.expires.isNotEmpty) 'expires ${r.expires}',
                        if (!r.active) 'EXPIRED',
                      ].join(' • '),
                    ),
                    trailing: r.active
                        ? Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              IconButton(
                                tooltip: 'Edit',
                                icon: const Icon(Icons.edit),
                                onPressed: () => _edit(r),
                              ),
                              IconButton(
                                tooltip: 'Expire now',
                                icon: const Icon(Icons.timer_off),
                                onPressed: () => _expire(r),
                              ),
                            ],
                          )
                        : null,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class OnDemandPage extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  final Stream<Update>? updates;
  const OnDemandPage({
    super.key,
    required this.client,
    required this.settings,
    this.updates,
  });
  @override
  State<OnDemandPage> createState() => _OnDemandPageState();
}

const runCategories = ['network', 'connectivity', 'configuration', 'services'];

class _OnDemandPageState extends State<OnDemandPage> with LiveRefresh {
  List<Machine> _machines = [];
  final Set<String> _selected = {};
  final Set<String> _categories = {};
  String? _status;
  String? _requestId;
  String? _error;
  bool _reportOnly = true;
  bool _repair = false;
  bool _force = false;

  @override
  void initState() {
    super.initState();
    _loadMachines();
    listenUpdates(widget.updates, {'request'}, _pollStatus);
  }

  Future<void> _loadMachines() async {
    try {
      final resp = await widget.client.listMachines(ListMachinesRequest());
      if (mounted) setState(() => _machines = resp.machines);
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  Future<void> _trigger() async {
    try {
      final resp = await widget.client.triggerRuns(
        TriggerRunsRequest(
          machines: _selected.toList(),
          reportOnly: _reportOnly,
          repair: _repair && !_reportOnly,
          force: _force,
          categories: _categories.toList(),
        ),
        options: widget.settings.authOptions(),
      );
      if (!mounted) return;
      setState(() {
        _requestId = resp.requestId;
        _status = 'queued';
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  Future<void> _pollStatus() async {
    final id = _requestId;
    if (id == null) return;
    try {
      final s = await widget.client.getRequestStatus(
        GetRequestStatusRequest(requestId: id),
      );
      if (mounted) setState(() => _status = '${s.status.name}: ${s.message}');
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              OutlinedButton(
                onPressed: _loadMachines,
                child: const Text('Reload machines'),
              ),
              const SizedBox(width: 12),
              Text(
                _selected.isEmpty
                    ? 'No selection = all machines'
                    : '${_selected.length} selected',
              ),
            ],
          ),
          Expanded(
            child: ListView(
              children: [
                for (final m in _machines)
                  CheckboxListTile(
                    title: Text(m.name),
                    value: _selected.contains(m.name),
                    onChanged: (v) => setState(
                      () => v == true
                          ? _selected.add(m.name)
                          : _selected.remove(m.name),
                    ),
                  ),
              ],
            ),
          ),
          Wrap(
            spacing: 8,
            children: [
              for (final c in runCategories)
                FilterChip(
                  label: Text(c),
                  selected: _categories.contains(c),
                  onSelected: (v) => setState(
                    () => v ? _categories.add(c) : _categories.remove(c),
                  ),
                ),
            ],
          ),
          Wrap(
            spacing: 16,
            children: [
              _toggle('Report only', _reportOnly, (v) => _reportOnly = v),
              _toggle('Repair', _repair, (v) => _repair = v),
              _toggle('Force (skip clinical guard)', _force, (v) => _force = v),
            ],
          ),
          Row(
            children: [
              FilledButton(onPressed: _trigger, child: const Text('Run now')),
              const SizedBox(width: 12),
              OutlinedButton(
                onPressed: _pollStatus,
                child: const Text('Refresh status'),
              ),
              const SizedBox(width: 12),
              if (_status != null) Expanded(child: Text('Status: $_status')),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
        ],
      ),
    );
  }

  Widget _toggle(String label, bool value, void Function(bool) set) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      Switch(value: value, onChanged: (v) => setState(() => set(v))),
      Text(label),
    ],
  );
}

class LogsPage extends StatefulWidget {
  final ItcmdrClient client;
  const LogsPage({super.key, required this.client});
  @override
  State<LogsPage> createState() => _LogsPageState();
}

class _LogsPageState extends State<LogsPage> {
  final _q = TextEditingController();
  final _machine = TextEditingController();
  List<String> _lines = [];
  String? _error;

  @override
  void dispose() {
    _q.dispose();
    _machine.dispose();
    super.dispose();
  }

  Future<void> _search() async {
    try {
      final resp = await widget.client.searchLogs(
        SearchLogsRequest(query: _q.text, machine: _machine.text, limit: 500),
      );
      if (mounted) {
        setState(() {
          _lines = resp.lines;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        children: [
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _q,
                  decoration: const InputDecoration(labelText: 'Text'),
                  onSubmitted: (_) => _search(),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: TextField(
                  controller: _machine,
                  decoration: const InputDecoration(
                    labelText: 'Machine (optional)',
                  ),
                  onSubmitted: (_) => _search(),
                ),
              ),
              const SizedBox(width: 8),
              FilledButton(onPressed: _search, child: const Text('Search')),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
          const SizedBox(height: 8),
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

class AuditPage extends StatefulWidget {
  final ItcmdrClient client;
  final Stream<Update>? updates;
  const AuditPage({super.key, required this.client, this.updates});
  @override
  State<AuditPage> createState() => _AuditPageState();
}

class _AuditPageState extends State<AuditPage> with LiveRefresh {
  List<AuditEntry> _entries = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
    listenUpdates(widget.updates, {'audit'}, _load);
  }

  Future<void> _load() async {
    try {
      final r = await widget.client.listAudit(ListAuditRequest(limit: 500));
      if (mounted) {
        setState(() {
          _entries = r.entries;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = errText(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        ListTile(
          title: const Text('Audit log (on-demand runs and exception changes)'),
          trailing: IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _load,
          ),
        ),
        if (_error != null) Text('Error: $_error'),
        Expanded(
          child: ListView(
            children: [
              for (final e in _entries)
                ListTile(
                  dense: true,
                  title: Text('${fmtTs(e.ts)}  ${e.action}  by ${e.actor}'),
                  subtitle: Text(
                    e.detail,
                    maxLines: 3,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

class SettingsPage extends StatefulWidget {
  final ServerSettings settings;
  final Future<void> Function(ServerSettings) onSave;
  const SettingsPage({super.key, required this.settings, required this.onSave});
  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  late final _host = TextEditingController(text: widget.settings.host);
  late final _port = TextEditingController(text: '${widget.settings.port}');
  late final _token = TextEditingController(text: widget.settings.token);
  late final _actor = TextEditingController(text: widget.settings.actor);
  late bool _tls = widget.settings.tls;
  bool _showToken = false;
  String? _msg;

  @override
  void dispose() {
    for (final c in [_host, _port, _token, _actor]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _save() async {
    final port = int.tryParse(_port.text.trim());
    if (port == null || port <= 0 || port > 65535) {
      setState(() => _msg = 'Port must be 1-65535');
      return;
    }
    await widget.onSave(
      widget.settings.copyWith(
        host: _host.text,
        port: port,
        tls: _tls,
        token: _token.text,
        actor: _actor.text,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        TextField(
          key: const Key('host'),
          controller: _host,
          decoration: const InputDecoration(
            labelText: 'Server host',
            hintText: 'name or address of the server',
          ),
        ),
        TextField(
          key: const Key('port'),
          controller: _port,
          keyboardType: TextInputType.number,
          decoration: const InputDecoration(labelText: 'Port'),
        ),
        SwitchListTile(
          title: const Text('Use TLS'),
          value: _tls,
          onChanged: (v) => setState(() => _tls = v),
        ),
        TextField(
          key: const Key('token'),
          controller: _token,
          obscureText: !_showToken,
          decoration: InputDecoration(
            labelText:
                'Admin token (needed for on-demand runs and exception changes)',
            suffixIcon: IconButton(
              icon: Icon(_showToken ? Icons.visibility_off : Icons.visibility),
              onPressed: () => setState(() => _showToken = !_showToken),
            ),
          ),
        ),
        TextField(
          key: const Key('actor'),
          controller: _actor,
          decoration: const InputDecoration(
            labelText: 'Your name (shown in the audit log)',
          ),
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            FilledButton(
              key: const Key('save'),
              onPressed: _save,
              child: const Text('Save and reconnect'),
            ),
            const SizedBox(width: 12),
            if (_msg != null) Text(_msg!),
          ],
        ),
      ],
    );
  }
}
