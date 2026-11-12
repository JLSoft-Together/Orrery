import 'dart:io';
import 'dart:async';

import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/remote_host.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/security/app_lock.dart';
import 'package:orrery_remote/security/device_auth.dart';

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
  test('first launch is locked until the device verifies', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      expect(lock.enabled, isTrue);
      expect(lock.timeout, LockTimeout.hour1);
      expect(lock.locked, isTrue);
      expect(settle(async, lock.unlock()), AuthOutcome.success);
      expect(lock.locked, isFalse);
      lock.dispose();
    });
  });

  test('cancelled, failed and unavailable verification keep it locked', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      for (final outcome in [
        AuthOutcome.cancelled,
        AuthOutcome.failed,
        AuthOutcome.notAvailable,
      ]) {
        auth.queue.add(outcome);
        expect(settle(async, lock.unlock()), outcome);
        expect(lock.locked, isTrue);
        expect(lock.lastOutcome, outcome);
      }
      lock.dispose();
    });
  });

  test('the unlock window ends after 1 hour even while the app is open', () {
    fakeAsync((async) {
      final lock = makeLock(FakeAuth());
      settle(async, lock.load());
      settle(async, lock.unlock());
      async.elapse(const Duration(minutes: 59));
      expect(lock.locked, isFalse);
      async.elapse(const Duration(minutes: 2));
      expect(lock.locked, isTrue);
      lock.dispose();
    });
  });

  test('coming back after the window locks and asks again', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      settle(async, lock.unlock());
      lock.appPaused();
      async.elapse(const Duration(minutes: 30));
      settle(async, lock.appResumed());
      expect(lock.locked, isFalse);
      expect(auth.calls, 1);
      lock.appPaused();
      async.elapse(const Duration(minutes: 40));
      auth.hold = Completer<AuthOutcome>();
      final resumed = lock.appResumed();
      async.flushMicrotasks();
      expect(lock.locked, isTrue);
      expect(auth.calls, 2);
      auth.hold!.complete(AuthOutcome.cancelled);
      settle(async, resumed);
      expect(lock.locked, isTrue);
      lock.dispose();
    });
  });

  test('every-open timeout locks the moment the app leaves the screen', () {
    fakeAsync((async) {
      final auth = FakeAuth();
      final lock = makeLock(auth);
      settle(async, lock.load());
      expect(settle(async, lock.apply(timeout: LockTimeout.everyOpen)), isTrue);
      settle(async, lock.unlock());
      async.elapse(const Duration(hours: 5));
      expect(lock.locked, isFalse);
      lock.appPaused();
      expect(lock.locked, isTrue);
      settle(async, lock.appResumed());
      expect(lock.locked, isFalse);
      expect(auth.calls, 2);
      lock.dispose();
    });
  });

  test('15 minute and 4 hour timeouts follow the setting', () {
    fakeAsync((async) {
      final lock = makeLock(FakeAuth());
      settle(async, lock.load());
      settle(async, lock.apply(timeout: LockTimeout.minutes15));
      settle(async, lock.unlock());
      async.elapse(const Duration(minutes: 14));
      expect(lock.locked, isFalse);
      async.elapse(const Duration(minutes: 2));
      expect(lock.locked, isTrue);
      settle(async, lock.apply(timeout: LockTimeout.hours4));
      settle(async, lock.unlock());
      async.elapse(const Duration(hours: 3, minutes: 59));
      expect(lock.locked, isFalse);
      async.elapse(const Duration(minutes: 2));
      expect(lock.locked, isTrue);
      lock.dispose();
    });
  });

  test('settings persist and a corrupt store falls back to the defaults', () {
    fakeAsync((async) {
      final kv = MemorySecureKv();
      final first = makeLock(FakeAuth(), kv: kv);
      settle(async, first.load());
      settle(async, first.apply(timeout: LockTimeout.minutes15));
      settle(async, first.apply(enabled: false));
      first.dispose();
      final second = makeLock(FakeAuth(), kv: kv);
      settle(async, second.load());
      expect(second.enabled, isFalse);
      expect(second.timeout, LockTimeout.minutes15);
      expect(second.locked, isFalse);
      second.dispose();
      final broken = makeLock(FakeAuth(), kv: ThrowingKv());
      settle(async, broken.load());
      expect(broken.enabled, isTrue);
      expect(broken.locked, isTrue);
      expect(settle(async, broken.apply(enabled: false)), isFalse);
      expect(broken.enabled, isTrue);
      broken.dispose();
    });
  });

  test('a restarted app is locked again even with a stored setting', () {
    fakeAsync((async) {
      final kv = MemorySecureKv();
      final first = makeLock(FakeAuth(), kv: kv);
      settle(async, first.load());
      settle(async, first.unlock());
      expect(first.locked, isFalse);
      first.dispose();
      final second = makeLock(FakeAuth(), kv: kv);
      settle(async, second.load());
      expect(second.locked, isTrue);
      second.dispose();
    });
  });

  test('turning the lock off opens the app and stops the timer', () {
    fakeAsync((async) {
      final lock = makeLock(FakeAuth());
      settle(async, lock.load());
      settle(async, lock.apply(enabled: false));
      expect(lock.locked, isFalse);
      lock.lock();
      expect(lock.locked, isFalse);
      expect(async.pendingTimers, isEmpty);
      lock.dispose();
    });
  });

  group('RemoteHost gate', () {
    Future<RemoteHost> build(FakeNet net) async {
      final deps = await testDeps(net: net, machines: [testMachine()]);
      return RemoteHost(
        deps: deps,
        cacheDirectory: Directory.systemTemp.createTempSync('lock_cache'),
        deviceName: 'phone',
      );
    }

    Future<void> until(bool Function() ok) async {
      for (var i = 0; i < 200 && !ok(); i++) {
        await Future<void>.delayed(const Duration(milliseconds: 10));
      }
      expect(ok(), isTrue);
    }

    test(
      'nothing connects while locked, everything does after unlock',
      () async {
        final net = FakeNet();
        final servers = <FakeServer>[];
        FakeServer make(Uri _) {
          final server = FakeServer();
          servers.add(server);
          return server;
        }

        net.lan = make;
        net.relay = make;
        final host = await build(net);
        await host.init();
        await host.setLocked(true);
        await host.startAll();
        await host.setForeground(false);
        await host.setForeground(true);
        expect(net.calls, isEmpty);
        expect(host.connections.single.state.status, LinkStatus.idle);
        await host.setLocked(false);
        await until(() => host.connections.single.isReady);
        expect(net.calls, isNotEmpty);
        final calls = net.calls.length;
        await host.setLocked(true);
        expect(host.connections.single.state.status, LinkStatus.idle);
        expect(host.connections.single.session, isNull);
        await Future<void>.delayed(const Duration(milliseconds: 100));
        expect(net.calls.length, calls);
        expect(servers.every((s) => s.closedByClient || s.isClosed), isTrue);
        await host.dispose();
      },
    );
  });
}
