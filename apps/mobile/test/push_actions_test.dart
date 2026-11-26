import 'dart:convert';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/menu_action.dart';
import 'package:orrery_remote/app/pane_link.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/push/local_notifier.dart';
import 'package:orrery_remote/push/push_background.dart';
import 'package:orrery_remote/push/push_menu.dart';
import 'package:orrery_remote/push/push_notice.dart';
import 'package:orrery_remote/push/push_present.dart';
import 'package:orrery_remote/push/push_seal.dart' show readStaticKey;
import 'package:orrery_remote/push/push_service.dart';
import 'package:orrery_remote/push/waiting_sink.dart';
import 'package:orrery_remote/push/waiting_update.dart';
import 'package:orrery_remote/remote/chat_protocol.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/ui/model/workspace_model.dart';

import 'push_present_test.dart' show FakeNotifier;
import 'support/push_fakes.dart';
import 'support/seal_maker.dart';

class SinkRecorder implements WaitingSink {
  final List<WaitingUpdate> applied = [];

  @override
  Future<void> start() async {}

  @override
  Future<void> apply(WaitingUpdate update, {required DateTime at}) async =>
      applied.add(update);

  @override
  Future<void> setPausedUntil(DateTime? until) async {}

  @override
  Stream<void> get pauseRequests => const Stream<void>.empty();
}

Map<String, Object?> menuJson({int count = 3}) => {
  'sig': 'ab12cd34',
  'items': [
    for (var i = 1; i <= count; i++) {'index': i, 'label': 'Choice $i'},
  ],
};

Future<PushPresenter> presenter(
  FakeGateway gateway,
  SealRig rig,
  MemorySecureKv kv, {
  SinkRecorder? sink,
  bool open = false,
}) async {
  await kv.write(KeyStore.keyName, base64Url.encode(rig.devicePrivate));
  return PushPresenter(
    gateway: gateway,
    kv: kv,
    sessionOpen: (_) => open,
    resumed: () => true,
    localeCode: () => 'en',
    machineKey: rig.machineKey,
    waiting: sink ?? SinkRecorder(),
  );
}

PaneInfo paneWith(RemoteMenu? menu) => PaneInfo(
  key: 'p1',
  kind: 'agent',
  title: 'Fix',
  rail: RailState.asked,
  subagents: 0,
  dirty: false,
  busy: false,
  menu: menu,
);

RemoteMenu remoteMenu(String sig) => RemoteMenu(
  sig: sig,
  kind: RemoteMenuKind.permission,
  question: 'Run?',
  items: const [
    RemoteMenuItem(index: 1, label: 'Yes'),
    RemoteMenuItem(index: 2, label: 'No'),
    RemoteMenuItem(index: 3, label: 'Other', other: true),
  ],
  numbered: true,
  confident: true,
);

