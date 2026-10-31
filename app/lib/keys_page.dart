// v0.12.0 Keys: this app's signing key (make or import), who the server says we are, and the
// registered keys (admins list; owners register bundles certified offline and revoke).
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';
import 'signing.dart';

/// The offline certify command an owner runs with the words for this app's public key.
String certifyCommand(String name, String role, String publicHex) =>
    "itcmdr-cli keys certify --kind user --name \"$name\" --role $role "
    "--public-key $publicHex --out app-key.pub.json";

/// Read a private key the user points at: 64 hex characters, or the 32-byte file
/// `itcmdr-cli keys new --out FILE` writes. Returns the seed as hex.
String parseKeyInput(String text, {List<int>? fileBytes}) {
  if (fileBytes != null) {
    if (fileBytes.length == 32) return hexOf(fileBytes);
    final t = String.fromCharCodes(fileBytes).trim();
    if (t.length == 64) return hexOf(bytesOfHex(t));
    throw const FormatException(
      'the key file must hold 32 bytes (or 64 hex characters)',
    );
  }
  final t = text.trim();
  if (t.length != 64) {
    throw const FormatException(
      'paste 64 hex characters, or a path to the key file',
    );
  }
  return hexOf(bytesOfHex(t));
}

class KeysPage extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  final Future<void> Function(ServerSettings) onSave;
  const KeysPage({
    super.key,
    required this.client,
    required this.settings,
    required this.onSave,
  });
  @override
  State<KeysPage> createState() => _KeysPageState();
}

class _KeysPageState extends State<KeysPage> {
  AppKey? _key;
  WhoAmIResponse? _me;
  List<KeyInfo> _keys = [];
  String? _error;
  String? _keysError;
  final _import = TextEditingController();
  final _bundle = TextEditingController();
  final _name = TextEditingController(text: 'Front desk');
  String _role = 'admin';

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final c in [_import, _bundle, _name]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _load() async {
    final k = await widget.settings.appKey();
    if (!mounted) return;
    setState(() => _key = k);
    WhoAmIResponse? me;
    String? err;
    try {
      me = await widget.client.whoAmI(
        WhoAmIRequest(),
        options: widget.settings.authOptions(),
      );
    } catch (e) {
      err = 'Server: ${e is Exception ? e.toString() : e}';
    }
    List<KeyInfo> keys = [];
    String? kerr;
    if (me != null && me.signed && (me.role == 'admin' || me.role == 'owner')) {
      try {
        keys = (await widget.client.listKeys(ListKeysRequest())).keys;
      } catch (e) {
        kerr = '$e';
      }
    }
    if (!mounted) return;
    setState(() {
      _key = k;
      _me = me;
      _error = err;
      _keys = keys;
      _keysError = kerr;
    });
  }

  Future<void> _useSeed(String seedHex) async {
    await widget.onSave(widget.settings.copyWith(keySeed: seedHex));
  }

  Future<void> _generate() async {
    if (_key != null) {
      final ok = await _confirm(
        'Replace this key?',
        'The current key stops working for this app. Revoke it on the server afterwards.',
      );
      if (!ok) return;
    }
    final k = await AppKey.generate();
    await _useSeed(hexOf(k.seed));
  }

  Future<void> _importKey() async {
    try {
      final t = _import.text.trim();
      final f = File(t);
      final seed = (t.length != 64 && f.existsSync())
          ? parseKeyInput('', fileBytes: f.readAsBytesSync())
          : parseKeyInput(t);
      _import.clear();
      await _useSeed(seed);
    } catch (e) {
      setState(() => _error = '$e');
    }
  }

