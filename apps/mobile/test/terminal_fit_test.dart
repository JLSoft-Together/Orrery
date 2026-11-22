import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/terminal/terminal_pane.dart';
import 'package:orrery_remote/ui/terminal/terminal_surface.dart';
import 'package:xterm/xterm.dart';

import 'support/ui_fakes.dart';
import 'terminal_pane_test.dart' show bytes, paneInfo, screen;

Terminal _terminal(WidgetTester tester) =>
    tester.widget<TerminalView>(find.byType(TerminalView)).terminal;

Future<Harness> _mount(
  WidgetTester tester, {
  double textScale = 1,
  Size size = const Size(360, 740),
}) async {
  final h = Harness(
    api: FakeMachineApi(),
    viewState: view(ws: workspace([]), snapshot: dashboard()),
    linkState: linkReady,
  );
  await h.pump(
    tester,
    Builder(
      builder: (context) => MediaQuery(
        data: MediaQuery.of(context)
            .copyWith(textScaler: TextScaler.linear(textScale)),
        child: Scaffold(
          body: TerminalPane(
            machineId: 'm1',
            tabKey: 't1',
            pane: paneInfo(pane('a', title: 'agent')),
          ),
        ),
      ),
    ),
    size: size,
  );
  await tester.pump(const Duration(milliseconds: 50));
  return h;
}

void _desk(FakeHandle handle, int cols, int rows, String data) {
  handle.ev.add({'t': 'pty.resized', 'stream': 1, 'cols': cols, 'rows': rows});
  handle.ev.add({
    't': 'pty.snapshot',
    'stream': 1,
    'cols': cols,
    'rows': rows,
    'gen': 1,
    'data': data,
  });
}

Size _cell(WidgetTester tester) {
  final view = tester.widget<TerminalView>(find.byType(TerminalView));
  final context = tester.element(find.byType(TerminalView));
  final scaler = view.textScaler ?? MediaQuery.textScalerOf(context);
  final painter = TextPainter(
    text: TextSpan(text: 'mmmmmmmmmm', style: view.textStyle.toTextStyle()),
    textScaler: scaler,
    textDirection: TextDirection.ltr,
  )..layout();
  final size = Size(painter.width / 10, painter.height);
  painter.dispose();
  return size;
}

double _scale(WidgetTester tester) => tester
    .widget<InteractiveViewer>(find.byKey(const ValueKey('term-zoom')))
    .transformationController!
    .value
    .getMaxScaleOnAxis();

void _fitsWidth(WidgetTester tester, int cols) {
  final width = tester.getSize(find.byType(TerminalView)).width;
  final room = width - terminalPadding.horizontal;
  final used = cols * _cell(tester).width;
  expect(width, 360);
  expect(used, lessThanOrEqualTo(room + 0.01));
  expect(used, greaterThan(room * 0.9));
}

void main() {
  testWidgets('a 180 column desktop terminal shrinks to the screen width, '
      'no sideways scroll', (tester) async {
    final h = await _mount(tester);
    _desk(h.api.handles.single, 180, 50, 'wide');
    await tester.pump();
    expect(_terminal(tester).viewWidth, 180);
    _fitsWidth(tester, 180);
    expect(
      find.descendant(
        of: find.byType(TerminalPane),
        matching: find.byWidgetPredicate(
          (w) =>
              w is SingleChildScrollView &&
              w.scrollDirection == Axis.horizontal,
        ),
      ),
      findsNothing,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('typing keeps the desktop size and never sends pty.resize', (
    tester,
  ) async {
    final h = await _mount(tester);
    final handle = h.api.handles.single;
    _desk(handle, 120, 30, '\x1b[mline-a\x1b[m\r\n\x1b[mline-b');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('input')));
    await tester.pump(const Duration(milliseconds: 300));
    await tester.enterText(find.byKey(const ValueKey('input')), 'ls');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('send')));
    await tester.pump(const Duration(milliseconds: 300));
    expect(_terminal(tester).viewWidth, 120);
    expect(_terminal(tester).viewHeight, 30);
    _fitsWidth(tester, 120);
    expect(screen(tester), contains('line-a'));
    expect(h.api.inputs, isNotEmpty);
    expect(h.api.commands.map((c) => c.$1), isNot(contains('pty.resize')));
  });

  testWidgets('a Claude box redrawn by the snapshot after a resize keeps '
      'every line whole', (tester) async {
    final h = await _mount(tester);
    final handle = h.api.handles.single;
    String box(int w) {
      final bar = '─' * (w - 2);
      final mid = '│ hello${' ' * (w - 9)}│';
      return '\x1b[m╭$bar╮\x1b[m\r\n\x1b[m$mid\x1b[m\r\n\x1b[m╰$bar╯';
    }

    _desk(handle, 120, 10, box(120));
    await tester.pump();
    handle.out.add(bytes('\r\n> typed at the old size'));
    _desk(handle, 40, 10, '${box(40)}\x1b[m\r\n\x1b[m> ok\x1b[4;5H');
    handle.out.add(bytes('!'));
    await tester.pump();
    final lines = screen(tester)
        .split('\n')
        .map((s) => s.trimRight())
        .where((s) => s.isNotEmpty);
    expect(lines.toList(), [
      '╭${'─' * 38}╮',
      '│ hello${' ' * 31}│',
      '╰${'─' * 38}╯',
      '> ok!',
    ]);
    expect(_terminal(tester).viewWidth, 40);
  });

  testWidgets('big system font still fits the desktop width', (tester) async {
    final h = await _mount(tester, textScale: 1.6);
    _desk(h.api.handles.single, 100, 30, 'x' * 100);
    await tester.pump();
    _fitsWidth(tester, 100);
    expect(tester.takeException(), isNull);
  });

  testWidgets('two finger pinch zooms in, double tap goes back to the '
      'screen width', (tester) async {
    final h = await _mount(tester);
    _desk(h.api.handles.single, 180, 50, 'wide');
    await tester.pump();
    expect(_scale(tester), 1);
    final center = tester.getCenter(find.byType(TerminalView));
    final a = await tester.startGesture(center - const Offset(20, 0));
    final b = await tester.startGesture(center + const Offset(20, 0));
    for (var i = 1; i <= 10; i++) {
      await a.moveTo(center - Offset(20.0 + i * 8, 0));
      await b.moveTo(center + Offset(20.0 + i * 8, 0));
      await tester.pump(const Duration(milliseconds: 16));
    }
    await a.up();
    await b.up();
    await tester.pump(const Duration(milliseconds: 400));
    expect(_scale(tester), greaterThan(1.5));
    await tester.tapAt(center);
    await tester.pump(const Duration(milliseconds: 60));
    await tester.tapAt(center);
    await tester.pump(const Duration(milliseconds: 400));
    expect(_scale(tester), 1);
    expect(tester.takeException(), isNull);
  });
}