void main() {
  group('menu parse', () {
    test('keeps at most three items with labels cut to 24', () {
      final menu = parsePushMenu({
        'sig': 'ab12',
        'items': [
          {'index': 1, 'label': 'Yes'},
          {'index': 2, 'label': 'Yes, and do not ask again for this project'},
          {'index': 2, 'label': 'dup'},
          {'index': 0, 'label': 'zero'},
          {'index': 3, 'label': 'No'},
          {'index': 4, 'label': 'Extra'},
        ],
      })!;
      expect(menu.items.map((i) => i.index), [1, 2, 3]);
      expect(menu.items[1].label.runes.length, pushMenuLabelMax);
    });

    test('rejects a bad sig, missing items and control characters', () {
      expect(parsePushMenu(null), isNull);
      expect(parsePushMenu({'sig': 'bad sig', 'items': []}), isNull);
      expect(parsePushMenu({'sig': 'ab', 'items': []}), isNull);
      final one = parsePushMenu({
        'sig': 'ab',
        'items': [
          {
            'index': 1,
            'label': String.fromCharCodes([0x61, 7, 0x202e, 0x62]),
          },
        ],
      })!;
      expect(one.items.single.label, 'a b');
    });

    test('action ids and tap data', () {
      expect(menuActionIndex('menu:2'), 2);
      expect(menuActionIndex('menu:0'), isNull);
      expect(menuActionIndex('open'), isNull);
      expect(menuActionIndex(null), isNull);
      final data = tapDataWithAction({'m': 'M1', 'sig': 'ab12'}, 'menu:2');
      expect(data['a'], 'menu');
      expect(data['i'], '2');
      expect(tapDataWithAction({'m': 'M1'}, 'menu:2').containsKey('a'), false);
      expect(
        tapDataWithAction({'m': 'M1', 'sig': 'ab12'}, null).containsKey('a'),
        false,
      );
    });
  });

  group('sealed push with a menu', () {
    test('builds a notice with menu, tag and sig payload', () async {
      final rig = await SealRig.create();
      final data = await rig.push('permission', {
        't': 'Claude, Fix',
        'b': 'Run it?',
        'p': 'p1',
        'menu': menuJson(),
      });
      final kv = MemorySecureKv();
      await kv.write(KeyStore.keyName, base64Url.encode(rig.devicePrivate));
      final notice = await pushNotice(
        data,
        key: () => readStaticKey(kv),
        machineKey: rig.machineKey,
        strings: pushStrings('en'),
      );
      expect(notice.sealed, isTrue);
      expect(notice.menu!.items, hasLength(3));
      expect(notice.tag, 'M1:p1');
      expect(pushTapData(notice.payload), {
        'm': 'M1',
        'p': 'p1',
        'sig': 'ab12cd34',
      });
    });

    test('a done push never carries a menu', () async {
      final rig = await SealRig.create();
      final data = await rig.push('done', {
        't': 'Claude',
        'b': 'Done',
        'p': 'p1',
        'menu': menuJson(),
      });
      final kv = MemorySecureKv();
      await kv.write(KeyStore.keyName, base64Url.encode(rig.devicePrivate));
      final notice = await pushNotice(
        data,
        key: () => readStaticKey(kv),
        machineKey: rig.machineKey,
        strings: pushStrings('en'),
      );
      expect(notice.menu, isNull);
    });

    test('a push without seal or menu keeps the old shape', () {
      final notice = fixedPushNotice({
        'm': 'M1',
        'k': 'question',
        'p': 'p1',
      }, pushStrings('en'));
      expect(notice.menu, isNull);
      expect(noticeActions(notice, 'Open'), isEmpty);
      expect(pushTapData(notice.payload), {'m': 'M1', 'p': 'p1'});
    });

    test('shows up to three actions that open the app', () async {
      final rig = await SealRig.create();
      final data = await rig.push('question', {
        't': 'Claude',
        'b': 'Q',
        'p': 'p1',
        'menu': menuJson(count: 4),
      });
      final kv = MemorySecureKv();
      await kv.write(KeyStore.keyName, base64Url.encode(rig.devicePrivate));
      final notice = await pushNotice(
        data,
        key: () => readStaticKey(kv),
        machineKey: rig.machineKey,
        strings: pushStrings('en'),
      );
      final actions = noticeActions(notice, 'Open');
      expect(actions.map((a) => a.id), ['menu:1', 'menu:2', 'menu:3']);
      expect(actions.every((a) => a.showsUserInterface), isTrue);
      expect(actions.map((a) => a.title), ['Choice 1', 'Choice 2', 'Choice 3']);
    });

    test('action tap becomes a pane link carrying sig and index', () {
      final data = tapDataWithAction({
        'm': 'M1',
        'p': 'p1',
        'sig': 'ab12cd34',
      }, 'menu:2');
      final uri = pushLink(data)!;
      expect(uri.toString(), 'orrery-remote://m/M1/p/p1?menu=ab12cd34&i=2');
      expect(
        parsePaneLink(uri),
        const PaneLink('M1', 'p1', menuSig: 'ab12cd34', menuIndex: 2),
      );
      expect(
        pushLink({'m': 'M1', 'p': 'p1', 'sig': 'ab12cd34'}).toString(),
        'orrery-remote://m/M1/p/p1',
      );
      expect(
        pushLink({'m': 'M1', 'p': 'p1', 'a': 'menu', 'sig': 'x y', 'i': '1'})
            .toString(),
        'orrery-remote://m/M1/p/p1',
      );
    });
  });

  group('running and done', () {
    test('running is ongoing with a chronometer from t0 and a 6h timeout', () {
      final notice = fixedPushNotice({
        'm': 'M1',
        'k': 'running',
        'p': 'p1',
        't0': '1764150000000',
      }, pushStrings('en'));
      expect(notice.ongoing, isTrue);
      expect(notice.title, 'Agent is working');
      final d = noticeDetails(
        notice,
        channelName: 'Running',
        openLabel: 'Open',
      );
      expect(d.channelId, runningChannel);
      expect(d.ongoing, isTrue);
      expect(d.usesChronometer, isTrue);
      expect(d.when, 1764150000000);
      expect(d.timeoutAfter, const Duration(hours: 6).inMilliseconds);
      expect(d.playSound, isFalse);
      expect(d.enableVibration, isFalse);
      expect(d.importance, Importance.low);
      expect(d.autoCancel, isFalse);
    });

    test('running without t0 has no chronometer', () {
      final notice = fixedPushNotice({
        'm': 'M1',
        'k': 'running',
        'p': 'p1',
      }, pushStrings('en'));
      expect(
        noticeDetails(notice, channelName: 'R', openLabel: 'O').usesChronometer,
        isFalse,
      );
    });

    test('done replaces running by tag and is a normal notice with Open', () {
      final running = fixedPushNotice({
        'm': 'M1',
        'k': 'running',
        'p': 'p1',
      }, pushStrings('en'));
      final done = fixedPushNotice({
        'm': 'M1',
        'k': 'done',
        'p': 'p1',
      }, pushStrings('en'));
      expect(done.tag, running.tag);
      expect(done.ongoing, isFalse);
      final d = noticeDetails(done, channelName: 'N', openLabel: 'Mở');
      expect(d.ongoing, isFalse);
      expect(d.channelId, attentionChannel);
      expect(d.actions!.single.id, openActionId);
      expect(d.actions!.single.showsUserInterface, isTrue);
      expect(d.actions!.single.title, 'Mở');
    });

    test('the Open kind opens the pane link', () {
      final notice = fixedPushNotice({
        'm': 'M1',
        'k': 'open',
        'p': 'p1',
      }, pushStrings('en'));
      expect(notice.kind, 'open');
      expect(notice.title, 'Opened from your computer');
      expect(
        pushLink(pushTapData(notice.payload)!).toString(),
        'orrery-remote://m/M1/p/p1',
      );
    });

    test(
      'a plain running push in the background is re-shown as ongoing',
      () async {
        final n = FakeNotifier();
        await handleBackgroundPush(
          const PushArrival({
            'm': 'M1',
            'k': 'running',
            'p': 'p1',
            't0': '1764150000000',
          }, shown: true),
          kv: MemorySecureKv(),
          machineKey: (_) async => null,
          notifier: n,
          languageCode: 'en',
        );
        expect(n.shown.single.ongoing, isTrue);
        final plainDone = FakeNotifier();
        await handleBackgroundPush(
          const PushArrival({'m': 'M1', 'k': 'done', 'p': 'p1'}, shown: true),
          kv: MemorySecureKv(),
          machineKey: (_) async => null,
          notifier: plainDone,
          languageCode: 'en',
        );
        expect(plainDone.shown, isEmpty);
      },
    );
  });

  group('clear and waiting labels', () {
    test('clear cancels that machine and shows nothing', () async {
      final gateway = FakeGateway();
      final sink = SinkRecorder();
      final rig = await SealRig.create();
      final p = await presenter(gateway, rig, MemorySecureKv(), sink: sink);
      final shown = await p.present(
        const PushArrival({'m': 'M1', 'k': 'clear', 'w': '0'}, shown: false),
      );
      expect(shown, isNull);
      expect(gateway.cleared, ['M1']);
      expect(gateway.shown, isEmpty);
      expect(sink.applied.single.count, 0);
    });

    test('background clear cancels notifications and shows nothing', () async {
      final n = FakeNotifier();
      await handleBackgroundPush(
        const PushArrival({'m': 'M1', 'k': 'clear', 'w': '0'}, shown: false),
        kv: MemorySecureKv(),
        machineKey: (_) async => null,
        notifier: n,
        languageCode: 'en',
      );
      expect(n.cleared, ['M1']);
      expect(n.shown, isEmpty);
    });

    test('labels inside the seal feed the waiting update', () async {
      final gateway = FakeGateway();
      final sink = SinkRecorder();
      final rig = await SealRig.create();
      final p = await presenter(gateway, rig, MemorySecureKv(), sink: sink);
      final data = await rig.push(
        'question',
        {
          't': 'Claude',
          'b': 'Q',
          'p': 'p1',
          'wl': ['Fix login', 'orrery'],
        },
        extra: {'w': '2'},
      );
      await p.present(PushArrival(data, shown: false));
      expect(sink.applied.single.count, 2);
      expect(sink.applied.single.labels, ['Fix login', 'orrery']);
    });

    test(
      'an open push shows even while the app is open on that machine',
      () async {
        final gateway = FakeGateway();
        final rig = await SealRig.create();
        final p = await presenter(gateway, rig, MemorySecureKv(), open: true);
        await p.present(
          const PushArrival({
            'm': 'M1',
            'k': 'question',
            'p': 'p1',
          }, shown: false),
        );
        expect(gateway.shown, isEmpty);
        await p.present(
          const PushArrival({
            'm': 'M1',
            'k': 'open',
            'p': 'p1',
            'w': '1',
          }, shown: false),
        );
        expect(gateway.shown.single.kind, 'open');
      },
    );

    test('an old push without the new fields behaves as before', () async {
      final gateway = FakeGateway();
      final rig = await SealRig.create();
      final p = await presenter(gateway, rig, MemorySecureKv());
      await p.present(
        const PushArrival({'m': 'M1', 'k': 'done', 'p': 'p1'}, shown: false),
      );
      final notice = gateway.shown.single;
      expect(notice.menu, isNull);
      expect(notice.startedAt, isNull);
      expect(notice.labels, isEmpty);
    });
  });

  group('runMenuAction', () {
    Future<List<(String, int)>> run(
      WidgetTester tester, {
      required PaneInfo? pane,
      Object? failWith,
      Future<void> Function(WidgetTester)? during,
    }) async {
      final sent = <(String, int)>[];
      late BuildContext context;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          home: Scaffold(
            body: Builder(
              builder: (c) {
                context = c;
                return const SizedBox();
              },
            ),
          ),
        ),
      );
      unawaited(
        runMenuAction(
          context: context,
          pane: pane,
          sig: 'sig1',
          index: 1,
          send: (sig, index) async {
            if (failWith != null) throw failWith;
            sent.add((sig, index));
          },
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      if (during != null) await during(tester);
      for (var i = 0; i < 16; i++) {
        await tester.pump(const Duration(milliseconds: 500));
      }
      return sent;
    }

    testWidgets('sends after the 3s undo window', (tester) async {
      final sent = await run(tester, pane: paneWith(remoteMenu('sig1')));
      expect(sent, [('sig1', 1)]);
    });

    testWidgets('Undo within the window sends nothing', (tester) async {
      final sent = await run(
        tester,
        pane: paneWith(remoteMenu('sig1')),
        during: (t) async {
          expect(find.text('Picked Yes'), findsOneWidget);
          await t.tap(find.text('Undo'));
          await t.pump();
        },
      );
      expect(sent, isEmpty);
    });

    testWidgets(
      'a changed menu sig shows the stale message and sends nothing',
      (tester) async {
        final sent = await run(
          tester,
          pane: paneWith(remoteMenu('other')),
          during: (t) async {
            expect(
              find.text('The question changed, open the pane to see it'),
              findsOneWidget,
            );
          },
        );
        expect(sent, isEmpty);
      },
    );

    testWidgets('no menu on the pane is stale too', (tester) async {
      final sent = await run(tester, pane: paneWith(null));
      expect(sent, isEmpty);
    });

    testWidgets('a stale error from the desktop shows the stale message', (
      tester,
    ) async {
      final sent = await run(
        tester,
        pane: paneWith(remoteMenu('sig1')),
        failWith: const RemoteCommandException('stale'),
      );
      expect(sent, isEmpty);
      expect(
        find.text('The question changed, open the pane to see it'),
        findsOneWidget,
      );
    });

    test('item lookup skips other, submit and multi-select entries', () {
      final pane = paneWith(remoteMenu('sig1'));
      expect(menuItemFor(pane, 'sig1', 1)!.label, 'Yes');
      expect(menuItemFor(pane, 'sig1', 3), isNull);
      expect(menuItemFor(pane, 'sig1', 9), isNull);
      expect(menuItemFor(pane, 'zzz', 1), isNull);
      expect(isStaleMenuError(const RemoteCommandException('busy')), isTrue);
      expect(isStaleMenuError(const RemoteCommandException('denied')), isFalse);
    });
  });
}
