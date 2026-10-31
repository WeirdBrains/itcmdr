// Drives a real itcmdr-server: init, certify the app key, then WhoAmI, schedules,
// backup and a restore dry run. Needs `cargo build -p itcmdr-server --bins` first.
import 'dart:convert';
import 'dart:io';

import 'package:app/backup_page.dart';
import 'package:grpc/grpc.dart';
import 'package:app/proto/itcmdr.pbgrpc.dart';
import 'package:app/server_settings.dart';
import 'package:app/settings.dart';
import 'package:app/signing.dart';
import 'package:flutter_test/flutter_test.dart';

import 'settings_doc.dart';

void main() {
  test('signed app against a local server', () async {
    final root = Directory.systemTemp.createTempSync('itcmdr-e2e-');
    addTearDown(() {
      if (root.existsSync()) root.deleteSync(recursive: true);
    });
    final logs = Directory('${root.path}/logs')..createSync();
    final data = Directory('${root.path}/data')..createSync();
    final sock = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    final port = sock.port;
    await sock.close();
    final cfg = File('${root.path}/server.json');
    cfg.writeAsStringSync(
      jsonEncode({
        'bind_addr': '127.0.0.1:$port',
        'logs_dir': logs.path,
        'data_dir': data.path,
        'backup': {'nightly': false},
      }),
    );
    final env = {'ITCMDR_SERVER_CONFIG': cfg.path};
    final bin = _bins();
    final words = _words(await _run(bin.cli, ['init'], env));

    final key = await AppKey.generate();
    final bundle = File('${root.path}/app-key.pub.json');
    await _run(
      bin.cli,
      [
        'keys',
        'certify',
        '--kind',
        'user',
        '--name',
        'Front desk',
        '--role',
        'owner',
        '--public-key',
        key.publicHex,
        '--out',
        bundle.path,
      ],
      env,
      stdinText: words,
    );
    await _run(bin.cli, ['keys', 'register', bundle.path], env);

    final server = await Process.start(bin.server, const [], environment: env);
    final serverLog = StringBuffer();
    server.stdout.transform(utf8.decoder).listen(serverLog.write);
    server.stderr.transform(utf8.decoder).listen(serverLog.write);
    addTearDown(() async {
      server.kill(ProcessSignal.sigterm);
      await server.exitCode.timeout(
        const Duration(seconds: 5),
        onTimeout: () {
          server.kill(ProcessSignal.sigkill);
          return -1;
        },
      );
    });
    try {
      await _waitPort(port);
    } catch (e) {
      fail('$e\n$serverLog');
    }

    final settings = ServerSettings(
      host: '127.0.0.1',
      port: port,
      keySeed: hexOf(key.seed),
    );
    final ch = settings.channel();
    addTearDown(ch.shutdown);
    final client = settings.client(ch, key);

    final me = await client.whoAmI(WhoAmIRequest());
    expect(me.signed, isTrue);
    expect(me.name, 'Front desk');
    expect(me.role, 'owner');
    expect(me.kind, 'user');
    expect(me.authMode, 'signed');
    expect(me.fingerprint, key.fingerprint);

    final listed = await client.listSettings(ListSettingsRequest());
    final doc = settingsFromReference();
    expect(
      listed.settings.map((s) => s.key).toSet(),
      doc.map((s) => s.key).toSet(),
    );
    for (final s in listed.settings) {
      if (s.readOnly) {
        expect(validateSettingValue(s, s.value), isNotNull, reason: s.key);
      } else {
        expect(
          validateSettingValue(s, s.value),
          isNull,
          reason: '${s.key}=${s.value} (${s.type})',
        );
      }
      if (s.type == 'bool') expect(s.value, isIn(['true', 'false']));
    }
    await client.updateSetting(
      UpdateSettingRequest(key: 'plugins.enable_on_install', value: 'false'),
    );
    final flipped = await client.listSettings(ListSettingsRequest());
    final b = flipped.settings.firstWhere(
      (s) => s.key == 'plugins.enable_on_install',
    );
    expect(b.value, 'false');
    expect(b.source.startsWith('db'), isTrue);

    final preview = await client.previewSchedule(
      SaveScheduleRequest(
        schedule: Schedule()
          ..name = 'Evening'
          ..enabled = true
          ..allMachines = true
          ..when = 'daily 18:30'
          ..timezone = 'local',
      ),
    );
    expect(preview.error, isEmpty, reason: preview.error);
    expect(preview.nextRuns, isNotEmpty);
    await client.createSchedule(
      SaveScheduleRequest(
        schedule: Schedule()
          ..name = 'Evening'
          ..enabled = true
          ..allMachines = true
          ..when = 'daily 18:30'
          ..timezone = 'local'
          ..guard = 'report-only-in-hours'
          ..missed = 'skip',
      ),
    );
    final schedules = await client.listSchedules(ListSchedulesRequest());
    expect(schedules.schedules.map((s) => s.name), contains('Evening'));

    final made = await client.createBackup(CreateBackupRequest());
    expect(made.hasBackup(), isTrue);
    final name = made.backup.name;
    expect(name, isNotEmpty);
    final dry = await client.stageRestore(
      StageRestoreRequest(
        backup: name,
        sealedWords: await sealFor(client, name, words),
        dryRun: true,
      ),
    );
    expect(dry.dryRunId, isNotEmpty);
    expect(dry.message, contains('dry run passed'));

    // v0.13.0 plugins from the app: an upload goes through the same validation; junk is
    // quarantined. Plain-http URLs to other hosts are refused before any download.
    final up = await client.installPlugin(
      InstallPluginRequest(
        package: utf8.encode('not a package'),
        fileName: 'junk.tar.gz',
      ),
    );
    expect(up.quarantined, isTrue);
    final q = await client.listQuarantine(ListQuarantineRequest());
    expect(q.entries.single.source, 'upload:junk.tar.gz');
    await expectLater(
      client.installPlugin(
        InstallPluginRequest(url: 'http://plugins.example.local/p.tar.gz'),
      ),
      throwsA(
        isA<GrpcError>().having(
          (e) => e.code,
          'code',
          StatusCode.invalidArgument,
        ),
      ),
    );
  }, timeout: const Timeout(Duration(minutes: 3)));
}

