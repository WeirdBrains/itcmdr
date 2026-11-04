import 'dart:io';

import 'package:app/app_id_migration.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Linux: old settings copied once to the new app id folder', () async {
    final d = await Directory.systemTemp.createTemp('appid');
    final env = {'XDG_DATA_HOME': d.path, 'HOME': '/nonexistent'};
    final old = File('${d.path}/com.example.app/shared_preferences.json');
    await old.create(recursive: true);
    await old.writeAsString('{"flutter.server":"a"}');
    expect(
      await migrateLegacyPrefs(linux: true, windows: false, env: env),
      isTrue,
    );
    final nf = File('${d.path}/com.weirdbrains.itcmdr/shared_preferences.json');
    expect(await nf.readAsString(), '{"flutter.server":"a"}');
    expect(await old.exists(), isTrue);
    // Never overwrites settings made after the upgrade.
    await nf.writeAsString('{"flutter.server":"b"}');
    expect(
      await migrateLegacyPrefs(linux: true, windows: false, env: env),
      isFalse,
    );
    expect(await nf.readAsString(), '{"flutter.server":"b"}');
    await d.delete(recursive: true);
  });

  test('nothing to move: fresh install, macOS, no APPDATA', () async {
    final d = await Directory.systemTemp.createTemp('appid');
    expect(
      await migrateLegacyPrefs(
        linux: true,
        windows: false,
        env: {'XDG_DATA_HOME': d.path},
      ),
      isFalse,
    );
    expect(
      await migrateLegacyPrefs(linux: false, windows: false, env: {}),
      isFalse,
    );
    expect(prefsFolders(linux: false, windows: true, env: {}), isNull);
    expect(
      prefsFolders(
        linux: false,
        windows: true,
        env: {'APPDATA': r'C:\U\AppData\Roaming'},
      ),
      (
        r'C:\U\AppData\Roaming\com.example\itcmdr',
        r'C:\U\AppData\Roaming\com.weirdbrains\itcmdr',
      ),
    );
    expect(
      prefsFolders(linux: true, windows: false, env: {'HOME': '/home/u'}),
      (
        '/home/u/.local/share/com.example.app',
        '/home/u/.local/share/com.weirdbrains.itcmdr',
      ),
    );
    await d.delete(recursive: true);
  });
}
