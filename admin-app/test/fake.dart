import 'dart:async';

import 'package:itcmdr_admin/backend.dart';

Me me0({String role = 'owner', int passkeys = 0, bool authenticator = false}) =>
    Me(
      id: 'u1',
      email: 'owner@example.com',
      name: 'Owner O',
      role: role,
      server: 'adminsvr.example:8444',
      passkeys: passkeys,
      locked: false,
      authenticator: authenticator,
      keyBackup: true,
    );

User user(
  String id,
  String email,
  String role, {
  List<Device> devices = const [],
  List<String> sites = const [],
}) => User(
  id: id,
  email: email,
  name: email.split('@').first,
  role: role,
  status: 'active',
  sites: sites,
  keyBackup: false,
  passkeys: 0,
  createdMs: 1,
  devices: devices,
);

Alert alert(
  String id,
  String sev, {
  String state = 'open',
  String site = 's1',
}) => Alert(
  site: site,
  id: id,
  code: 'agent:m-1:disk-low',
  severity: sev,
  state: state,
  source: 'agent',
  plugin: '',
  machine: 'm-1',
  machineDisplayName: '',
  firstSeenMs: 1,
  lastSeenMs: DateTime.now().millisecondsSinceEpoch,
  count: 2,
  origin: 'site',
);

Site site(
  String id,
  String name,
  String version, {
  List<Machine> machines = const [],
}) => Site(
  id: id,
  name: name,
  status: 'online',
  version: version,
  lastSeenMs: DateTime.now().millisecondsSinceEpoch,
  shareDisplayNames: false,
  lastStatus: SiteStatus(
    version: version,
    uptimeSecs: 7200,
    authMode: 'signed',
    cutoverState: 'done',
    backupAgeSecs: 3600,
    backupSizeBytes: 1048576,
    backupLastResult: 'ok',
    agentsOnline: 3,
    agentsLate: 1,
    agentsNonResponsive: 0,
    agentsOther: 0,
    alertsCritical: 1,
    alertsCautious: 2,
    alertsInformational: 0,
    machines: machines,
    diskFreeBytes: 50 * 1073741824,
    diskTotalBytes: 200 * 1073741824,
    runs24H: 12,
    runsFailed24H: 1,
    runSecsAvg24H: 40,
    runSecsMax24H: 95,
    pluginErrors24H: 2,
    alertsOpened24H: 3,
    deepCheckText: 'last deep check: never',
    deepCheckWarning: 'deep backup check never run',
  ),
);

CommandRow cmd(String kind) => CommandRow(
  id: 'c1',
  site: 's1',
  kind: kind,
  state: 'pending',
  resultCode: '',
  requestedBy: 'u1',
  createdMs: 1,
  dryRun: false,
  preview: const [],
  approvedBy: '',
  args: const [],
);

class FakeBackend implements Backend {
  FakeBackend({Me? me}) : current = me;
  Me? current;
  final calls = <String>[];
  List<Alert> alertList = [];
  List<Site> siteList = [];
  List<User> userList = [];
  Policy pol = const Policy(
    stepupActions: ['SetRole', 'AckAlert', 'ApproveDevice'],
    passkeyRequiredRoles: [],
    requireUserVerification: false,
  );

  /// When set, calls wait for it (to see the touch dialog).
  Completer<void>? gate;
  Object? enrollError;

  Future<void> _wait() async {
    final g = gate;
    if (g != null) await g.future;
  }

  @override
  Future<Me?> open(String dir) async => current;
  @override
  Future<Enrolled> enroll(String dir, String code, bool keyBackup) async {
    calls.add('enroll $code $keyBackup');
    if (enrollError != null) throw enrollError!;
    current = me0();
    return Enrolled(
      words: List.generate(24, (i) => 'w$i').join(' '),
      me: current!,
    );
  }

  @override
  Future<void> forget(String dir) async {
    calls.add('forget');
    current = null;
  }

  @override
  Future<Me> me() async => current!;
  @override
  Future<List<User>> users() async => userList;
  @override
  Future<String> createUser(String email, String name, String role) async {
    calls.add('createUser $email $role');
    return 'itcmdr-admin:code';
  }

  @override
  Future<User> setRole(String u, String role) async {
    await _wait();
    calls.add('setRole $u $role');
    final x = userList.firstWhere((e) => e.id == u);
    return user(x.id, x.email, role, devices: x.devices);
  }

  @override
  Future<User> setDisabled(String u, bool disabled) async {
    calls.add('setDisabled $u $disabled');
    return userList.firstWhere((e) => e.id == u);
  }

  @override
  Future<User> setScopes(String u, List<String> sites) async {
    calls.add('setScopes $u ${sites.join(",")}');
    final x = userList.firstWhere((e) => e.id == u);
    return user(x.id, x.email, x.role, sites: sites, devices: x.devices);
  }

