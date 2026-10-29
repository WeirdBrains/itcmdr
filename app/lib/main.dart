import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';
import 'proto/itcmdr.pbgrpc.dart';

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

class HomeShell extends StatefulWidget {
  const HomeShell({super.key});
  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _index = 0;
  late final ClientChannelBase _channel;
  late final ItcmdrClient _client;

  @override
  void initState() {
    super.initState();
    _channel = createClientChannel();
    _client = ItcmdrClient(_channel);
    // Subscribe to live updates and refresh overview on events
    try {
      final stream = _client.watchUpdates(WatchUpdatesRequest());
      stream.listen((_) {
        if (mounted && _index == 0) {
          // refresh overview silently
          _overviewKey.currentState?.refresh();
        }
      });
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final pages = [
      OverviewPage(key: _overviewKey, client: _client),
      MetricsPage(client: _client),
      ExceptionsPage(client: _client),
      OnDemandPage(client: _client),
      LogsPage(client: _client),
    ];
    return Scaffold(
      appBar: AppBar(title: const Text('itcmdr')),
      body: pages[_index],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _index,
        destinations: const [
          NavigationDestination(icon: Icon(Icons.grid_view), label: 'Overview'),
          NavigationDestination(icon: Icon(Icons.query_stats), label: 'Metrics'),
          NavigationDestination(icon: Icon(Icons.rule), label: 'Exceptions'),
          NavigationDestination(icon: Icon(Icons.playlist_add_check), label: 'On-demand'),
          NavigationDestination(icon: Icon(Icons.description), label: 'Logs'),
        ],
        onDestinationSelected: (i) => setState(() => _index = i),
      ),
    );
  }
}

class OverviewPage extends StatefulWidget {
  final ItcmdrClient client;
  const OverviewPage({super.key, required this.client});

  @override
  State<OverviewPage> createState() => _OverviewPageState();
}

class _OverviewPageState extends State<OverviewPage> {
  List<Machine> _machines = [];
  String? _error;

