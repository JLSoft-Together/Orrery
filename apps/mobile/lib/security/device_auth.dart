import 'package:local_auth/local_auth.dart';

import 'privacy_screen.dart';

enum AuthOutcome { success, cancelled, notAvailable, failed }

abstract class DeviceAuth {
  Future<AuthOutcome> authenticate(String reason);

  Future<bool> isDeviceInsecure();
}

typedef AuthCall = Future<bool> Function(String reason);

class LocalDeviceAuth implements DeviceAuth {
  LocalDeviceAuth({
    LocalAuthentication? plugin,
    this.privacy = const PrivacyScreen(),
    AuthCall? call,
  }) : _call =
           call ??
           ((reason) => (plugin ?? LocalAuthentication()).authenticate(
             localizedReason: reason,
           ));

  final PrivacyScreen privacy;
  final AuthCall _call;
  bool _noCredentials = false;

  @override
  Future<bool> isDeviceInsecure() async {
    final secure = await privacy.isDeviceSecure();
    if (secure != null) return !secure;
    return _noCredentials;
  }

  @override
  Future<AuthOutcome> authenticate(String reason) async {
    try {
      if (await privacy.isDeviceSecure() == false) {
        _noCredentials = true;
        return AuthOutcome.notAvailable;
      }
      final ok = await _call(reason);
      return ok ? AuthOutcome.success : AuthOutcome.cancelled;
    } on LocalAuthException catch (error) {
      return switch (error.code) {
        LocalAuthExceptionCode.noCredentialsSet => await _noScreenLock(),
        LocalAuthExceptionCode.userCanceled ||
        LocalAuthExceptionCode.systemCanceled ||
        LocalAuthExceptionCode.timeout ||
        LocalAuthExceptionCode.userRequestedFallback => AuthOutcome.cancelled,
        _ => AuthOutcome.failed,
      };
    } on Object {
      return AuthOutcome.failed;
    }
  }

  Future<AuthOutcome> _noScreenLock() async {
    final secure = await privacy.isDeviceSecure();
    if (secure == true) return AuthOutcome.failed;
    _noCredentials = true;
    return AuthOutcome.notAvailable;
  }
}
