// Servers (sites): status, configuration and versions per server, metrics history, commands,
// remote backup start; admins create and revoke sites. Everything here is what the site sent
// redacted: opaque machine ids (names only with the site owner's opt-in), numbers and enums.
import 'package:flutter/material.dart';

import '../backend.dart';
import '../session.dart';
import 'alerts.dart';
import 'chart.dart';

class ServersPage extends StatefulWidget {
  const ServersPage({super.key});
  @override
  State<ServersPage> createState() => _ServersPageState();
}

class _ServersPageState extends State<ServersPage> {
  List<Site>? sites;
  String? error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    try {
      final l = await SessionScope.of(context).backend.sites();
      if (mounted) setState(() => sites = l);
    } catch (e) {
      if (mounted) setState(() => error = errorText(e));
    }
  }

  /// The newest site server version seen (for "update available").
  String get newest {
    final v =
        (sites ?? []).map((s) => s.version).where((v) => v.isNotEmpty).toList()
          ..sort(compareVersions);
    return v.isEmpty ? '' : v.last;
  }

  Future<void> _create(Session s) async {
    final name = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        title: const Text('New server'),
        content: TextField(
          key: const Key('site-name'),
          controller: name,
          decoration: const InputDecoration(
            labelText: 'Name (only shown here)',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(c, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(c, true),
            child: const Text('Create'),
          ),
        ],
      ),
    );
    if (ok != true || !mounted) return;
    final tok = await s.guard(
      context,
      Act.createSite,
      () => s.backend.createSite(name.text.trim()),
    );
    if (tok == null || !mounted) return;
    await showSecret(
      context,
      'Site token',
      'On the site server run: itcmdr-cli adminsvr enroll <token>. Single use.',
      tok,
    );
    _load();
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final l = sites;
    return Scaffold(
      floatingActionButton: s.isAdmin
          ? FloatingActionButton.extended(
              key: const Key('add-site'),
              heroTag: null,
              onPressed: () => _create(s),
              icon: const Icon(Icons.add),
              label: const Text('Server'),
            )
          : null,
      body: error != null
          ? Center(child: Text(error!))
          : l == null
          ? const Center(child: CircularProgressIndicator())
          : l.isEmpty
          ? const Center(child: Text('No servers yet'))
          : RefreshIndicator(
              onRefresh: _load,
              child: ListView(
                children: [
                  for (final x in l)
                    ListTile(
                      key: Key('site-${x.id}'),
                      leading: Icon(Icons.dns, color: statusColor(x.status)),
                      title: Text(x.name),
                      subtitle: Text(
                        [
                          x.status,
                          if (x.version.isNotEmpty) 'v${x.version}',
                          'seen ${ago(x.lastSeenMs)}',
                          if (x.lastStatus != null)
                            '${x.lastStatus!.alertsCritical} critical · ${x.lastStatus!.alertsCautious} cautious',
                        ].join(' · '),
                      ),
                      trailing:
                          x.version.isNotEmpty &&
                              newest.isNotEmpty &&
                              compareVersions(x.version, newest) < 0
                          ? const Chip(label: Text('update available'))
                          : null,
                      onTap: () async {
                        await Navigator.push(
                          context,
                          MaterialPageRoute(
                            builder: (_) => SessionScope(
                              session: s,
                              child: ServerPage(site: x, newest: newest),
                            ),
                          ),
                        );
                        _load();
                      },
                    ),
                ],
              ),
            ),
    );
  }
}

int compareVersions(String a, String b) {
  List<int> p(String v) => v
      .split('.')
      .map((x) => int.tryParse(x.split(RegExp(r'[^0-9]')).first) ?? 0)
      .toList();
  final x = p(a), y = p(b);
  for (var i = 0; i < 3; i++) {
    final d = (i < x.length ? x[i] : 0) - (i < y.length ? y[i] : 0);
    if (d != 0) return d;
  }
  return 0;
}

Color statusColor(String s) => switch (s) {
  'online' => Colors.green,
  'late' => Colors.orange,
  'unreachable' => Colors.red,
  'revoked' => Colors.grey,
  _ => Colors.blueGrey,
};

