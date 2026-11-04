// In-app help: the guides in docs/, embedded by scripts/gen-help.py (docs/HELP.md).
import 'package:flutter/material.dart';

import 'help_topics.g.dart';

String helpTitle(String md) {
  for (final l in md.split('\n')) {
    if (l.startsWith('# ')) return l.substring(2).trim();
  }
  return '';
}

/// Topic ids linked from [md] (`[text](FILE.md)`), in order, without repeats.
List<String> helpLinks(String md) {
  final out = <String>[];
  for (final m in RegExp(
    r'\]\(([A-Za-z0-9_-]+)\.md(#[^)]*)?\)',
  ).allMatches(md)) {
    var id = m.group(1)!.toLowerCase();
    if (id == 'readme') id = 'index';
    if (helpTopics.containsKey(id) && !out.contains(id)) out.add(id);
  }
  return out;
}

/// Topic ids whose title or id contains [q] first, then those whose text does.
List<String> helpSearch(String q) {
  final s = q.toLowerCase().trim();
  if (s.isEmpty) return helpTopics.keys.toList();
  final first = <String>[], second = <String>[];
  helpTopics.forEach((id, md) {
    if (id.contains(s) || helpTitle(md).toLowerCase().contains(s)) {
      first.add(id);
    } else if (md.toLowerCase().contains(s)) {
      second.add(id);
    }
  });
  return [...first, ...second];
}

class HelpPage extends StatefulWidget {
  const HelpPage({super.key, this.topic = 'index'});
  final String topic;
  @override
  State<HelpPage> createState() => _HelpPageState();
}

class _HelpPageState extends State<HelpPage> {
  late final List<String> _history = [widget.topic];
  String _query = '';

  String get _topic => _history.last;

  void _open(String id) => setState(() {
    _history.add(id);
    _query = '';
  });

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final md = helpTopics[_topic] ?? helpTopics['index']!;
    final results = _query.isEmpty ? const <String>[] : helpSearch(_query);
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              IconButton(
                tooltip: 'Back',
                icon: const Icon(Icons.arrow_back),
                onPressed: _history.length > 1
                    ? () => setState(_history.removeLast)
                    : null,
              ),
              IconButton(
                tooltip: 'Contents',
                icon: const Icon(Icons.home),
                onPressed: () => _open('index'),
              ),
              Expanded(
                child: TextField(
                  key: const Key('help-search'),
                  decoration: const InputDecoration(
                    prefixIcon: Icon(Icons.search),
                    hintText: 'Search help',
                  ),
                  onChanged: (v) => setState(() => _query = v),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Expanded(
            child: _query.isNotEmpty
                ? ListView(
                    children: [
                      if (results.isEmpty)
                        const ListTile(title: Text('No matches')),
                      for (final id in results)
                        ListTile(
                          title: Text(helpTitle(helpTopics[id]!)),
                          subtitle: Text(id),
                          onTap: () => _open(id),
                        ),
                    ],
                  )
                : ListView(
                    children: [
                      for (final line in md.split('\n')) _line(line, t),
                      const Divider(),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: [
                          for (final id in helpLinks(md))
                            ActionChip(
                              label: Text(helpTitle(helpTopics[id]!)),
                              onPressed: () => _open(id),
                            ),
                        ],
                      ),
                    ],
                  ),
          ),
        ],
      ),
    );
  }

  Widget _line(String line, TextTheme t) {
    final plain = line.replaceAllMapped(
      RegExp(r'\[([^\]]+)\]\([^)]+\)'),
      (m) => m.group(1)!,
    );
    if (plain.startsWith('# ')) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 8),
        child: Text(plain.substring(2), style: t.headlineSmall),
      );
    }
    if (plain.startsWith('## ') || plain.startsWith('### ')) {
      return Padding(
        padding: const EdgeInsets.only(top: 12, bottom: 4),
        child: Text(
          plain.replaceFirst(RegExp(r'^#+ '), ''),
          style: t.titleMedium,
        ),
      );
    }
    final mono = line.startsWith('    ') || line.startsWith('|');
    return SelectableText(
      plain,
      style: mono
          ? t.bodyMedium?.copyWith(fontFamily: 'monospace')
          : t.bodyMedium,
    );
  }
}
