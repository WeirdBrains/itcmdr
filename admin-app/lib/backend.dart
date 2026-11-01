// The app's view of adminsvr. RustBackend calls the bridge (itcmdr-admin-cli's signed client);
// tests use a fake. Every call is signed with this device's key; step-up actions ask for a
// security key touch inside the call.
import 'src/rust/api/admin.dart' as api;
export 'src/rust/api/admin.dart'
    show
        Me,
        Enrolled,
        User,
        Device,
        Site,
        SiteStatus,
        Machine,
        Sample,
        Alert,
        CommandRow,
        AuditRow,
        Policy,
        Passkey,
        PushResult;

abstract class Backend {
  Future<api.Me?> open(String dir);
  Future<api.Enrolled> enroll(String dir, String code, bool keyBackup);
  Future<void> forget(String dir);
  Future<api.Me> me();
  Future<List<api.User>> users();
  Future<String> createUser(String email, String name, String role);
  Future<api.User> setRole(String user, String role);
  Future<api.User> setDisabled(String user, bool disabled);
  Future<api.User> setScopes(String user, List<String> sites);
  Future<api.User> approveDevice(String user, String fingerprint);
  Future<List<api.Device>> revokeDevice(String user, String fingerprint);
  Future<String> newEnrollCode(String user);
  Future<List<api.Site>> sites();
  Future<String> createSite(String name);
  Future<void> revokeSite(String site);
  Future<List<api.Sample>> metrics(String site, int hours);
  Future<List<api.CommandRow>> commands(String site);
  Future<api.CommandRow> startBackup(String site);
  Future<List<api.Alert>> alerts(String site, String state);
  Future<api.CommandRow> ackAlert(String site, String alert);
  Future<api.CommandRow> resolveAlert(String site, String alert);
  Future<List<api.AuditRow>> audit(int limit);
  Future<String> verifyAudit();
  Future<api.Policy> policy();
  Future<api.Policy> setPolicy(api.Policy p);
  Future<List<api.Passkey>> passkeys(String user);
  Future<int> addPasskey(String label);
  Future<int> removePasskey(String user, String credentialId);
  Future<void> registerPush(String provider, String token, String platform);
  Future<api.PushResult> testPush();
}

class RustBackend implements Backend {
  @override
  Future<api.Me?> open(String dir) => api.open(dir: dir);
  @override
  Future<api.Enrolled> enroll(String dir, String code, bool keyBackup) =>
      api.enroll(dir: dir, code: code, keyBackup: keyBackup);
  @override
  Future<void> forget(String dir) => api.forget(dir: dir);
  @override
  Future<api.Me> me() => api.me();
  @override
  Future<List<api.User>> users() => api.users();
  @override
  Future<String> createUser(String email, String name, String role) =>
      api.createUser(email: email, name: name, role: role);
  @override
  Future<api.User> setRole(String user, String role) =>
      api.setRole(user: user, role: role);
  @override
  Future<api.User> setDisabled(String user, bool disabled) =>
      api.setDisabled(user: user, disabled: disabled);
  @override
  Future<api.User> setScopes(String user, List<String> sites) =>
      api.setScopes(user: user, sites: sites);
  @override
  Future<api.User> approveDevice(String user, String fingerprint) =>
      api.approveDevice(user: user, fingerprint: fingerprint);
  @override
  Future<List<api.Device>> revokeDevice(String user, String fingerprint) =>
      api.revokeDevice(user: user, fingerprint: fingerprint);
  @override
  Future<String> newEnrollCode(String user) => api.newEnrollCode(user: user);
  @override
  Future<List<api.Site>> sites() => api.sites();
  @override
  Future<String> createSite(String name) => api.createSite(name: name);
  @override
  Future<void> revokeSite(String site) => api.revokeSite(site: site);
  @override
  Future<List<api.Sample>> metrics(String site, int hours) =>
      api.metrics(site: site, hours: hours);
  @override
  Future<List<api.CommandRow>> commands(String site) =>
      api.commands(site: site);
  @override
  Future<api.CommandRow> startBackup(String site) =>
      api.startBackup(site: site);
  @override
  Future<List<api.Alert>> alerts(String site, String state) =>
      api.alerts(site: site, state: state);
  @override
  Future<api.CommandRow> ackAlert(String site, String alert) =>
      api.ackAlert(site: site, alert: alert);
  @override
  Future<api.CommandRow> resolveAlert(String site, String alert) =>
      api.resolveAlert(site: site, alert: alert);
  @override
  Future<List<api.AuditRow>> audit(int limit) => api.audit(limit: limit);
  @override
  Future<String> verifyAudit() => api.verifyAudit();
  @override
  Future<api.Policy> policy() => api.policy();
  @override
  Future<api.Policy> setPolicy(api.Policy p) => api.setPolicy(p: p);
  @override
  Future<List<api.Passkey>> passkeys(String user) => api.passkeys(user: user);
  @override
  Future<int> addPasskey(String label) => api.addPasskey(label: label);
  @override
  Future<int> removePasskey(String user, String credentialId) =>
      api.removePasskey(user: user, credentialId: credentialId);
  @override
  Future<void> registerPush(String provider, String token, String platform) =>
      api.registerPush(provider: provider, token: token, platform: platform);
  @override
  Future<api.PushResult> testPush() => api.testPush();
}

/// AdminApi method names of each app action (for the step-up list).
class Act {
  static const setRole = 'SetRole';
  static const setUserStatus = 'SetUserStatus';
  static const createUser = 'CreateUser';
  static const newEnrollCode = 'NewEnrollCode';
  static const approveDevice = 'ApproveDevice';
  static const revokeDevice = 'RevokeDevice';
  static const createSite = 'CreateSite';
  static const revokeSite = 'RevokeSite';
  static const removePasskey = 'RemovePasskey';
  static const addPasskey = 'FinishPasskeyRegistration';
  static const setAuthPolicy = 'SetAuthPolicy';
  static const setScopes = 'SetScopes';
  static const ackAlert = 'AckAlert';
  static const resolveAlert = 'ResolveAlert';
  static const startBackup = 'StartBackup';
}

String roleLabel(String r) => switch (r) {
  'owner' => 'Owner',
  'admin' => 'Admin',
  'sre' => 'SRE',
  _ => r,
};

int roleRank(String r) => switch (r) {
  'owner' => 3,
  'admin' => 2,
  'sre' => 1,
  _ => 0,
};
