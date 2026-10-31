// v0.12.0 Backup & Restore (decision 13). Admins and owners: start a backup, list and check saved
// backups. Owners: restore, always with a dry run first. The 24 words are sealed on this PC for a
// one-time server key (signing.dart sealWords) and never sent or stored in plaintext.
import 'dart:async';

import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';

import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';
import 'signing.dart';

/// 24 lowercase words separated by spaces (the server checks the checksum).
String? checkWords(String w) {
  final parts = w
      .trim()
      .toLowerCase()
      .split(RegExp(r'\s+'))
      .where((x) => x.isNotEmpty)
      .toList();
  if (parts.length != 24) return 'Enter all 24 words (${parts.length} so far)';
  if (parts.any((p) => !RegExp(r'^[a-z]+$').hasMatch(p))) {
    return 'Words are letters only';
  }
  return null;
}

String normalizeWords(String w) => w
    .trim()
    .toLowerCase()
    .split(RegExp(r'\s+'))
    .where((x) => x.isNotEmpty)
    .join(' ');

const restoreTerminal = {'done', 'rolled-back', 'failed'};

String fmtSize(Int64 b) {
  final v = b.toInt();
  if (v < 1024) return '$v B';
  if (v < 1024 * 1024) return '${(v / 1024).toStringAsFixed(1)} KB';
  return '${(v / 1024 / 1024).toStringAsFixed(1)} MB';
}

/// Seal [words] for a fresh one-time restore key from the server.
Future<SealedWords> sealFor(ItcmdrClient c, String backup, String words) async {
  final k = await c.getRestoreKey(GetRestoreKeyRequest());
  final s = await sealWords(
    serverPublicHex: k.x25519Pub,
    keyId: k.keyId,
    backup: backup,
    words: normalizeWords(words),
  );
  return SealedWords(keyId: s.keyId, eph: s.eph, nonce: s.nonce, ct: s.ct);
}

class BackupPage extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  const BackupPage({super.key, required this.client, required this.settings});
  @override
  State<BackupPage> createState() => _BackupPageState();
}

class _BackupPageState extends State<BackupPage> {
  WhoAmIResponse? _me;
  List<BackupInfo> _backups = [];
  RestoreStatus? _status;
  String? _error;
  String? _info;
  bool _busy = false;
  Timer? _poll;

  CallOptions get _opts => widget.settings.authOptions();

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final me = await widget.client.whoAmI(WhoAmIRequest(), options: _opts);
      List<BackupInfo> b = [];
      RestoreStatus? st;
      if (me.role == 'admin' || me.role == 'owner') {
        b = (await widget.client.listBackups(
          ListBackupsRequest(),
          options: _opts,
        )).backups;
        if (me.signed) {
          final r = await widget.client.getRestoreStatus(
            GetRestoreStatusRequest(),
            options: _opts,
          );
          st = r.hasStatus() ? r.status : null;
        }
      }
      if (!mounted) return;
      setState(() {
        _me = me;
        _backups = b..sort((a, c) => c.createdTs.compareTo(a.createdTs));
        _status = st;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  Future<void> _create() async {
    setState(() => _busy = true);
    try {
      final r = await widget.client.createBackup(
        CreateBackupRequest(),
        options: _opts,
      );
      _info = 'Backup saved: ${r.backup.name}';
      await _load();
    } catch (e) {
      setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _verify(BackupInfo b) async {
    try {
      final r = await widget.client.verifyBackup(
        VerifyBackupRequest(name: b.name),
        options: _opts,
      );
      setState(
        () => _info = '${b.name}: ${r.ok ? 'OK' : 'FAILED'} ${r.message}',
      );
    } catch (e) {
      setState(() => _error = '$e');
    }
  }

  void _watchRestore() {
    _poll?.cancel();
    _poll = Timer.periodic(const Duration(seconds: 2), (_) async {
      try {
        final r = await widget.client.getRestoreStatus(
          GetRestoreStatusRequest(),
          options: _opts,
        );
        if (!mounted) return;
        setState(() => _status = r.hasStatus() ? r.status : null);
        if (r.hasStatus() && restoreTerminal.contains(r.status.state)) {
          _poll?.cancel();
        }
      } catch (_) {
        // The server is restarting during the restore; keep asking.
        if (mounted) {
          setState(() => _info = 'Waiting for the server to come back…');
        }
      }
    });
  }

  Future<void> _restore(BackupInfo b) async {
    final started = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (_) => RestoreDialog(client: widget.client, backup: b),
    );
    if (started == true) {
      setState(() => _info = 'Restore of ${b.name} started');
      _watchRestore();
    }
  }

  @override
  Widget build(BuildContext context) {
    final me = _me;
    final t = Theme.of(context).textTheme;
    final canAdmin = me != null && (me.role == 'admin' || me.role == 'owner');
    final canRestore = me != null && me.signed && me.role == 'owner';
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            Text('Backup & Restore', style: t.titleMedium),
            const Spacer(),
            IconButton(onPressed: _load, icon: const Icon(Icons.refresh)),
            if (canAdmin)
              FilledButton.icon(
                key: const Key('backup-now'),
                onPressed: _busy ? null : _create,
                icon: const Icon(Icons.save),
                label: const Text('Back up now'),
              ),
          ],
        ),
        if (_error != null)
          Text(_error!, style: const TextStyle(color: Colors.red)),
        if (_info != null) Text(_info!),
        if (_busy) const LinearProgressIndicator(),
        if (me != null && !canAdmin)
          const Text(
            'Backups are for admins and owners. Ask an owner to register your key with that role.',
          ),
        if (canAdmin && !canRestore)
          const Text(
            'Restoring needs an owner key (signed). Admins can back up and check backups.',
          ),
        if (_status != null) ...[
          const SizedBox(height: 8),
          Card(
            child: ListTile(
              leading: Icon(
                _status!.state == 'done'
                    ? Icons.check_circle
                    : restoreTerminal.contains(_status!.state)
                    ? Icons.error
                    : Icons.sync,
              ),
              title: Text(
                'Last restore: ${_status!.state} (${_status!.backup})',
              ),
              subtitle: Text(
                '${_status!.message}\nby ${_status!.actor}'
                '${_status!.safetyCopy.isEmpty ? '' : '\nsafety copy: ${_status!.safetyCopy}'}',
              ),
            ),
          ),
        ],
        for (final b in _backups)
          ListTile(
            dense: true,
            title: Text(b.name),
            subtitle: Text(
              '${b.kind} · ${fmtSize(b.size)} · v${b.itcmdrVersion} · schema ${b.schemaVersion}',
            ),
            trailing: Wrap(
              children: [
                IconButton(
                  tooltip: 'Check',
                  icon: const Icon(Icons.verified),
                  onPressed: () => _verify(b),
                ),
                if (canRestore)
                  IconButton(
                    tooltip: 'Restore…',
                    icon: const Icon(Icons.settings_backup_restore),
                    onPressed: () => _restore(b),
                  ),
              ],
            ),
          ),
      ],
    );
  }
}

