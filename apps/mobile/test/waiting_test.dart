import 'package:clock/clock.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/push/push_background.dart';
import 'package:orrery_remote/push/push_notice.dart';
import 'package:orrery_remote/push/push_pause.dart';
import 'package:orrery_remote/push/push_service.dart';
import 'package:orrery_remote/push/waiting_sink.dart';
import 'package:orrery_remote/push/waiting_update.dart';
import 'package:orrery_remote/remote/secure_kv.dart';

import 'push_present_test.dart' show FakeNotifier;
import 'support/push_fakes.dart';

class RecordingSink implements WaitingSink {
  final List<(WaitingUpdate, DateTime)> applied = [];
  final List<DateTime?> paused = [];
  int starts = 0;

  @override
  Future<void> start() async => starts++;

  @override
  Future<void> apply(WaitingUpdate update, {required DateTime at}) async =>
      applied.add((update, at));

  @override
  Future<void> setPausedUntil(DateTime? until) async => paused.add(until);

  @override
  Stream<void> get pauseRequests => const Stream<void>.empty();
}

class Setup {
  Setup() {
    service = PushService(
      gateway: gateway,
      kv: kv,
      host: host,
      platform: 'android',
      localeCode: () => 'vi',
      waiting: sink,
    );
  }

  final FakeGateway gateway = FakeGateway();
  final FakeHost host = FakeHost();
  final MemorySecureKv kv = MemorySecureKv();
  final RecordingSink sink = RecordingSink();
  late final PushService service;
}

