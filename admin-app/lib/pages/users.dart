// Users per role (owners and admins): create with a one-time code, role, disable, SRE server
// scopes, device approval and revocation, passkey reset. adminsvr enforces the rules (admins
// cannot change owners; the last owner stays); the app hides what you may not do.
import 'package:flutter/material.dart';

import '../backend.dart';
import '../session.dart';
import 'servers.dart' show showSecret, confirm, kv;

class UsersPage extends StatefulWidget {
  const UsersPage({super.key});
  @override
  State<UsersPage> createState() => _UsersPageState();
}

class _UsersPageState extends State<UsersPage> {
  List<User>? users;
  String? error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    try {
      final l = await SessionScope.of(context).backend.users();
      if (mounted) setState(() => users = l);
    } catch (e) {
      if (mounted) setState(() => error = errorText(e));
    }
  }

  Future<void> _create(Session s) async {
    final email = TextEditingController(), name = TextEditingController();
    var role = 'sre';
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => StatefulBuilder(
        builder: (c, set) => AlertDialog(
          title: const Text('New user'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                key: const Key('u-email'),
                controller: email,
                decoration: const InputDecoration(labelText: 'Email'),
              ),
              TextField(
                key: const Key('u-name'),
                controller: name,
                decoration: const InputDecoration(labelText: 'Name'),
              ),
              DropdownButton<String>(
                key: const Key('u-role'),
                value: role,
                items: [
                  for (final r in ['sre', 'admin', if (s.isOwner) 'owner'])
                    DropdownMenuItem(value: r, child: Text(roleLabel(r))),
                ],
                onChanged: (v) => set(() => role = v ?? role),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(c, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const Key('u-create'),
              onPressed: () => Navigator.pop(c, true),
              child: const Text('Create'),
            ),
          ],
        ),
      ),
    );
    if (ok != true || !mounted) return;
    final code = await s.guard(
      context,
      Act.createUser,
      () => s.backend.createUser(email.text.trim(), name.text.trim(), role),
    );
    if (code == null || !mounted) return;
    await showSecret(
      context,
      'Enrollment code',
      'Give this to ${name.text.trim()} over a channel you trust. Single use, 24 hours.',
      code,
    );
    _load();
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final l = users;
    return Scaffold(
      floatingActionButton: s.isAdmin
          ? FloatingActionButton.extended(
              key: const Key('add-user'),
              heroTag: null,
              onPressed: () => _create(s),
              icon: const Icon(Icons.person_add),
              label: const Text('User'),
            )
          : null,
      body: error != null
          ? Center(child: Text(error!))
          : l == null
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _load,
              child: ListView(
                children: [
                  for (final u in l)
                    ListTile(
                      key: Key('user-${u.email}'),
                      leading: CircleAvatar(
                        child: Text(roleLabel(u.role).substring(0, 1)),
                      ),
                      title: Text('${u.name} <${u.email}>'),
                      subtitle: Text(
                        [
                          roleLabel(u.role),
                          u.status,
                          '${u.devices.where((d) => d.status == 'active').length} device(s)',
                          if (u.devices.any((d) => d.status == 'pending'))
                            'device waiting for approval',
                          if (u.passkeys > 0) '${u.passkeys} passkey(s)',
                        ].join(' · '),
                      ),
                      onTap: () async {
                        await Navigator.push(
                          context,
                          MaterialPageRoute(
                            builder: (_) => SessionScope(
                              session: s,
                              child: UserPage(user: u),
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

class UserPage extends StatefulWidget {
  const UserPage({super.key, required this.user});
  final User user;
  @override
  State<UserPage> createState() => _UserPageState();
}

class _UserPageState extends State<UserPage> {
  late User u = widget.user;
  List<Passkey> keys = [];
  List<Site> sites = [];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final b = SessionScope.of(context).backend;
    try {
      final k = await b.passkeys(u.id);
      final st = await b.sites();
      if (mounted) {
        setState(() {
          keys = k;
          sites = st;
        });
      }
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final me = s.me!;
    final self = me.id == u.id;
    // Owners edit everyone; admins edit admins and SREs.
    final canEdit = s.isOwner || (s.isAdmin && u.role != 'owner');
    return Scaffold(
      appBar: AppBar(title: Text(u.name)),
      body: ListView(
        key: const Key('user-page'),
        padding: const EdgeInsets.all(16),
        children: [
          kv('Email', u.email),
          kv('Status', u.status),
          kv('Key backup', u.keyBackup ? 'stored (encrypted)' : 'none'),
          Row(
            children: [
              const SizedBox(
                width: 180,
                child: Text(
                  'Role',
                  style: TextStyle(fontWeight: FontWeight.w600),
                ),
              ),
              DropdownButton<String>(
                key: const Key('role'),
                value: u.role,
                items: [
                  for (final r in ['sre', 'admin', 'owner'])
                    DropdownMenuItem(
                      value: r,
                      enabled: s.isOwner || r != 'owner',
                      child: Text(roleLabel(r)),
                    ),
                ],
                onChanged: !canEdit || self
                    ? null
                    : (r) async {
                        if (r == null || r == u.role) return;
                        final n = await s.guard(
                          context,
                          Act.setRole,
                          () => s.backend.setRole(u.id, r),
                          done: 'Role changed',
                        );
                        if (n != null) setState(() => u = n);
                      },
              ),
            ],
          ),
          if (canEdit && !self)
            SwitchListTile(
              key: const Key('disabled'),
              title: const Text('Disabled'),
              value: u.status == 'disabled',
              onChanged: (v) async {
                final n = await s.guard(
                  context,
                  Act.setUserStatus,
                  () => s.backend.setDisabled(u.id, v),
                );
                if (n != null) setState(() => u = n);
              },
            ),
          if (u.role == 'sre') ...[
            const Divider(),
            Text(
              'Servers this SRE may see',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            for (final x in sites)
              CheckboxListTile(
                key: Key('scope-${x.id}'),
                title: Text(x.name),
                value: u.sites.contains(x.id),
                onChanged: !canEdit
                    ? null
                    : (v) async {
                        final next = [
                          ...u.sites.where((i) => i != x.id),
                          if (v == true) x.id,
                        ];
                        final n = await s.guard(
                          context,
                          Act.setScopes,
                          () => s.backend.setScopes(u.id, next),
                        );
                        if (n != null) setState(() => u = n);
                      },
              ),
          ],
          const Divider(),
          Text('Devices', style: Theme.of(context).textTheme.titleMedium),
          for (final d in u.devices)
            ListTile(
              key: Key('device-${d.fingerprint}'),
              dense: true,
              title: Text(
                '${d.platform} · ${d.fingerprint.substring(0, d.fingerprint.length.clamp(0, 16))}…',
              ),
              subtitle: Text('${d.status} · last used ${ago(d.lastUsedMs)}'),
              trailing: !canEdit
                  ? null
                  : Wrap(
                      children: [
                        if (d.status == 'pending')
                          TextButton(
                            key: Key('approve-${d.fingerprint}'),
                            onPressed: () async {
                              final n = await s.guard(
                                context,
                                Act.approveDevice,
                                () => s.backend.approveDevice(
                                  u.id,
                                  d.fingerprint,
                                ),
                                done: 'Device approved',
                              );
                              if (n != null) setState(() => u = n);
                            },
                            child: const Text('Approve'),
                          ),
                        if (d.status != 'revoked')
                          TextButton(
                            onPressed: () async {
                              if (!await confirm(
                                context,
                                'Revoke this device?',
                                'It can no longer sign requests.',
                              )) {
                                return;
                              }
                              if (!context.mounted) return;
                              await s.guard(
                                context,
                                Act.revokeDevice,
                                () =>
                                    s.backend.revokeDevice(u.id, d.fingerprint),
                                done: 'Device revoked',
                              );
                              final l = await s.backend.users();
                              setState(
                                () => u = l.firstWhere(
                                  (x) => x.id == u.id,
                                  orElse: () => u,
                                ),
                              );
                            },
                            child: const Text('Revoke'),
                          ),
                      ],
                    ),
            ),
          if (canEdit)
            TextButton.icon(
              key: const Key('new-code'),
              icon: const Icon(Icons.key),
              label: const Text('New enrollment code (new device)'),
              onPressed: () async {
                final c = await s.guard(
                  context,
                  Act.newEnrollCode,
                  () => s.backend.newEnrollCode(u.id),
                );
                if (c != null && context.mounted) {
                  await showSecret(
                    context,
                    'Enrollment code',
                    'Single use, 24 hours.',
                    c,
                  );
                }
              },
            ),
          const Divider(),
          Text('Passkeys', style: Theme.of(context).textTheme.titleMedium),
          for (final k in keys)
            ListTile(
              dense: true,
              title: Text(k.label),
              subtitle: Text(
                'used ${ago(k.lastUsedMs)} · count ${k.signCount}',
              ),
              trailing: canEdit && !self
                  ? TextButton(
                      key: Key('reset-${k.credentialId}'),
                      onPressed: () async {
                        if (!await confirm(
                          context,
                          'Reset this passkey?',
                          'For a lost key. ${u.name} can register a new one.',
                        )) {
                          return;
                        }
                        if (!context.mounted) return;
                        await s.guard(
                          context,
                          Act.removePasskey,
                          () => s.backend.removePasskey(u.id, k.credentialId),
                          done: 'Passkey reset',
                        );
                        _load();
                      },
                      child: const Text('Reset'),
                    )
                  : null,
            ),
          if (keys.isEmpty) const ListTile(dense: true, title: Text('None')),
        ],
      ),
    );
  }
}