class RestoreDialog extends StatefulWidget {
  final ItcmdrClient client;
  final BackupInfo backup;
  const RestoreDialog({super.key, required this.client, required this.backup});
  @override
  State<RestoreDialog> createState() => RestoreDialogState();
}

class RestoreDialogState extends State<RestoreDialog> {
  final _words = TextEditingController();
  String? _dryRunId;
  String? _msg;
  String? _err;
  bool _busy = false;
  bool _understood = false;
  bool _show = false;

  @override
  void dispose() {
    _words.clear();
    _words.dispose();
    super.dispose();
  }

  Future<void> _dryRun() async {
    final e = checkWords(_words.text);
    if (e != null) {
      setState(() => _err = e);
      return;
    }
    setState(() {
      _busy = true;
      _err = null;
    });
    try {
      final sw = await sealFor(widget.client, widget.backup.name, _words.text);
      final r = await widget.client.stageRestore(
        StageRestoreRequest(
          backup: widget.backup.name,
          sealedWords: sw,
          dryRun: true,
        ),
      );
      setState(() {
        _dryRunId = r.dryRunId;
        _msg = r.message;
      });
    } catch (e) {
      setState(() => _err = e is GrpcError ? (e.message ?? '$e') : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _go() async {
    setState(() => _busy = true);
    try {
      final sw = await sealFor(widget.client, widget.backup.name, _words.text);
      await widget.client.stageRestore(
        StageRestoreRequest(
          backup: widget.backup.name,
          sealedWords: sw,
          dryRunId: _dryRunId,
        ),
      );
      _words.clear();
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      setState(() {
        _err = e is GrpcError ? (e.message ?? '$e') : '$e';
        _busy = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final passed = _dryRunId != null && _dryRunId!.isNotEmpty;
    return AlertDialog(
      title: Text('Restore ${widget.backup.name}'),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'The server stops, restores this backup, starts again and checks itself. '
              'A safety copy is made first and put back automatically if the check fails.',
            ),
            const SizedBox(height: 8),
            TextField(
              key: const Key('restore-words'),
              controller: _words,
              obscureText: !_show,
              enableSuggestions: false,
              autocorrect: false,
              maxLines: _show ? 3 : 1,
              enabled: !passed,
              decoration: InputDecoration(
                labelText: 'The 24 recovery words',
                helperText:
                    'Sealed on this PC for the server only; never stored.',
                suffixIcon: IconButton(
                  icon: Icon(_show ? Icons.visibility_off : Icons.visibility),
                  onPressed: () => setState(() => _show = !_show),
                ),
              ),
            ),
            if (_err != null)
              Text(
                _err!,
                key: const Key('restore-error'),
                style: const TextStyle(color: Colors.red),
              ),
            if (_msg != null)
              Text(
                _msg!,
                key: const Key('restore-dry-ok'),
                style: const TextStyle(color: Colors.green),
              ),
            if (passed)
              CheckboxListTile(
                key: const Key('restore-understood'),
                value: _understood,
                onChanged: (v) => setState(() => _understood = v ?? false),
                title: const Text(
                  'I understand the server restarts and changes since this backup are lost',
                ),
              ),
            if (_busy) const LinearProgressIndicator(),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.pop(context, false),
          child: const Text('Cancel'),
        ),
        if (!passed)
          FilledButton(
            key: const Key('restore-dry'),
            onPressed: _busy ? null : _dryRun,
            child: const Text('Dry run'),
          )
        else
          FilledButton(
            key: const Key('restore-go'),
            style: FilledButton.styleFrom(backgroundColor: Colors.red),
            onPressed: _busy || !_understood ? null : _go,
            child: const Text('Restore now'),
          ),
      ],
    );
  }
}
