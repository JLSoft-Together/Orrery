import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/deep_link_host.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/push/push_config.dart';
import 'package:orrery_remote/push/push_gateway.dart';
import 'package:orrery_remote/push/push_providers.dart';
import 'package:orrery_remote/push/push_service.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';

import 'support/ui_samples.dart';

const _configA = PushConfig(
  apiKey: 'key-a',
  appId: '1:1:android:a',
  senderId: '111',
  projectId: 'proj-a',
);
const _configB = PushConfig(
  apiKey: 'key-b',
  appId: '1:2:android:b',
  senderId: '222',
  projectId: 'proj-b',
);

class FakeGateway implements PushGateway {
  PushPermission permission = PushPermission.granted;
  final Map<String, String?> tokens = {'proj-a': 'token-a-1', 'proj-b': 'token-b-1'};
  final List<String> deleted = [];
  final List<String> permissionAsks = [];
  Map<String, String>? initial;
  final StreamController<PushTokenChange> refresh = StreamController.broadcast();
  final StreamController<Map<String, String>> open = StreamController.broadcast();

  @override
  Future<PushPermission> requestPermission(PushConfig config) async {
    permissionAsks.add(config.projectId);
    return permission;
  }

  @override
  Future<String?> token(PushConfig config) async => tokens[config.projectId];

  @override
  Future<void> deleteToken(PushConfig config) async {
    deleted.add(config.projectId);
    tokens[config.projectId] = null;
  }

  @override
  Stream<PushTokenChange> get tokenRefresh => refresh.stream;

  @override
  Stream<Map<String, String>> get opened => open.stream;

  @override
  Future<Map<String, String>?> initialOpen(Iterable<PushConfig> configs) async =>
      initial;
}

class FakeSession implements PushSession {
  FakeSession(this.machineId);

  @override
  final String machineId;
  final List<(String, Map<String, Object?>)> sent = [];
  bool failing = false;

  @override
  Future<void> send(String type, Map<String, Object?> fields) async {
    if (failing) throw StateError('offline');
    sent.add((type, fields));
  }
}

class FakeHost implements PushHost {
  final StreamController<PushSession> ready = StreamController.broadcast();
  final StreamController<PushConfigUpdate> config = StreamController.broadcast();
  final List<PushSession> now = [];

  @override
  Stream<PushSession> get readySessions => ready.stream;

  @override
  List<PushSession> get readyNow => now;

  @override
  Stream<PushConfigUpdate> get configs => config.stream;

  void connect(FakeSession s) {
    now.add(s);
    ready.add(s);
  }
}

Future<void> settle() => Future<void>.delayed(Duration.zero);

class Rig {
  Rig({String locale = 'vi'}) {
    service = PushService(
      gateway: gateway,
      kv: kv,
      host: host,
      platform: 'android',
      localeCode: () => locale,
    );
  }

  final FakeGateway gateway = FakeGateway();
  final FakeHost host = FakeHost();
  final MemorySecureKv kv = MemorySecureKv();
  late final PushService service;
}

