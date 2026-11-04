// v1.2.0: the office app's id changed from the Flutter placeholder `com.example.app` to
// `com.weirdbrains.itcmdr` (Linux application id, macOS bundle id, Windows CompanyName).
// shared_preferences keeps the settings (server address, TLS choice, certificate pins) in a
// folder named after the id, so on Linux and Windows the old file is copied once to the new
// folder before the settings are read. The old file is left in place (harmless, and a way back).
// macOS keeps them in the app's sandbox container, which another bundle id cannot read: there
// the settings are entered once again after the upgrade. The app key is not affected on any
// platform (DPAPI file, keyring or file under itcmdr/, and the macOS keychain; see key_store.dart).
import 'dart:io';

const legacyAppId = 'com.example.app';
const appId = 'com.weirdbrains.itcmdr';
const prefsFile = 'shared_preferences.json';

/// (old folder, new folder) for this platform, or null where nothing can be moved.
(String, String)? prefsFolders({
  required bool linux,
  required bool windows,
  required Map<String, String> env,
}) {
  if (linux) {
    final xdg = env['XDG_DATA_HOME'];
    final base = (xdg != null && xdg.isNotEmpty)
        ? xdg
        : '${env['HOME'] ?? '.'}/.local/share';
    return ('$base/$legacyAppId', '$base/$appId');
  }
  if (windows) {
    final appData = env['APPDATA'];
    if (appData == null || appData.isEmpty) return null;
    return (
      '$appData\\com.example\\itcmdr',
      '$appData\\com.weirdbrains\\itcmdr',
    );
  }
  return null;
}

/// Copy the old settings file to the new folder when the new one has none. True if copied.
Future<bool> migrateLegacyPrefs({
  bool? linux,
  bool? windows,
  Map<String, String>? env,
}) async {
  final f = prefsFolders(
    linux: linux ?? Platform.isLinux,
    windows: windows ?? Platform.isWindows,
    env: env ?? Platform.environment,
  );
  if (f == null) return false;
  final sep = (windows ?? Platform.isWindows) ? '\\' : '/';
  final oldFile = File('${f.$1}$sep$prefsFile');
  final newFile = File('${f.$2}$sep$prefsFile');
  try {
    if (await newFile.exists() || !await oldFile.exists()) return false;
    await Directory(f.$2).create(recursive: true);
    await oldFile.copy(newFile.path);
    return true;
  } on FileSystemException {
    return false; // never block start-up; the settings can be entered again
  }
}
