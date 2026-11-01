// Signed-in shell: Alerts, Servers, Plugins, Users (owners/admins), Settings; a banner while the
// owners' global freeze is on. A rail on wide screens,
// a bottom bar on phones. New open alerts show a banner (in-app notification).
import 'package:flutter/material.dart';

import 'backend.dart';
import 'push.dart';
import 'session.dart';
import 'pages/alerts.dart';
import 'pages/plugins.dart';
import 'pages/servers.dart';
import 'pages/settings.dart';
import 'pages/users.dart';

class Home extends StatefulWidget {
  const Home({super.key, this.push = const NoPush()});
  final PushService push;
  @override
  State<Home> createState() => _HomeState();
}

class _HomeState extends State<Home> {
  int tab = 0;
  Map<String, String> names = {};
  Freeze? freeze;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final b = SessionScope.of(context).backend;
      try {
        final l = await b.sites();
        if (mounted) setState(() => names = {for (final s in l) s.id: s.name});
      } catch (_) {}
      try {
        final f = await b.freeze();
        if (mounted) setState(() => freeze = f);
      } catch (_) {}
    });
  }

  void _newAlerts(List<Alert> a) {
    if (!mounted) return;
    final crit = a.where((x) => x.severity == 'critical').length;
    ScaffoldMessenger.of(context).showMaterialBanner(
      MaterialBanner(
        key: const Key('new-alerts'),
        leading: Icon(
          crit > 0 ? Icons.error : Icons.warning_amber,
          color: crit > 0 ? Colors.red : Colors.orange,
        ),
        content: Text(
          '${a.length} new alert(s)${crit > 0 ? ', $crit critical' : ''}',
        ),
        actions: [
          TextButton(
            onPressed: () {
              ScaffoldMessenger.of(context).hideCurrentMaterialBanner();
              setState(() => tab = 0);
            },
            child: const Text('Show'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final pages = <(String, IconData, Widget)>[
      (
        'Alerts',
        Icons.notifications,
        AlertsPage(siteNames: names, onNew: _newAlerts),
      ),
      ('Servers', Icons.dns, const ServersPage()),
      (
        'Plugins',
        Icons.extension,
        PluginsPage(
          siteNames: names,
          onFreeze: (f) => setState(() => freeze = f),
        ),
      ),
      if (s.isAdmin) ('Users', Icons.people, const UsersPage()),
      ('Settings', Icons.settings, SettingsPage(push: widget.push)),
    ];
    if (tab >= pages.length) tab = 0;
    final wide = MediaQuery.sizeOf(context).width >= 720;
    final stack = IndexedStack(
      index: tab,
      children: [for (final p in pages) p.$3],
    );
    final body = freeze?.frozen == true
        ? Column(
            children: [
              MaterialBanner(
                key: const Key('freeze-banner'),
                leading: const Icon(Icons.ac_unit, color: Colors.blue),
                content: Text(
                  'Global freeze is on (${freeze!.by}): no command or approval is sent to any site.',
                ),
                actions: [
                  TextButton(
                    onPressed: () => setState(() => tab = 2),
                    child: const Text('Details'),
                  ),
                ],
              ),
              Expanded(child: stack),
            ],
          )
        : stack;
    return Scaffold(
      appBar: AppBar(
        title: Text('itcmdr Admin · ${pages[tab].$1}'),
        actions: [
          Padding(
            padding: const EdgeInsets.all(12),
            child: Text('${s.me!.name} (${roleLabel(s.me!.role)})'),
          ),
        ],
      ),
      body: wide
          ? Row(
              children: [
                NavigationRail(
                  selectedIndex: tab,
                  labelType: NavigationRailLabelType.all,
                  onDestinationSelected: (i) => setState(() => tab = i),
                  destinations: [
                    for (final p in pages)
                      NavigationRailDestination(
                        icon: Icon(p.$2),
                        label: Text(p.$1),
                      ),
                  ],
                ),
                const VerticalDivider(width: 1),
                Expanded(child: body),
              ],
            )
          : body,
      bottomNavigationBar: wide
          ? null
          : NavigationBar(
              selectedIndex: tab,
              onDestinationSelected: (i) => setState(() => tab = i),
              destinations: [
                for (final p in pages)
                  NavigationDestination(icon: Icon(p.$2), label: p.$1),
              ],
            ),
    );
  }
}
