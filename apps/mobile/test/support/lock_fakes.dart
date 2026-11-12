import 'dart:async';

import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/security/app_lock.dart';
import 'package:orrery_remote/security/device_auth.dart';

class FakeAuth implements DeviceAuth {
  final List<AuthOutcome> queue = [];
  AuthOutcome fallback = AuthOutcome.success;
  Completer<AuthOutcome>? hold;
  int calls = 0;
  bool insecure = false;

  @override
  Future<bool> isDeviceInsecure() async => insecure;

  @override
  Future<AuthOutcome> authenticate(String reason) {
    calls++;
    final pending = hold;
    if (pending != null) return pending.future;
    return Future.value(queue.isEmpty ? fallback : queue.removeAt(0));
  }
}

class ThrowingKv implements SecureKv {
  @override
  Future<String?> read(String key) => Future.error(StateError('kv'));

  @override
  Future<void> write(String key, String value) =>
      Future.error(StateError('kv'));

  @override
  Future<void> delete(String key) => Future.error(StateError('kv'));
}

AppLock makeLock(
  FakeAuth auth, {
  SecureKv? kv,
  Future<int?> Function()? elapsedRealtime,
}) => AppLock(
  auth: auth,
  kv: kv ?? MemorySecureKv(),
  reason: () => 'unlock',
  elapsedRealtime: elapsedRealtime,
);
