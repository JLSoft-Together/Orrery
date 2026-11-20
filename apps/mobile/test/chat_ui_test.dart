import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/chat_protocol.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/ui/chat/chat_controller.dart';
import 'package:orrery_remote/ui/chat/chat_home_page.dart';
import 'package:orrery_remote/ui/chat/chat_screen.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/ui_fakes.dart';

Map<String, dynamic> menuOf() => {
  'sig': 'abcd',
  'kind': 'permission',
  'question': 'Do you want to proceed?',
  'items': [
    {'index': 1, 'label': 'Yes', 'selected': true},
    {'index': 2, 'label': 'No'},
  ],
  'numbered': true,
  'confident': true,
};

Map<String, dynamic> ws({String rail = 'idle', Map<String, dynamic>? menu}) {
  final a = pane('a', title: 'fix login', rail: rail);
  if (menu != null) a['menu'] = menu;
  return workspace([
    tab('t1', [
      a,
      pane('b', title: 'dark mode', rail: 'asked', agent: 'codex'),
      pane('s', title: 'dev server', agent: null),
    ], project: 'C:\\work\\api'),
  ]);
}

Map<String, dynamic> chatPage() => {
  'paneKey': 'a',
  'agent': 'claude',
  'gen': 'g1',
  'older': null,
  'items': [
    {'id': 'u1', 'k': 'me', 'seq': 1, 'rev': 0, 'text': 'Login fails with 500'},
    {
      'id': 't1',
      'k': 'tools',
      'seq': 2,
      'rev': 0,
      'sum': {'read': 2, 'run': 1},
      'items': [
        {'tool': 'Read', 'cat': 'read', 'target': 'auth/google.ts'},
      ],
    },
    {
      'id': 'a1',
      'k': 'ai',
      'seq': 3,
      'rev': 0,
      'text':
          'The `redirect_uri` lacks a **slash**.\n\n```ts\nconst a = 1;\n```',
    },
    {
      'id': 'f1',
      'k': 'files',
      'seq': 2,
      'rev': 0,
      'root': 'C:\\work\\api',
      'files': [
        {'path': 'auth/google.ts', 'add': 2, 'del': 1},
      ],
    },
  ],
};

Harness harness({
  Map<String, dynamic>? workspaceRaw,
  bool chat = true,
  FakeMachineApi? api,
}) {
  final fake = api ?? FakeMachineApi();
  fake.responder ??= (type, fields) {
    if (type == 'chat.watch') return chatPage();
    if (type == 'ws.send') return {'submitted': true};
    return {};
  };
  return Harness(
    api: fake,
    viewState: view(
      ws: workspaceRaw ?? ws(),
      snapshot: dashboard(
        terminals: [
          {'id': 't-a', 'lastActivityAt': '2026-11-20T09:00:00Z'},
        ],
      ),
      hello: chat ? {'chat': 1} : const {},
    ),
    linkState: linkReady,
  );
}

