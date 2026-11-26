import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/deep_link_host.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/app/pair_request.dart';
import 'package:orrery_remote/app/pane_link.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/crypto.dart';
import 'package:orrery_remote/remote/discovery.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/pairing.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/security/app_lock.dart';
import 'package:orrery_remote/security/device_auth.dart';
import 'package:orrery_remote/security/lock_gate.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';

import 'support/lock_fakes.dart';
import 'support/ui_fakes.dart' as ui;

class _Links implements LinkSource {
  final StreamController<Uri> controller = StreamController<Uri>.broadcast();

  @override
  Stream<Uri> get links => controller.stream;
}

class _Pairs implements PairSource {
  final StreamController<PairRequest> controller =
      StreamController<PairRequest>.broadcast();

  @override
  Stream<PairRequest> get requests => controller.stream;
}

class _Machines extends MachinesNotifier {
  final List<String> codes = [];
  Object? failure;

  @override
  List<Machine> build() => const [];

  @override
  Future<Machine> pair(String qrText, {PairRouteListener? onRoute}) async {
    codes.add(qrText);
    final error = failure;
    if (error != null) throw error;
    return ui.machine('m9', 'Studio PC');
  }
}

QrData _info({List<String> lan = const ['192.168.1.20'], String? host}) =>
    QrData(
      machine: 'm9',
      name: 'Studio PC',
      mkey: Uint8List(32),
      exp: 4000000000,
      host: host,
      lan: [for (final h in lan) LanEndpoint(h, 4417)],
    );

QrData _decode(String text) {
  if (text == 'bad') throw const CryptoException(CryptoFailure.crypto);
  if (text == 'loop') return _info(lan: const ['127.0.0.1']);
  if (text == 'mixed') return _info(lan: const ['127.0.0.1', '192.168.1.20']);
  return _info();
}

Future<List<String>> _wifi() async => ['192.168.1.55'];
Future<List<String>> _other() async => ['10.0.0.5'];

class _Rig {
  _Rig(this.links, this.pairs, this.machines);

  final _Links links;
  final _Pairs pairs;
  final _Machines machines;
}

Future<_Rig> _pump(
  WidgetTester tester, {
  LocalAddresses local = _wifi,
  AppLock? lock,
}) async {
  tester.view.physicalSize = const Size(390, 844);
  tester.view.devicePixelRatio = 1;
  final links = _Links();
  final pairs = _Pairs();
  final machines = _Machines();
  final navigator = GlobalKey<NavigatorState>();
  addTearDown(() async {
    await tester.pumpWidget(const SizedBox());
    await links.controller.close();
    await pairs.controller.close();
    tester.view.reset();
  });
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: ProviderContainer(
        overrides: [machinesProvider.overrideWith(() => machines)],
      ),
      child: MaterialApp(
        navigatorKey: navigator,
        theme: buildOrreryTheme(RemoteTheme.fallback, Brightness.light),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        locale: const Locale('en'),
        builder: (context, child) {
          final host = DeepLinkHost(
            navigatorKey: navigator,
            source: links,
            pairSource: pairs,
            decodeQr: _decode,
            localAddresses: local,
            lock: lock,
            child: child ?? const SizedBox.shrink(),
          );
          return lock == null ? host : LockGate(lock: lock, child: host);
        },
        home: const Scaffold(body: Text('home')),
      ),
    ),
  );
  await tester.pump();
  return _Rig(links, pairs, machines);
}

