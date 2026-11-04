// You, your passkeys, the step-up policy (owners), push notifications, the audit chain, and
// forgetting this device.
import 'package:flutter/material.dart';

import '../backend.dart';
import '../help_page.dart';
import '../push.dart';
import '../session.dart';
import 'servers.dart' show confirm, kv;

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key, required this.push});
  final PushService push;
  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  List<Passkey> keys = [];
  String? auditState;
  List<AuditRow> audit = [];
  TrustedCert? cert;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final s = SessionScope.of(context);
    try {
      final k = await s.backend.passkeys('');
      final a = s.isAdmin ? await s.backend.audit(30) : <AuditRow>[];
      final c = await s.backend.trustedCert(s.dir);
      if (mounted) {
        setState(() {
          keys = k;
          audit = a;
          cert = c;
        });
      }
    } catch (_) {}
  }

  Future<void> _editPolicy(Session s) async {
    final p = s.policy;
    if (p == null) return;
    final acts = TextEditingController(text: p.stepupActions.join(', '));
    var ownersNeedKey = p.passkeyRequiredRoles.contains('owner');
    var adminsNeedKey = p.passkeyRequiredRoles.contains('admin');
    var uv = p.requireUserVerification;
    final ok = await showDialog<bool>(
      context: context,
      builder: (c) => StatefulBuilder(
        builder: (c, set) => AlertDialog(
          title: const Text('Passkey policy'),
          content: SizedBox(
            width: 480,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  key: const Key('stepup-actions'),
                  controller: acts,
                  maxLines: 4,
                  decoration: const InputDecoration(
                    labelText: 'Actions that need a touch (API method names)',
                  ),
                ),
                CheckboxListTile(
                  title: const Text('Owners must have a passkey'),
                  value: ownersNeedKey,
                  onChanged: (v) => set(() => ownersNeedKey = v ?? false),
                ),
                CheckboxListTile(
                  title: const Text('Admins must have a passkey'),
                  value: adminsNeedKey,
                  onChanged: (v) => set(() => adminsNeedKey = v ?? false),
                ),
                CheckboxListTile(
                  title: const Text(
                    'Require user verification (PIN or biometrics)',
                  ),
                  value: uv,
                  onChanged: (v) => set(() => uv = v ?? false),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(c, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(c, true),
              child: const Text('Save'),
            ),
          ],
        ),
      ),
    );
    if (ok != true || !mounted) return;
    final np = Policy(
      stepupActions: acts.text
          .split(RegExp(r'[,\s]+'))
          .where((x) => x.isNotEmpty)
          .toList(),
      passkeyRequiredRoles: [
        if (ownersNeedKey) 'owner',
        if (adminsNeedKey) 'admin',
      ],
      requireUserVerification: uv,
    );
    final r = await s.guard(
      context,
      Act.setAuthPolicy,
      () => s.backend.setPolicy(np),
      done: 'Policy saved',
    );
    if (r != null) {
      s.policy = r;
      setState(() {});
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final me = s.me!;
    final p = s.policy;
    final t = Theme.of(context).textTheme;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        ListTile(
          key: const Key('open-help'),
          leading: const Icon(Icons.help_outline),
          title: const Text('Help'),
          subtitle: const Text('Guides for adminsvr, the app and the CLI'),
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => Scaffold(
                appBar: AppBar(title: const Text('Help')),
                body: const HelpPage(topic: 'admin-app'),
              ),
            ),
          ),
        ),
        Text('You', style: t.titleMedium),
        kv('Name', me.name),
        kv('Email', me.email),
        kv('Role', roleLabel(me.role)),
        kv('adminsvr', me.server),
        kv('Key backup', me.keyBackup ? 'stored (encrypted)' : 'none'),
        const Divider(),
        Text('Certificate', style: t.titleMedium),
        if (cert != null)
          ListTile(
            key: const Key('trusted-cert'),
            dense: true,
            leading: const Icon(Icons.verified_user),
            title: Text(
              cert!.fingerprint.isEmpty
                  ? 'Checked against this system\'s trust store'
                  : cert!.fromEnrollment
                  ? 'Pinned from the enrollment code'
                  : 'Trusted on this device',
            ),
            subtitle: Text(
              cert!.fingerprint.isEmpty
                  ? cert!.server
                  : '${cert!.server}\nSHA-256 ${cert!.fingerprint}',
            ),
            trailing: cert!.fingerprint.isEmpty || cert!.fromEnrollment
                ? null
                : IconButton(
                    key: const Key('trusted-cert-remove'),
                    tooltip: 'Remove (ask again next time)',
                    icon: const Icon(Icons.delete),
                    onPressed: () async {
                      await s.backend.forgetCert(s.dir);
                      await _load();
                    },
                  ),
          ),
        const Divider(),
        Text('Passkeys / security keys', style: t.titleMedium),
        Text(
          me.authenticator
              ? 'Your 24-word key signs everything; a passkey adds a touch for high-risk actions.'
              : 'No security key driver on this device. Desktop: install libfido2 (fido2-token, '
                    'fido2-cred, fido2-assert). Phone passkeys come later.',
          key: const Key('authenticator-note'),
        ),
        for (final k in keys)
          ListTile(
            dense: true,
            leading: const Icon(Icons.key),
            title: Text(k.label),
            subtitle: Text('used ${ago(k.lastUsedMs)}'),
            trailing: IconButton(
              icon: const Icon(Icons.delete),
              onPressed: () async {
                if (!await confirm(
                  context,
                  'Remove ${k.label}?',
                  'You need another passkey or your words to recover.',
                )) {
                  return;
                }
                if (!context.mounted) return;
                await s.guard(
                  context,
                  Act.removePasskey,
                  () => s.backend.removePasskey('', k.credentialId),
                  done: 'Removed',
                );
                await s.refreshMe();
                _load();
              },
            ),
          ),
        if (me.authenticator)
          TextButton.icon(
            key: const Key('add-passkey'),
            icon: const Icon(Icons.add),
            label: const Text('Add the plugged-in security key'),
            onPressed: () async {
              final r = await s.guard(
                context,
                Act.addPasskey,
                () => s.backend.addPasskey('security key'),
                done: 'Passkey added',
              );
              if (r != null) {
                await s.refreshMe();
                _load();
              }
            },
          ),
        if (p != null) ...[
          const Divider(),
          Text('Step-up policy', style: t.titleMedium),
          Text(
            'Actions needing a touch: ${p.stepupActions.join(', ')}',
            key: const Key('policy-actions'),
          ),
          Text(
            'Passkey required for: ${p.passkeyRequiredRoles.isEmpty ? 'nobody' : p.passkeyRequiredRoles.join(', ')}',
          ),
          if (s.isOwner)
            TextButton(
              key: const Key('edit-policy'),
              onPressed: () => _editPolicy(s),
              child: const Text('Edit (owners)'),
            ),
        ],
        const Divider(),
        Text('Notifications', style: t.titleMedium),
        Text(widget.push.description, key: const Key('push-note')),
        Wrap(
          spacing: 8,
          children: [
            if (widget.push.available)
              TextButton(
                key: const Key('push-enable'),
                onPressed: () async {
                  await s.guard(context, 'RegisterPush', () async {
                    final tok = await widget.push.token();
                    await s.backend.registerPush(
                      widget.push.provider,
                      tok,
                      widget.push.platform,
                    );
                  }, done: 'Push notifications on for this device');
                },
                child: const Text('Turn on for this device'),
              ),
            TextButton(
              key: const Key('push-test'),
              onPressed: () async {
                final r = await s.guard(
                  context,
                  'TestPush',
                  () => s.backend.testPush(),
                );
                if (r != null && context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text(
                        'Test push: ${r.sent} sent, ${r.failed} failed to ${r.devices} device(s)'
                        '${r.providers.isEmpty ? ' (push is not configured on adminsvr)' : ''}',
                      ),
                    ),
                  );
                }
              },
              child: const Text('Send a test push'),
            ),
          ],
        ),
        if (s.isAdmin) ...[
          const Divider(),
          Text('Audit', style: t.titleMedium),
          Row(
            children: [
              TextButton(
                key: const Key('verify-audit'),
                onPressed: () async {
                  final r = await s.guard(
                    context,
                    'VerifyAudit',
                    () => s.backend.verifyAudit(),
                  );
                  setState(() => auditState = r);
                },
                child: const Text('Verify the hash chain'),
              ),
              if (auditState != null)
                Text(auditState!, key: const Key('audit-state')),
            ],
          ),
          for (final a in audit)
            ListTile(
              dense: true,
              title: Text(a.action),
              subtitle: Text('${a.actor} · ${a.target} · ${ago(a.tsMs)}'),
            ),
        ],
        const Divider(),
        TextButton.icon(
          key: const Key('forget'),
          icon: const Icon(Icons.logout),
          label: const Text('Forget this device'),
          onPressed: () async {
            if (!await confirm(
              context,
              'Forget this device?',
              'The device key is deleted here. Enroll again with a new code (or your 24 words).',
            )) {
              return;
            }
            await s.signOut();
          },
        ),
      ],
    );
  }
}