  void refresh() => _refresh();

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  void _refresh() async {
    try {
      final resp = await widget.client.listMachines(ListMachinesRequest());
      setState(() {
        _machines = resp.machines;
        _error = null;
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
        appBar: AppBar(title: const Text('itcmdr')),
        body: _error != null
            ? Center(child: Text('Server unavailable: \'$_error\''))
            : ListView.builder(
                itemCount: _machines.length,
                itemBuilder: (context, i) {
                  final m = _machines[i];
                  return ListTile(
                    leading: const Icon(Icons.desktop_windows),
                    title: Text(m.name),
                    subtitle:
                        Text('Incidents: \\${m.openIncidents}  Last run: \\${m.lastRunTs}'),
                    trailing: Text(m.status),
                  );
                },
              ),
        floatingActionButton:
            FloatingActionButton(onPressed: _refresh, child: const Icon(Icons.refresh)));
  }
}

class MetricsPage extends StatefulWidget {
  final ItcmdrClient client;
  const MetricsPage({super.key, required this.client});
  @override
  State<MetricsPage> createState() => _MetricsPageState();
}

class _MetricsPageState extends State<MetricsPage> {
  int _window = 7;
  Rollup _rollup = Rollup.ROLLUP_DAILY;
  List<MetricPoint> _points = [];
  String? _error;
  Future<void> _load() async {
    try {
      final resp = await widget.client.getMetrics(GetMetricsRequest(windowDays: _window, rollup: _rollup));
      setState(() {
        _points = resp.points;
        _error = null;
      });
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(children: [
            DropdownButton<int>(
              value: _window,
              items: const [7, 30, 90, 365]
                  .map((d) => DropdownMenuItem(value: d, child: Text('$d days')))
                  .toList(),
              onChanged: (v) => setState(() => _window = v ?? 7),
            ),
            const SizedBox(width: 12),
            DropdownButton<Rollup>(
              value: _rollup,
              items: const [
                DropdownMenuItem(value: Rollup.ROLLUP_DAILY, child: Text('Daily')),
                DropdownMenuItem(value: Rollup.ROLLUP_WEEKLY, child: Text('Weekly')),
                DropdownMenuItem(value: Rollup.ROLLUP_MONTHLY, child: Text('Monthly')),
                DropdownMenuItem(value: Rollup.ROLLUP_QUARTERLY, child: Text('Quarterly')),
                DropdownMenuItem(value: Rollup.ROLLUP_YEARLY, child: Text('Yearly')),
              ],
              onChanged: (v) => setState(() => _rollup = v ?? Rollup.ROLLUP_DAILY),
            ),
            const SizedBox(width: 12),
            ElevatedButton(onPressed: _load, child: const Text('Load')),
          ]),
          const SizedBox(height: 12),
          if (_error != null) Text('Error: $_error'),
          Expanded(
            child: ListView.builder(
              itemCount: _points.length,
              itemBuilder: (context, i) {
                final p = _points[i];
                return ListTile(
                  title: Text(p.key),
                  subtitle: Text('ts=${p.ts}'),
                  trailing: Text(p.value.toStringAsFixed(2)),
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}

class ExceptionsPage extends StatefulWidget {
  final ItcmdrClient client;
  const ExceptionsPage({super.key, required this.client});
  @override
  State<ExceptionsPage> createState() => _ExceptionsPageState();
}

class _ExceptionsPageState extends State<ExceptionsPage> {
  List<ExceptionRule> _rules = [];
  String? _error;
  Future<void> _load() async {
    try {
      final resp = await widget.client.listExceptions(ListExceptionsRequest());
      setState(() {
        _rules = resp.rules;
        _error = null;
      });
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _add() async {
    final rule = ExceptionRule(
        id: '',
        scope: ExceptionScope(machines: ['WS-01'], role: ''),
        checkId: 'CHK001',
        checkPattern: '',
        action: 'ignore',
        reason: 'example',
        expires: '',
        active: true);
    await widget.client.saveException(SaveExceptionRequest(rule: rule));
    await _load();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        children: [
          Row(
            children: [
              ElevatedButton(onPressed: _load, child: const Text('Refresh')),
              const SizedBox(width: 12),
              ElevatedButton(onPressed: _add, child: const Text('Add example')),
            ],
          ),
          if (_error != null) Text('Error: $_error'),
          Expanded(
            child: ListView.builder(
              itemCount: _rules.length,
              itemBuilder: (context, i) {
                final r = _rules[i];
                return ListTile(
                  title: Text(r.checkId.isNotEmpty ? r.checkId : r.checkPattern),
                  subtitle: Text('${r.scope.machines.join(", ")} • ${r.action} • ${r.reason}'),
                );
              },
            ),
          )
        ],
      ),
    );
  }
}

class OnDemandPage extends StatefulWidget {
  final ItcmdrClient client;
  const OnDemandPage({super.key, required this.client});
  @override
  State<OnDemandPage> createState() => _OnDemandPageState();
}

class _OnDemandPageState extends State<OnDemandPage> {
  List<Machine> _machines = [];
  final Set<String> _selected = {};
  String? _status;
  String? _requestId;
  bool _reportOnly = true;
  bool _repair = false;
  bool _force = false;

  Future<void> _loadMachines() async {
    final resp = await widget.client.listMachines(ListMachinesRequest());
    setState(() => _machines = resp.machines);
  }

  Future<void> _trigger() async {
    final resp = await widget.client.triggerRuns(TriggerRunsRequest(
        machines: _selected.toList(), reportOnly: _reportOnly, repair: _repair, force: _force));
    setState(() => _requestId = resp.requestId);
  }

  Future<void> _pollStatus() async {
    if (_requestId == null) return;
    final s = await widget.client.getRequestStatus(GetRequestStatusRequest(requestId: _requestId!));
    setState(() => _status = s.message);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        children: [
          Row(children: [
            ElevatedButton(onPressed: _loadMachines, child: const Text('Load machines')),
          ]),
          Expanded(
            child: ListView(
              children: _machines
                  .map((m) => CheckboxListTile(
                        title: Text(m.name),
                        value: _selected.contains(m.name),
                        onChanged: (v) => setState(() {
                          if (v == true) {
                            _selected.add(m.name);
                          } else {
                            _selected.remove(m.name);
                          }
                        }),
                      ))
                  .toList(),
            ),
          ),
          Row(children: [
            SwitchListTile(title: const Text('Report only'), value: _reportOnly, onChanged: (v) => setState(() => _reportOnly = v)),
            SwitchListTile(title: const Text('Repair'), value: _repair, onChanged: (v) => setState(() => _repair = v)),
            SwitchListTile(title: const Text('Force'), value: _force, onChanged: (v) => setState(() => _force = v)),
          ]),
          Row(children: [
            ElevatedButton(onPressed: _trigger, child: const Text('Trigger')),
            const SizedBox(width: 12),
            ElevatedButton(onPressed: _pollStatus, child: const Text('Poll status')),
            const SizedBox(width: 12),
            if (_status != null) Text('Status: $_status'),
          ]),
        ],
      ),
    );
  }
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
  Future<void> _search() async {
    final resp = await widget.client.searchLogs(SearchLogsRequest(
        query: _q.text, machine: _machine.text, startTs: 0, endTs: 0, limit: 100));
    setState(() => _lines = resp.lines);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        children: [
          Row(children: [
            Expanded(
                child: TextField(
              controller: _q,
              decoration: const InputDecoration(labelText: 'Query'),
            )),
            const SizedBox(width: 8),
            Expanded(
                child: TextField(
              controller: _machine,
              decoration: const InputDecoration(labelText: 'Machine (optional)'),
            )),
            const SizedBox(width: 8),
            ElevatedButton(onPressed: _search, child: const Text('Search')),
          ]),
          const SizedBox(height: 8),
          Expanded(
              child: ListView.builder(
                  itemCount: _lines.length,
                  itemBuilder: (c, i) => SelectableText(_lines[i],
                      style: const TextStyle(fontFamily: 'monospace'))))
        ],
      ),
    );
  }
}

ClientChannelBase createClientChannel() {
  // In browsers, this selects a GrpcWebClientChannel; on desktop, a native channel.
  const host = 'http://localhost:50051';
  return GrpcOrGrpcWebClientChannel.toSingleEndpoint(
    host: host,
    port: 0,
    transportSecure: false,
  );
}
