import 'dart:async';
import 'dart:io';

import 'package:clock/clock.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:local_auth/local_auth.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/remote_host.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/security/app_lock.dart';
import 'package:orrery_remote/security/device_auth.dart';
import 'package:orrery_remote/security/privacy_screen.dart';

import 'support/fakes.dart';
import 'support/lock_fakes.dart';

T settle<T>(FakeAsync async, Future<T> future) {
  T? value;
  Object? error;
  var done = false;
  future.then(
    (v) {
      value = v;
      done = true;
    },
    onError: (Object e) {
      error = e;
      done = true;
    },
  );
  async.flushMicrotasks();
  if (!done) throw StateError('future still pending');
  if (error != null) throw error!;
  return value as T;
}

void main() {
  test('home pressed during a settings prompt locks in every-open mode', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      settle(async, lock.apply(timeout: LockTimeout.everyOpen));
      settle(async, lock.unlock());
      auth.hold = Completer<AuthOutcome>();
      final confirm = lock.confirm();
      lock.appPaused();
      expect(lock.locked, isFalse);
      auth.hold!.complete(AuthOutcome.cancelled);
      settle(async, confirm);
      expect(lock.locked, isTrue);
      lock.dispose();
    });
  });

  test('home pressed during a prompt that then succeeds still locks', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      settle(async, lock.apply(timeout: LockTimeout.everyOpen));
      settle(async, lock.unlock());
      auth.hold = Completer<AuthOutcome>();
      final confirm = lock.confirm();
      lock.appPaused();
      lock.appResumed();
      auth.hold!.complete(AuthOutcome.success);
      settle(async, confirm);
      expect(lock.locked, isTrue);
      lock.dispose();
    });
  });

  test('a pause during a prompt in 1 hour mode stays open once back', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      settle(async, lock.unlock());
      auth.hold = Completer<AuthOutcome>();
      final confirm = lock.confirm();
      lock.appPaused();
      lock.appResumed();
      auth.hold!.complete(AuthOutcome.success);
      settle(async, confirm);
      expect(lock.locked, isFalse);
      lock.dispose();
    });
  });

  test('a prompt that ends while the app is still in the background locks', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      settle(async, lock.unlock());
      auth.hold = Completer<AuthOutcome>();
      final confirm = lock.confirm();
      lock.appPaused();
      auth.hold!.complete(AuthOutcome.success);
      settle(async, confirm);
      expect(lock.locked, isTrue);
      lock.dispose();
    });
  });

  test(
    'device sleep counted by elapsedRealtime locks while timers stood still',
    () {
      fakeAsync((async) {
        var native = 1000000;
        final auth = FakeAuth();
        final lock = makeLock(auth, elapsedRealtime: () async => native);
        settle(async, lock.load());
        settle(async, lock.unlock());
        lock.appPaused();
        native += const Duration(hours: 2).inMilliseconds;
        auth.hold = Completer<AuthOutcome>();
        unawaited(lock.appResumed());
        async.flushMicrotasks();
        expect(lock.locked, isTrue);
        expect(auth.calls, 2);
        lock.dispose();
      });
    },
  );

  test('a native clock that fails falls back to the other two', () {
    fakeAsync((async) {
      final lock = makeLock(
        FakeAuth(),
        elapsedRealtime: () => Future<int?>.error(StateError('channel')),
      );
      settle(async, lock.load());
      settle(async, lock.unlock());
      expect(lock.locked, isFalse);
      async.elapse(const Duration(minutes: 61));
      expect(lock.locked, isTrue);
      lock.dispose();
    });
  });

  test('turning the phone clock back does not extend the window', () async {
    var now = DateTime(2026, 11, 12, 10);
    var native = 5000;
    final auth = FakeAuth();
    final lock = makeLock(auth, elapsedRealtime: () async => native);
    await withClock(Clock(() => now), () async {
      await lock.load();
      await lock.unlock();
      lock.appPaused();
      now = now.subtract(const Duration(hours: 3));
      native += const Duration(minutes: 90).inMilliseconds;
      auth.hold = Completer<AuthOutcome>();
      unawaited(lock.appResumed());
      await Future<void>.delayed(Duration.zero);
      expect(lock.locked, isTrue);
      expect(auth.calls, 2);
    });
    lock.dispose();
  });

  test('turning the phone clock forward locks early, never late', () async {
    var now = DateTime(2026, 11, 12, 10);
    final auth = FakeAuth();
    final lock = makeLock(auth);
    await withClock(Clock(() => now), () async {
      await lock.load();
      await lock.unlock();
      lock.appPaused();
      now = now.add(const Duration(hours: 2));
      auth.hold = Completer<AuthOutcome>();
      unawaited(lock.appResumed());
      await Future<void>.delayed(Duration.zero);
      expect(lock.locked, isTrue);
      expect(auth.calls, 2);
    });
    lock.dispose();
  });

  test(
    'expired reports true once the window passed, before the timer ran',
    () async {
      var now = DateTime(2026, 11, 12, 10);
      final lock = makeLock(FakeAuth());
      await withClock(Clock(() => now), () async {
        await lock.load();
        await lock.unlock();
        expect(lock.expired, isFalse);
        now = now.add(const Duration(hours: 1, minutes: 1));
        expect(lock.expired, isTrue);
      });
      lock.dispose();
    },
  );

  test(
    'pairing while locked saves the machine but opens no connection',
    () async {
      final net = FakeNet();
      net.lan = (_) => FakePairServer();
      final deps = await testDeps(
        net: net,
        machines: [],
        config: const ConnectionConfig(lanBudget: Duration(milliseconds: 20)),
      );
      final host = RemoteHost(
        deps: deps,
        cacheDirectory: Directory.systemTemp.createTempSync('lock_pair'),
        deviceName: 'phone',
      );
      await host.init();
      await host.setLocked(true);
      final machine = await host.pairWithQr(
        FakeCrypto.qrText(
          machine: 'machine-x',
          host: '192.168.1.20',
          port: 4320,
          relay: 'https://relay.example',
          exp: 4000000000,
        ),
      );
      expect(host.machines.all.map((m) => m.id), contains(machine.id));
      final calls = net.calls.length;
      expect(calls, 1);
      expect(host.connections.single.state.status, LinkStatus.idle);
      await Future<void>.delayed(const Duration(milliseconds: 100));
      expect(net.calls.length, calls);
      await host.dispose();
    },
  );

  test(
    'a pair that lands during an unlock-then-lock race ends locked',
    () async {
      final net = FakeNet();
      final host = RemoteHost(
        deps: await testDeps(net: net, machines: [testMachine()]),
        cacheDirectory: Directory.systemTemp.createTempSync('lock_race'),
        deviceName: 'phone',
      );
      await host.init();
      await host.startAll();
      final unlock = host.setLocked(false);
      final lock = host.setLocked(true);
      await Future.wait([unlock, lock]);
      await Future<void>.delayed(const Duration(milliseconds: 50));
      expect(host.locked, isTrue);
      expect(host.connections.single.state.status, LinkStatus.idle);
      await host.dispose();
    },
  );

  group('LocalDeviceAuth', () {
    const channel = MethodChannel(privacyChannelName);

    LocalDeviceAuth auth(bool? secure, Object? throws) {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            if (call.method == 'isDeviceSecure') return secure;
            return null;
          });
      addTearDown(
        () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(channel, null),
      );
      return LocalDeviceAuth(
        call: (_) async {
          if (throws != null) throw throws;
          return true;
        },
      );
    }

    LocalAuthException error(LocalAuthExceptionCode code) =>
        LocalAuthException(code: code);

    test('a PIN-only phone is not treated as having no screen lock', () async {
      TestWidgetsFlutterBinding.ensureInitialized();
      final device = auth(
        true,
        error(LocalAuthExceptionCode.noBiometricsEnrolled),
      );
      expect(await device.authenticate('r'), AuthOutcome.failed);
      expect(await device.isDeviceInsecure(), isFalse);
      final lock = AppLock(
        auth: device,
        kv: MemorySecureKv(),
        reason: () => 'unlock',
      );
      await lock.load();
      expect(await lock.turnOffWithoutAuth(), isFalse);
      expect(lock.enabled, isTrue);
      lock.dispose();
    });

    test('no biometric hardware, or a stray no-credentials error on a secure device, is failed', () async {
      TestWidgetsFlutterBinding.ensureInitialized();
      for (final code in [
        LocalAuthExceptionCode.noBiometricHardware,
        LocalAuthExceptionCode.noCredentialsSet,
      ]) {
        final device = auth(true, error(code));
        expect(await device.authenticate('r'), AuthOutcome.failed);
        expect(await device.isDeviceInsecure(), isFalse);
      }
    });

    test(
      'a phone without any screen lock lets the lock be turned off',
      () async {
        TestWidgetsFlutterBinding.ensureInitialized();
        final device = auth(false, null);
        expect(await device.authenticate('r'), AuthOutcome.notAvailable);
        expect(await device.isDeviceInsecure(), isTrue);
        final lock = AppLock(
          auth: device,
          kv: MemorySecureKv(),
          reason: () => 'unlock',
        );
        await lock.load();
        expect(await lock.turnOffWithoutAuth(), isTrue);
        expect(lock.enabled, isFalse);
        lock.dispose();
      },
    );

    test(
      'cancel codes map to cancelled, success and plugin errors too',
      () async {
        TestWidgetsFlutterBinding.ensureInitialized();
        expect(
          await auth(
            true,
            error(LocalAuthExceptionCode.userCanceled),
          ).authenticate('r'),
          AuthOutcome.cancelled,
        );
        expect(await auth(true, null).authenticate('r'), AuthOutcome.success);
        expect(
          await auth(true, StateError('plugin')).authenticate('r'),
          AuthOutcome.failed,
        );
      },
    );
  });
}
