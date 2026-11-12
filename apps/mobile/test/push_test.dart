import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/push/push_config.dart';
import 'package:orrery_remote/push/push_gateway.dart';
import 'package:orrery_remote/push/push_service.dart';

import 'support/push_fakes.dart';

void main() {
  test('push link only accepts safe machine and pane keys', () {
    expect(
      pushLink({'m': 'machine-1', 'p': 'pane_1', 'k': 'done'}).toString(),
      'orrery-remote://m/machine-1/p/pane_1',
    );
    expect(
      pushLink({'m': 'machine-1'}).toString(),
      'orrery-remote://m/machine-1',
    );
    expect(pushLink({'m': 'm 1', 'p': 'p1'}), isNull);
    expect(pushLink({'m': 'm1', 'p': '../x'}), isNull);
    expect(pushLink({'m': 'm1', 'p': 'a/b'}), isNull);
  });

  test('push config parses plain values and refuses the rest', () {
    final ok = PushConfig.tryParse({
      't': 'push.config',
      'apiKey': 'AIza-key_1',
      'appId': '1:123:android:abc',
      'senderId': '123',
      'projectId': 'orrery-remote',
    });
    expect(ok?.projectId, 'orrery-remote');
    expect(PushConfig.tryParse({'t': 'push.config'}), isNull);
    expect(
      PushConfig.tryParse({
        'apiKey': 'a',
        'appId': 'b',
        'senderId': 'c',
        'projectId': 'Bad Id',
      }),
      isNull,
    );
    expect(
      PushConfig.tryParse({
        'apiKey': 'a b',
        'appId': 'b',
        'senderId': 'c',
        'projectId': 'ok',
      }),
      isNull,
    );
    expect(
      PushConfig.tryParse({
        'apiKey': 1,
        'appId': 'b',
        'senderId': 'c',
        'projectId': 'ok',
      }),
      isNull,
    );
  });

  test('enabling asks permission, takes the token and registers every ready machine', () async {
    final r = Rig();
    final s1 = FakeSession('m1');
    final s2 = FakeSession('m2');
    await r.service.init();
    r.host.connect(s1);
    r.host.connect(s2);
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    r.host.config.add(const PushConfigUpdate('m2', pushConfigA));
    await settle();
    expect(r.service.configured, isTrue);
    expect(s1.sent, isEmpty);
    expect(await r.service.setEnabled(true), PushResult.on);
    expect(r.gateway.permissionAsks, 1);
    expect(r.gateway.opened_, ['proj-a']);
    for (final s in [s1, s2]) {
      expect(s.sent.single.$1, 'push.register');
      expect(s.sent.single.$2, {
        'token': 'token-a-1',
        'platform': 'android',
        'locale': 'vi',
        'mute': ['error'],
        'content': true,
        'seal': 2,
        'sealOk': false,
      });
    }
    expect(r.kv.data[pushEnabledKey], 'on');
  });

  test('a muted group is saved and re-sent to every machine', () async {
    final r = Rig();
    final s1 = FakeSession('m1');
    await r.service.init();
    r.host.connect(s1);
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await settle();
    expect(await r.service.setEnabled(true), PushResult.on);
    expect(r.service.wants('error'), isFalse);
    await r.service.setGroup('done', false);
    await r.service.setGroup('error', true);
    expect(r.service.wants('done'), isFalse);
    expect(r.service.wants('error'), isTrue);
    expect(s1.sent.last.$2['mute'], ['done']);
    expect(r.kv.data[pushMuteKey], isNotNull);
  });

  test('a locale other than vi is sent as en', () async {
    final r = Rig(locale: 'fr');
    final s = FakeSession('m1');
    await r.service.init();
    r.host.connect(s);
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await settle();
    await r.service.setEnabled(true);
    expect(s.sent.single.$2['locale'], 'en');
  });

  test(
    'a refused permission or a missing token leaves it off and sends nothing',
    () async {
      final r = Rig();
      final s = FakeSession('m1');
      await r.service.init();
      r.host.connect(s);
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      await settle();
      r.gateway.asked = PushPermission.denied;
      expect(await r.service.setEnabled(true), PushResult.denied);
      r.gateway.asked = PushPermission.granted;
      r.gateway.tokens['proj-a'] = null;
      expect(await r.service.setEnabled(true), PushResult.unavailable);
      expect(r.service.enabled, isFalse);
      expect(r.kv.data[pushEnabledKey], isNull);
      expect(s.sent, isEmpty);
    },
  );

  test('without a desktop config the switch has nothing to enable', () async {
    final r = Rig();
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', null));
    await settle();
    expect(r.service.configured, isFalse);
    expect(await r.service.setEnabled(true), PushResult.unavailable);
    expect(r.gateway.permissionAsks, 0);
  });

  test(
    'a machine that connects or sets up Firebase later is registered',
    () async {
      final r = Rig();
      await r.service.init();
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      await settle();
      await r.service.setEnabled(true);
      final late = FakeSession('m1');
      r.host.connect(late);
      await settle();
      expect(late.sent.single.$1, 'push.register');
      final other = FakeSession('m2');
      r.host.connect(other);
      await settle();
      expect(other.sent, isEmpty);
      r.host.config.add(const PushConfigUpdate('m2', pushConfigA));
      await settle();
      expect(other.sent.single.$2['token'], 'token-a-1');
    },
  );

  test(
    'a machine on another Firebase project is not registered and gets a hint',
    () async {
      final r = Rig();
      final a = FakeSession('m1');
      final b = FakeSession('m2');
      await r.service.init();
      r.host.connect(a);
      r.host.connect(b);
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      r.host.config.add(const PushConfigUpdate('m2', pushConfigB));
      await settle();
      expect(await r.service.setEnabled(true, machineId: 'm1'), PushResult.on);
      expect(a.sent.single.$2['token'], 'token-a-1');
      expect(b.sent, isEmpty);
      expect(r.service.isForeign('m2'), isTrue);
      expect(r.service.isForeign('m1'), isFalse);
      expect(r.service.foreignCount, 1);
      expect(r.service.usable, isTrue);
    },
  );

  test('enabling uses the tapped machine when nothing is initialized, never the first', () async {
    final r = Rig();
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    r.host.config.add(const PushConfigUpdate('m2', pushConfigB));
    await settle();
    expect(await r.service.setEnabled(true, machineId: 'm2'), PushResult.on);
    expect(r.gateway.opened_, ['proj-b']);
    expect(r.service.isForeign('m1'), isTrue);
  });

  test(
    'once a project is initialized enabling uses it whatever machine is passed',
    () async {
      final r = Rig();
      r.gateway.project = 'proj-b';
      await r.service.init();
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      r.host.config.add(const PushConfigUpdate('m2', pushConfigB));
      await settle();
      expect(await r.service.setEnabled(true, machineId: 'm1'), PushResult.on);
      expect(r.gateway.opened_, ['proj-b']);
    },
  );

  test('when every machine is foreign nothing can be enabled', () async {
    final r = Rig();
    r.gateway.project = 'proj-z';
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await settle();
    expect(r.service.usable, isFalse);
    expect(await r.service.setEnabled(true), PushResult.unavailable);
  });

  test('a failing Firebase init is reported and leaves it off', () async {
    final r = Rig();
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await settle();
    r.gateway.openFails = true;
    expect(
      await r.service.setEnabled(true, machineId: 'm1'),
      PushResult.unavailable,
    );
    expect(r.service.error, 'boom');
    expect(r.service.enabled, isFalse);
  });

  test(
    'a machine offline while turning off is told when it comes back',
    () async {
      final r = Rig();
      final a = FakeSession('m1');
      await r.service.init();
      r.host.connect(a);
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      r.host.config.add(const PushConfigUpdate('m2', pushConfigA));
      await settle();
      await r.service.setEnabled(true, machineId: 'm1');
      await r.service.setEnabled(false);
      expect(a.sent.last.$1, 'push.unregister');
      expect(r.kv.data[pushPendingKey], '["m2"]');
      final b = FakeSession('m2');
      r.host.connect(b);
      await settle();
      expect(b.sent.single.$1, 'push.unregister');
      expect(r.kv.data[pushPendingKey], '[]');
    },
  );

  test('a revoked notification permission shows the switch off and comes back on resume', () async {
    final r = Rig();
    final s = FakeSession('m1');
    r.host.now.add(s);
    r.kv.data[pushEnabledKey] = 'on';
    r.kv.data[pushConfigsKey] = '{"m1":{"apiKey":"key-a","appId":"1:1:android:a","senderId":"111","projectId":"proj-a"}}';
    r.gateway.current = PushPermission.denied;
    await r.service.init();
    expect(r.service.revoked, isTrue);
    expect(r.service.enabled, isFalse);
    expect(s.sent, isEmpty);
    r.gateway.current = PushPermission.granted;
    await r.service.refreshPermission();
    expect(r.service.revoked, isFalse);
    expect(r.service.enabled, isTrue);
    expect(s.sent.single.$1, 'push.register');
  });

  test('overlapping toggles run one after the other', () async {
    final r = Rig();
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await settle();
    final results = await Future.wait([
      r.service.setEnabled(true, machineId: 'm1'),
      r.service.setEnabled(false),
      r.service.setEnabled(true, machineId: 'm1'),
    ]);
    expect(results, [PushResult.on, PushResult.off, PushResult.on]);
    expect(r.service.enabled, isTrue);
  });

  test('dispose stops everything', () async {
    final r = Rig();
    await r.service.init();
    await r.service.dispose();
    expect(r.gateway.disposed, isTrue);
    expect(r.host.disposed, isTrue);
  });

  test(
    'a refreshed token is sent again to the machines of that project only',
    () async {
      final r = Rig();
      final a = FakeSession('m1');
      final b = FakeSession('m2');
      await r.service.init();
      r.host.connect(a);
      r.host.connect(b);
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      r.host.config.add(const PushConfigUpdate('m2', pushConfigB));
      await settle();
      await r.service.setEnabled(true, machineId: 'm1');
      r.gateway.refresh.add(const PushTokenChange('proj-a', 'token-a-2'));
      await settle();
      expect(a.sent.last.$2['token'], 'token-a-2');
      expect(a.sent, hasLength(2));
      expect(b.sent, isEmpty);
    },
  );

  test(
    'turning it off unregisters, deletes the token and stops registering',
    () async {
      final r = Rig();
      final s = FakeSession('m1');
      await r.service.init();
      r.host.connect(s);
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      await settle();
      await r.service.setEnabled(true);
      expect(await r.service.setEnabled(false), PushResult.off);
      expect(s.sent.last.$1, 'push.unregister');
      expect(s.sent.last.$2, isEmpty);
      expect(r.gateway.deleted, ['proj-a']);
      expect(r.kv.data[pushEnabledKey], 'off');
      final again = FakeSession('m1');
      r.host.connect(again);
      await settle();
      expect(again.sent, isEmpty);
    },
  );

  test('send failures never throw', () async {
    final r = Rig();
    final s = FakeSession('m1')..failing = true;
    await r.service.init();
    r.host.connect(s);
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await settle();
    expect(await r.service.setEnabled(true), PushResult.on);
  });

  test('the switch and the desktop config survive a restart', () async {
    final first = Rig();
    await first.service.init();
    first.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await settle();
    await first.service.setEnabled(true);

    final second = Rig();
    second.kv.data.addAll(first.kv.data);
    final s = FakeSession('m1');
    second.host.now.add(s);
    await second.service.init();
    expect(second.service.enabled, isTrue);
    expect(second.service.configured, isTrue);
    await settle();
    expect(s.sent.single.$1, 'push.register');
  });

  test(
    'a notification tap becomes a pane link, a forged one is dropped',
    () async {
      final r = Rig();
      await r.service.init();
      final links = <Uri>[];
      final sub = r.service.links.listen(links.add);
      r.gateway.open_.add({'m': 'm1', 'p': 'b', 'k': 'done'});
      r.gateway.open_.add({'m': 'm1', 'p': '../b', 'k': 'done'});
      r.gateway.open_.add({'k': 'done'});
      await settle();
      expect(links.map((u) => u.toString()), ['orrery-remote://m/m1/p/b']);
      await sub.cancel();
    },
  );

  test(
    'a tap that started the app from a killed state is delivered once',
    () async {
      final r = Rig();
      await r.service.init();
      r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
      await settle();
      await r.service.setEnabled(true);

      final cold = Rig();
      cold.kv.data.addAll(r.kv.data);
      cold.gateway.initial = {'m': 'm1', 'p': 'c', 'k': 'permission'};
      await cold.service.init();
      final first = <Uri>[];
      final second = <Uri>[];
      final a = cold.service.links.listen(first.add);
      await settle();
      final b = cold.service.links.listen(second.add);
      await settle();
      expect(first.map((u) => u.toString()), ['orrery-remote://m/m1/p/c']);
      expect(second, isEmpty);
      await a.cancel();
      await b.cancel();
    },
  );

  test('merged link source forwards every source', () async {
    final one = StreamController<Uri>.broadcast();
    final two = StreamController<Uri>.broadcast();
    final merged = MergedLinkSource([_Src(one), _Src(two)]);
    final got = <Uri>[];
    final sub = merged.links.listen(got.add);
    one.add(Uri.parse('a:/1'));
    two.add(Uri.parse('b:/2'));
    await settle();
    expect(got, [Uri.parse('a:/1'), Uri.parse('b:/2')]);
    await sub.cancel();
  });
}

class _Src implements LinkSource {
  _Src(this.c);

  final StreamController<Uri> c;

  @override
  Stream<Uri> get links => c.stream;
}