void main() {
  setUp(chatLogCache.clear);

  testWidgets('a desktop with chat opens the inbox, waiting first', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    expect(find.byType(ChatHomePage), findsOneWidget);
    expect(find.text('Waiting for you, 1'), findsOneWidget);
    expect(find.text('Recent'), findsOneWidget);
    final waitY = tester
        .getTopLeft(find.byKey(const ValueKey('chat-row-b')))
        .dy;
    final restY = tester
        .getTopLeft(find.byKey(const ValueKey('chat-row-a')))
        .dy;
    expect(waitY, lessThan(restY));
  });

  testWidgets('an old desktop keeps the classic workspace', (tester) async {
    final h = harness(chat: false);
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    expect(find.byType(ChatHomePage), findsNothing);
    expect(find.byType(WorkspaceShell), findsOneWidget);
  });

  testWidgets('tapping an agent row opens the chat with its items', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    await tester.tap(find.byKey(const ValueKey('chat-row-a')));
    await tester.pumpAndSettle();
    expect(find.byType(ChatScreen), findsOneWidget);
    expect(h.api.commands.first.$1, 'chat.watch');
    expect(find.text('Login fails with 500'), findsOneWidget);
    expect(find.text('Ran 1 command, read 2 files'), findsOneWidget);
    expect(find.text('Edited 1 file'), findsOneWidget);
    expect(find.byKey(const ValueKey('chat-code-copy')), findsOneWidget);
    final files = tester.getTopLeft(find.text('Edited 1 file')).dy;
    final answer = tester.getTopLeft(find.byKey(const ValueKey('chat-a1'))).dy;
    expect(files, greaterThan(answer));
  });

  testWidgets('a shell row goes straight to the terminal', (tester) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    await tester.tap(find.byKey(const ValueKey('chat-row-s')));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(find.byType(ChatScreen), findsNothing);
  });

  testWidgets('send shows the message at once and calls ws.send', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const ChatScreen(machineId: 'm1', paneKey: 'a'));
    await tester.pump();
    await tester.enterText(
      find.byKey(const ValueKey('chat-input')),
      'run the tests',
    );
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('chat-send')));
    await tester.pump();
    expect(find.text('run the tests'), findsOneWidget);
    expect(h.api.commands.last.$1, 'ws.send');
    expect(h.api.commands.last.$2, {'paneKey': 'a', 'text': 'run the tests'});
  });

  testWidgets('send scrolls a chat read higher up back to the newest', (
    tester,
  ) async {
    final long = List.generate(80, (i) => 'line $i').join('\n\n');
    final api = FakeMachineApi()
      ..responder = (type, fields) {
        if (type == 'chat.watch') {
          return {
            ...chatPage(),
            'items': [
              {'id': 'u1', 'k': 'me', 'seq': 1, 'rev': 0, 'text': 'hi'},
              {'id': 'a1', 'k': 'ai', 'seq': 2, 'rev': 0, 'text': long},
            ],
          };
        }
        if (type == 'ws.send') return {'submitted': true};
        return {};
      };
    final h = harness(api: api);
    await h.pump(tester, const ChatScreen(machineId: 'm1', paneKey: 'a'));
    await tester.pump();
    final list = find.descendant(
      of: find.byKey(const ValueKey('chat-list')),
      matching: find.byType(Scrollable),
    );
    await tester.drag(list, const Offset(0, 600));
    await tester.pumpAndSettle();
    final position = tester.state<ScrollableState>(list).position;
    expect(position.pixels, greaterThan(100));
    await tester.enterText(find.byKey(const ValueKey('chat-input')), 'next');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('chat-send')));
    await tester.pumpAndSettle();
    expect(position.pixels, 0);
  });

  testWidgets('a suggestion chip on an empty chat sends at once', (
    tester,
  ) async {
    final api = FakeMachineApi()
      ..responder = (type, fields) {
        if (type == 'chat.watch') {
          return {...chatPage(), 'items': <Object>[]};
        }
        if (type == 'ws.send') return {'submitted': true};
        return {};
      };
    final h = harness(api: api);
    await h.pump(tester, const ChatScreen(machineId: 'm1', paneKey: 'a'));
    await tester.pump();
    final chip = find.byKey(const ValueKey('chat-chip-Run the tests'));
    expect(chip, findsOneWidget);
    await tester.tap(chip);
    await tester.pump();
    expect(h.api.commands.last.$1, 'ws.send');
    expect(h.api.commands.last.$2, {'paneKey': 'a', 'text': 'Run the tests'});
    expect(find.byKey(const ValueKey('chat-chip-Run the tests')), findsNothing);
  });

  testWidgets('working shows the live line and Stop sends esc', (tester) async {
    final h = harness(workspaceRaw: ws(rail: 'working'));
    await h.pump(tester, const ChatScreen(machineId: 'm1', paneKey: 'a'));
    await tester.pump();
    expect(find.byKey(const ValueKey('chat-live')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('chat-stop')));
    await tester.pump();
    expect(h.api.commands.last.$1, 'ws.keys');
    expect(h.api.commands.last.$2!['keys'], 'stop');
  });

  testWidgets('a menu answer waits 3 s and can be undone', (tester) async {
    final h = harness(
      workspaceRaw: ws(rail: 'asked', menu: menuOf()),
    );
    await h.pump(tester, const ChatScreen(machineId: 'm1', paneKey: 'a'));
    await tester.pump();
    expect(find.text('Do you want to proceed?'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('chat-menu-1')));
    await tester.pump();
    expect(find.byKey(const ValueKey('chat-menu-pending')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('chat-menu-undo')));
    await tester.pump(const Duration(seconds: 4));
    expect(h.api.commands.where((c) => c.$1 == 'ws.menu'), isEmpty);
    await tester.tap(find.byKey(const ValueKey('chat-menu-2')));
    await tester.pump(const Duration(seconds: 2));
    expect(h.api.commands.where((c) => c.$1 == 'ws.menu'), isEmpty);
    await tester.pump(const Duration(seconds: 2));
    final sent = h.api.commands.where((c) => c.$1 == 'ws.menu').single;
    expect(sent.$2, {'paneKey': 'a', 'sig': 'abcd', 'index': 2});
    expect(find.text('Sent No'), findsOneWidget);
  });

  testWidgets('denied chat falls back to the terminal', (tester) async {
    final api = FakeMachineApi()
      ..responder = (type, fields) {
        if (type == 'chat.watch') {
          throw const RemoteCommandException('denied');
        }
        return {};
      };
    final h = harness(api: api);
    await h.pump(tester, const ChatScreen(machineId: 'm1', paneKey: 'a'));
    await tester.pump();
    expect(h.api.attached, ['a']);
    expect(find.byKey(const ValueKey('chat-term-toggle')), findsNothing);
  });

  test('ChatLimits proto matches hello', () {
    expect(ChatLimits.proto, 1);
  });
}
