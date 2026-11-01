// v0.20.0: a server's configuration and plugins as the site sent them (redacted: paths and text
// stay on the site, schedules have opaque ids). Changes go down as signed commands: preview
// first (dry run, the site answers with before/after and changes nothing), then apply. Plugin
// changes wait for a second owner/admin. The site checks everything again on its side.
import 'package:flutter/material.dart';

import '../backend.dart';
import '../session.dart';

/// Send `kind` as a dry run, show the site's preview, then apply on OK. Returns the final row
/// (null when cancelled or refused).
Future<CommandRow?> previewThenApply(
  BuildContext context,
  String site,
  String kind,
  List<String> args, {
  required String title,
}) async {
  final s = SessionScope.of(context);
  final b = s.backend;
  final dry = await s.guard<CommandRow>(
    context,
    Act.sendCommand,
    () async =>
        waitCommand(b, (await b.sendCommand(site, kind, args, true)).id),
  );
  if (dry == null || !context.mounted) return null;
  if (dry.state != 'previewed') {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        key: const Key('preview-refused'),
        content: Text(
          'The site refused: ${dry.resultCode.isEmpty ? dry.state : dry.resultCode}',
        ),
      ),
    );
    return null;
  }
  final two = twoPersonKinds.contains(kind);
  final ok = await showDialog<bool>(
    context: context,
    builder: (c) => AlertDialog(
      key: const Key('preview-dialog'),
      title: Text(title),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('The site says this would change:'),
          const SizedBox(height: 8),
          for (final p in dry.preview)
            Text(
              '${p.target}: ${p.before.isEmpty ? '-' : p.before} → ${p.after.isEmpty ? '-' : p.after}',
              key: Key('preview-${p.target}'),
            ),
          if (dry.preview.isEmpty) const Text('nothing'),
          if (two) ...[
            const SizedBox(height: 8),
            const Text(
              'A second owner or admin must approve before the site runs it.',
            ),
          ],
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(c, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('apply'),
          onPressed: () => Navigator.pop(c, true),
          child: Text(two ? 'Request' : 'Apply'),
        ),
      ],
    ),
  );
  if (ok != true || !context.mounted) return null;
  final r = await s.guard<CommandRow>(context, Act.sendCommand, () async {
    final c = await b.sendCommand(site, kind, args, false);
    return c.state == 'awaiting-approval' ? c : waitCommand(b, c.id);
  });
  if (r != null && context.mounted) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        key: const Key('command-result'),
        content: Text(
          r.state == 'awaiting-approval'
              ? 'Waiting for a second owner or admin to approve'
              : '${r.state}${r.resultCode.isEmpty || r.resultCode == 'ok' ? '' : ' (${r.resultCode})'}',
        ),
      ),
    );
  }
  return r;
}

class SiteConfigTab extends StatefulWidget {
  const SiteConfigTab({super.key, required this.site});
  final String site;
  @override
  State<SiteConfigTab> createState() => _SiteConfigTabState();
}