  @override
  Future<User> approveDevice(String u, String fp) async {
    await _wait();
    calls.add('approve $u $fp');
    final x = userList.firstWhere((e) => e.id == u);
    return user(
      x.id,
      x.email,
      x.role,
      devices: [
        for (final d in x.devices)
          Device(
            fingerprint: d.fingerprint,
            platform: d.platform,
            status: 'active',
            createdMs: 1,
            lastUsedMs: 0,
          ),
      ],
    );
  }

  @override
  Future<List<Device>> revokeDevice(String u, String fp) async => [];
  @override
  Future<String> newEnrollCode(String u) async => 'itcmdr-admin:new';
  @override
  Future<List<Site>> sites() async => siteList;
  @override
  Future<String> createSite(String name) async {
    calls.add('createSite $name');
    return 'site-token';
  }

  @override
  Future<void> revokeSite(String s) async => calls.add('revokeSite $s');
  @override
  Future<List<Sample>> metrics(String s, int hours) async => [
    for (var i = 0; i < 5; i++)
      Sample(
        tsMs: 1000 * i,
        agentsOnline: 3,
        agentsLate: i % 2,
        agentsNonResponsive: 0,
        agentsOther: 0,
        alertsCritical: 1,
        alertsCautious: i,
        alertsInformational: 0,
        backupAgeSecs: 3600,
        uptimeSecs: 100,
      ),
  ];
  @override
  Future<List<CommandRow>> commands(String s) async => [cmd('backup.start')];
  @override
  Future<CommandRow> startBackup(String s) async {
    calls.add('backup $s');
    return cmd('backup.start');
  }

  @override
  Future<List<Alert>> alerts(String s, String state) async => alertList
      .where(
        (a) =>
            (s.isEmpty || a.site == s) &&
            (state == 'all' || (state == 'open') == (a.state != 'resolved')),
      )
      .toList();
  @override
  Future<CommandRow> ackAlert(String s, String a) async {
    await _wait();
    calls.add('ack $s $a');
    return cmd('alert.ack');
  }

  @override
  Future<CommandRow> resolveAlert(String s, String a) async {
    calls.add('resolve $s $a');
    return cmd('alert.resolve');
  }

  @override
  Future<List<AuditRow>> audit(int limit) async => [
    const AuditRow(
      seq: 1,
      tsMs: 1,
      actor: 'u1',
      action: 'site.create',
      target: 's1',
    ),
  ];
  @override
  Future<String> verifyAudit() async => 'ok (1 entries)';
  @override
  Future<Policy> policy() async => pol;
  @override
  Future<Policy> setPolicy(Policy p) async {
    calls.add('setPolicy ${p.stepupActions.join(",")}');
    pol = p;
    return p;
  }

  @override
  Future<List<Passkey>> passkeys(String u) async => [];
  @override
  Future<int> addPasskey(String label) async => 1;
  @override
  Future<int> removePasskey(String u, String c) async => 0;
  @override
  Future<void> registerPush(
    String provider,
    String token,
    String platform,
  ) async => calls.add('registerPush $provider $token');
  @override
  Future<PushResult> testPush() async {
    calls.add('testPush');
    return const PushResult(devices: 0, sent: 0, failed: 0, providers: []);
  }

  // ---- v0.20.0
  Freeze fz = const Freeze(frozen: false, by: '', sinceMs: 0);
  List<CatalogEntry> cat = [];
  List<CommandRow> pending = [];
  final sent = <CommandRow>[];
  SiteConfig cfg = SiteConfig(
    receivedMs: DateTime.now().millisecondsSinceEpoch,
    settings: const [
      Setting(
        key: 'backup.keep_daily',
        kind: 'int',
        value: '7',
        hidden: false,
        remote: true,
        source: 'default',
        scope: 'server',
        min: 1,
        max: 365,
      ),
      Setting(
        key: 'backup.dir',
        kind: 'path',
        value: '',
        hidden: true,
        remote: false,
        source: 'db',
        scope: 'server',
        min: 0,
        max: 0,
      ),
    ],
    schedules: const [
      Schedule(
        id: 's-1',
        name: '',
        enabled: true,
        when: 'daily 03:00',
        timezone: 'UTC',
        action: 'backup',
        builtin: 'nightly-backup',
        allMachines: true,
        machines: 0,
        lastFireMs: 0,
      ),
    ],
    requireTwoPerson: true,
    allowed: const [
      'setting.set',
      'schedule.run',
      'schedule.pause',
      'schedule.resume',
      'plugin.install',
      'plugin.enable',
      'plugin.disable',
    ],
  );
  SitePlugins plugins = SitePlugins(
    receivedMs: 1,
    installed: const [
      InstalledPlugin(
        name: 'disk-watch',
        version: '1.2.0',
        enabled: true,
        sha256: 'aa',
      ),
    ],
    quarantine: [
      Quarantined(sha256: 'b' * 64, reason: 'bad-signature', tsMs: 1),
    ],
  );