({String server, String cli}) _bins() {
  // ITCMDR_E2E_BIN_DIR: e.g. target/release on the Windows VM.
  final dir = Directory(
    Platform.environment['ITCMDR_E2E_BIN_DIR'] ?? '../target/debug',
  );
  final ext = Platform.isWindows ? '.exe' : '';
  final server = File('${dir.path}/itcmdr-server$ext');
  final cli = File('${dir.path}/itcmdr-cli$ext');
  if (!server.existsSync() || !cli.existsSync()) {
    fail('build the server first: cargo build -p itcmdr-server --bins');
  }
  return (server: server.path, cli: cli.path);
}

Future<ProcessResult> _run(
  String bin,
  List<String> args,
  Map<String, String> env, {
  String? stdinText,
}) async {
  final p = await Process.start(bin, args, environment: env);
  if (stdinText != null) {
    p.stdin.writeln(stdinText);
  }
  await p.stdin.close();
  final out = await p.stdout.transform(utf8.decoder).join();
  final err = await p.stderr.transform(utf8.decoder).join();
  final code = await p.exitCode;
  if (code != 0) {
    fail('$bin ${args.join(' ')} exited $code\n$out\n$err');
  }
  return ProcessResult(p.pid, code, out, err);
}

String _words(ProcessResult r) {
  final words = RegExp(
    r'^\s+\d+\.\s+(\S+)\s*$',
    multiLine: true,
  ).allMatches(r.stdout as String).map((m) => m.group(1)!).join(' ');
  expect(words.split(' '), hasLength(24), reason: r.stdout as String);
  return words;
}

Future<void> _waitPort(int port) async {
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  Object? last;
  while (DateTime.now().isBefore(deadline)) {
    try {
      final s = await Socket.connect(
        '127.0.0.1',
        port,
        timeout: const Duration(milliseconds: 200),
      );
      await s.close();
      return;
    } catch (e) {
      last = e;
      await Future<void>.delayed(const Duration(milliseconds: 200));
    }
  }
  throw StateError('server did not listen on $port ($last)');
}
