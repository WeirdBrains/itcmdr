// Alert inbox: open (open/acked/silenced), resolved or all, across the servers you may see.
// Ack and resolve go to the site as commands signed by you and adminsvr.
import 'dart:async';

import 'package:flutter/material.dart';

import '../backend.dart';
import '../session.dart';

class AlertsPage extends StatefulWidget {
  const AlertsPage({
    super.key,
    this.site = '',
    this.siteNames = const {},
    this.onNew,
  });
  final String site;
  final Map<String, String> siteNames;

  /// Called with alerts that appeared since the last refresh (in-app notification).
  final void Function(List<Alert>)? onNew;
  @override
  State<AlertsPage> createState() => _AlertsPageState();
}

class _AlertsPageState extends State<AlertsPage> {
  String state = 'open';
  List<Alert>? items;
  String? error;
  Timer? timer;
  final seen = <String>{};

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
    timer = Timer.periodic(const Duration(seconds: 30), (_) => _load());
  }

  @override
  void dispose() {
    timer?.cancel();
    super.dispose();
  }

  Future<void> _load() async {
    if (!mounted) return;
    final s = SessionScope.of(context);
    try {
      final l = await s.backend.alerts(widget.site, state);
      final fresh = l
          .where(
            (a) => a.state == 'open' && !seen.contains('${a.site}/${a.id}'),
          )
          .toList();
      final first = seen.isEmpty;
      seen.addAll(l.map((a) => '${a.site}/${a.id}'));
      if (!first && fresh.isNotEmpty) widget.onNew?.call(fresh);
      if (mounted) {
        setState(() {
          items = l;
          error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => error = errorText(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final l = items;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(8),
          child: SegmentedButton<String>(
            segments: const [
              ButtonSegment(value: 'open', label: Text('Open')),
              ButtonSegment(value: 'resolved', label: Text('Resolved')),
              ButtonSegment(value: 'all', label: Text('All')),
            ],
            selected: {state},
            onSelectionChanged: (v) {
              setState(() {
                state = v.first;
                items = null;
              });
              _load();
            },
          ),
        ),
        if (error != null)
          Padding(padding: const EdgeInsets.all(8), child: Text(error!)),
        Expanded(
          child: l == null
              ? const Center(child: CircularProgressIndicator())
              : l.isEmpty
              ? const Center(child: Text('No alerts', key: Key('no-alerts')))
              : RefreshIndicator(
                  onRefresh: _load,
                  child: ListView.builder(
                    itemCount: l.length,
                    itemBuilder: (c, i) {
                      final a = l[i];
                      final cs = Theme.of(c).colorScheme;
                      final where = a.machineDisplayName.isNotEmpty
                          ? a.machineDisplayName
                          : a.machine;
                      return ListTile(
                        key: Key('alert-${a.id}'),
                        leading: Icon(
                          a.severity == 'critical'
                              ? Icons.error
                              : Icons.warning_amber,
                          color: severityColor(a.severity, cs),
                        ),
                        title: Text(a.code),
                        subtitle: Text(
                          [
                            widget.siteNames[a.site] ?? a.site,
                            if (where.isNotEmpty) where,
                            '${a.severity} · ${a.state} · ×${a.count} · ${ago(a.lastSeenMs)}',
                          ].join(' · '),
                        ),
                        trailing:
                            a.state == 'resolved' || a.origin == 'adminsvr'
                            ? null
                            : PopupMenuButton<String>(
                                key: Key('alert-menu-${a.id}'),
                                onSelected: (v) async {
                                  if (v == 'ack') {
                                    await s.guard(
                                      c,
                                      Act.ackAlert,
                                      () => s.backend.ackAlert(a.site, a.id),
                                      done: 'Acknowledge sent to the site',
                                    );
                                  } else {
                                    await s.guard(
                                      c,
                                      Act.resolveAlert,
                                      () =>
                                          s.backend.resolveAlert(a.site, a.id),
                                      done: 'Resolve sent to the site',
                                    );
                                  }
                                  _load();
                                },
                                itemBuilder: (_) => [
                                  if (a.state == 'open')
                                    const PopupMenuItem(
                                      value: 'ack',
                                      child: Text('Acknowledge'),
                                    ),
                                  const PopupMenuItem(
                                    value: 'resolve',
                                    child: Text('Resolve'),
                                  ),
                                ],
                              ),
                      );
                    },
                  ),
                ),
        ),
      ],
    );
  }
}
