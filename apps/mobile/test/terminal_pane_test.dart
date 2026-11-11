import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/ui/model/workspace_model.dart';
import 'package:orrery_remote/ui/terminal/terminal_pane.dart';
import 'package:xterm/xterm.dart';

import 'support/ui_fakes.dart';

String screen(WidgetTester tester) {
  final terminal = tester
      .widget<TerminalView>(find.byType(TerminalView))
      .terminal;
  final lines = terminal.buffer.lines;
  final out = StringBuffer();
  for (var i = 0; i < lines.length; i++) {
    out.writeln(lines[i].getText());
  }
  return out.toString();
}

Uint8List bytes(String text) => Uint8List.fromList(utf8.encode(text));

String sent(FakeMachineApi api) => api.inputs.map((b) => utf8.decode(b)).join();

PaneInfo paneInfo(Map<String, dynamic> raw) => PaneInfo.parse(raw)!;

Future<Harness> mount(
  WidgetTester tester, {
  Map<String, dynamic>? raw,
  bool online = true,
  Map<String, dynamic>? snapshot,
  FakeMachineApi? api,
}) async {
  final h = Harness(
    api: api ?? FakeMachineApi(),
    viewState: view(ws: workspace([]), snapshot: snapshot ?? dashboard()),
    linkState: online ? linkReady : linkOffline,
  );
  await h.pump(
    tester,
    Scaffold(
      body: TerminalPane(
        machineId: 'm1',
        tabKey: 't1',
        pane: paneInfo(raw ?? pane('a', title: 'agent')),
      ),
    ),
  );
  await tester.pump(const Duration(milliseconds: 50));
  return h;
}