Future<void> showSecret(
  BuildContext context,
  String title,
  String help,
  String secret,
) => showDialog<void>(
  context: context,
  builder: (c) => AlertDialog(
    title: Text(title),
    content: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(help),
        const SizedBox(height: 12),
        SelectableText(
          secret,
          key: const Key('secret'),
          style: const TextStyle(fontFamily: 'monospace'),
        ),
      ],
    ),
    actions: [
      FilledButton(
        onPressed: () => Navigator.pop(c),
        child: const Text('Done'),
      ),
    ],
  ),
);

class ServerPage extends StatefulWidget {
  const ServerPage({super.key, required this.site, required this.newest});
  final Site site;
  final String newest;
  @override
  State<ServerPage> createState() => _ServerPageState();
}

class _ServerPageState extends State<ServerPage> {
  int hours = 24;
  List<Sample>? samples;
  List<CommandRow>? cmds;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final b = SessionScope.of(context).backend;
    try {
      final m = await b.metrics(widget.site.id, hours);
      final c = await b.commands(widget.site.id);
      if (mounted) {
        setState(() {
          samples = m;
          cmds = c;
        });
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(errorText(e))));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final x = widget.site;
    final st = x.lastStatus;
    final t = Theme.of(context).textTheme;
    return DefaultTabController(
      length: 4,
      child: Scaffold(
        appBar: AppBar(
          title: Text(x.name),
          actions: [
            IconButton(
              key: const Key('backup'),
              tooltip: 'Start a backup on the site',
              icon: const Icon(Icons.backup),
              onPressed: x.status == 'revoked'
                  ? null
                  : () async {
                      await s.guard(
                        context,
                        Act.startBackup,
                        () => s.backend.startBackup(x.id),
                        done:
                            'Backup requested (it runs and stays on the site)',
                      );
                      _load();
                    },
            ),
            if (s.isAdmin && x.status != 'revoked')
              IconButton(
                key: const Key('revoke-site'),
                tooltip: 'Revoke this server',
                icon: const Icon(Icons.block),
                onPressed: () async {
                  final ok = await confirm(
                    context,
                    'Revoke ${x.name}?',
                    'The site can no longer link to adminsvr; enroll it again with a new token.',
                  );
                  if (ok && context.mounted) {
                    await s.guard(
                      context,
                      Act.revokeSite,
                      () => s.backend.revokeSite(x.id),
                      done: 'Revoked',
                    );
                    if (context.mounted) Navigator.pop(context);
                  }
                },
              ),
          ],
          bottom: const TabBar(
            tabs: [
              Tab(text: 'Status'),
              Tab(text: 'Metrics'),
              Tab(text: 'Alerts'),
              Tab(text: 'Commands'),
            ],
          ),
        ),
        body: TabBarView(
          children: [
            // Status, configuration and versions
            ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Text('Configuration', style: t.titleMedium),
                kv('Status', '${x.status} (seen ${ago(x.lastSeenMs)})'),
                kv(
                  'Site server version',
                  x.version.isEmpty ? 'unknown' : 'v${x.version}',
                ),
                if (widget.newest.isNotEmpty &&
                    x.version.isNotEmpty &&
                    compareVersions(x.version, widget.newest) < 0)
                  kv(
                    'Update',
                    'v${widget.newest} runs on another server',
                    key: const Key('site-update'),
                  ),
                if (st != null) ...[
                  kv('Auth mode', st.authMode),
                  kv(
                    'Cutover',
                    st.cutoverState.isEmpty ? '-' : st.cutoverState,
                  ),
                  kv(
                    'Display names shared',
                    x.shareDisplayNames ? 'yes (site opt-in)' : 'no',
                  ),
                  kv(
                    'Uptime',
                    '${(st.uptimeSecs / 3600).toStringAsFixed(1)} h',
                  ),
                  kv(
                    'Backup',
                    st.backupAgeSecs < 0
                        ? 'none'
                        : '${st.backupLastResult}, ${(st.backupAgeSecs / 3600).toStringAsFixed(1)} h old, ${(st.backupSizeBytes / 1048576).toStringAsFixed(1)} MB',
                  ),
                  kv(
                    'Agents',
                    '${st.agentsOnline} online · ${st.agentsLate} late · ${st.agentsNonResponsive} not responding · ${st.agentsOther} other',
                  ),
                  const SizedBox(height: 16),
                  Text('Machines and versions', style: t.titleMedium),
                  for (final m in st.machines)
                    ListTile(
                      key: Key('machine-${m.id}'),
                      dense: true,
                      leading: Icon(
                        Icons.computer,
                        color: statusColor(
                          m.state == 'online'
                              ? 'online'
                              : m.state == 'late'
                              ? 'late'
                              : 'unreachable',
                        ),
                      ),
                      title: Text(
                        m.displayName.isNotEmpty ? m.displayName : m.id,
                      ),
                      subtitle: Text(
                        '${m.state} · v${m.version} · seen ${ago(m.lastSeenMs)}',
                      ),
                      trailing: m.updateAvailable
                          ? const Chip(label: Text('update available'))
                          : null,
                    ),
                ] else
                  const Text('No status from this server yet.'),
                const SizedBox(height: 12),
                Text(
                  'Changing a server\'s settings or updating it remotely comes with v0.20/v0.21 '
                  '(commands over the link with two-person approval).',
                  style: t.bodySmall,
                ),
              ],
            ),
            // Metrics
            ListView(
              padding: const EdgeInsets.all(16),
              children: [
                SegmentedButton<int>(
                  segments: const [
                    ButtonSegment(value: 6, label: Text('6 h')),
                    ButtonSegment(value: 24, label: Text('24 h')),
                    ButtonSegment(value: 168, label: Text('7 d')),
                    ButtonSegment(value: 720, label: Text('30 d')),
                  ],
                  selected: {hours},
                  onSelectionChanged: (v) {
                    setState(() {
                      hours = v.first;
                      samples = null;
                    });
                    _load();
                  },
                ),
                const SizedBox(height: 12),
                if (samples == null)
                  const Center(child: CircularProgressIndicator())
                else if (samples!.isEmpty)
                  const Text(
                    'No samples in this window.',
                    key: Key('no-samples'),
                  )
                else ...[
                  MetricChart(
                    key: const Key('chart-agents'),
                    title: 'Agents',
                    series: {
                      'online': [
                        for (final p in samples!)
                          (p.tsMs, p.agentsOnline.toDouble()),
                      ],
                      'late': [
                        for (final p in samples!)
                          (p.tsMs, p.agentsLate.toDouble()),
                      ],
                      'not responding': [
                        for (final p in samples!)
                          (p.tsMs, p.agentsNonResponsive.toDouble()),
                      ],
                    },
                  ),
                  MetricChart(
                    title: 'Open alerts',
                    series: {
                      'critical': [
                        for (final p in samples!)
                          (p.tsMs, p.alertsCritical.toDouble()),
                      ],
                      'cautious': [
                        for (final p in samples!)
                          (p.tsMs, p.alertsCautious.toDouble()),
                      ],
                    },
                  ),
                  MetricChart(
                    title: 'Backup age (h)',
                    series: {
                      'age': [
                        for (final p in samples!)
                          if (p.backupAgeSecs >= 0)
                            (p.tsMs, p.backupAgeSecs / 3600),
                      ],
                    },
                  ),
                ],
              ],
            ),
            SessionScope(
              session: s,
              child: AlertsPage(site: x.id, siteNames: {x.id: x.name}),
            ),
            cmds == null
                ? const Center(child: CircularProgressIndicator())
                : ListView(
                    children: [
                      for (final c in cmds!)
                        ListTile(
                          dense: true,
                          title: Text(c.kind),
                          subtitle: Text(
                            '${c.state}${c.resultCode.isEmpty ? '' : ' (${c.resultCode})'} · ${ago(c.createdMs)}',
                          ),
                        ),
                      if (cmds!.isEmpty)
                        const ListTile(title: Text('No commands yet')),
                    ],
                  ),
          ],
        ),
      ),
    );
  }
}

Widget kv(String k, String v, {Key? key}) => Padding(
  key: key,
  padding: const EdgeInsets.symmetric(vertical: 4),
  child: Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      SizedBox(
        width: 180,
        child: Text(k, style: const TextStyle(fontWeight: FontWeight.w600)),
      ),
      Expanded(child: Text(v)),
    ],
  ),
);

Future<bool> confirm(BuildContext context, String title, String body) async =>
    await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        title: Text(title),
        content: Text(body),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(c, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('confirm'),
            onPressed: () => Navigator.pop(c, true),
            child: const Text('OK'),
          ),
        ],
      ),
    ) ??
    false;
