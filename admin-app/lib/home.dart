// Signed-in shell: Alerts, Servers, Users (owners/admins), Settings. A rail on wide screens,
// a bottom bar on phones. New open alerts show a banner (in-app notification).
import 'package:flutter/material.dart';

import 'backend.dart';
import 'push.dart';
import 'session.dart';
import 'pages/alerts.dart';
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

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      try {
        final l = await SessionScope.of(context).backend.sites();
        if (mounted) setState(() => names = {for (final s in l) s.id: s.name});
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
      if (s.isAdmin) ('Users', Icons.people, const UsersPage()),
      ('Settings', Icons.settings, SettingsPage(push: widget.push)),
    ];
    if (tab >= pages.length) tab = 0;
    final wide = MediaQuery.sizeOf(context).width >= 720;
    final body = IndexedStack(
      index: tab,
      children: [for (final p in pages) p.$3],
    );
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
