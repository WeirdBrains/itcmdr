// v0.13.0: the app's private key at rest.
// - Windows: DPAPI (CurrentUser) blob in %APPDATA%\itcmdr\app-key.dpapi. Only this Windows user
//   on this PC can decrypt it.
// - Linux: the Secret Service keyring via `secret-tool` (libsecret) when it works.
// - macOS: the login keychain via `security`.
// - Otherwise (no keyring, e.g. a headless Linux box): a plaintext file readable by this user only
//   (0600) at $XDG_CONFIG_HOME/itcmdr/app-key. Documented in docs/SIGNING.md as the fallback.
// v0.12.0 kept the key in shared_preferences; `migrateLegacyKey` moves it and removes the old copy.
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

abstract class KeyStore {
  /// Short name shown in the app (e.g. "Windows DPAPI").
  String get kind;

  /// True when the key is encrypted by the OS (not the plaintext fallback).
  bool get encrypted;
  Future<String?> read();
  Future<void> write(String seedHex);
  Future<void> delete();
}

class MemoryKeyStore implements KeyStore {
  String? value;
  @override
  final String kind = 'memory (tests)';
  @override
  final bool encrypted = true;
  @override
  Future<String?> read() async => value;
  @override
  Future<void> write(String seedHex) async => value = seedHex;
  @override
  Future<void> delete() async => value = null;
}

/// Overwrite a file with random bytes, flush, then delete it (best effort on SSDs and
/// copy-on-write file systems, where old blocks may survive).
Future<void> shredFile(File f) async {
  if (!await f.exists()) return;
  final n = await f.length();
  final r = Random.secure();
  final raf = await f.open(mode: FileMode.writeOnly);
  try {
    await raf.writeFrom(List<int>.generate(n, (_) => r.nextInt(256)));
    await raf.flush();
  } finally {
    await raf.close();
  }
  await f.delete();
}

/// Plaintext fallback, permissions 0600 (POSIX). Not encrypted.
class FileKeyStore implements KeyStore {
  final File file;
  FileKeyStore(this.file);
  @override
  String get kind => 'file (plaintext, owner-only) ${file.path}';
  @override
  bool get encrypted => false;
  @override
  Future<String?> read() async =>
      await file.exists() ? (await file.readAsString()).trim() : null;
  @override
  Future<void> write(String seedHex) async {
    await file.parent.create(recursive: true);
    if (await file.exists()) await shredFile(file);
    await file.writeAsString('');
    if (!Platform.isWindows) {
      await Process.run('chmod', ['600', file.path]);
    }
    await file.writeAsString(seedHex, flush: true);
  }

  @override
  Future<void> delete() => shredFile(file);
}

/// Linux Secret Service (GNOME Keyring, KWallet) through `secret-tool`.
class SecretToolKeyStore implements KeyStore {
  final String exe;
  final String account;
  SecretToolKeyStore({this.exe = 'secret-tool', this.account = 'app-key'});
  List<String> get _attrs => ['service', 'itcmdr', 'account', account];
  @override
  String get kind => 'Secret Service keyring';
  @override
  bool get encrypted => true;

  @override
  Future<String?> read() async {
    final r = await Process.run(exe, ['lookup', ..._attrs]);
    final v = '${r.stdout}'.trim();
    return r.exitCode == 0 && v.isNotEmpty ? v : null;
  }

  @override
  Future<void> write(String seedHex) async {
    final p = await Process.start(exe, [
      'store',
      '--label=itcmdr app key',
      ..._attrs,
    ]);
    p.stdin.write(seedHex);
    await p.stdin.close();
    final code = await p.exitCode;
    if (code != 0) {
      throw StateError(
        'secret-tool store failed ($code): ${await p.stderr.transform(utf8.decoder).join()}',
      );
    }
  }

  @override
  Future<void> delete() async {
    await Process.run(exe, ['clear', ..._attrs]);
  }

  /// Usable when a keyring answers a round trip.
  static Future<bool> usable([String exe = 'secret-tool']) async {
    try {
      final probe = ['service', 'itcmdr', 'account', 'probe'];
      final p = await Process.start(exe, [
        'store',
        '--label=itcmdr probe',
        ...probe,
      ]);
      p.stdin.write('ok');
      await p.stdin.close();
      if (await p.exitCode.timeout(const Duration(seconds: 5)) != 0) {
        return false;
      }
      final r = await Process.run(exe, ['lookup', ...probe]);
      await Process.run(exe, ['clear', ...probe]);
      return r.exitCode == 0 && '${r.stdout}'.trim() == 'ok';
    } catch (_) {
      return false;
    }
  }
}

/// macOS login keychain. Note: `security` takes the secret as an argument.
class MacKeychainKeyStore implements KeyStore {
  @override
  String get kind => 'macOS keychain';
  @override
  bool get encrypted => true;
  @override
  Future<String?> read() async {
    final r = await Process.run('security', [
      'find-generic-password',
      '-s',
      'itcmdr',
      '-a',
      'app-key',
      '-w',
    ]);
    final v = '${r.stdout}'.trim();
    return r.exitCode == 0 && v.isNotEmpty ? v : null;
  }

