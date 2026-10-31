// v0.12.0 Service plugins: what is installed on the server, run one (report-only by default),
// recent runs, and the metrics a plugin logs (itcmdr.log/v1) as a daily chart.
import 'dart:io';

import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';

import 'charts.dart';
import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';

/// What the Install form sends: exactly one of an https URL or a local package file.
/// Returns an error message, or null.
String? validateInstall(String url, String path) {
  final u = url.trim(), p = path.trim();
  if (u.isEmpty == p.isEmpty) {
    return 'Enter a package URL or a file path (one of them)';
  }
  if (u.isNotEmpty && !u.startsWith('https://')) {
    return 'Only https:// URLs (the server downloads the package)';
  }
  if (p.isNotEmpty && !p.toLowerCase().endsWith('.tar.gz')) {
    return 'Plugin packages are .tar.gz files';
  }
  return null;
}

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
  bool _canManage = false;
  bool _installDisabled = false;
  List<QuarantineEntry> _quarantine = [];
  final _url = TextEditingController();
  final _path = TextEditingController();

  @override
  void dispose() {
    _url.dispose();
    _path.dispose();
    super.dispose();
  }

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
      var manage = false;
      List<QuarantineEntry> q = [];
      try {
        final me = await widget.client.whoAmI(WhoAmIRequest(), options: _opts);
        manage = me.signed && (me.role == 'admin' || me.role == 'owner');
        if (manage) {
          q = (await widget.client.listQuarantine(
            ListQuarantineRequest(),
            options: _opts,
          )).entries;
        }
      } catch (_) {}
      if (!mounted) return;
      setState(() {
        _canManage = manage;
        _quarantine = q;
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

  Future<void> _act(Future<PluginActionResponse> Function() f) async {
    setState(() => _busy = true);
    try {
      _info = (await f()).message;
      await _load();
    } catch (e) {
      setState(() => _error = e is GrpcError ? (e.message ?? '$e') : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _install() async {
    final err = validateInstall(_url.text, _path.text);
    if (err != null) {
      setState(() => _error = err);
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final req = InstallPluginRequest(disabled: _installDisabled);
      if (_url.text.trim().isNotEmpty) {
        req.url = _url.text.trim();
      } else {
        final f = File(_path.text.trim());
        req.package = await f.readAsBytes();
        req.fileName = f.uri.pathSegments.last;
      }
      final r = await widget.client.installPlugin(req, options: _opts);
      _info = r.message;
      if (!r.quarantined) {
        _url.clear();
        _path.clear();
      } else {
        _error = r.message;
      }
      await _load();
    } catch (e) {
      setState(() => _error = e is GrpcError ? (e.message ?? '$e') : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _remove(PluginInfo p) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Remove ${p.name} ${p.version}?'),
        content: Text(
          p.enabled
              ? 'It is enabled: it is disabled and removed.'
              : 'Its files are deleted; runs stay in the history.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('plugin-remove-go'),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Remove'),
          ),
        ],
      ),
    );
    if (ok != true) return;
    await _act(
      () => widget.client.removePlugin(
        PluginRef(spec: '${p.name}@${p.version}', force: p.enabled),
        options: _opts,
      ),
    );
  }

  Widget _installCard() => Card(
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Install a plugin package (checked and quarantined like itcmdr-cli plugins install)',
          ),
          TextField(
            key: const Key('plugin-url'),
            controller: _url,
            decoration: const InputDecoration(
              labelText: 'Package URL (https://, downloaded by the server)',
            ),
          ),
          TextField(
            key: const Key('plugin-path'),
            controller: _path,
            decoration: const InputDecoration(
              labelText: 'or a .tar.gz file on this PC (uploaded)',
            ),
          ),
          CheckboxListTile(
            key: const Key('plugin-disabled'),
            contentPadding: EdgeInsets.zero,
            value: _installDisabled,
            onChanged: (v) => setState(() => _installDisabled = v ?? false),
            title: const Text('Install disabled (enable it later)'),
          ),
          FilledButton.icon(
            key: const Key('plugin-install'),
            onPressed: _busy ? null : _install,
            icon: const Icon(Icons.download),
            label: const Text('Install'),
          ),
        ],
      ),
    ),
  );

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
        if (_canManage) _installCard(),
        if (_plugins.isEmpty) const Text('No plugins installed.'),
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
            trailing: Wrap(
              children: [
                if (_canManage)
                  IconButton(
                    tooltip: p.enabled ? 'Disable' : 'Enable',
                    icon: Icon(p.enabled ? Icons.toggle_on : Icons.toggle_off),
                    onPressed: _busy
                        ? null
                        : () => _act(
                            () => p.enabled
                                ? widget.client.disablePlugin(
                                    PluginRef(spec: p.name),
                                    options: _opts,
                                  )
                                : widget.client.enablePlugin(
                                    PluginRef(spec: '${p.name}@${p.version}'),
                                    options: _opts,
                                  ),
                          ),
                  ),
                if (_canManage)
                  IconButton(
                    tooltip: 'Remove',
                    icon: const Icon(Icons.delete_outline),
                    onPressed: _busy ? null : () => _remove(p),
                  ),
                if (p.enabled) ...[
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
              ],
            ),
          ),
        if (_canManage && _quarantine.isNotEmpty) ...[
          const Divider(height: 24),
          Text('Quarantined packages', style: t.titleSmall),
          for (final q in _quarantine)
            ListTile(
              dense: true,
              title: Text('${q.ts}  ${q.source}'),
              subtitle: Text(q.reason),
            ),
        ],
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
