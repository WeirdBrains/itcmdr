// v0.12.0 Schedules (decision 14): list, create/edit with a live preview of the next runs,
// pause/resume, run now, history. Built-in schedules (nightly agent run, nightly backup) can be
// edited, paused or reset, and deleted only by typing their exact name.
import 'dart:async';

import 'package:fixnum/fixnum.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';

import 'proto/itcmdr.pbgrpc.dart';
import 'settings.dart';

List<String> splitList(String s) => s
    .split(RegExp(r'[,\s]+'))
    .map((e) => e.trim())
    .where((e) => e.isNotEmpty)
    .toList();

String fmtMs(Int64 ms) {
  if (ms == Int64.ZERO) return '-';
  final d = DateTime.fromMillisecondsSinceEpoch(ms.toInt());
  String two(int n) => n.toString().padLeft(2, '0');
  return '${d.year}-${two(d.month)}-${two(d.day)} ${two(d.hour)}:${two(d.minute)}';
}

/// Client-side form check (the server validates again). Null = ok.
String? validateScheduleForm(Schedule s) {
  if (s.name.trim().isEmpty) return 'Give the schedule a name';
  if (s.when.trim().isEmpty) return 'Say when it runs (e.g. daily 18:30)';
  if (s.action != 'backup' &&
      !s.allMachines &&
      s.machines.isEmpty &&
      s.groups.isEmpty) {
    return 'Pick all machines, some machines or a group';
  }
  return null;
}

/// Whether a delete of [s] may go ahead with what the user typed.
bool deleteAllowed(Schedule s, String typed) => !s.builtin || typed == s.name;

class SchedulesPage extends StatefulWidget {
  final ItcmdrClient client;
  final ServerSettings settings;
  const SchedulesPage({
    super.key,
    required this.client,
    required this.settings,
  });
  @override
  State<SchedulesPage> createState() => _SchedulesPageState();
}

class _SchedulesPageState extends State<SchedulesPage> {
  List<Schedule> _items = [];
  String? _error;
  bool _loaded = false;

