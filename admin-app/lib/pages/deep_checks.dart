// v1.3.0: Deep checks: every server's last deep backup check (date and ok/failed) and the
// overdue warning, worst first. Only what the site sends: adminsvr never holds, opens or
// inspects a backup; the check itself runs on the site with its 24 words
// (`itcmdr-cli backup drill --deep`).
import 'package:flutter/material.dart';

import '../backend.dart';
import '../session.dart';

/// 0: needs attention (warning), 1: no status yet, 2: fine.
int deepRank(Site s) {
  final st = s.lastStatus;
  if (st == null) return 1;
  return st.deepCheckWarning.isNotEmpty ? 0 : 2;
}

class DeepChecksPage extends StatefulWidget {
  const DeepChecksPage({super.key});
  @override
  State<DeepChecksPage> createState() => _DeepChecksPageState();
}

class _DeepChecksPageState extends State<DeepChecksPage> {
  List<Site>? sites;
  int? maxDays;
  String? error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final b = SessionScope.of(context).backend;
    try {
      final l = await b.sites();
      l.sort((a, c) {
        final r = deepRank(a).compareTo(deepRank(c));
        return r != 0 ? r : a.name.compareTo(c.name);
      });
      if (mounted) setState(() => sites = l);
    } catch (e) {
      if (mounted) setState(() => error = errorText(e));
    }
    try {
      final p = await b.orgPolicy();
      if (mounted) setState(() => maxDays = p.deepCheckMaxDays);
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final l = sites;
    final due = (l ?? []).where((s) => deepRank(s) == 0).length;
    return Scaffold(
      appBar: AppBar(title: const Text('Deep backup checks')),
      body: error != null
          ? Center(child: Text(error!))
          : l == null
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _load,
              child: ListView(
                children: [
                  ListTile(
                    key: const Key('deep-summary'),
                    title: Text(
                      due == 0
                          ? 'All servers are within the limit'
                          : '$due server(s) need a deep check',
                    ),
                    subtitle: Text(
                      'Warn after ${maxDays ?? '?'} days. The check runs on the site '
                      '(itcmdr-cli backup drill --deep); only its date and result are sent here.',
                    ),
                  ),
                  const Divider(),
                  if (l.isEmpty) const ListTile(title: Text('No servers yet')),
                  for (final x in l) _row(x),
                ],
              ),
            ),
    );
  }

  Widget _row(Site x) {
    final st = x.lastStatus;
    final warn = st?.deepCheckWarning ?? '';
    return ListTile(
      key: Key('deep-${x.id}'),
      leading: st == null
          ? const Icon(Icons.help_outline, color: Colors.grey)
          : warn.isNotEmpty
          ? const Icon(Icons.warning_amber, color: Colors.orange)
          : const Icon(Icons.verified, color: Colors.green),
      title: Text(x.name),
      subtitle: Text(
        st == null
            ? 'No status from this server yet'
            : [
                st.deepCheckText,
                if (warn.isNotEmpty) 'Warning: $warn',
              ].join('\n'),
      ),
      isThreeLine: warn.isNotEmpty,
    );
  }
}
