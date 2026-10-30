// v0.9.1 basic Settings screen for the server and agent settings (decision 15).
// Admin only on the server side; the full Settings section comes in v0.12.0.
import 'package:flutter/material.dart';

import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';

/// Client-side check mirroring the server's validation (the server stays authoritative).
/// Returns an error message, or null when [text] is acceptable for [s].
String? validateSettingValue(SettingItem s, String text) {
  final t = text.trim();
  if (s.readOnly) return '${s.key} is read-only (server.json)';
  switch (s.type) {
    case 'int':
      final v = int.tryParse(t);
      if (v == null) return 'Enter a whole number';
      if (v < s.min.toInt() || v > s.max.toInt()) {
        return 'Must be between ${s.min} and ${s.max}';
      }
      return null;
    case 'bool':
      return (t == 'true' || t == 'false') ? null : 'Enter true or false';
    case 'time':
      return RegExp(r'^([01]\d|2[0-3]):[0-5]\d$').hasMatch(t)
          ? null
          : 'Use HH:mm (24h)';
    case 'path':
      return null; // existence/writability is checked on the server
    default:
      return null;
  }
}

class ServerSettingsSection extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  const ServerSettingsSection({
    super.key,
    required this.client,
    required this.settings,
  });
  @override
  State<ServerSettingsSection> createState() => _ServerSettingsSectionState();
}

class _ServerSettingsSectionState extends State<ServerSettingsSection> {
  List<SettingItem> _items = [];
  String _scope = 'server';
  final _machine = TextEditingController();
  String? _error;
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _machine.dispose();
    super.dispose();
  }

  String get _m => _scope == 'agent' ? _machine.text.trim() : '';

  Future<void> _load() async {
    try {
      final r = await widget.client.listSettings(
        ListSettingsRequest(machine: _m),
        options: widget.settings.authOptions(),
      );
      if (!mounted) return;
      setState(() {
        _items = r.settings;
        _error = null;
        _loaded = true;
      });
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = 'Settings need an admin connection: $e';
          _loaded = true;
        });
      }
    }
  }

  Future<void> _update(
    SettingItem s, {
    String? value,
    bool unset = false,
  }) async {
    try {
      await widget.client.updateSetting(
        UpdateSettingRequest(
          key: s.key,
          value: value ?? '',
          machine: _m,
          unset: unset,
        ),
        options: widget.settings.authOptions(),
      );
      await _load();
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  Future<void> _edit(SettingItem s) async {
    final c = TextEditingController(text: s.value);
    String? err;
    final v = await showDialog<String>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, set) => AlertDialog(
          title: Text(s.key),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(s.description),
              const SizedBox(height: 8),
              TextField(
                key: const Key('setting-value'),
                controller: c,
                decoration: InputDecoration(
                  labelText: 'Value (default ${s.defaultValue})',
                  errorText: err,
                ),
              ),
              if (s.applies.isNotEmpty) Text('Applies: ${s.applies}'),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const Key('setting-save'),
              onPressed: () {
                final e = validateSettingValue(s, c.text);
                if (e != null) {
                  set(() => err = e);
                  return;
                }
                Navigator.pop(ctx, c.text.trim());
              },
              child: const Text('Save'),
            ),
          ],
        ),
      ),
    );
    c.dispose();
    if (v != null) await _update(s, value: v);
  }

  @override
  Widget build(BuildContext context) {
    final shown = _items.where((s) => s.scope == _scope).toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Server & agent settings',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            SegmentedButton<String>(
              segments: const [
                ButtonSegment(value: 'server', label: Text('Server')),
                ButtonSegment(value: 'agent', label: Text('Agents')),
              ],
              selected: {_scope},
              onSelectionChanged: (v) {
                setState(() => _scope = v.first);
                _load();
              },
            ),
            const SizedBox(width: 12),
            if (_scope == 'agent')
              SizedBox(
                width: 180,
                child: TextField(
                  key: const Key('settings-machine'),
                  controller: _machine,
                  decoration: const InputDecoration(
                    labelText: 'Machine (blank = all PCs)',
                  ),
                  onSubmitted: (_) => _load(),
                ),
              ),
            IconButton(
              tooltip: 'Refresh',
              icon: const Icon(Icons.refresh),
              onPressed: _load,
            ),
          ],
        ),
        if (_error != null)
          Text(_error!, style: const TextStyle(color: Colors.red)),
        if (!_loaded) const LinearProgressIndicator(),
        for (final s in shown)
          ListTile(
            dense: true,
            title: Text(s.key),
            subtitle: Text('${s.value}   [${s.source}]  ${s.description}'),
            trailing: s.readOnly
                ? const Tooltip(
                    message: 'server.json only',
                    child: Icon(Icons.lock),
                  )
                : Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (s.source.startsWith('db'))
                        IconButton(
                          tooltip: 'Reset to default',
                          icon: const Icon(Icons.restart_alt),
                          onPressed: () => _update(s, unset: true),
                        ),
                      IconButton(
                        tooltip: 'Edit',
                        icon: const Icon(Icons.edit),
                        onPressed: () => _edit(s),
                      ),
                    ],
                  ),
          ),
      ],
    );
  }
}
