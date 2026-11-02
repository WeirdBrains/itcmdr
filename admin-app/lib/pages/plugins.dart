// v0.20.0 Plugins: the catalog of signed packages (owners/admins add them; adminsvr cannot sign
// one, each site checks signature and files on install), commands waiting for a second person,
// and the owners' global freeze.
import 'package:flutter/material.dart';

import '../backend.dart';
import '../session.dart';
import 'servers.dart' show confirm;

class PluginsPage extends StatefulWidget {
  const PluginsPage({super.key, this.siteNames = const {}, this.onFreeze});
  final Map<String, String> siteNames;
  final void Function(Freeze)? onFreeze;
  @override
  State<PluginsPage> createState() => _PluginsPageState();
}

class _PluginsPageState extends State<PluginsPage> {
  List<CatalogEntry>? cat;
  List<CommandRow> pending = [];
  Freeze? fz;
  OrgPolicy? org;
  List<Release> rels = [];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final b = SessionScope.of(context).backend;
    try {
      final c = await b.catalog();
      final p = await b.pendingApprovals();
      final f = await b.freeze();
      final o = await b.orgPolicy();
      final r = await b.releases();
      if (mounted) {
        setState(() {
          org = o;
          rels = r;
          cat = c;
          pending = p;
          fz = f;
        });
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(errorText(e))));
      }
    }
  }

  Future<void> _upload() async {
    final ctl = TextEditingController();
    final path = await showDialog<String>(
      context: context,
      builder: (c) => AlertDialog(
        title: const Text('Add a signed package'),
        content: TextField(
          key: const Key('upload-path'),
          controller: ctl,
          decoration: const InputDecoration(
            labelText: 'Package file (.tar.gz, from itcmdr-cli plugin pack)',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(c),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('upload-ok'),
            onPressed: () => Navigator.pop(c, ctl.text.trim()),
            child: const Text('Add'),
          ),
        ],
      ),
    );
    if (path == null || path.isEmpty || !mounted) return;
    final s = SessionScope.of(context);
    await s.guard(
      context,
      Act.uploadPlugin,
      () => s.backend.uploadPlugin(path),
      done: 'Added to the catalog',
    );
    _load();
  }

  Future<void> _decide(CommandRow c, bool approve) async {
    final s = SessionScope.of(context);
    if (approve &&
        !await confirm(
          context,
          'Approve ${c.kind}?',
          '${c.args.join(', ')} on ${widget.siteNames[c.site] ?? c.site}, requested by ${c.requestedBy}. '
              'Your key signs the approval; the site checks it.',
        )) {
      return;
    }
    if (!mounted) return;
    await s.guard(
      context,
      approve ? Act.approveCommand : Act.rejectCommand,
      () => approve
          ? s.backend.approveCommand(c.id)
          : s.backend.rejectCommand(c.id),
      done: approve ? 'Approved: sent to the site' : 'Rejected',
    );
    _load();
  }

  Future<void> _freeze(bool on) async {
    final s = SessionScope.of(context);
    if (on &&
        !await confirm(
          context,
          'Freeze all commands?',
          'No command or approval goes to any site until an owner lifts the freeze.',
        )) {
      return;
    }
    if (!mounted) return;
    final f = await s.guard(
      context,
      Act.setFreeze,
      () => s.backend.setFreeze(on),
    );
    if (f != null) widget.onFreeze?.call(f);
    _load();
  }

  Future<void> _secondApprover(bool on) async {
    final s = SessionScope.of(context);
    if (!await confirm(
      context,
      on ? 'Require a second approver?' : 'Allow one approver?',
      on
          ? 'Plugin commands and release approvals will need two different owners or admins.'
          : 'One owner or admin will be enough for plugin commands and release approvals.',
    )) {
      return;
    }
    if (!mounted) return;
    await s.guard(
      context,
      Act.setOrgPolicy,
      () => s.backend.setOrgPolicy(on),
      done: 'Saved (audited)',
    );
    _load();
  }

  Future<void> _approveRelease(Release r) async {
    final s = SessionScope.of(context);
    if (!await confirm(
      context,
      'Approve ${r.name} ${r.version}?',
      'Your key signs this exact release (checksum ${r.sha256.substring(0, 12)}…) once; '
          'adminsvr may then roll it out to sites, which check your signature.',
    )) {
      return;
    }
    if (!mounted) return;
    await s.guard(
      context,
      Act.approveRelease,
      () => s.backend.approveRelease(r.sha256),
      done: 'Approved',
    );
    _load();
  }

  Future<void> _rollout(Release r) async {
    final s = SessionScope.of(context);
    var site = widget.siteNames.keys.isEmpty ? '' : widget.siteNames.keys.first;
    var push = false;
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => StatefulBuilder(
        builder: (c, set) => AlertDialog(
          title: Text('Roll out ${r.name} ${r.version}'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              DropdownButton<String>(
                key: const Key('rollout-site'),
                value: site.isEmpty ? null : site,
                items: [
                  for (final e in widget.siteNames.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) => set(() => site = v ?? ''),
              ),
              CheckboxListTile(
                key: const Key('rollout-now'),
                title: const Text('Now (ignore the site\'s update window)'),
                value: push,
                onChanged: (v) => set(() => push = v ?? false),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(c, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const Key('rollout-ok'),
              onPressed: () => Navigator.pop(c, true),
              child: const Text('Roll out'),
            ),
          ],
        ),
      ),
    );
    if (ok != true || site.isEmpty || !mounted) return;
    await s.guard(
      context,
      Act.sendCommand,
      () => s.backend.rolloutRelease(site, r.sha256, push),
      done: push
          ? 'Sent: installs now'
          : 'Sent: installs in the site\'s update window',
    );
    _load();
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final t = Theme.of(context).textTheme;
    final c = cat;
    if (c == null) return const Center(child: CircularProgressIndicator());
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          if (fz != null)
            SwitchListTile(
              key: const Key('freeze-switch'),
              secondary: Icon(
                Icons.ac_unit,
                color: fz!.frozen ? Colors.blue : null,
              ),
              title: const Text('Global freeze'),
              subtitle: Text(
                fz!.frozen
                    ? 'On since ${ago(fz!.sinceMs)} (${fz!.by}): nothing is sent to any site'
                    : s.isOwner
                    ? 'Off'
                    : 'Off (owners only)',
              ),
              value: fz!.frozen,
              onChanged: s.isOwner ? _freeze : null,
            ),
          if (org != null)
            SwitchListTile(
              key: const Key('second-approver-switch'),
              secondary: const Icon(Icons.people),
              title: const Text('Require a second approver'),
              subtitle: Text(
                org!.requireSecondApprover
                    ? 'On: two different owners or admins approve'
                    : s.isOwner
                    ? 'Off: one owner or admin approves'
                    : 'Off: one owner or admin approves (owners change this)',
              ),
              value: org!.requireSecondApprover,
              onChanged: s.isOwner ? _secondApprover : null,
            ),
          const SizedBox(height: 8),
          Text('Releases', style: t.titleMedium),
          for (final r in rels)
            ListTile(
              key: Key('release-${r.sha256}'),
              dense: true,
              leading: Icon(
                r.approved ? Icons.verified : Icons.pending,
                color: r.approved ? Colors.green : null,
              ),
              title: Text('${r.name} ${r.version}'),
              subtitle: Text(
                '${r.approvedBy.length} of ${r.needed} approvals'
                '${r.approvedBy.isEmpty ? '' : ' (${r.approvedBy.join(', ')})'}',
              ),
              trailing: !s.isAdmin
                  ? null
                  : r.approved
                  ? TextButton(
                      key: Key('rollout-${r.sha256}'),
                      onPressed: () => _rollout(r),
                      child: const Text('Roll out'),
                    )
                  : TextButton(
                      key: Key('approve-release-${r.sha256}'),
                      onPressed: r.approvedBy.contains(s.me?.id)
                          ? null
                          : () => _approveRelease(r),
                      child: const Text('Approve'),
                    ),
            ),
          if (rels.isEmpty)
            const Text('No server or agent releases in the catalog.'),
          const SizedBox(height: 8),
          Text('Waiting for approval', style: t.titleMedium),
          for (final p in pending)
            ListTile(
              key: Key('pending-${p.id}'),
              dense: true,
              title: Text('${p.kind} · ${widget.siteNames[p.site] ?? p.site}'),
              subtitle: Text(
                '${p.args.join(', ')} · by ${p.requestedBy} · ${ago(p.createdMs)}',
              ),
              trailing: s.isAdmin
                  ? Wrap(
                      children: [
                        IconButton(
                          key: Key('approve-${p.id}'),
                          tooltip: 'Approve',
                          icon: const Icon(Icons.check, color: Colors.green),
                          onPressed: p.requestedBy == s.me?.id
                              ? null
                              : () => _decide(p, true),
                        ),
                        IconButton(
                          key: Key('reject-${p.id}'),
                          tooltip: 'Reject',
                          icon: const Icon(Icons.close, color: Colors.red),
                          onPressed: () => _decide(p, false),
                        ),
                      ],
                    )
                  : null,
            ),
          if (pending.isEmpty) const Text('Nothing waiting.'),
          const SizedBox(height: 16),
          Row(
            children: [
              Expanded(child: Text('Catalog', style: t.titleMedium)),
              if (s.isAdmin)
                FilledButton.icon(
                  key: const Key('upload-plugin'),
                  icon: const Icon(Icons.upload),
                  label: const Text('Add package'),
                  onPressed: _upload,
                ),
            ],
          ),
          for (final e in c)
            ListTile(
              key: Key('catalog-${e.sha256}'),
              dense: true,
              leading: const Icon(Icons.inventory_2),
              title: Text('${e.name} v${e.version}'),
              subtitle: Text(
                '${(e.size / 1024).toStringAsFixed(0)} KB · ${e.sha256.substring(0, 12)}… · '
                'added by ${e.uploadedBy} ${ago(e.uploadedMs)}',
              ),
            ),
          if (c.isEmpty) const Text('No packages yet.'),
          const SizedBox(height: 8),
          Text(
            'Install a package on a server from its Plugins tab. With a second approver required, '
            'install, enable and disable need another owner or admin; the site validates the '
            'package and quarantines a bad one.',
            style: t.bodySmall,
          ),
        ],
      ),
    );
  }
}