void main() {
  testWidgets('attaches when shown and detaches when removed', (tester) async {
    final h = await mount(tester);
    expect(h.api.attached, ['a']);
    expect(h.api.detached, isEmpty);
    await tester.pumpWidget(const SizedBox());
    await tester.pump();
    expect(h.api.detached, ['0']);
  });

  testWidgets('output is drawn and pty.ready clears before the replay', (
    tester,
  ) async {
    final h = await mount(tester);
    final handle = h.api.handles.single;
    handle.out.add(bytes('hello world\r\n'));
    await tester.pump();
    expect(screen(tester), contains('hello world'));
    handle.ev.add({'t': 'pty.ready', 'stream': 1, 'cols': 100, 'rows': 30});
    await tester.pump();
    handle.out.add(bytes('replayed tiếng Việt\r\n'));
    await tester.pump();
    final text = screen(tester);
    expect(text, contains('replayed tiếng Việt'));
    expect(text, isNot(contains('hello world')));
  });

  testWidgets('a multibyte character split across chunks still decodes', (
    tester,
  ) async {
    final h = await mount(tester);
    final raw = utf8.encode('Việt');
    h.api.handles.single.out.add(Uint8List.fromList(raw.sublist(0, 3)));
    h.api.handles.single.out.add(Uint8List.fromList(raw.sublist(3)));
    await tester.pump();
    expect(screen(tester), contains('Việt'));
  });

  testWidgets('cached tail is drawn straight away while offline', (
    tester,
  ) async {
    final api = FakeMachineApi()..tail = bytes('last output\r\n');
    await mount(tester, online: false, api: api);
    expect(screen(tester), contains('last output'));
  });

  testWidgets('line input pastes the text and presses enter', (tester) async {
    final h = await mount(tester);
    await tester.enterText(find.byKey(const ValueKey('input')), 'xin chào');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('send')));
    await tester.pump();
    expect(sent(h.api), 'xin chào\r');
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('input')))
          .controller!
          .text,
      isEmpty,
    );
  });

  testWidgets('user keys claim the pty, terminal query replies are not sent', (
    tester,
  ) async {
    final h = await mount(tester);
    h.api.handles.single.out.add(bytes('[6n'));
    await tester.pump();
    expect(h.api.inputs, isEmpty);
    final replies = h.api.claims.length;
    await tester.enterText(find.byKey(const ValueKey('input')), 'ls');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('send')));
    await tester.pump();
    expect(h.api.claims.length, greaterThan(replies));
    expect(h.api.claims.skip(replies), everyElement(isTrue));
    await tester.tap(find.byKey(const ValueKey('key-esc')));
    await tester.pump();
    expect(h.api.claims.last, isTrue);
  });

  testWidgets('extra keys send their escape sequences', (tester) async {
    final h = await mount(tester);
    Future<String> press(String id) async {
      h.api.inputs.clear();
      await tester.ensureVisible(find.byKey(ValueKey('key-$id')));
      await tester.tap(find.byKey(ValueKey('key-$id')));
      await tester.pump();
      return sent(h.api);
    }

    expect(await press('esc'), '\x1b');
    expect(await press('tab'), '\t');
    expect(await press('shift-tab'), '\x1b[Z');
    expect(await press('ctrl-c'), '\x03');
    expect(await press('up'), '\x1b[A');
    expect(await press('enter'), '\r');
  });

  testWidgets('focusing the input switches to the phone size', (tester) async {
    final h = await mount(tester);
    expect(h.api.resizes, isEmpty);
    await tester.tap(find.byKey(const ValueKey('input')));
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.text('You are typing'), findsOneWidget);
    expect(h.api.resizes, isNotEmpty);
    expect(h.api.resizes.last.$1, lessThan(80));
  });

  testWidgets('view only keeps the desktop size and scrolls sideways', (
    tester,
  ) async {
    final h = await mount(tester);
    h.api.handles.single.ev.add({
      't': 'pty.ready',
      'stream': 1,
      'cols': 160,
      'rows': 40,
    });
    await tester.pump();
    final terminal = tester
        .widget<TerminalView>(find.byType(TerminalView))
        .terminal;
    expect(terminal.viewWidth, 160);
    expect(find.text('View only'), findsOneWidget);
    expect(find.byType(SingleChildScrollView), findsWidgets);
  });

  testWidgets('permission bar answers with ws.keys and the statusSince', (
    tester,
  ) async {
    final snapshot = dashboard(
      terminals: [
        {'id': 't-a', 'statusSince': '2026-11-12T08:59:00.000Z'},
      ],
    );
    final h = await mount(
      tester,
      raw: pane('a', rail: 'asked', waitText: 'Bash: ls'),
      snapshot: snapshot,
    );
    expect(find.text('Bash: ls'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('answer-approve')));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('answer-2')));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('answer-deny')));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('answer-esc')));
    await tester.pump();
    final since = DateTime.parse('2026-11-12T08:59:00.000Z')
        .millisecondsSinceEpoch;
    expect(h.api.commands.map((c) => c.$1).toSet(), {'ws.keys'});
    expect(h.api.commands.map((c) => c.$2!['keys']), [
      'approve',
      '2',
      'deny',
      'esc',
    ]);
    expect(h.api.commands.every((c) => c.$2!['since'] == since), isTrue);
    expect(h.api.commands.every((c) => c.$2!['paneKey'] == 'a'), isTrue);
  });

  testWidgets('permission bar is hidden unless the pane asked', (tester) async {
    await mount(tester);
    expect(find.byKey(const ValueKey('answer-approve')), findsNothing);
  });

  testWidgets('offline disables typing and keys', (tester) async {
    final h = await mount(tester, online: false);
    expect(h.api.attached, ['a']);
    expect(
      tester.widget<TextField>(find.byKey(const ValueKey('input'))).enabled,
      isFalse,
    );
    final key = tester.widget<ButtonStyleButton>(
      find.byKey(const ValueKey('key-esc')),
    );
    expect(key.onPressed, isNull);
    expect(h.api.inputs, isEmpty);
  });

  testWidgets(
    'going offline mid-typing disables the field but keeps the text',
    (tester) async {
      final h = await mount(tester);
      await tester.enterText(find.byKey(const ValueKey('input')), 'half typed');
      await tester.pump();
      h.linkNotifier.emit(linkOffline);
      await tester.pump();
      final field = tester.widget<TextField>(
        find.byKey(const ValueKey('input')),
      );
      expect(field.enabled, isFalse);
      expect(field.controller!.text, 'half typed');
      final send = tester.widget<IconButton>(
        find.byKey(const ValueKey('send')),
      );
      expect(send.onPressed, isNull);
      expect(h.api.inputs, isEmpty);
    },
  );

  testWidgets('attach limit error is explained', (tester) async {
    final api = FakeMachineApi()
      ..attachError = const RemoteCommandException('limit');
    await mount(tester, api: api);
    expect(
      find.text('Only 4 terminals can be watched at once. Close one first.'),
      findsOneWidget,
    );
  });

  testWidgets('pty.exit shows the end of the session and stops typing', (
    tester,
  ) async {
    final h = await mount(tester);
    h.api.handles.single.ev.add({'t': 'pty.exit', 'stream': 1, 'code': 2});
    await tester.pump();
    expect(find.text('Session ended (code 2)'), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byKey(const ValueKey('input'))).enabled,
      isFalse,
    );
  });

  testWidgets('pty.exit without a code means the terminal was lost', (
    tester,
  ) async {
    final h = await mount(tester);
    h.api.handles.single.ev.add({'t': 'pty.exit', 'stream': 1, 'code': null});
    await tester.pump();
    expect(find.text('Lost the terminal on the computer'), findsOneWidget);
  });

  testWidgets('non terminal panes say so and never attach', (tester) async {
    final h = await mount(tester, raw: pane('f', kind: 'file', agent: null));
    expect(h.api.attached, isEmpty);
    expect(
      find.text('This pane is not a terminal. Open it on the computer.'),
      findsOneWidget,
    );
  });

  testWidgets('header shows the running tool with elapsed time', (
    tester,
  ) async {
    final snapshot = dashboard(
      agents: [
        {
          'terminalId': 't-a',
          'role': 'main',
          'currentTool': 'Bash',
          'currentToolSummary': 'npm test',
          'currentToolStartedAt': DateTime.now()
              .subtract(const Duration(seconds: 42))
              .toUtc()
              .toIso8601String(),
        },
      ],
    );
    await mount(
      tester,
      raw: pane('a', rail: 'working'),
      snapshot: snapshot,
    );
    expect(find.textContaining('Bash · npm test · '), findsOneWidget);
    expect(find.text('Running'), findsOneWidget);
  });
}
