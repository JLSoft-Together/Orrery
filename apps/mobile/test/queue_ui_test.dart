import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/ui/chat/chat_home_page.dart';
import 'package:orrery_remote/ui/queue/queue_nudge.dart';
import 'package:orrery_remote/ui/queue/queue_page.dart';
import 'package:orrery_remote/ui/workspace/inbox_page.dart';

import 'support/ui_fakes.dart';

Map<String, dynamic> menuOf(String sig, {String question = 'Proceed?'}) => {
  'sig': sig,
  'kind': 'ask',
  'question': question,
  'items': [
    {'index': 1, 'label': 'Yes'},
    {'index': 2, 'label': 'No'},
  ],
  'numbered': true,
  'confident': true,
};

Map<String, dynamic> asked(
  String key, {
  String? since,
  Map<String, dynamic>? menu,
  String? question,
  String rail = 'asked',
  bool snoozed = false,
}) {
  final p = pane(key, title: 'pane $key', rail: rail, attention: 'waiting');
  if (menu != null) p['menu'] = menu;
  if (question != null) p['question'] = question;
  if (since != null) p['inbox'] = {'reason': 'question', 'since': since};
  if (snoozed) p['snoozed'] = true;
  return p;
}

Map<String, dynamic> wsOf(List<Map<String, dynamic>> panes) => workspace([
  tab('t1', panes, project: 'C:\\work\\api'),
]);

Harness harness(
  List<Map<String, dynamic>> panes, {
  bool cap = true,
  List<String> danger = const [],
  FakeMachineApi? api,
}) {
  final fake = api ?? FakeMachineApi();
  fake.responder ??= (type, fields) {
    if (type == 'ws.snooze') {
      final m = fields?['minutes'];
      return m == null ? {'ok': true, 'minutes': null} : {'ok': true, 'minutes': m};
    }
    return {};
  };
  return Harness(
    api: fake,
    viewState: view(
      ws: wsOf(panes),
      hello: {
        'caps': [if (cap) 'queue'],
        if (danger.isNotEmpty) 'dangerRules': danger,
      },
    ),
    linkState: linkReady,
  );
}

const QueuePage page = QueuePage(machineId: 'm1');

Future<void> pumpPage(
  Harness h,
  WidgetTester tester, {
  Widget home = page,
}) async {
  await h.pump(tester, home);
  await tester.pump();
}

Future<void> settleSnack(WidgetTester tester) async {
  await tester.pump(const Duration(seconds: 3));
  await tester.pump(const Duration(milliseconds: 400));
  await tester.pump();
  await tester.pump();
}

Iterable<(String, JsonMap?)> sent(Harness h, String type) =>
    h.api.commands.where((c) => c.$1 == type);