void main() {
  test('push link only accepts safe machine and pane keys', () {
    expect(
      pushLink({'m': 'machine-1', 'p': 'pane_1', 'k': 'done'}).toString(),
      'orrery-remote://m/machine-1/p/pane_1',
    );
    expect(pushLink({'m': 'machine-1'}), isNull);
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
    expect(PushConfig.tryParse({'apiKey': 'a', 'appId': 'b', 'senderId': 'c', 'projectId': 'Bad Id'}), isNull);
    expect(PushConfig.tryParse({'apiKey': 'a b', 'appId': 'b', 'senderId': 'c', 'projectId': 'ok'}), isNull);
    expect(PushConfig.tryParse({'apiKey': 1, 'appId': 'b', 'senderId': 'c', 'projectId': 'ok'}), isNull);
  });

  test('enabling asks permission, takes the token and registers every ready machine', () async {
    final r = Rig();
    final s1 = FakeSession('m1');
    final s2 = FakeSession('m2');
    await r.service.init();
    r.host.connect(s1);
    r.host.connect(s2);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    r.host.config.add(const PushConfigUpdate('m2', _configA));
    await settle();
    expect(r.service.configured, isTrue);
    expect(s1.sent, isEmpty);
    expect(await r.service.setEnabled(true), PushResult.on);
    expect(r.gateway.permissionAsks, ['proj-a']);
    for (final s in [s1, s2]) {
      expect(s.sent.single.$1, 'push.register');
      expect(s.sent.single.$2, {
        'token': 'token-a-1',
        'platform': 'android',
        'locale': 'vi',
      });
    }
    expect(r.kv.data[pushEnabledKey], 'on');
  });

  test('a locale other than vi is sent as en', () async {
    final r = Rig(locale: 'fr');
    final s = FakeSession('m1');
    await r.service.init();
    r.host.connect(s);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    await settle();
    await r.service.setEnabled(true);
    expect(s.sent.single.$2['locale'], 'en');
  });

  test('a refused permission or a missing token leaves it off and sends nothing', () async {
    final r = Rig();
    final s = FakeSession('m1');
    await r.service.init();
    r.host.connect(s);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    await settle();
    r.gateway.permission = PushPermission.denied;
    expect(await r.service.setEnabled(true), PushResult.denied);
    r.gateway.permission = PushPermission.granted;
    r.gateway.tokens['proj-a'] = null;
    expect(await r.service.setEnabled(true), PushResult.unavailable);
    expect(r.service.enabled, isFalse);
    expect(r.kv.data[pushEnabledKey], isNull);
    expect(s.sent, isEmpty);
  });

  test('without a desktop config the switch has nothing to enable', () async {
    final r = Rig();
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', null));
    await settle();
    expect(r.service.configured, isFalse);
    expect(await r.service.setEnabled(true), PushResult.unavailable);
    expect(r.gateway.permissionAsks, isEmpty);
  });

  test('a machine that connects or sets up Firebase later is registered', () async {
    final r = Rig();
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', _configA));
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
    r.host.config.add(const PushConfigUpdate('m2', _configA));
    await settle();
    expect(other.sent.single.$2['token'], 'token-a-1');
  });

  test('each project keeps its own token', () async {
    final r = Rig();
    final a = FakeSession('m1');
    final b = FakeSession('m2');
    await r.service.init();
    r.host.connect(a);
    r.host.connect(b);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    r.host.config.add(const PushConfigUpdate('m2', _configB));
    await settle();
    await r.service.setEnabled(true);
    expect(a.sent.single.$2['token'], 'token-a-1');
    expect(b.sent.single.$2['token'], 'token-b-1');
  });

  test('a refreshed token is sent again to the machines of that project only', () async {
    final r = Rig();
    final a = FakeSession('m1');
    final b = FakeSession('m2');
    await r.service.init();
    r.host.connect(a);
    r.host.connect(b);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    r.host.config.add(const PushConfigUpdate('m2', _configB));
    await settle();
    await r.service.setEnabled(true);
    r.gateway.refresh.add(const PushTokenChange('proj-a', 'token-a-2'));
    await settle();
    expect(a.sent.last.$2['token'], 'token-a-2');
    expect(a.sent, hasLength(2));
    expect(b.sent, hasLength(1));
  });

  test('turning it off unregisters, deletes the token and stops registering', () async {
    final r = Rig();
    final s = FakeSession('m1');
    await r.service.init();
    r.host.connect(s);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
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
  });

  test('send failures never throw', () async {
    final r = Rig();
    final s = FakeSession('m1')..failing = true;
    await r.service.init();
    r.host.connect(s);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    await settle();
    expect(await r.service.setEnabled(true), PushResult.on);
  });

  test('the switch and the desktop config survive a restart', () async {
    final first = Rig();
    await first.service.init();
    first.host.config.add(const PushConfigUpdate('m1', _configA));
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

  test('a notification tap becomes a pane link, a forged one is dropped', () async {
    final r = Rig();
    await r.service.init();
    final links = <Uri>[];
    final sub = r.service.links.listen(links.add);
    r.gateway.open.add({'m': 'm1', 'p': 'b', 'k': 'done'});
    r.gateway.open.add({'m': 'm1', 'p': '../b', 'k': 'done'});
    r.gateway.open.add({'k': 'done'});
    await settle();
    expect(links.map((u) => u.toString()), ['orrery-remote://m/m1/p/b']);
    await sub.cancel();
  });

  test('a tap that started the app from a killed state is delivered once', () async {
    final r = Rig();
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', _configA));
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
  });

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

  testWidgets('tapping a notification opens the pane in the app', (tester) async {
    final h = harness();
    final r = Rig();
    await r.service.init();
    final navigator = GlobalKey<NavigatorState>();
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox());
      tester.view.reset();
    });
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: h.container = ProviderContainer(overrides: h.overrides),
        child: MaterialApp(
          navigatorKey: navigator,
          theme: buildOrreryTheme(RemoteTheme.fallback, Brightness.light),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          builder: (context, child) => DeepLinkHost(
            navigatorKey: navigator,
            source: MergedLinkSource([r.service]),
            child: child ?? const SizedBox.shrink(),
          ),
          home: const Scaffold(body: Text('home')),
        ),
      ),
    );
    await tester.pump();
    r.gateway.open.add({'m': 'm1', 'p': 'b', 'k': 'done'});
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(h.api.attached, contains('b'));
  });

  testWidgets('the switch shows only when the desktop has Firebase set up', (tester) async {
    final r = Rig();
    await r.service.init();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [pushServiceProvider.overrideWithValue(r.service)],
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          home: const Scaffold(body: PushTile()),
        ),
      ),
    );
    expect(find.byKey(const ValueKey('push-switch')), findsNothing);
    expect(find.byKey(const ValueKey('push-not-set-up')), findsOneWidget);
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('push-switch')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('push-switch')));
    await tester.pump();
    await tester.pump();
    expect(r.service.enabled, isTrue);
  });

  testWidgets('a refused permission says so and leaves the switch off', (tester) async {
    final r = Rig();
    r.gateway.permission = PushPermission.denied;
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', _configA));
    await tester.pump();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [pushServiceProvider.overrideWithValue(r.service)],
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          home: const Scaffold(body: PushTile()),
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('push-switch')));
    await tester.pump();
    await tester.pump();
    expect(find.textContaining('permission was refused'), findsOneWidget);
    expect(r.service.enabled, isFalse);
  });
}

class _Src implements LinkSource {
  _Src(this.c);

  final StreamController<Uri> c;

  @override
  Stream<Uri> get links => c.stream;
}
