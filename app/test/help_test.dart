import 'dart:io';

import 'package:app/help_page.dart';
import 'package:app/help_topics.g.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('help topics match docs/ (run scripts/gen-help.py)', () {
    final docs = Directory('../docs')
        .listSync()
        .whereType<File>()
        .where((f) => f.path.endsWith('.md'))
        .toList();
    expect(helpTopics.length, docs.length);
    for (final f in docs) {
      var id = f.uri.pathSegments.last.replaceAll('.md', '').toLowerCase();
      if (id == 'readme') id = 'index';
      expect(helpTopics[id], f.readAsStringSync(), reason: id);
    }
  });

  test('search and links', () {
    expect(helpSearch('certificate').first, 'certificates');
    expect(helpLinks(helpTopics['index']!), contains('backup'));
  });

  testWidgets('help page opens a topic and searches', (t) async {
    await t.pumpWidget(const MaterialApp(home: Scaffold(body: HelpPage())));
    expect(find.text('itcmdr documentation'), findsOneWidget);
    await t.enterText(find.byKey(const Key('help-search')), 'backup');
    await t.pump();
    await t.tap(find.text('Backup and restore').first);
    await t.pump();
    expect(find.text('Backup and restore'), findsWidgets);
  });
}
