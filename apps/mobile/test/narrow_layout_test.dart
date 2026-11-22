import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/chat/chat_controller.dart';
import 'package:orrery_remote/ui/chat/chat_screen.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/ui/workspace/inbox_page.dart';
import 'package:orrery_remote/ui/workspace/mission_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';
import 'package:orrery_remote/ui/model/workspace_model.dart';
import 'package:orrery_remote/ui/terminal/terminal_pane.dart';

import 'chat_ui_test.dart' as chat;
import 'support/ui_fakes.dart';

const Size narrow = Size(360, 740);

Map<String, dynamic> longPane(String key, {String? agent = 'claude'}) => {
  ...pane(
    key,
    title: 'fix the remote layout on narrow Android phones with big fonts',
    agent: agent,
    rail: 'asked',
    waitText: 'Do you want to make this edit to terminal_surface.dart?',
  ),
  'cwd': 'C:\\work\\web\\Orrery\\apps\\mobile\\lib\\ui\\terminal',
  'branch': 'feat/v98-pwa-very-long-branch-name-for-tests',
  'inbox': {'reason': 'permission'},
};

Map<String, dynamic> longWs() => {
  ...workspace([
    tab('t1', [
      longPane('a'),
      longPane('b', agent: 'codex'),
      longPane('s', agent: null),
    ], project: 'C:\\work\\web\\Orrery-with-a-long-project-folder-name'),
  ]),
  'inbox': true,
};

Map<String, dynamic> longChat() {
  final page = chat.chatPage();
  final items = List<Map<String, dynamic>>.of(
    (page['items'] as List).cast<Map<String, dynamic>>(),
  );
  items.add({
    'id': 'a2',
    'k': 'ai',
    'seq': 4,
    'rev': 0,
    'text':
        'Run https://orrery-relay.example.workers.dev/app/m/abcdefabcdef/c/p1?from=push&x=1 '
        'then\n\n```sh\nnpm run qa -- --no-e2e && JIN_RAW=1 npm test -w @ccm/pwa --reporter=dot\n```',
  });
  return {...page, 'items': items};
}

Harness _harness({bool chatOn = true}) {
  final api = FakeMachineApi();
  api.responder = (type, fields) {
    if (type == 'chat.watch') return longChat();
    return {};
  };
  return Harness(
    api: api,
    viewState: view(
      ws: longWs(),
      snapshot: dashboard(),
      hello: chatOn ? {'chat': 1} : const {},
    ),
    linkState: linkReady,
  );
}

Future<void> _check(
  WidgetTester tester,
  Widget home, {
  double scale = 1.3,
  double keyboard = 0,
  bool chatOn = true,
}) async {
  tester.platformDispatcher.textScaleFactorTestValue = scale;
  addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
  final h = _harness(chatOn: chatOn);
  await h.pump(tester, home, size: narrow);
  if (keyboard > 0) {
    tester.view.viewInsets = FakeViewPadding(bottom: keyboard);
    addTearDown(tester.view.resetViewInsets);
  }
  await tester.pump(const Duration(milliseconds: 400));
  expect(tester.takeException(), isNull);
}

void main() {
  setUp(chatLogCache.clear);

  for (final scale in [1.0, 1.3]) {
    testWidgets('home fits 360 dp at text scale $scale', (tester) async {
      await _check(tester, const HomePage(), scale: scale);
    });

    testWidgets('chat home fits 360 dp at text scale $scale', (tester) async {
      await _check(tester, const WorkspacePage(machineId: 'm1'), scale: scale);
    });

    testWidgets('classic workspace fits 360 dp at text scale $scale', (
      tester,
    ) async {
      await _check(
        tester,
        const WorkspacePage(machineId: 'm1'),
        scale: scale,
        chatOn: false,
      );
    });

    testWidgets('inbox fits 360 dp at text scale $scale', (tester) async {
      await _check(tester, const InboxPage(machineId: 'm1'), scale: scale);
    });

    testWidgets('mission control fits 360 dp at text scale $scale', (
      tester,
    ) async {
      await _check(tester, const MissionPage(machineId: 'm1'), scale: scale);
    });

    testWidgets('chat with long link and code fits 360 dp with the keyboard '
        'at text scale $scale', (tester) async {
      await _check(
        tester,
        const ChatScreen(machineId: 'm1', paneKey: 'a'),
        scale: scale,
        keyboard: 300,
      );
    });

    testWidgets('terminal pane fits 360 dp with the keyboard '
        'at text scale $scale', (tester) async {
      await _check(
        tester,
        Scaffold(
          body: TerminalPane(
            machineId: 'm1',
            tabKey: 't1',
            pane: PaneInfo.parse(longPane('a'))!,
          ),
        ),
        scale: scale,
        keyboard: 300,
      );
      expect(
        tester.getSize(find.byKey(const ValueKey('term-zoom'))).width,
        360,
      );
    });
  }

  testWidgets('inbox opens an agent pane in chat', (tester) async {
    final h = _harness();
    await h.pump(tester, const InboxPage(machineId: 'm1'), size: narrow);
    await tester.tap(find.textContaining('fix the remote layout').first);
    await tester.pumpAndSettle();
    expect(find.byType(ChatScreen), findsOneWidget);
  });

  testWidgets('mission control opens an agent pane in chat', (tester) async {
    final h = _harness();
    await h.pump(tester, const MissionPage(machineId: 'm1'), size: narrow);
    await tester.tap(find.textContaining('fix the remote layout').first);
    await tester.pumpAndSettle();
    expect(find.byType(ChatScreen), findsOneWidget);
  });
}