class _SiteConfigTabState extends State<SiteConfigTab> {
  SiteConfig? cfg;
  String filter = '';

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    try {
      final c = await SessionScope.of(context).backend.siteConfig(widget.site);
      if (mounted) setState(() => cfg = c);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(errorText(e))));
      }
    }
  }

  bool _allowed(String kind) => cfg?.allowed.contains(kind) ?? false;

  Future<void> _edit(Setting x) async {
    final ctl = TextEditingController(text: x.value);
    var b = x.value == 'true';
    final v = await showDialog<String>(
      context: context,
      builder: (c) => StatefulBuilder(
        builder: (c, set) => AlertDialog(
          title: Text(x.key),
          content: x.kind == 'bool'
              ? SwitchListTile(
                  key: const Key('edit-bool'),
                  title: Text(b ? 'on' : 'off'),
                  value: b,
                  onChanged: (n) => set(() => b = n),
                )
              : TextField(
                  key: const Key('edit-value'),
                  controller: ctl,
                  decoration: InputDecoration(
                    helperText: x.kind == 'int'
                        ? '${x.min} to ${x.max}'
                        : x.kind == 'time'
                        ? 'HH:mm'
                        : null,
                  ),
                ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(c),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const Key('edit-preview'),
              onPressed: () =>
                  Navigator.pop(c, x.kind == 'bool' ? '$b' : ctl.text.trim()),
              child: const Text('Preview'),
            ),
          ],
        ),
      ),
    );
    if (v == null || !mounted) return;
    await previewThenApply(context, widget.site, 'setting.set', [
      'key=${x.key}',
      'value=$v',
    ], title: 'Change ${x.key}?');
    _load();
  }

  Future<void> _schedule(Schedule x, String kind) async {
    await previewThenApply(context, widget.site, kind, [
      'schedule=${x.id}',
    ], title: '${kind.split('.').last} schedule?');
    _load();
  }

  @override
  Widget build(BuildContext context) {
    final c = cfg;
    final t = Theme.of(context).textTheme;
    if (c == null) return const Center(child: CircularProgressIndicator());
    if (c.receivedMs == 0) {
      return const Center(
        key: Key('no-config'),
        child: Text(
          'This server has not sent its configuration yet (v0.20.0 or later).',
        ),
      );
    }
    final canSet = _allowed('setting.set');
    final list = c.settings
        .where((x) => filter.isEmpty || x.key.contains(filter))
        .toList();
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(
            'Received ${ago(c.receivedMs)} · two-person approval ${c.requireTwoPerson ? 'on' : 'off'} · '
            'the site allows: ${c.allowed.isEmpty ? 'nothing' : c.allowed.join(', ')}',
            style: t.bodySmall,
          ),
          TextField(
            key: const Key('config-search'),
            decoration: const InputDecoration(
              prefixIcon: Icon(Icons.search),
              hintText: 'Filter settings',
            ),
            onChanged: (v) => setState(() => filter = v.trim()),
          ),
          for (final x in list)
            ListTile(
              key: Key('cfg-${x.key}'),
              dense: true,
              title: Text(x.key),
              subtitle: Text(
                '${x.hidden ? 'hidden (stays on the site)' : x.value} · ${x.kind} · ${x.source}',
              ),
              trailing: x.remote && canSet
                  ? IconButton(
                      key: Key('edit-${x.key}'),
                      tooltip: 'Change (preview first)',
                      icon: const Icon(Icons.edit),
                      onPressed: () => _edit(x),
                    )
                  : const Tooltip(
                      message: 'Only on the site itself',
                      child: Icon(Icons.lock_outline, size: 18),
                    ),
            ),
          const SizedBox(height: 16),
          Text('Schedules', style: t.titleMedium),
          for (final x in c.schedules)
            ListTile(
              key: Key('sched-${x.id}'),
              dense: true,
              leading: Icon(
                x.enabled ? Icons.schedule : Icons.pause_circle_outline,
              ),
              title: Text(
                x.name.isNotEmpty
                    ? x.name
                    : (x.builtin.isNotEmpty ? x.builtin : x.id),
              ),
              subtitle: Text(
                '${x.action} · ${x.when.isEmpty ? '(custom)' : x.when} ${x.timezone} · '
                '${x.allMachines ? 'all machines' : '${x.machines} machine(s)'}'
                '${x.lastFireMs > 0 ? ' · last ${ago(x.lastFireMs)}' : ''}',
              ),
              trailing: Wrap(
                children: [
                  if (_allowed('schedule.run'))
                    IconButton(
                      key: Key('run-${x.id}'),
                      tooltip: 'Run now',
                      icon: const Icon(Icons.play_arrow),
                      onPressed: () => _schedule(x, 'schedule.run'),
                    ),
                  if (x.enabled && _allowed('schedule.pause'))
                    IconButton(
                      key: Key('pause-${x.id}'),
                      tooltip: 'Pause',
                      icon: const Icon(Icons.pause),
                      onPressed: () => _schedule(x, 'schedule.pause'),
                    ),
                  if (!x.enabled && _allowed('schedule.resume'))
                    IconButton(
                      key: Key('resume-${x.id}'),
                      tooltip: 'Resume',
                      icon: const Icon(Icons.replay),
                      onPressed: () => _schedule(x, 'schedule.resume'),
                    ),
                ],
              ),
            ),
          if (c.schedules.isEmpty) const Text('No schedules.'),
        ],
      ),
    );
  }
}