void main() {
  group('parseWaiting', () {
    test('reads the count and up to two labels', () {
      final u = parseWaiting({
        'w': '3',
        'wl': '["api: migrate","web: lint","extra"]',
      });
      expect(
        u,
        const WaitingUpdate(count: 3, labels: ['api: migrate', 'web: lint']),
      );
    });

    test('accepts newline separated labels', () {
      final u = parseWaiting({'w': '2', 'wl': 'one\ntwo\nthree'});
      expect(u?.labels, ['one', 'two']);
    });

    test('cuts long labels to 24 characters', () {
      final long = 'a' * 40;
      final u = parseWaiting({'w': '1', 'wl': '["$long"]'});
      expect(u?.labels.single.length, 24);
    });

    test('cuts by character, not by UTF-16 unit', () {
      final long = 'ẩ😀' * 20;
      final u = parseWaiting({'w': '1', 'wl': '["$long"]'});
      expect(u?.labels.single.runes.length, 24);
    });

    test('a missing field gives no update', () {
      expect(parseWaiting({'k': 'done'}), isNull);
      expect(parseWaiting({'wl': '["x"]'}), isNull);
    });

    test('a bad count gives no update', () {
      expect(parseWaiting({'w': '-1'}), isNull);
      expect(parseWaiting({'w': 'abc'}), isNull);
      expect(parseWaiting({'w': ''}), isNull);
      expect(parseWaiting({'w': '1.5'}), isNull);
    });

    test('count without labels is fine and zero drops labels', () {
      expect(parseWaiting({'w': '4'}), const WaitingUpdate(count: 4));
      expect(
        parseWaiting({'w': '0', 'wl': '["x"]'}),
        const WaitingUpdate(count: 0),
      );
    });

    test('a huge count is capped', () {
      expect(parseWaiting({'w': '999999'})?.count, waitingCountMax);
    });

    test('clear push means zero waiting', () {
      expect(isClearPush({'k': 'clear'}), isTrue);
      expect(parseWaiting({'k': 'clear'}), const WaitingUpdate(count: 0));
    });
  });

  group('foreground sink', () {
    test(
      'an arrival with w and wl is sent to the sink and still shown',
      () async {
        final s = Setup();
        await s.service.init();
        s.gateway.arrive.add(
          PushArrival({
            'k': 'question',
            'm': 'm1',
            'w': '2',
            'wl': '["a","b"]',
          }, shown: false),
        );
        await settle();
        await settle();
        expect(
          s.sink.applied.single.$1,
          const WaitingUpdate(count: 2, labels: ['a', 'b']),
        );
        expect(s.gateway.shown, hasLength(1));
        expect(s.sink.starts, 1);
      },
    );

    test('an old desktop push without w leaves the sink alone', () async {
      final s = Setup();
      await s.service.init();
      s.gateway.arrive.add(PushArrival({'k': 'done', 'm': 'm1'}, shown: false));
      await settle();
      await settle();
      expect(s.sink.applied, isEmpty);
      expect(s.gateway.shown, hasLength(1));
    });

    test('a clear push updates the sink and shows nothing', () async {
      final s = Setup();
      await s.service.init();
      s.gateway.arrive.add(
        PushArrival({'k': 'clear', 'm': 'm1'}, shown: false),
      );
      await settle();
      await settle();
      expect(s.sink.applied.single.$1.count, 0);
      expect(s.gateway.shown, isEmpty);
    });
  });

  group('background handler', () {
    Future<FakeNotifier> run(
      Map<String, String> data,
      SecureKv kv, {
      DateTime? now,
    }) async {
      final n = FakeNotifier();
      await handleBackgroundPush(
        PushArrival(data, shown: false),
        kv: kv,
        machineKey: (_) async => null,
        notifier: n,
        languageCode: 'vi',
        now: now,
      );
      return n;
    }

    test('a clear push shows nothing', () async {
      final n = await run({'k': 'clear', 'm': 'm1'}, MemorySecureKv());
      expect(n.shown, isEmpty);
    });

    test('a normal push shows when not paused', () async {
      final n = await run({'k': 'done', 'm': 'm1'}, MemorySecureKv());
      expect(n.shown, hasLength(1));
    });

    test(
      'a data-only push is held back while paused and shown after',
      () async {
        final kv = MemorySecureKv();
        final now = DateTime(2026, 11, 26, 10);
        await kv.write(
          pushPausedKey,
          '${now.add(const Duration(minutes: 30)).millisecondsSinceEpoch}',
        );
        expect(
          (await run({'k': 'done', 'm': 'm1'}, kv, now: now)).shown,
          isEmpty,
        );
        final later = now.add(const Duration(minutes: 31));
        expect(
          (await run({'k': 'done', 'm': 'm1'}, kv, now: later)).shown,
          hasLength(1),
        );
      },
    );
  });

  group('pause', () {
    test('pauseOneHour mutes every group, sends pauseUntil and mirrors to the sink', () async {
      final now = DateTime(2026, 11, 26, 10);
      await withClock(Clock.fixed(now), () async {
        final s = Setup();
        final session = FakeSession('m1');
        await s.service.init();
        s.host.connect(session);
        s.host.config.add(const PushConfigUpdate('m1', pushConfigA));
        await settle();
        expect(await s.service.setEnabled(true), PushResult.on);
        await s.service.pauseOneHour();
        await settle();
        final until = now.add(const Duration(hours: 1));
        expect(s.service.pausedUntil, until);
        expect(s.sink.paused.last, until);
        expect(s.kv.data[pushPausedKey], '${until.millisecondsSinceEpoch}');
        expect(session.sent.last.$2['mute'], ['waiting', 'done', 'error']);
        expect(
          session.sent.last.$2['pauseUntil'],
          until.millisecondsSinceEpoch,
        );
        s.service.dispose();
      });
    });

    test('resumeNow restores the saved mute and drops pauseUntil', () async {
      final s = Setup();
      final session = FakeSession('m1');
      await s.service.init();
      s.host.connect(session);
      s.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      await settle();
      await s.service.setEnabled(true);
      await s.service.pauseOneHour();
      await s.service.resumeNow();
      await settle();
      expect(s.service.pausedUntil, isNull);
      expect(s.kv.data[pushPausedKey], isNull);
      expect(session.sent.last.$2['mute'], ['error']);
      expect(session.sent.last.$2.containsKey('pauseUntil'), isFalse);
      expect(s.sink.paused.last, isNull);
    });

    test('a pause that ran out is dropped when the app loads', () async {
      final s = Setup();
      await s.kv.write(
        pushPausedKey,
        '${DateTime.now().subtract(const Duration(minutes: 1)).millisecondsSinceEpoch}',
      );
      await s.service.init();
      expect(s.service.pausedUntil, isNull);
      expect(s.kv.data[pushPausedKey], isNull);
      expect(s.sink.paused.last, isNull);
    });

    test(
      'a live pause survives a restart and blocks foreground notices',
      () async {
        final s = Setup();
        final until = DateTime.now().add(const Duration(minutes: 20));
        await s.kv.write(pushPausedKey, '${until.millisecondsSinceEpoch}');
        await s.service.init();
        expect(s.service.pausedUntil, isNotNull);
        s.gateway.arrive.add(
          PushArrival({'k': 'done', 'm': 'm1', 'w': '1'}, shown: false),
        );
        await settle();
        await settle();
        expect(s.gateway.shown, isEmpty);
        expect(s.sink.applied, hasLength(1));
        s.service.dispose();
      },
    );

    test('the pause ends by itself after one hour', () {
      fakeAsync((async) {
        final s = Setup();
        s.service.init();
        async.flushMicrotasks();
        s.service.pauseOneHour();
        async.flushMicrotasks();
        expect(s.service.pausedUntil, isNotNull);
        async.elapse(const Duration(hours: 1, seconds: 1));
        async.flushMicrotasks();
        expect(s.service.pausedUntil, isNull);
        expect(s.kv.data[pushPausedKey], isNull);
      });
    });
  });

  group('channel sink', () {
    test('the pause stream can be listened to again after a cancel', () async {
      TestWidgetsFlutterBinding.ensureInitialized();
      const channel = MethodChannel('dev.ccm.orrery.remote/waiting');
      final sink = ChannelWaitingSink(channel: channel);
      final first = sink.pauseRequests.listen((_) {});
      await first.cancel();
      var got = 0;
      final second = sink.pauseRequests.listen((_) => got++);
      await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .handlePlatformMessage(
            channel.name,
            channel.codec.encodeMethodCall(const MethodCall('pause')),
            (_) {},
          );
      await Future<void>.delayed(Duration.zero);
      expect(got, 1);
      await second.cancel();
    });
  });
}