  Future<bool> _confirm(String title, String body) async =>
      await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text(title),
          content: Text(body),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('Continue'),
            ),
          ],
        ),
      ) ??
      false;

  Future<void> _register() async {
    try {
      await widget.client.registerKey(
        RegisterKeyRequest(bundleJson: _bundle.text.trim()),
      );
      _bundle.clear();
      await _load();
    } catch (e) {
      setState(() => _keysError = '$e');
    }
  }

  Future<void> _revoke(KeyInfo k) async {
    final ok = await _confirm(
      'Revoke ${k.name}?',
      'Calls signed with ${k.fingerprint.substring(0, 16)}… are refused at once.',
    );
    if (!ok) return;
    try {
      await widget.client.revokeKey(
        RevokeKeyRequest(fingerprint: k.fingerprint),
      );
      await _load();
    } catch (e) {
      setState(() => _keysError = '$e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final k = _key;
    final me = _me;
    final t = Theme.of(context).textTheme;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text('This app\'s key', style: t.titleMedium),
        if (k == null)
          const Text(
            'No key yet: calls are unsigned and only the shared token works '
            '(servers in signed mode refuse them). Make a key, then have an owner certify it.',
          )
        else ...[
          SelectableText(
            'Fingerprint: ${k.fingerprint}',
            key: const Key('key-fp'),
          ),
          SelectableText('Public key: ${k.publicHex}'),
        ],
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            FilledButton.icon(
              key: const Key('key-generate'),
              icon: const Icon(Icons.key),
              label: Text(k == null ? 'Make a key' : 'Make a new key'),
              onPressed: _generate,
            ),
            SizedBox(
              width: 360,
              child: TextField(
                key: const Key('key-import'),
                controller: _import,
                decoration: const InputDecoration(
                  labelText: 'Or import: key file path or 64 hex characters',
                ),
                onSubmitted: (_) => _importKey(),
              ),
            ),
            OutlinedButton(onPressed: _importKey, child: const Text('Import')),
          ],
        ),
        if (k != null) ...[
          const SizedBox(height: 12),
          Text('Register this key', style: t.titleSmall),
          Row(
            children: [
              SizedBox(
                width: 200,
                child: TextField(
                  controller: _name,
                  decoration: const InputDecoration(labelText: 'Name'),
                  onChanged: (_) => setState(() {}),
                ),
              ),
              const SizedBox(width: 8),
              DropdownButton<String>(
                value: _role,
                items: const [
                  DropdownMenuItem(value: 'user', child: Text('user')),
                  DropdownMenuItem(value: 'admin', child: Text('admin')),
                  DropdownMenuItem(value: 'owner', child: Text('owner')),
                ],
                onChanged: (v) => setState(() => _role = v ?? 'admin'),
              ),
            ],
          ),
          const Text(
            'An owner runs this on the server PC (it asks for the 24 words, offline), '
            'then `itcmdr-cli keys register app-key.pub.json`, or pastes the bundle below:',
          ),
          Row(
            children: [
              Expanded(
                child: SelectableText(
                  certifyCommand(_name.text.trim(), _role, k.publicHex),
                  key: const Key('certify-cmd'),
                  style: const TextStyle(fontFamily: 'monospace'),
                ),
              ),
              IconButton(
                tooltip: 'Copy',
                icon: const Icon(Icons.copy),
                onPressed: () => Clipboard.setData(
                  ClipboardData(
                    text: certifyCommand(_name.text.trim(), _role, k.publicHex),
                  ),
                ),
              ),
            ],
          ),
        ],
        const Divider(height: 32),
        Text('Signed in as', style: t.titleMedium),
        if (_error != null)
          Text(_error!, style: const TextStyle(color: Colors.red)),
        if (me != null)
          Text(
            key: const Key('whoami'),
            me.signed
                ? '${me.name} (${me.role}, ${me.kind}) — signed; server auth mode: ${me.authMode}'
                : 'Not signed (shared token: ${me.role}); server auth mode: ${me.authMode}',
          ),
        if (me != null &&
            me.signed &&
            (me.role == 'admin' || me.role == 'owner')) ...[
          const Divider(height: 32),
          Row(
            children: [
              Text('Registered keys', style: t.titleMedium),
              IconButton(onPressed: _load, icon: const Icon(Icons.refresh)),
            ],
          ),
          if (_keysError != null)
            Text(_keysError!, style: const TextStyle(color: Colors.red)),
          for (final x in _keys)
            ListTile(
              dense: true,
              title: Text('${x.name}  (${x.role}, ${x.kind})'),
              subtitle: Text(
                '${x.fingerprint.substring(0, 16)}…  added ${x.createdAt}'
                '${x.revokedAt.isEmpty ? '' : '  REVOKED ${x.revokedAt}'}',
              ),
              trailing:
                  me.role == 'owner' &&
                      x.revokedAt.isEmpty &&
                      x.fingerprint != me.fingerprint
                  ? IconButton(
                      tooltip: 'Revoke',
                      icon: const Icon(Icons.block),
                      onPressed: () => _revoke(x),
                    )
                  : null,
            ),
          if (me.role == 'owner') ...[
            const SizedBox(height: 8),
            TextField(
              key: const Key('bundle'),
              controller: _bundle,
              maxLines: 4,
              decoration: const InputDecoration(
                labelText:
                    'Register a key bundle (JSON from itcmdr-cli keys certify)',
              ),
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton(
                onPressed: _register,
                child: const Text('Register'),
              ),
            ),
          ],
        ],
      ],
    );
  }
}