void main() {
  group('parsePairLink', () {
    test('accepts scheme, host pair and a q parameter', () {
      expect(
        parsePairLink(Uri.parse('orrery-remote://pair?q=%7B%22a%22%3A1%7D')),
        '{"a":1}',
      );
    });

    test('rejects missing or empty q', () {
      expect(parsePairLink(Uri.parse('orrery-remote://pair')), isNull);
      expect(parsePairLink(Uri.parse('orrery-remote://pair?q=')), isNull);
      expect(parsePairLink(Uri.parse('orrery-remote://pair?x=1')), isNull);
    });

    test('rejects q over 640 bytes and accepts exactly 640', () {
      expect(
        parsePairLink(Uri.parse('orrery-remote://pair?q=${'a' * 640}')),
        hasLength(640),
      );
      expect(
        parsePairLink(Uri.parse('orrery-remote://pair?q=${'a' * 641}')),
        isNull,
      );
    });

    test('counts bytes not characters', () {
      final q = Uri.encodeQueryComponent('é' * 321);
      expect(parsePairLink(Uri.parse('orrery-remote://pair?q=$q')), isNull);
    });

    test('rejects control characters', () {
      expect(parsePairLink(Uri.parse('orrery-remote://pair?q=a%00b')), isNull);
      expect(parsePairLink(Uri.parse('orrery-remote://pair?q=a%0Ab')), isNull);
    });

    test('rejects foreign hosts and schemes', () {
      expect(parsePairLink(Uri.parse('orrery-remote://m?q=abc')), isNull);
      expect(parsePairLink(Uri.parse('orrery-remote://other?q=abc')), isNull);
      expect(parsePairLink(Uri.parse('https://pair/?q=abc')), isNull);
      expect(parsePairLink(Uri.parse('other://pair?q=abc')), isNull);
    });

    test('a pair link is not a pane link and the reverse', () {
      expect(parsePaneLink(Uri.parse('orrery-remote://pair?q=abc')), isNull);
      expect(parsePairLink(Uri.parse('orrery-remote://m/abc/p/b')), isNull);
    });
  });

  group('subnet', () {
    test('same /24 matches, different does not', () {
      expect(sameSubnet24('192.168.1.20', '192.168.1.55'), isTrue);
      expect(sameSubnet24('192.168.1.20', '192.168.2.20'), isFalse);
      expect(sameSubnet24('10.0.0.1', '192.168.0.1'), isFalse);
    });

    test('garbage addresses never match', () {
      expect(sameSubnet24('abc', '192.168.1.1'), isFalse);
      expect(sameSubnet24('192.168.1', '192.168.1.1'), isFalse);
      expect(sameSubnet24('192.168.1.300', '192.168.1.1'), isFalse);
      expect(sameSubnet24('fe80::1', 'fe80::2'), isFalse);
    });

    test('any QR address against any phone address', () {
      expect(
        lanMatchesPhone(
          ['10.1.1.1', '192.168.1.9'],
          ['172.16.0.2', '192.168.1.5'],
        ),
        isTrue,
      );
      expect(lanMatchesPhone(['10.1.1.1'], ['192.168.1.5']), isFalse);
      expect(lanMatchesPhone(const [], ['192.168.1.5']), isFalse);
      expect(lanMatchesPhone(['192.168.1.9'], const []), isFalse);
    });
  });

  group('ChannelPairSource', () {
    const channel = MethodChannel(pairChannelName);

    tearDown(() {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null);
    });

    test('delivers pending on ready and later pushes from native', () async {
      TestWidgetsFlutterBinding.ensureInitialized();
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            if (call.method == 'ready') {
              return {'qr': 'first', 'fromShell': true};
            }
            return null;
          });
      final source = ChannelPairSource(channel: channel);
      final got = <PairRequest>[];
      final sub = source.requests.listen(got.add);
      await Future<void>.delayed(Duration.zero);
      expect(got, [const PairRequest('first', fromShell: true)]);
      await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .handlePlatformMessage(
            pairChannelName,
            const StandardMethodCodec().encodeMethodCall(
              const MethodCall('pair', {'qr': 'second', 'fromShell': false}),
            ),
            (_) {},
          );
      expect(got.last, const PairRequest('second', fromShell: false));
      await sub.cancel();
    });

    test('malformed payloads and a missing plugin are ignored', () async {
      TestWidgetsFlutterBinding.ensureInitialized();
      final source = ChannelPairSource(channel: channel);
      final got = <PairRequest>[];
      final sub = source.requests.listen(got.add);
      await Future<void>.delayed(Duration.zero);
      expect(got, isEmpty);
      await sub.cancel();
    });
  });

  group('DeepLinkHost pair', () {
    testWidgets('shell request asks first, then pairs on confirm', (
      tester,
    ) async {
      final rig = await _pump(tester);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm')), findsOneWidget);
      expect(find.text('Pair with Studio PC?'), findsOneWidget);
      expect(find.text('Via USB cable'), findsOneWidget);
      expect(find.text('Address 192.168.1.20'), findsOneWidget);
      expect(rig.machines.codes, isEmpty);
      await tester.tap(find.byKey(const ValueKey('pair-confirm-pair')));
      await tester.pumpAndSettle();
      expect(rig.machines.codes, ['qr1']);
      expect(find.byKey(const ValueKey('pair-confirm')), findsNothing);
    });

    testWidgets('shell request skips the subnet check', (tester) async {
      final rig = await _pump(tester, local: _other);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm-pair')), findsOneWidget);
      expect(find.byKey(const ValueKey('pair-confirm-blocked')), findsNothing);
    });

    testWidgets('extra without a shell referrer skips subnet and is labelled', (
      tester,
    ) async {
      final rig = await _pump(tester, local: _other);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: false));
      await tester.pumpAndSettle();
      expect(
        find.text('Via USB cable or an app on this phone'),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('pair-confirm-blocked')), findsNothing);
      await tester.tap(find.byKey(const ValueKey('pair-confirm-pair')));
      await tester.pumpAndSettle();
      expect(rig.machines.codes, ['qr1']);
    });

    testWidgets('extra with only loopback shows over USB cable', (
      tester,
    ) async {
      final rig = await _pump(tester, local: () async => const []);
      rig.pairs.controller.add(const PairRequest('loop', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.text('Over USB cable'), findsOneWidget);
      expect(find.textContaining('127.0.0.1'), findsNothing);
      expect(find.byKey(const ValueKey('pair-confirm-pair')), findsOneWidget);
    });

    testWidgets('extra with loopback and LAN shows the LAN address', (
      tester,
    ) async {
      final rig = await _pump(tester);
      rig.pairs.controller.add(const PairRequest('mixed', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.text('Address 192.168.1.20'), findsOneWidget);
      expect(find.text('Over USB cable'), findsNothing);
    });

    testWidgets('web link on the same Wi-Fi asks first, then pairs', (
      tester,
    ) async {
      final rig = await _pump(tester);
      rig.links.controller.add(Uri.parse('orrery-remote://pair?q=qr2'));
      await tester.pumpAndSettle();
      expect(find.text('Opened from a web page'), findsOneWidget);
      expect(rig.machines.codes, isEmpty);
      await tester.tap(find.byKey(const ValueKey('pair-confirm-pair')));
      await tester.pumpAndSettle();
      expect(rig.machines.codes, ['qr2']);
    });

    testWidgets('web link from another subnet is blocked and cannot pair', (
      tester,
    ) async {
      final rig = await _pump(tester, local: _other);
      rig.links.controller.add(Uri.parse('orrery-remote://pair?q=qr2'));
      await tester.pumpAndSettle();
      expect(
        find.text('This code is not on the current Wi-Fi network'),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('pair-confirm-pair')), findsNothing);
      expect(rig.machines.codes, isEmpty);
    });

    testWidgets('cancel never pairs', (tester) async {
      final rig = await _pump(tester);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('pair-confirm-cancel')));
      await tester.pumpAndSettle();
      expect(rig.machines.codes, isEmpty);
      expect(find.byKey(const ValueKey('pair-confirm')), findsNothing);
    });

    testWidgets('undecodable code says it is invalid and shows no sheet', (
      tester,
    ) async {
      final rig = await _pump(tester);
      rig.pairs.controller.add(const PairRequest('bad', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.text('Invalid pairing code'), findsOneWidget);
      expect(find.byKey(const ValueKey('pair-confirm')), findsNothing);
      expect(rig.machines.codes, isEmpty);
    });

    testWidgets('pair failure stays on the sheet with the reason', (
      tester,
    ) async {
      final rig = await _pump(tester);
      rig.machines.failure = const PairException(PairFailure.expired);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('pair-confirm-pair')));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm-error')), findsOneWidget);
      expect(find.byKey(const ValueKey('pair-confirm-pair')), findsOneWidget);
    });

    testWidgets('the same request within 2s opens one sheet', (tester) async {
      final rig = await _pump(tester);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm')), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('pair-confirm-cancel')));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm')), findsNothing);
    });

    testWidgets('a different request replaces the open sheet', (tester) async {
      final rig = await _pump(tester);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      await tester.pumpAndSettle();
      rig.pairs.controller.add(const PairRequest('qr3', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm')), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('pair-confirm-pair')));
      await tester.pumpAndSettle();
      expect(rig.machines.codes, ['qr3']);
    });

    testWidgets('pane links still go through', (tester) async {
      final rig = await _pump(tester);
      rig.links.controller.add(Uri.parse('orrery-remote://m/m1'));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm')), findsNothing);
    });

    testWidgets('a request while locked is held until unlock', (tester) async {
      final auth = FakeAuth()..queue.add(AuthOutcome.cancelled);
      final lock = makeLock(auth);
      await lock.load();
      final rig = await _pump(tester, lock: lock);
      await tester.pumpAndSettle();
      expect(lock.locked, isTrue);
      rig.pairs.controller.add(const PairRequest('qr1', fromShell: true));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('pair-confirm')), findsNothing);
      await tester.tap(find.byKey(const ValueKey('lock-unlock')));
      await tester.pumpAndSettle();
      expect(lock.locked, isFalse);
      expect(find.byKey(const ValueKey('pair-confirm')), findsOneWidget);
      expect(rig.machines.codes, isEmpty);
      lock.dispose();
    });
  });
}
