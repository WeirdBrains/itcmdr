// The settings reference table in docs/SETTINGS.md, as SettingItems the app can validate.
import 'dart:io';

import 'package:app/proto/itcmdr.pb.dart';
import 'package:fixnum/fixnum.dart';

List<SettingItem> settingsFromReference([String? markdown]) {
  final text = markdown ?? File('../docs/SETTINGS.md').readAsStringSync();
  final out = <SettingItem>[];
  for (final line in text.split('\n')) {
    if (!line.startsWith('| `')) continue;
    final cells = line
        .split('|')
        .map((c) => c.trim())
        .where((c) => c.isNotEmpty)
        .toList();
    if (cells.length < 6) continue;
    final key = cells[0].replaceAll('`', '');
    final scope = cells[1];
    final typeCell = cells[2];
    var value = cells[3].replaceAll('`', '').trim();
    final who = cells[5];
    var type = 'text';
    var min = 0;
    var max = 0;
    if (typeCell.startsWith('int')) {
      type = 'int';
      final m = RegExp(r'(-?\d+)\.\.(-?\d+)')
          .firstMatch(typeCell.replaceAll('_', ''));
      if (m != null) {
        min = int.parse(m.group(1)!);
        max = int.parse(m.group(2)!);
      }
    } else if (typeCell.startsWith('bool')) {
      type = 'bool';
    } else if (typeCell.startsWith('time')) {
      type = 'time';
    } else if (typeCell.startsWith('path')) {
      type = 'path';
      if (value.startsWith('(')) value = '';
    }
    out.add(
      SettingItem(
        key: key,
        scope: scope,
        type: type,
        value: value,
        defaultValue: value,
        description: cells.length > 6 ? cells[6] : '',
        min: Int64(min),
        max: Int64(max),
        readOnly: who.contains('nobody'),
      ),
    );
  }
  return out;
}
