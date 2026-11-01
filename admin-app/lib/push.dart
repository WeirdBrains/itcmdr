// Push notifications. adminsvr sends FCM (Android) and APNs (iOS/macOS) pushes that carry only
// opaque ids and the severity; the app then loads the alert over the signed API. Getting a token
// needs the platform push SDK and the accounts behind it (a Firebase project and an Apple
// developer team, see docs/ADMINSVR.md), so this build ships without one: NoPush. A build with
// Firebase/APNs configured plugs a PushService in here. Desktops get in-app alert banners instead.
abstract class PushService {
  bool get available;
  String get provider; // fcm | apns
  String get platform;
  String get description;
  Future<String> token();
}

class NoPush implements PushService {
  const NoPush();
  @override
  bool get available => false;
  @override
  String get provider => '';
  @override
  String get platform => '';
  @override
  String get description =>
      'Push notifications need a build with Firebase (Android) or APNs (iOS/macOS) configured. '
      'This build shows new alerts while the app is open (checked every 30 seconds).';
  @override
  Future<String> token() =>
      Future.error(UnsupportedError('push is not configured in this build'));
}