  /// The site's answer to a command (by kind); default: applied / previewed with one change.
  String Function(String kind, bool dryRun)? siteAnswer;

  CommandRow _row(
    String id,
    String site,
    String kind,
    List<String> args,
    bool dry,
    String state,
    String code,
  ) => CommandRow(
    id: id,
    site: site,
    kind: kind,
    state: state,
    resultCode: code,
    requestedBy: 'u1',
    createdMs: 1,
    dryRun: dry,
    preview: dry || state == 'applied'
        ? const [Change(target: 'backup.keep_daily', before: '7', after: '9')]
        : const [],
    approvedBy: '',
    args: args,
  );

  @override
  Future<SiteConfig> siteConfig(String site) async => cfg;
  @override
  Future<SitePlugins> sitePlugins(String site) async => plugins;
  @override
  Future<CommandRow> sendCommand(
    String site,
    String kind,
    List<String> args,
    bool dryRun,
  ) async {
    calls.add('send $kind ${args.join(',')}${dryRun ? ' dry' : ''}');
    if (fz.frozen) throw Exception('adminsvr is frozen');
    final id = 'c${sent.length + 1}';
    final two = !dryRun && kind.startsWith('plugin.');
    final r = _row(
      id,
      site,
      kind,
      args,
      dryRun,
      two ? 'awaiting-approval' : 'pending',
      '',
    );
    sent.add(r);
    if (two) pending.add(r);
    return r;
  }

  @override
  Future<CommandRow> getCommand(String id) async {
    final r = sent.firstWhere((x) => x.id == id);
    final ans = siteAnswer?.call(r.kind, r.dryRun);
    final state = ans ?? (r.dryRun ? 'previewed' : 'applied');
    return _row(
      r.id,
      r.site,
      r.kind,
      r.args,
      r.dryRun,
      state,
      state == 'failed' ? 'not-allowed' : 'ok',
    );
  }

  @override
  Future<CommandRow> approveCommand(String id) async {
    calls.add('approve $id');
    if (fz.frozen) throw Exception('adminsvr is frozen');
    pending.removeWhere((x) => x.id == id);
    return cmd('x');
  }

  @override
  Future<CommandRow> rejectCommand(String id) async {
    calls.add('reject $id');
    pending.removeWhere((x) => x.id == id);
    return cmd('x');
  }

  @override
  Future<List<CommandRow>> pendingApprovals() async => List.of(pending);
  @override
  Future<List<CatalogEntry>> catalog() async => cat;
  @override
  Future<CatalogEntry> uploadPlugin(String path) async {
    calls.add('upload $path');
    final e = CatalogEntry(
      sha256: 'c' * 64,
      name: 'net-watch',
      version: '2.0.0',
      size: 4096,
      uploadedBy: 'me',
      uploadedMs: 1,
      description: '',
    );
    cat = [...cat, e];
    return e;
  }

  @override
  Future<Freeze> freeze() async => fz;
  @override
  Future<Freeze> setFreeze(bool frozen) async {
    calls.add('freeze $frozen');
    fz = Freeze(frozen: frozen, by: 'owner@example.com', sinceMs: 1);
    return fz;
  }

  // v0.21.0
  OrgPolicy org = OrgPolicy(
    requireSecondApprover: false,
    updatedBy: '',
    updatedMs: 0,
    deepCheckMaxDays: 90,
  );
  List<Release> rels = [];
  @override
  Future<OrgPolicy> orgPolicy() async => org;
  @override
  Future<OrgPolicy> setOrgPolicy(bool requireSecondApprover) async {
    calls.add('org-policy $requireSecondApprover');
    org = OrgPolicy(
      requireSecondApprover: requireSecondApprover,
      updatedBy: 'owner@example.com',
      updatedMs: 1,
      deepCheckMaxDays: org.deepCheckMaxDays,
    );
    return org;
  }

  @override
  Future<List<Release>> releases() async => List.of(rels);
  @override
  Future<Release> approveRelease(String sha256) async {
    calls.add('approve-release $sha256');
    final need = org.requireSecondApprover ? 2 : 1;
    rels = [
      for (final r in rels)
        if (r.sha256 == sha256)
          Release(
            sha256: r.sha256,
            name: r.name,
            version: r.version,
            approvedBy: [...r.approvedBy, 'me'],
            needed: need,
            approved: r.approvedBy.length + 1 >= need,
          )
        else
          r,
    ];
    return rels.firstWhere((r) => r.sha256 == sha256);
  }

  @override
  Future<CommandRow> rolloutRelease(
    String site,
    String sha256,
    bool push,
  ) async {
    calls.add('rollout $site $sha256 $push');
    return cmd('x');
  }
}
