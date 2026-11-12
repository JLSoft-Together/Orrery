import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/push/local_notifier.dart';
import 'package:orrery_remote/push/push_background.dart';
import 'package:orrery_remote/push/push_notice.dart';
import 'package:orrery_remote/push/push_service.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/secure_kv.dart';

import 'support/push_vector.dart';
import 'support/push_fakes.dart';

class FakeNotifier implements PushNotifier {
  final List<PushNotice> shown = [];
  bool ready = true;
  int failures = 0;

  @override
  Future<bool> init({void Function(String? payload)? onTap}) async => ready;

  @override
  Future<void> show(PushNotice notice, {required String channelName}) async {
    if (failures > 0) {
      failures--;
      throw StateError('plugin');
    }
    shown.add(notice);
  }

  @override
  Future<String?> launchPayload() async => null;
}

Future<MemorySecureKv> keyed() async {
  final kv = MemorySecureKv();
  await kv.write(KeyStore.keyName, base64Url.encode(devicePrivate));
  return kv;
}

Future<FakeNotifier> background(
  Map<String, String> data, {
  required bool shown,
  required SecureKv kv,
  FakeNotifier? notifier,
}) async {
  final n = notifier ?? FakeNotifier();
  await handleBackgroundPush(
    PushArrival(data, shown: shown),
    kv: kv,
    machineKey: pairedVector,
    notifier: n,
    languageCode: 'vi',
  );
  return n;
}

void main() {
  test(
    'a data-only sealed push shows the decrypted text in the background',
    () async {
      final kv = await keyed();
      final n = await background(sealedData(), shown: false, kv: kv);
      expect(n.shown.single.sealed, isTrue);
      expect(n.shown.single.body, 'Có chạy migration không?');
      expect(n.shown.single.tag, '$machine:p1');
      expect(kv.data[pushSealOkKey], isNull);
    },
  );

  test(
    'a data-only push that does not open still shows the fixed text',
    () async {
      final n = await background(
        sealedData(),
        shown: false,
        kv: MemorySecureKv(),
      );
      expect(n.shown.single.sealed, isFalse);
      expect(n.shown.single.title, 'Agent đang hỏi');
    },
  );

  test(
    'the probe is not shown twice and proves the background path once it opens',
    () async {
      final kv = await keyed();
      final probe = {...sealedData(), 'p': 'p1'};
      final n = await background(probe, shown: true, kv: kv);
      expect(n.shown, isEmpty);
      expect(kv.data[pushSealOkKey], 'on');
      final cold = MemorySecureKv();
      expect((await background(probe, shown: true, kv: cold)).shown, isEmpty);
      expect(cold.data[pushSealOkKey], isNull);
      final noPlugin = await keyed();
      await background(
        probe,
        shown: true,
        kv: noPlugin,
        notifier: FakeNotifier()..ready = false,
      );
      expect(noPlugin.data[pushSealOkKey], isNull);
    },
  );

  test(
    'a failing plugin retries with fixed text, then drops the proof',
    () async {
      final kv = await keyed();
      await kv.write(pushSealOkKey, 'on');
      final once = await background(
        sealedData(),
        shown: false,
        kv: kv,
        notifier: FakeNotifier()..failures = 1,
      );
      expect(once.shown.single.sealed, isFalse);
      expect(kv.data[pushSealOkKey], 'on');
      await background(
        sealedData(),
        shown: false,
        kv: kv,
        notifier: FakeNotifier()..failures = 2,
      );
      expect(kv.data[pushSealOkKey], 'off');
    },
  );

  group('foreground', () {
    Future<Rig> started() async {
      final r = Rig();
      await r.kv.write(KeyStore.keyName, base64Url.encode(devicePrivate));
      r.machineKeys[machine] = desktopPublic;
      await r.service.init();
      return r;
    }

    test('hidden only while the app is open on that machine', () async {
      final r = await started();
      r.host.connect(FakeSession(machine));
      r.gateway.arrive.add(PushArrival(sealedData(), shown: false));
      await pumpEventQueue();
      expect(r.gateway.shown, isEmpty);
      r.resumed = false;
      r.gateway.arrive.add(PushArrival(sealedData(), shown: true));
      await pumpEventQueue();
      expect(r.gateway.shown.single.body, 'Có chạy migration không?');
      r.resumed = true;
      r.host.now.clear();
      r.gateway.arrive.add(
        const PushArrival({'m': 'other', 'k': 'done', 'p': 'p2'}, shown: true),
      );
      await pumpEventQueue();
      expect(r.gateway.shown.last.title, 'Agent đã xong lượt');
      expect(r.gateway.shown.last.tag, 'other:p2');
      r.gateway.showFails = true;
      r.gateway.arrive.add(PushArrival(sealedData(e: sealed), shown: false));
      await pumpEventQueue();
      expect(r.gateway.shown, hasLength(2));
    });

    test('register sends content, seal and the background proof', () async {
      final r = await started();
      final s = FakeSession('m1');
      r.host.connect(s);
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      await settle();
      expect(await r.service.setEnabled(true), PushResult.on);
      expect(s.sent.last.$2['sealOk'], false);
      await r.kv.write(pushSealOkKey, 'on');
      await r.service.setContent(false);
      expect(r.service.content, isFalse);
      expect(r.kv.data[pushContentKey], 'off');
      expect(
        (
          s.sent.last.$2['content'],
          s.sent.last.$2['seal'],
          s.sent.last.$2['sealOk'],
        ),
        (false, 2, true),
      );
      r.gateway.sealing = false;
      await r.service.setContent(true);
      expect(s.sent.last.$2['content'], true);
      expect(s.sent.last.$2.containsKey('seal'), isFalse);
      expect(s.sent.last.$2.containsKey('sealOk'), isFalse);
      final again = Rig();
      await again.kv.write(pushContentKey, 'off');
      await again.service.init();
      expect(again.service.content, isFalse);
    });
  });
}
