// v0.12.0 Service plugins: what is installed on the server, run one (report-only by default),
// recent runs, and the metrics a plugin logs (itcmdr.log/v1) as a daily chart.
import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';

import 'charts.dart';
import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';

/// One bar per day for metric [name] (all label sets summed).
List<ChartBar> pluginSeries(Iterable<PluginMetricPoint> pts, String name) {
  final m = <int, double>{};
  for (final p in pts.where((p) => p.name == name)) {
    m[p.ts.toInt()] = (m[p.ts.toInt()] ?? 0) + p.value;
  }
  final ks = m.keys.toList()..sort();
  return [for (final k in ks) ChartBar(k, m[k]!)];
}

String _ts(Int64 ms) {
  if (ms == Int64.ZERO) return '-';
  final d = DateTime.fromMillisecondsSinceEpoch(ms.toInt());
  String two(int n) => n.toString().padLeft(2, '0');
  return '${d.month}/${d.day} ${two(d.hour)}:${two(d.minute)}';
}

class PluginsPage extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  const PluginsPage({super.key, required this.client, required this.settings});
  @override
  State<PluginsPage> createState() => _PluginsPageState();
}

class _PluginsPageState extends State<PluginsPage> {
  List<PluginInfo> _plugins = [];
  List<PluginRun> _runs = [];
  List<PluginMetricPoint> _points = [];
  List<LogRejects> _rejects = [];
  String? _error;
  String? _info;
  String? _selected;
  bool _busy = false;

  CallOptions get _opts => widget.settings.authOptions();

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final p = await widget.client.listPlugins(
        ListPluginsRequest(),
        options: _opts,
      );
      final r = await widget.client.listPluginRuns(
        ListPluginRunsRequest(plugin: _selected ?? '', limit: 30),
        options: _opts,
      );
      final m = await widget.client.getPluginMetrics(
        GetPluginMetricsRequest(windowDays: 30, daily: true),
        options: _opts,
      );
      if (!mounted) return;
      setState(() {
        _plugins = p.plugins;
        _runs = r.runs;
        _points = m.points;
        _rejects = m.rejects;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  Future<void> _run(PluginInfo p, bool reportOnly) async {
    setState(() => _busy = true);
    try {
      final r = await widget.client.runPlugin(
        RunPluginRequest(
          plugin: '${p.name}@${p.version}',
          reportOnly: reportOnly,
        ),
        options: _opts,
      );
      _info =
          '${p.name}: ${r.status}${r.message.isEmpty ? '' : ' (${r.message})'}'
          '${r.results.isEmpty ? '' : '\n${r.results.map((x) => '${x.checkId} ${x.status} ${x.message}').join('\n')}'}';
      await _load();
    } catch (e) {
      setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final names = _points.map((p) => p.name).toSet().toList()..sort();
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            Text('Service plugins', style: t.titleMedium),
            const Spacer(),
            IconButton(onPressed: _load, icon: const Icon(Icons.refresh)),
          ],
        ),
        const Text(
          'Plugins run on the server only. Packages are fetched and checked by the server; '
          'PCs run only the built-in checks installed with the agent.',
        ),
        if (_error != null)
          Text(_error!, style: const TextStyle(color: Colors.red)),
        if (_info != null) SelectableText(_info!),
        if (_busy) const LinearProgressIndicator(),
        if (_plugins.isEmpty)
          const Text('No plugins installed (itcmdr-cli plugins install).'),
        for (final p in _plugins)
          ListTile(
            dense: true,
            selected: _selected == p.name,
            leading: Icon(p.enabled ? Icons.extension : Icons.extension_off),
            title: Text(
              '${p.name} ${p.version}${p.enabled ? '' : '  (disabled)'}',
            ),
            subtitle: Text('installed ${p.installedAt} · signer ${p.signer}'),
            onTap: () {
              setState(() => _selected = _selected == p.name ? null : p.name);
              _load();
            },
            trailing: p.enabled
                ? Wrap(
                    children: [
                      IconButton(
                        tooltip: 'Run (report only)',
                        icon: const Icon(Icons.play_arrow),
                        onPressed: _busy ? null : () => _run(p, true),
                      ),
                      IconButton(
                        tooltip: 'Run (may repair)',
                        icon: const Icon(Icons.build),
                        onPressed: _busy ? null : () => _run(p, false),
                      ),
                    ],
                  )
                : null,
          ),
        const Divider(height: 24),
        Text(
          'Recent runs${_selected == null ? '' : ' of $_selected'}',
          style: t.titleSmall,
        ),
        for (final r in _runs)
          ListTile(
            dense: true,
            title: Text(
              '${_ts(r.started)}  ${r.plugin} ${r.version}  ${r.status}',
            ),
            subtitle: Text(
              [r.machine, r.message].where((x) => x.isNotEmpty).join(' · '),
            ),
          ),
        const Divider(height: 24),
        Text('Plugin metrics (30 days, daily)', style: t.titleSmall),
        if (names.isEmpty) const Text('No metrics logged yet.'),
        for (final n in names)
          BarChart(title: n, bars: pluginSeries(_points, n)),
        for (final r in _rejects.where((r) => r.count > 0))
          Text(
            '${r.file}: ${r.count} malformed log lines',
            style: const TextStyle(color: Colors.orange),
          ),
      ],
    );
  }
}