  @override
  Future<void> write(String seedHex) async {
    final r = await Process.run('security', [
      'add-generic-password',
      '-U',
      '-s',
      'itcmdr',
      '-a',
      'app-key',
      '-w',
      seedHex,
    ]);
    if (r.exitCode != 0) throw StateError('keychain write failed: ${r.stderr}');
  }

  @override
  Future<void> delete() async {
    await Process.run('security', [
      'delete-generic-password',
      '-s',
      'itcmdr',
      '-a',
      'app-key',
    ]);
  }
}

// ---- Windows DPAPI (crypt32.dll), CurrentUser scope.
final class _Blob extends Struct {
  @Uint32()
  external int cbData;
  external Pointer<Uint8> pbData;
}

typedef _CryptN = Int32 Function(
  Pointer<_Blob>,
  Pointer<Utf16>,
  Pointer<_Blob>,
  Pointer<Void>,
  Pointer<Void>,
  Uint32,
  Pointer<_Blob>,
);
typedef _CryptD = int Function(
  Pointer<_Blob>,
  Pointer<Utf16>,
  Pointer<_Blob>,
  Pointer<Void>,
  Pointer<Void>,
  int,
  Pointer<_Blob>,
);
typedef _CryptUN = Int32 Function(
  Pointer<_Blob>,
  Pointer<Pointer<Utf16>>,
  Pointer<_Blob>,
  Pointer<Void>,
  Pointer<Void>,
  Uint32,
  Pointer<_Blob>,
);
typedef _CryptUD = int Function(
  Pointer<_Blob>,
  Pointer<Pointer<Utf16>>,
  Pointer<_Blob>,
  Pointer<Void>,
  Pointer<Void>,
  int,
  Pointer<_Blob>,
);
typedef _FreeN = Pointer<Void> Function(Pointer<Void>);

const _entropy = 'itcmdr-app-key/v1';
const _uiForbidden = 0x1;

Pointer<_Blob> _blobOf(Arena a, List<int> data) {
  final b = a<_Blob>();
  final p = a<Uint8>(data.isEmpty ? 1 : data.length);
  p.asTypedList(data.isEmpty ? 1 : data.length).setAll(0, data);
  b.ref
    ..cbData = data.length
    ..pbData = p;
  return b;
}

Uint8List _dpapi(List<int> input, {required bool protect}) => using((a) {
  final crypt = DynamicLibrary.open('crypt32.dll');
  final kernel = DynamicLibrary.open('kernel32.dll');
  final free = kernel.lookupFunction<_FreeN, _FreeN>('LocalFree');
  final inB = _blobOf(a, input);
  final ent = _blobOf(a, utf8.encode(_entropy));
  final out = a<_Blob>();
  final ok = protect
      ? crypt.lookupFunction<_CryptN, _CryptD>('CryptProtectData')(
          inB,
          'itcmdr app key'.toNativeUtf16(allocator: a),
          ent,
          nullptr,
          nullptr,
          _uiForbidden,
          out,
        )
      : crypt.lookupFunction<_CryptUN, _CryptUD>('CryptUnprotectData')(
          inB,
          nullptr,
          ent,
          nullptr,
          nullptr,
          _uiForbidden,
          out,
        );
  if (ok == 0) {
    throw StateError('DPAPI ${protect ? 'protect' : 'unprotect'} failed');
  }
  final r = Uint8List.fromList(out.ref.pbData.asTypedList(out.ref.cbData));
  if (!protect) {
    // Wipe the plaintext copy DPAPI handed back before freeing it.
    out.ref.pbData.asTypedList(out.ref.cbData).fillRange(0, out.ref.cbData, 0);
  }
  free(out.ref.pbData.cast());
  return r;
});

Uint8List dpapiProtect(List<int> data) => _dpapi(data, protect: true);
Uint8List dpapiUnprotect(List<int> blob) => _dpapi(blob, protect: false);

class DpapiKeyStore implements KeyStore {
  final File file;
  DpapiKeyStore(this.file);
  @override
  String get kind => 'Windows DPAPI (current user)';
  @override
  bool get encrypted => true;
  @override
  Future<String?> read() async {
    if (!await file.exists()) return null;
    return utf8.decode(dpapiUnprotect(await file.readAsBytes()));
  }

  @override
  Future<void> write(String seedHex) async {
    await file.parent.create(recursive: true);
    final tmp = File('${file.path}.tmp');
    await tmp.writeAsBytes(dpapiProtect(utf8.encode(seedHex)), flush: true);
    await tmp.rename(file.path);
  }

  @override
  Future<void> delete() => shredFile(file);
}

String _home() =>
    Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'] ?? '.';

/// The store for this OS (see the file comment).
Future<KeyStore> platformKeyStore() async {
  if (Platform.isWindows) {
    final appData =
        Platform.environment['APPDATA'] ?? '${_home()}\\AppData\\Roaming';
    return DpapiKeyStore(File('$appData\\itcmdr\\app-key.dpapi'));
  }
  if (Platform.isMacOS) return MacKeychainKeyStore();
  if (await SecretToolKeyStore.usable()) return SecretToolKeyStore();
  final cfg = Platform.environment['XDG_CONFIG_HOME'] ?? '${_home()}/.config';
  return FileKeyStore(File('$cfg/itcmdr/app-key'));
}
