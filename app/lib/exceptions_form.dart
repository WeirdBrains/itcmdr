import 'package:flutter/material.dart';

import 'proto/itcmdr.pb.dart';

const exceptionActions = ['ignore', 'downgrade:warn', 'downgrade:info'];

/// Build a rule from form fields. Machines are comma/space separated.
ExceptionRule buildRule({
  String id = '',
  required String machines,
  String role = '',
  String checkId = '',
  String checkPattern = '',
  required String action,
  required String reason,
  String expires = '',
}) {
  final ms = machines
      .split(RegExp(r'[,\s]+'))
      .map((s) => s.trim())
      .where((s) => s.isNotEmpty)
      .toList();
  return ExceptionRule(
    id: id,
    scope: ExceptionScope(machines: ms, role: role.trim()),
    checkId: checkId.trim(),
    checkPattern: checkPattern.trim(),
    action: action,
    reason: reason.trim(),
    expires: expires.trim(),
    active: true,
  );
}

/// Null when valid, otherwise a message (mirrors the server's validation).
String? validateRule(ExceptionRule r) {
  if (r.checkId.isEmpty && r.checkPattern.isEmpty) {
    return 'Enter a check id or a check pattern';
  }
  if (!exceptionActions.contains(r.action)) return 'Pick an action';
  if (r.reason.isEmpty) return 'A reason is required';
  if (r.expires.isNotEmpty &&
      !RegExp(r'^\d{4}-\d{2}-\d{2}$').hasMatch(r.expires) &&
      DateTime.tryParse(r.expires) == null) {
    return 'Expires must be YYYY-MM-DD';
  }
  return null;
}

/// Add/edit dialog. Returns the rule to save, or null when cancelled.
Future<ExceptionRule?> showExceptionDialog(
  BuildContext context, {
  ExceptionRule? existing,
}) {
  return showDialog<ExceptionRule>(
    context: context,
    builder: (_) => ExceptionDialog(existing: existing),
  );
}

class ExceptionDialog extends StatefulWidget {
  final ExceptionRule? existing;
  const ExceptionDialog({super.key, this.existing});
  @override
  State<ExceptionDialog> createState() => _ExceptionDialogState();
}

class _ExceptionDialogState extends State<ExceptionDialog> {
  late final TextEditingController _machines;
  late final TextEditingController _role;
  late final TextEditingController _checkId;
  late final TextEditingController _pattern;
  late final TextEditingController _reason;
  late final TextEditingController _expires;
  late String _action;
  String? _error;

  @override
  void initState() {
    super.initState();
    final e = widget.existing;
    _machines = TextEditingController(text: e?.scope.machines.join(', ') ?? '');
    _role = TextEditingController(text: e?.scope.role ?? '');
    _checkId = TextEditingController(text: e?.checkId ?? '');
    _pattern = TextEditingController(text: e?.checkPattern ?? '');
    _reason = TextEditingController(text: e?.reason ?? '');
    _expires = TextEditingController(text: e?.expires ?? '');
    _action = exceptionActions.contains(e?.action) ? e!.action : 'ignore';
  }

  @override
  void dispose() {
    for (final c in [_machines, _role, _checkId, _pattern, _reason, _expires]) {
      c.dispose();
    }
    super.dispose();
  }

  void _submit() {
    final rule = buildRule(
      id: widget.existing?.id ?? '',
      machines: _machines.text,
      role: _role.text,
      checkId: _checkId.text,
      checkPattern: _pattern.text,
      action: _action,
      reason: _reason.text,
      expires: _expires.text,
    );
    final err = validateRule(rule);
    if (err != null) {
      setState(() => _error = err);
      return;
    }
    Navigator.of(context).pop(rule);
  }

  Widget _field(TextEditingController c, String label, {String? hint}) =>
      TextField(
        controller: c,
        decoration: InputDecoration(labelText: label, hintText: hint),
      );

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.existing == null ? 'Add exception' : 'Edit exception'),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              _field(
                _machines,
                'Machines',
                hint: 'Comma separated; empty = all',
              ),
              _field(_role, 'Role', hint: 'Server, RCU or Workstation'),
              _field(_checkId, 'Check id', hint: 'e.g. NET-SERVER-TCP-445'),
              _field(_pattern, 'Check pattern (regex)', hint: 'or a pattern'),
              DropdownButtonFormField<String>(
                initialValue: _action,
                decoration: const InputDecoration(labelText: 'Action'),
                items: exceptionActions
                    .map((a) => DropdownMenuItem(value: a, child: Text(a)))
                    .toList(),
                onChanged: (v) => setState(() => _action = v ?? 'ignore'),
              ),
              _field(_reason, 'Reason'),
              _field(_expires, 'Expires', hint: 'YYYY-MM-DD, empty = never'),
              if (_error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    _error!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(onPressed: _submit, child: const Text('Save')),
      ],
    );
  }
}