  CallOptions get _opts => widget.settings.authOptions();

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final r = await widget.client.listSchedules(
        ListSchedulesRequest(),
        options: _opts,
      );
      if (!mounted) return;
      setState(() {
        _items = r.schedules;
        _error = null;
        _loaded = true;
      });
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = '$e';
          _loaded = true;
        });
      }
    }
  }

  Future<void> _run(Future<void> Function() f) async {
    try {
      await f();
      await _load();
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  void _say(String m) {
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(m)));
    }
  }

  Future<void> _edit(Schedule? s) async {
    final out = await showDialog<Schedule>(
      context: context,
      builder: (_) =>
          ScheduleDialog(client: widget.client, opts: _opts, initial: s),
    );
    if (out == null) return;
    await _run(() async {
      final req = SaveScheduleRequest(schedule: out);
      if (s == null) {
        await widget.client.createSchedule(req, options: _opts);
      } else {
        await widget.client.updateSchedule(req, options: _opts);
      }
    });
  }

  Future<void> _delete(Schedule s) async {
    final typed = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, set) => AlertDialog(
          title: Text('Delete ${s.name}?'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (s.builtin) ...[
                const Text(
                  'This is a built-in schedule. Pausing it is usually what you want. '
                  'To delete it anyway, type its exact name. "Reset to default" brings it back.',
                ),
                TextField(
                  key: const Key('delete-confirm'),
                  controller: typed,
                  onChanged: (_) => set(() {}),
                  decoration: InputDecoration(labelText: s.name),
                ),
              ] else
                const Text('Its history stays in the audit log.'),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const Key('delete-go'),
              onPressed: deleteAllowed(s, typed.text)
                  ? () => Navigator.pop(ctx, true)
                  : null,
              child: const Text('Delete'),
            ),
          ],
        ),
      ),
    );
    final t = typed.text;
    typed.dispose();
    if (ok != true) return;
    await _run(
      () => widget.client.deleteSchedule(
        ScheduleRef(id: s.id, force: s.builtin, confirm: s.builtin ? t : ''),
        options: _opts,
      ),
    );
  }

  Future<void> _history(Schedule s) async {
    List<ScheduleRun> runs = [];
    String? err;
    try {
      runs = (await widget.client.listScheduleRuns(
        ListScheduleRunsRequest(id: s.id, limit: 50),
        options: _opts,
      )).runs;
    } catch (e) {
      err = '$e';
    }
    if (!mounted) return;
    await showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('History: ${s.name}'),
        content: SizedBox(
          width: 560,
          height: 400,
          child: err != null
              ? Text(err)
              : runs.isEmpty
              ? const Text('No runs yet')
              : ListView(
                  children: [
                    for (final r in runs)
                      ListTile(
                        dense: true,
                        title: Text('${fmtMs(r.ts)}  ${r.outcome}'),
                        subtitle: Text(
                          [
                            if (r.requestId.isNotEmpty)
                              'request ${r.requestId}',
                            r.note,
                          ].where((x) => x.isNotEmpty).join('  '),
                        ),
                      ),
                  ],
                ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            Text('Schedules', style: Theme.of(context).textTheme.titleMedium),
            const Spacer(),
            IconButton(
              tooltip: 'Refresh',
              onPressed: _load,
              icon: const Icon(Icons.refresh),
            ),
            FilledButton.icon(
              key: const Key('schedule-new'),
              onPressed: () => _edit(null),
              icon: const Icon(Icons.add),
              label: const Text('New schedule'),
            ),
          ],
        ),
        if (_error != null)
          Text(_error!, style: const TextStyle(color: Colors.red)),
        if (!_loaded) const LinearProgressIndicator(),
        for (final s in _items)
          Card(
            child: ListTile(
              leading: Icon(
                s.enabled ? Icons.schedule : Icons.pause_circle,
                color: s.enabled ? Colors.green : Colors.grey,
              ),
              title: Text('${s.name}${s.builtin ? '  (built-in)' : ''}'),
              subtitle: Text(
                [
                  '${s.when} (${s.timezone.isEmpty ? 'local' : s.timezone})',
                  s.action == 'backup'
                      ? 'server backup'
                      : s.allMachines
                      ? 'all machines'
                      : [...s.machines, ...s.groups].join(', '),
                  if (s.action != 'backup') s.repair ? 'repair' : 'report-only',
                  s.enabled && s.nextRuns.isNotEmpty
                      ? 'next ${fmtMs(s.nextRuns.first)}'
                      : 'paused',
                ].join(' · '),
              ),
              trailing: Wrap(
                children: [
                  IconButton(
                    tooltip: s.enabled ? 'Pause' : 'Resume',
                    icon: Icon(s.enabled ? Icons.pause : Icons.play_arrow),
                    onPressed: () => _run(
                      () => s.enabled
                          ? widget.client.pauseSchedule(
                              ScheduleRef(id: s.id),
                              options: _opts,
                            )
                          : widget.client.resumeSchedule(
                              ScheduleRef(id: s.id),
                              options: _opts,
                            ),
                    ),
                  ),
                  IconButton(
                    tooltip: 'Run now',
                    icon: const Icon(Icons.bolt),
                    onPressed: () => _run(() async {
                      final r = await widget.client.runScheduleNow(
                        ScheduleRef(id: s.id),
                        options: _opts,
                      );
                      _say(
                        '${s.name}: ${r.outcome}${r.note.isEmpty ? '' : ' (${r.note})'}',
                      );
                    }),
                  ),
                  IconButton(
                    tooltip: 'History',
                    icon: const Icon(Icons.history),
                    onPressed: () => _history(s),
                  ),
                  IconButton(
                    tooltip: 'Edit',
                    icon: const Icon(Icons.edit),
                    onPressed: () => _edit(s),
                  ),
                  if (s.builtin)
                    IconButton(
                      tooltip: 'Reset to default',
                      icon: const Icon(Icons.restart_alt),
                      onPressed: () => _run(
                        () => widget.client.resetScheduleToDefault(
                          ScheduleRef(id: s.id),
                          options: _opts,
                        ),
                      ),
                    ),
                  IconButton(
                    tooltip: 'Delete',
                    icon: const Icon(Icons.delete_outline),
                    onPressed: () => _delete(s),
                  ),
                ],
              ),
            ),
          ),
      ],
    );
  }
}

class ScheduleDialog extends StatefulWidget {
  final ItcmdrClient? client;
  final CallOptions? opts;
  final Schedule? initial;
  const ScheduleDialog({super.key, this.client, this.opts, this.initial});
  @override
  State<ScheduleDialog> createState() => ScheduleDialogState();
}

class ScheduleDialogState extends State<ScheduleDialog> {
  late final Schedule _s =
      widget.initial?.deepCopy() ??
      (Schedule()
        ..enabled = true
        ..allMachines = true
        ..when = 'daily 18:30'
        ..timezone = 'local'
        ..guard = 'report-only-in-hours'
        ..missed = 'skip');
  late final _name = TextEditingController(text: _s.name);
  late final _when = TextEditingController(text: _s.when);
  late final _tz = TextEditingController(text: _s.timezone);
  late final _machines = TextEditingController(text: _s.machines.join(', '));
  late final _groups = TextEditingController(text: _s.groups.join(', '));
  late final _cats = TextEditingController(text: _s.categories.join(', '));
  late final _plugins = TextEditingController(text: _s.plugins.join(', '));
  List<Int64> _next = [];
  String? _previewError;
  String? _error;
  Timer? _debounce;

  bool get _isBackup => _s.action == 'backup';

  @override
  void initState() {
    super.initState();
    _preview();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    for (final c in [_name, _when, _tz, _machines, _groups, _cats, _plugins]) {
      c.dispose();
    }
    super.dispose();
  }