void main() {
  testWidgets('the oldest wait is the first card and the counter counts', (
    tester,
  ) async {
    final h = harness([
      asked('late', since: '2026-11-27T10:30:00Z', menu: menuOf('s1')),
      asked('old', since: '2026-11-27T09:00:00Z', menu: menuOf('s2')),
    ]);
    await pumpPage(h, tester);
    expect(find.text('pane old'), findsOneWidget);
    expect(find.text('pane late'), findsNothing);
    expect(find.byKey(const ValueKey('queue-counter')), findsOneWidget);
    expect(find.text('1 / 2'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-hint')), findsOneWidget);
  });

  testWidgets('a menu answer waits 3 s, undo returns the card, then sends', (
    tester,
  ) async {
    final h = harness([
      asked('a', since: '2026-11-27T09:00:00Z', menu: menuOf('s1')),
      asked('b', since: '2026-11-27T10:00:00Z', menu: menuOf('s2')),
    ]);
    await pumpPage(h, tester);
    await tester.tap(find.byKey(const ValueKey('queue-menu-1')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byKey(const ValueKey('queue-pending')), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-menu-1')), findsNothing);
    await tester.tap(find.text('Undo'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump(const Duration(seconds: 1));
    expect(sent(h, 'ws.menu'), isEmpty);
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
    expect(find.text('pane a'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('queue-menu-2')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump(const Duration(seconds: 2));
    expect(sent(h, 'ws.menu'), isEmpty);
    await settleSnack(tester);
    final call = sent(h, 'ws.menu').single;
    expect(call.$2, {'paneKey': 'a', 'sig': 's1', 'index': 2});
    await tester.pump(const Duration(seconds: 1));
    expect(find.text('pane b'), findsOneWidget);
    expect(find.text('2 / 2'), findsOneWidget);
  });

  testWidgets('a stale answer from the desktop turns the card stale', (
    tester,
  ) async {
    final api = FakeMachineApi()
      ..responder = (type, fields) {
        if (type == 'ws.menu') throw const RemoteCommandException('stale');
        return {};
      };
    final h = harness([asked('a', menu: menuOf('s1'))], api: api);
    await pumpPage(h, tester);
    await tester.tap(find.byKey(const ValueKey('queue-menu-1')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await settleSnack(tester);
    expect(find.byKey(const ValueKey('queue-stale')), findsOneWidget);
    expect(find.text('The question changed'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-open-pane')), findsOneWidget);
  });

  testWidgets('a changed menu while open shows stale, reload shows the new one', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(h, tester);
    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', menu: menuOf('s2', question: 'Other?'))]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-stale')), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-menu-1')), findsNothing);
    await tester.tap(find.byKey(const ValueKey('queue-reload')));
    await tester.pump();
    await tester.pump();
    expect(find.text('Other?'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
  });

  testWidgets('a pane answered on the desktop shows gone and leaves in 1.5 s', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1')), asked('b', menu: menuOf('s2'))]);
    await pumpPage(h, tester);
    h.viewNotifier.emit(
      view(
        ws: wsOf([
          asked('a', rail: 'working'),
          asked('b', menu: menuOf('s2')),
        ]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-gone')), findsOneWidget);
    expect(find.text('Answered on the computer'), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 1400));
    expect(find.byKey(const ValueKey('queue-gone')), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 200));
    await tester.pump();
    expect(find.text('pane b'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-gone')), findsNothing);
  });

  testWidgets('skip moves on, the end offers to review skipped cards', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1')), asked('b', menu: menuOf('s2'))]);
    await pumpPage(h, tester);
    await tester.tap(find.byKey(const ValueKey('queue-skip')));
    await tester.pump();
    await tester.pump();
    expect(find.text('pane b'), findsOneWidget);
    expect(h.api.commands, isEmpty);
    await tester.tap(find.byKey(const ValueKey('queue-skip')));
    await tester.pump();
    await tester.pump();
    expect(find.text('All done, nothing is waiting for you'), findsOneWidget);
    expect(find.text('Skipped 2'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('queue-review')));
    await tester.pump();
    await tester.pump();
    expect(find.text('pane a'), findsOneWidget);
  });

  testWidgets('swipe left skips and swipe right snoozes 15 minutes', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1')), asked('b', menu: menuOf('s2'))]);
    await pumpPage(h, tester);
    await tester.drag(find.byKey(const ValueKey('queue-card')), const Offset(-400, 0));
    await tester.pump();
    await tester.pump();
    expect(find.text('pane b'), findsOneWidget);
    expect(h.api.commands, isEmpty);
    await tester.drag(find.byKey(const ValueKey('queue-card')), const Offset(400, 0));
    await tester.pump();
    await tester.pump();
    final call = sent(h, 'ws.snooze').single;
    expect(call.$2, {'paneKey': 'b', 'minutes': 15});
  });

  testWidgets('snooze sends ws.snooze, shows undo 5 s, then moves on', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1')), asked('b', menu: menuOf('s2'))]);
    await pumpPage(h, tester);
    await tester.tap(find.byKey(const ValueKey('queue-snooze')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    await tester.tap(find.byKey(const ValueKey('queue-snooze-60')));
    await tester.pump();
    await tester.pump();
    expect(sent(h, 'ws.snooze').single.$2, {'paneKey': 'a', 'minutes': 60});
    expect(find.byKey(const ValueKey('queue-snoozed')), findsOneWidget);
    expect(find.text('Snoozed for 60 minutes'), findsOneWidget);
    await tester.pump(const Duration(seconds: 4));
    expect(find.byKey(const ValueKey('queue-undo-snooze')), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    expect(find.text('pane b'), findsOneWidget);
  });

  testWidgets('undo of a snooze sends minutes null and brings the card back', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(h, tester);
    await tester.tap(find.byKey(const ValueKey('queue-snooze')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    await tester.tap(find.byKey(const ValueKey('queue-snooze-15')));
    await tester.pump();
    await tester.pump();
    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', menu: menuOf('s1'), snoozed: true)]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('queue-undo-snooze')));
    await tester.pump();
    await tester.pump();
    expect(sent(h, 'ws.snooze').last.$2, {'paneKey': 'a', 'minutes': null});
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
    await tester.pump(const Duration(seconds: 6));
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
  });

  testWidgets('undo after the pane stopped waiting ends as gone', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(h, tester);
    await tester.tap(find.byKey(const ValueKey('queue-snooze')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    await tester.tap(find.byKey(const ValueKey('queue-snooze-15')));
    await tester.pump();
    await tester.pump();
    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', rail: 'working')]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-snoozed')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('queue-undo-snooze')));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-gone')), findsOneWidget);
  });

  testWidgets('a text question has a reply box that sends ws.reply', (
    tester,
  ) async {
    final h = harness([
      asked('a', question: 'Which branch?'),
      asked('b', menu: menuOf('s2')),
    ]);
    await pumpPage(h, tester);
    expect(find.text('Which branch?'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-reply')), findsOneWidget);
    final send = find.byKey(const ValueKey('queue-send'));
    await tester.enterText(find.byKey(const ValueKey('queue-reply')), 'main');
    await tester.pump();
    await tester.tap(send);
    await tester.pump();
    await tester.pump();
    expect(sent(h, 'ws.reply').single.$2, {'paneKey': 'a', 'text': 'main'});
    expect(find.byKey(const ValueKey('queue-sent')), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pump();
    expect(find.text('pane b'), findsOneWidget);
  });

  testWidgets('a dangerous reply is marked authed after the device check', (
    tester,
  ) async {
    final h = harness([asked('a', question: 'Run?')], danger: ['rm -rf']);
    await pumpPage(h, tester);
    await tester.enterText(find.byKey(const ValueKey('queue-reply')), 'rm -rf /');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('queue-send')));
    await tester.pump();
    await tester.pump();
    expect(sent(h, 'ws.reply').single.$2, {
      'paneKey': 'a',
      'text': 'rm -rf /',
      'authed': true,
    });
  });

  testWidgets('a reply refused with useMenu turns the card stale', (
    tester,
  ) async {
    final api = FakeMachineApi()
      ..responder = (type, fields) {
        if (type == 'ws.reply') throw const RemoteCommandException('useMenu');
        return {};
      };
    final h = harness([asked('a', question: 'Name?')], api: api);
    await pumpPage(h, tester);
    await tester.enterText(find.byKey(const ValueKey('queue-reply')), 'x');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('queue-send')));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-stale')), findsOneWidget);
  });

  testWidgets('a reply refused with notWaiting turns the card gone', (
    tester,
  ) async {
    final api = FakeMachineApi()
      ..responder = (type, fields) {
        if (type == 'ws.reply') {
          throw const RemoteCommandException('notWaiting');
        }
        return {};
      };
    final h = harness([asked('a', question: 'Name?')], api: api);
    await pumpPage(h, tester);
    await tester.enterText(find.byKey(const ValueKey('queue-reply')), 'x');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('queue-send')));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-gone')), findsOneWidget);
  });

  testWidgets('a reply that times out is unsure and is not resent by itself', (
    tester,
  ) async {
    var fail = true;
    final api = FakeMachineApi()
      ..responder = (type, fields) {
        if (type == 'ws.reply' && fail) {
          throw const RemoteCommandException('timeout');
        }
        return {};
      };
    final h = harness([asked('a', question: 'Name?')], api: api);
    await pumpPage(h, tester);
    await tester.enterText(find.byKey(const ValueKey('queue-reply')), 'x');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('queue-send')));
    await tester.pump();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byKey(const ValueKey('queue-unsure')), findsOneWidget);
    expect(sent(h, 'ws.reply'), hasLength(1));
    expect(find.byKey(const ValueKey('queue-send')), findsOneWidget);

    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', rail: 'working')]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byKey(const ValueKey('queue-gone')), findsOneWidget);
    expect(sent(h, 'ws.reply'), hasLength(1));
    fail = false;
  });

  testWidgets('tapping send twice during the danger check sends once', (
    tester,
  ) async {
    final h = harness([asked('a', question: 'Run?')], danger: ['rm -rf']);
    await pumpPage(h, tester);
    await tester.enterText(find.byKey(const ValueKey('queue-reply')), 'rm -rf /');
    await tester.pump();
    final send = find.byKey(const ValueKey('queue-send'));
    await tester.tap(send);
    await tester.tap(send, warnIfMissed: false);
    await tester.pump();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(sent(h, 'ws.reply'), hasLength(1));
  });

  testWidgets('leaving the queue page drops a pending menu answer', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(
      h,
      tester,
      home: Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            key: const ValueKey('open-queue'),
            onPressed: () => openQueue(context, 'm1'),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('open-queue')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('queue-menu-1')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byKey(const ValueKey('queue-pending')), findsOneWidget);
    await tester.pageBack();
    await tester.pump();
    await tester.pump(const Duration(seconds: 4));
    await tester.pump(const Duration(milliseconds: 400));
    expect(sent(h, 'ws.menu'), isEmpty);
  });

  testWidgets('a snoozed card that expires brings the pane back to the queue', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(h, tester);
    await tester.tap(find.byKey(const ValueKey('queue-snooze')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    await tester.tap(find.byKey(const ValueKey('queue-snooze-15')));
    await tester.pump();
    await tester.pump();
    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', menu: menuOf('s1'), snoozed: true)]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(seconds: 6));
    await tester.pump();
    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', menu: menuOf('s1'))]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
  });

  testWidgets('a question that changes while typing keeps the draft, stale', (
    tester,
  ) async {
    final h = harness([asked('a', question: 'Name?')]);
    await pumpPage(h, tester);
    await tester.enterText(find.byKey(const ValueKey('queue-reply')), 'draft');
    await tester.pump();
    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', question: 'Which file?')]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-stale')), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-send')), findsNothing);
    await tester.tap(find.byKey(const ValueKey('queue-reload')));
    await tester.pump();
    await tester.pump();
    expect(find.text('Which file?'), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byKey(const ValueKey('queue-reply'))).controller!.text,
      'draft',
    );
  });

  testWidgets('without the queue cap there is no snooze and no reply box', (
    tester,
  ) async {
    final h = harness([asked('a', question: 'Name?')], cap: false);
    await pumpPage(h, tester);
    expect(find.byKey(const ValueKey('queue-reply')), findsNothing);
    expect(find.byKey(const ValueKey('queue-snooze')), findsNothing);
    expect(find.text('Choose in the pane'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-open-pane')), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-skip')), findsOneWidget);
  });

  testWidgets('without the cap menu buttons still answer', (tester) async {
    final h = harness([asked('a', menu: menuOf('s1'))], cap: false);
    await pumpPage(h, tester);
    expect(find.byKey(const ValueKey('queue-snooze')), findsNothing);
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
  });

  testWidgets('a menu with only checkbox and other items has no choice buttons', (
    tester,
  ) async {
    final menu = menuOf('s1');
    menu['items'] = [
      {'index': 1, 'label': 'A', 'checked': false},
      {'index': 2, 'label': 'Other', 'other': true},
    ];
    final h = harness([asked('a', menu: menu)]);
    await pumpPage(h, tester);
    expect(find.byKey(const ValueKey('queue-menu-1')), findsNothing);
    expect(find.text('Choose in the pane'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-reply')), findsNothing);
  });

  testWidgets('an empty queue says all done and goes back', (tester) async {
    final h = harness([asked('a', rail: 'idle')]);
    await pumpPage(h, tester);
    expect(find.text('All done, nothing is waiting for you'), findsOneWidget);
    expect(find.byKey(const ValueKey('queue-back')), findsOneWidget);
  });

  testWidgets('buttons are at least 48 dp tall', (tester) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(h, tester);
    for (final key in ['queue-menu-1', 'queue-skip', 'queue-open-pane', 'queue-snooze']) {
      expect(
        tester.getSize(find.byKey(ValueKey(key))).height,
        greaterThanOrEqualTo(48),
        reason: key,
      );
    }
  });

  testWidgets('offline before the first sync shows a spinner, not all done', (
    tester,
  ) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(
        ws: wsOf([asked('a', menu: menuOf('s1'))]),
        hello: {
          'caps': ['queue'],
        },
      ),
      linkState: linkOffline,
    );
    await pumpPage(h, tester);
    expect(find.byType(CircularProgressIndicator), findsWidgets);
    expect(find.text('All done, nothing is waiting for you'), findsNothing);
    h.linkNotifier.emit(linkReady);
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-menu-1')), findsOneWidget);
  });

  testWidgets('losing the connection mid card keeps it and disables answers', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(h, tester);
    h.linkNotifier.emit(linkOffline);
    await tester.pump();
    await tester.pump();
    h.viewNotifier.emit(
      view(
        ws: wsOf([asked('a', rail: 'idle')]),
        stale: true,
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    await tester.pump();
    final button = find.byKey(const ValueKey('queue-menu-1'));
    expect(button, findsOneWidget);
    expect(tester.widget<AnimatedOpacity>(find.descendant(
      of: button,
      matching: find.byType(AnimatedOpacity),
    )).opacity, 0.45);
    await tester.tap(button, warnIfMissed: false);
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-pending')), findsNothing);
    expect(find.byKey(const ValueKey('queue-gone')), findsNothing);
    expect(h.api.commands, isEmpty);
  });

  testWidgets('the inbox offers the queue and opens it', (tester) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(h, tester, home: const InboxPage(machineId: 'm1'));
    await tester.tap(find.byKey(const ValueKey('inbox-queue')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byType(QueuePage), findsOneWidget);
  });

  testWidgets('the machine screen shows Waiting for you (N) and opens it', (
    tester,
  ) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(
        ws: wsOf([
          asked('a', menu: menuOf('s1')),
          asked('b', menu: menuOf('s2')),
        ]),
        snapshot: dashboard(),
        hello: {
          'chat': 1,
          'caps': ['queue'],
        },
      ),
      linkState: linkReady,
    );
    await pumpPage(h, tester, home: const ChatHomePage(machineId: 'm1'));
    expect(find.text('Waiting for you (2)'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('home-queue')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byType(QueuePage), findsOneWidget);
  });

  testWidgets('the nudge shows how many others wait and opens the queue', (
    tester,
  ) async {
    final h = harness([
      asked('a', menu: menuOf('s1')),
      asked('b', menu: menuOf('s2')),
      asked('c', menu: menuOf('s3')),
    ]);
    await pumpPage(
      h,
      tester,
      home: const Scaffold(
        body: QueueNudge(machineId: 'm1', paneKey: 'a', child: SizedBox.expand()),
      ),
    );
    expect(find.text('2 more waiting'), findsOneWidget);
    await tester.tap(find.text('2 more waiting'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byType(QueuePage), findsOneWidget);
  });

  testWidgets('the nudge is hidden when nothing else waits, or after Hide', (
    tester,
  ) async {
    final h = harness([asked('a', menu: menuOf('s1'))]);
    await pumpPage(
      h,
      tester,
      home: const Scaffold(
        body: QueueNudge(machineId: 'm1', paneKey: 'a', child: SizedBox.expand()),
      ),
    );
    expect(find.byKey(const ValueKey('queue-nudge')), findsNothing);
    h.viewNotifier.emit(
      view(
        ws: wsOf([
          asked('a', menu: menuOf('s1')),
          asked('b', menu: menuOf('s2')),
        ]),
        hello: {
          'caps': ['queue'],
        },
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-nudge')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('queue-nudge-hide')));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('queue-nudge')), findsNothing);
  });
}