class SitePluginsTab extends StatefulWidget {
  const SitePluginsTab({super.key, required this.site});
  final String site;
  @override
  State<SitePluginsTab> createState() => _SitePluginsTabState();
}

class _SitePluginsTabState extends State<SitePluginsTab> {
  SitePlugins? p;
  List<CatalogEntry> cat = [];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final b = SessionScope.of(context).backend;
    try {
      final x = await b.sitePlugins(widget.site);
      final c = await b.catalog();
      if (mounted) {
        setState(() {
          p = x;
          cat = c;
        });
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(errorText(e))));
      }
    }
  }

  Future<void> _push() async {
    final e = await showDialog<CatalogEntry>(
      context: context,
      builder: (c) => SimpleDialog(
        key: const Key('push-dialog'),
        title: const Text('Install from the catalog'),
        children: [
          for (final x in cat)
            SimpleDialogOption(
              key: Key('push-${x.sha256}'),
              onPressed: () => Navigator.pop(c, x),
              child: Text(
                '${x.name} v${x.version} (${x.sha256.substring(0, 12)}…)',
              ),
            ),
          if (cat.isEmpty)
            const Padding(
              padding: EdgeInsets.all(16),
              child: Text('The catalog is empty: add a package under Plugins.'),
            ),
        ],
      ),
    );
    if (e == null || !mounted) return;
    await previewThenApply(context, widget.site, 'plugin.install', [
      'sha256=${e.sha256}',
    ], title: 'Install ${e.name} v${e.version}?');
    _load();
  }

  Future<void> _toggle(InstalledPlugin x) async {
    final kind = x.enabled ? 'plugin.disable' : 'plugin.enable';
    await previewThenApply(context, widget.site, kind, [
      'name=${x.name}',
      if (!x.enabled) 'version=${x.version}',
    ], title: '${x.enabled ? 'Disable' : 'Enable'} ${x.name}?');
    _load();
  }

  @override
  Widget build(BuildContext context) {
    final x = p;
    final t = Theme.of(context).textTheme;
    if (x == null) return const Center(child: CircularProgressIndicator());
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(
            children: [
              Expanded(child: Text('Installed', style: t.titleMedium)),
              FilledButton.icon(
                key: const Key('push-plugin'),
                icon: const Icon(Icons.download),
                label: const Text('Install…'),
                onPressed: _push,
              ),
            ],
          ),
          if (x.receivedMs == 0)
            const Text('This server has not sent its plugin list yet.'),
          for (final i in x.installed)
            ListTile(
              key: Key('plugin-${i.name}-${i.version}'),
              dense: true,
              leading: Icon(i.enabled ? Icons.extension : Icons.extension_off),
              title: Text('${i.name} v${i.version}'),
              subtitle: Text(i.enabled ? 'enabled' : 'disabled'),
              trailing: Switch(
                key: Key('toggle-${i.name}-${i.version}'),
                value: i.enabled,
                onChanged: (_) => _toggle(i),
              ),
            ),
          const SizedBox(height: 16),
          Text('Quarantine', style: t.titleMedium),
          for (final q in x.quarantine)
            ListTile(
              key: Key('quarantine-${q.sha256}'),
              dense: true,
              leading: const Icon(Icons.gpp_bad, color: Colors.red),
              title: Text(q.reason),
              subtitle: Text('${q.sha256.substring(0, 12)}… · ${ago(q.tsMs)}'),
            ),
          if (x.quarantine.isEmpty) const Text('Nothing quarantined.'),
        ],
      ),
    );
  }
}