  Schedule build_() {
    final s = _s.deepCopy()
      ..name = _name.text.trim()
      ..when = _when.text.trim()
      ..timezone = _tz.text.trim().isEmpty ? 'local' : _tz.text.trim();
    s.machines
      ..clear()
      ..addAll(_isBackup || s.allMachines ? [] : splitList(_machines.text));
    s.groups
      ..clear()
      ..addAll(_isBackup || s.allMachines ? [] : splitList(_groups.text));
    s.categories
      ..clear()
      ..addAll(_isBackup ? [] : splitList(_cats.text));
    s.plugins
      ..clear()
      ..addAll(_isBackup ? [] : splitList(_plugins.text));
    s.nextRuns.clear();
    return s;
  }

  void _changed() {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 400), _preview);
    setState(() {});
  }

  Future<void> _preview() async {
    final c = widget.client;
    if (c == null) return;
    try {
      final r = await c.previewSchedule(
        SaveScheduleRequest(schedule: build_()),
        options: widget.opts,
      );
      if (!mounted) return;
      setState(() {
        _next = r.nextRuns;
        _previewError = r.error.isEmpty ? null : r.error;
      });
    } catch (e) {
      if (mounted) setState(() => _previewError = '$e');
    }
  }

  Widget _field(
    TextEditingController c,
    String label, {
    String? hint,
    Key? key,
  }) => TextField(
    key: key,
    controller: c,
    onChanged: (_) => _changed(),
    decoration: InputDecoration(labelText: label, hintText: hint),
  );

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(
        widget.initial == null
            ? 'New schedule'
            : 'Edit ${widget.initial!.name}',
      ),
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              _field(_name, 'Name', key: const Key('s-name')),
              _field(
                _when,
                'When',
                hint: 'hourly | every 4h | daily 18:30 | weekdays 07:00 | weekly sun 03:00 | cron m h dom mon dow',
                key: const Key('s-when'),
              ),
              _field(
                _tz,
                'Time zone',
                hint: 'local or an IANA name (America/New_York)',
              ),
              SwitchListTile(
                title: const Text('Enabled'),
                value: _s.enabled,
                onChanged: (v) => setState(() => _s.enabled = v),
              ),
              if (_isBackup)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 8),
                  child: Text(
                    'This schedule runs the server backup; it has no machines.',
                  ),
                )
              else ...[
                SwitchListTile(
                  title: const Text('All machines'),
                  value: _s.allMachines,
                  onChanged: (v) {
                    _s.allMachines = v;
                    _changed();
                  },
                ),
                if (!_s.allMachines) ...[
                  _field(
                    _machines,
                    'Machines',
                    hint: 'WS-01, WS-02',
                    key: const Key('s-machines'),
                  ),
                  _field(_groups, 'Groups', hint: 'Server, Workstation, RCU'),
                ],
                _field(
                  _cats,
                  'Check categories (blank = all)',
                  hint: 'network, services',
                ),
                _field(_plugins, 'Plugins (optional)'),
                SwitchListTile(
                  title: const Text('Repair (off = report-only)'),
                  value: _s.repair,
                  onChanged: (v) => setState(() => _s.repair = v),
                ),
                DropdownButtonFormField<String>(
                  initialValue: _s.guard.isEmpty
                      ? 'report-only-in-hours'
                      : _s.guard,
                  decoration: const InputDecoration(
                    labelText: 'During office hours',
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: 'report-only-in-hours',
                      child: Text('report only'),
                    ),
                    DropdownMenuItem(
                      value: 'skip-in-hours',
                      child: Text('skip'),
                    ),
                    DropdownMenuItem(
                      value: 'none',
                      child: Text('run normally'),
                    ),
                  ],
                  onChanged: (v) => setState(() => _s.guard = v ?? 'none'),
                ),
              ],
              DropdownButtonFormField<String>(
                initialValue: _s.missed.isEmpty ? 'skip' : _s.missed,
                decoration: const InputDecoration(
                  labelText: 'If the server was off at that time',
                ),
                items: const [
                  DropdownMenuItem(value: 'skip', child: Text('skip it')),
                  DropdownMenuItem(
                    value: 'catch-up',
                    child: Text('run once when back'),
                  ),
                ],
                onChanged: (v) => setState(() => _s.missed = v ?? 'skip'),
              ),
              const SizedBox(height: 12),
              Text('Next runs', style: Theme.of(context).textTheme.titleSmall),
              if (_previewError != null)
                Text(
                  _previewError!,
                  key: const Key('s-preview-error'),
                  style: const TextStyle(color: Colors.red),
                )
              else if (!_s.enabled)
                const Text('paused (shown for when it is enabled)'),
              for (final t in _next) Text(fmtMs(t)),
              if (_error != null)
                Text(_error!, style: const TextStyle(color: Colors.red)),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('s-save'),
          onPressed: () {
            final s = build_();
            final e = validateScheduleForm(s);
            if (e != null) {
              setState(() => _error = e);
              return;
            }
            Navigator.pop(context, s);
          },
          child: const Text('Save'),
        ),
      ],
    );
  }
}
