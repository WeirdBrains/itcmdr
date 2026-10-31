// The app must produce the server's bytes exactly (docs/SIGNING.md). Never edit the vectors.
import 'dart:convert';
import 'dart:io';

import 'package:app/proto/itcmdr.pb.dart';
import 'package:app/signing.dart';
import 'package:fixnum/fixnum.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:protobuf/protobuf.dart';

dynamic _json(String name) =>
    jsonDecode(File('../proto/$name').readAsStringSync());

final _built = <String, List<int> Function()>{
  'ListMachines/empty': () => ListMachinesRequest().canon(),
  'TriggerRuns/typical': () =>
      (TriggerRunsRequest()
            ..machines.addAll(['WS-01', 'WS-02'])
            ..reportOnly = true
            ..categories.addAll(['network', 'services']))
          .canon(),
  'TriggerRuns/defaults-omitted': () =>
      (TriggerRunsRequest()..reportOnly = false).canon(),
  'SaveException/nested-unicode': () =>
      (SaveExceptionRequest()
            ..rule = (ExceptionRule()
              ..checkId = 'NET-DNS'
              ..action = 'downgrade:warn'
              ..reason = 'Praxis-Drucker – known'
              ..expires = '2026-12-31'))
          .canon(),
  'SearchLogs/ints': () =>
      (SearchLogsRequest()
            ..query = 'timeout'
            ..startTs = Int64(1790000000000)
            ..endTs = Int64(-1)
            ..limit = 100
            ..offset = 300)
          .canon(),
  'VerifyBackup/name': () =>
      (VerifyBackupRequest()..name = 'itcmdr-20261030-020000000-nightly.itcb')
          .canon(),
  'SaveSchedule/typical': () =>
      (SaveScheduleRequest()
            ..schedule = (Schedule()
              ..name = 'Evening scan'
              ..enabled = true
              ..machines.add('WS-01')
              ..groups.add('Workstation')
              ..categories.add('network')
              ..when = 'weekdays 18:30'
              ..timezone = 'local'
              ..guard = 'none'
              ..missed = 'skip'))
          .canon(),
  'StageRestore/sealed-dry-run': () =>
      (StageRestoreRequest()
            ..backup = 'itcmdr-20261030-020000000-nightly.itcb'
            ..sealedWords = (SealedWords()
              ..keyId = 'k1'
              ..eph = 'ab'
              ..nonce = 'cd'
              ..ct = 'ef')
            ..dryRun = true)
          .canon(),
  'UpdateSetting/override': () =>
      (UpdateSettingRequest()
            ..key = 'agent.nightlyTime'
            ..value = '01:30'
            ..machine = 'WS-01'
            ..unset = false)
          .canon(),
};

extension on GeneratedMessage {
  List<int> canon() => canonicalBytes(this);
}

void main() {
  test(
    'explicit defaults are not signed (Dart writes them, the server does not)',
    () {
      final m = TriggerRunsRequest()..reportOnly = false;
      expect(
        m.writeToBuffer(),
        isNotEmpty,
        reason: 'why canonicalBytes exists',
      );
      expect(canonicalBytes(m), isEmpty);
      final n = SaveScheduleRequest()..schedule = (Schedule()..enabled = false);
      expect(
        hexOf(canonicalBytes(n)),
        '0a00',
        reason: 'a set sub-message stays',
      );
    },
  );

  test('every pinned request encodes and signs like the server', () async {
    final vs = _json('signing_vectors.json') as List;
    expect(
      vs.map((v) => v['name']).toSet(),
      _built.keys.toSet(),
      reason: 'a vector without a Dart builder (or the reverse)',
    );
    for (final v in vs) {
      final body = _built[v['name']]!();
      expect(hexOf(body), v['body_hex'], reason: v['name']);
      final msg = await requestMessage(
        v['method'],
        v['signed_message_ts'],
        v['signed_message_nonce'],
        body,
      );
      expect(utf8.decode(msg), v['signed_message'], reason: v['name']);
    }
  });

  test('Ed25519 signature and fingerprint match the server', () async {
    final e = _json('app_vectors.json')['ed25519'];
    final k = await AppKey.fromSeed(bytesOfHex(e['seed_hex']));
    expect(k.publicHex, e['public_hex']);
    expect(k.fingerprint, e['fingerprint']);
    final body = (TriggerRunsRequest()..machines.add('WS-01')).canon();
    expect(hexOf(body), e['body_hex']);
    final md = await k.sign(
      e['method'],
      body,
      tsMs: e['ts'],
      nonce: e['nonce'],
    );
    expect(md['x-itcmdr-sig'], e['sig_hex']);
    expect(md['x-itcmdr-key'], e['fingerprint']);
    expect(md['x-itcmdr-ts'], '${e['ts']}');
  });

  test('sealed restore words match the server (it opens them)', () async {
    final s = _json('app_vectors.json')['sealed_words'];
    final out = await sealWords(
      serverPublicHex: s['server_public_hex'],
      keyId: s['key_id'],
      backup: s['backup'],
      words: s['words'],
      ephSeed: bytesOfHex(s['eph_secret_hex']),
      nonce: bytesOfHex(s['nonce_hex']),
    );
    expect(out.eph, s['eph']);
    expect(out.ct, s['ct']);
    expect(out.nonce, s['nonce_hex']);
    // Random by default: two seals differ.
    final a = await sealWords(
      serverPublicHex: s['server_public_hex'],
      keyId: 'k',
      backup: 'b',
      words: 'w',
    );
    final b = await sealWords(
      serverPublicHex: s['server_public_hex'],
      keyId: 'k',
      backup: 'b',
      words: 'w',
    );
    expect(a.ct == b.ct, isFalse);
  });

  test('a fresh key is random and its seed round-trips', () async {
    final a = await AppKey.generate();
    final b = await AppKey.fromSeed(a.seed);
    expect(b.fingerprint, a.fingerprint);
    expect((await AppKey.generate()).fingerprint == a.fingerprint, isFalse);
    expect(() => AppKey.fromSeed([1, 2, 3]), throwsFormatException);
  });
}
