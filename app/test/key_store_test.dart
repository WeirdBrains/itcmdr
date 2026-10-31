import 'dart:convert';
import 'dart:io';

import 'package:app/key_store.dart';
import 'package:app/settings.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

const seed = '0707070707070707070707070707070707070707070707070707070707070707';

void main() {
  late MemoryKeyStore mem;
  setUp(() {
    mem = MemoryKeyStore();
    ServerSettings.keyStoreFactory = () async => mem;
    ServerSettings.resetKeyStore();
  });

  test(
    'a v0.12.0 plaintext key moves into the key store and leaves preferences',
    () async {
      SharedPreferences.setMockInitialValues({
        'auth.key_seed': seed,
        'server.host': 'SERVER01',
      });
      final s = await ServerSettings.load();
      expect(s.keySeed, seed);
      expect(mem.value, seed);
      final p = await SharedPreferences.getInstance();
      expect(p.containsKey('auth.key_seed'), isFalse);
      expect(s.host, 'SERVER01');
      // Saving keeps it in the store only; clearing the key deletes it.
      await s.save();
      expect(p.containsKey('auth.key_seed'), isFalse);
      await s.copyWith(keySeed: '').save();
      expect(mem.value, isNull);
    },
  );

  test(
    'migration keeps an existing store key and still removes the legacy copy',
    () async {
      mem.value = 'aa' * 32;
      SharedPreferences.setMockInitialValues({'auth.key_seed': seed});
      final p = await SharedPreferences.getInstance();
      expect(await ServerSettings.migrateLegacyKey(p, mem), isTrue);
      expect(mem.value, 'aa' * 32);
      expect(p.containsKey('auth.key_seed'), isFalse);
    },
  );

  test('a failed read-back keeps the legacy key', () async {
    SharedPreferences.setMockInitialValues({'auth.key_seed': seed});
    final p = await SharedPreferences.getInstance();
    await expectLater(
      ServerSettings.migrateLegacyKey(p, _Broken()),
      throwsStateError,
    );
    expect(p.getString('auth.key_seed'), seed);
  });

  test('file fallback is owner-only and shredded on delete', () async {
    final d = Directory.systemTemp.createTempSync('itcmdr-ks-');
    addTearDown(() => d.deleteSync(recursive: true));
    final f = File('${d.path}/sub/app-key');
    final ks = FileKeyStore(f);
    expect(ks.encrypted, isFalse);
    await ks.write(seed);
    expect(await ks.read(), seed);
    if (!Platform.isWindows) {
      final mode = (await f.stat()).mode & 0x1ff;
      expect(mode, 0x180, reason: '0600');
    }
    await ks.delete();
    expect(f.existsSync(), isFalse);
    expect(await ks.read(), isNull);
  });

  test(
    'Secret Service keyring round trip (when a keyring is running)',
    () async {
      if (Platform.isWindows || !await SecretToolKeyStore.usable()) {
        markTestSkipped('no Secret Service keyring');
        return;
      }
      final ks = SecretToolKeyStore(
        account: 'test-${DateTime.now().microsecondsSinceEpoch}',
      );
      await ks.write(seed);
      expect(await ks.read(), seed);
      await ks.delete();
      expect(await ks.read(), isNull);
    },
  );

  test('Windows DPAPI round trip, bound to this user (Windows only)', () async {
    if (!Platform.isWindows) {
      markTestSkipped('Windows only');
      return;
    }
    final blob = dpapiProtect(seed.codeUnits);
    expect(
      String.fromCharCodes(blob).contains(seed),
      isFalse,
      reason: 'encrypted',
    );
    expect(String.fromCharCodes(dpapiUnprotect(blob)), seed);
    final d = Directory.systemTemp.createTempSync('itcmdr-dpapi-');
    addTearDown(() => d.deleteSync(recursive: true));
    final ks = DpapiKeyStore(File('${d.path}\\app-key.dpapi'));
    await ks.write(seed);
    expect(
      File('${d.path}\\app-key.dpapi')
          .readAsStringSync(encoding: const _Latin1())
          .contains(seed),
      isFalse,
    );
    expect(await ks.read(), seed);
    await ks.delete();
    expect(await ks.read(), isNull);
    // Tampered blobs do not open.
    final bad = List<int>.from(blob)..[blob.length ~/ 2] ^= 0xff;
    expect(() => dpapiUnprotect(bad), throwsStateError);
  });

  test('the platform store on this OS', () async {
    final ks = await platformKeyStore();
    if (Platform.isWindows) expect(ks, isA<DpapiKeyStore>());
    expect(ks.kind, isNotEmpty);
  });
}

class _Latin1 extends Encoding {
  const _Latin1();
  @override
  Converter<List<int>, String> get decoder =>
      const Latin1Decoder(allowInvalid: true);
  @override
  Converter<String, List<int>> get encoder => const Latin1Encoder();
  @override
  String get name => 'latin1';
}

class _Broken extends MemoryKeyStore {
  @override
  Future<String?> read() async => null;
}
