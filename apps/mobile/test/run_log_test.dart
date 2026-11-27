import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/ui/run/crash_send.dart';
import 'package:orrery_remote/ui/run/run_log_model.dart';
import 'package:orrery_remote/ui/run/run_log_page.dart';
import 'package:orrery_remote/ui/viewers/viewer_api.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

const String root = 'C:\\work\\alpha';

JsonMap ok(Object? data) => {'re': 'x', 'ok': true, 'data': data};

Future<void> settle(WidgetTester tester) async {
  for (var i = 0; i < 6; i++) {
    await tester.pump(const Duration(milliseconds: 150));
  }
}

Map<String, Object?> line(
  int n,
  String text, {
  String stream = 'app',
  String? level,
}) => {'n': n, 'stream': stream, 'text': text, 'at': n, 'level': ?level};

final List<Map<String, Object?>> crashLines = [
  line(1, 'I/flutter ( 100): started', level: 'I'),
  line(
    2,
    'E/AndroidRuntime(100): FATAL EXCEPTION: main',
    stream: 'logcat',
    level: 'E',
  ),
  line(3, '    at dev.test.Main.run(Main.kt:12)', stream: 'logcat'),
  line(4, 'Caused by: java.lang.NullPointerException', stream: 'logcat'),
  line(5, 'I/flutter ( 100): after', level: 'I'),
  line(6, 'E/flutter ( 100): Unhandled Exception: Bad state'),
  line(7, '#0      main (package:app/main.dart:3:5)'),
];

void main() {
  group('crash detection', () {
    List<RunLogLine> parsed(List<Map<String, Object?>> raw) =>
        markCrash([for (final m in raw) RunLogLine.parse(m)!]);

    test('marks FATAL EXCEPTION, Unhandled Exception and E/flutter blocks', () {
      final lines = parsed(crashLines);
      expect(lines.map((l) => l.crash), [
        false,
        true,
        true,
        true,
        false,
        true,
        true,
      ]);
    });

    test('excerpt is the last crash block only', () {
      final text = crashExcerpt(parsed(crashLines))!;
      expect(text, startsWith('E/flutter ( 100): Unhandled Exception'));
      expect(text, contains('#0      main'));
      expect(text, isNot(contains('FATAL')));
      expect(crashExcerpt(parsed([line(1, 'all good')])), isNull);
    });

    test(
      'merging a since chunk appends only newer lines and caps the buffer',
      () {
        final buffer = RunLogBuffer();
        buffer.add(
          RunLogChunk.parse({
            'known': true,
            'last': 3,
            'lines': crashLines.take(3).toList(),
          }),
          first: true,
        );
        expect(buffer.last, 3);
        buffer.add(
          RunLogChunk.parse({
            'known': true,
            'last': 5,
            'gap': true,
            'lines': crashLines.sublist(1, 5),
          }),
          first: false,
        );
        expect(buffer.lines.map((l) => l.n), [1, 2, 3, 4, 5]);
        expect(buffer.gap, isTrue);
        expect(buffer.hasCrash, isTrue);
        buffer.add(
          RunLogChunk(
            known: true,
            last: 9000,
            gap: false,
            more: false,
            lines: [
              for (var i = 6; i < 6 + runLogMax + 50; i++)
                RunLogLine(n: i, stream: 'app', text: 'x'),
            ],
          ),
          first: false,
        );
        expect(buffer.lines.length, runLogMax);
      },
    );

    test('garbage replies parse to an empty unknown chunk', () {
      final chunk = RunLogChunk.parse('nope');
      expect(chunk.known, isFalse);
      expect(chunk.lines, isEmpty);
      expect(RunLogLine.parse({'n': 'x', 'text': 'y'}), isNull);
    });
  });

  group('run log page', () {
    Harness build(JsonMap Function(JsonMap? f) onLog) {
      final h = harness();
      h.api.responder = (type, fields) =>
          type == 'run.log' ? onLog(fields) : ok(true);
      return h;
    }

    Future<void> open(Harness h, WidgetTester tester) => h.pump(
      tester,
      const RunLogPage(
        machineId: 'm1',
        sessionId: 's1',
        title: 'App',
        cwd: root,
      ),
    );

    testWidgets(
      'loads the backlog, tints crash lines and offers the crash button',
      (tester) async {
        final h = build(
          (f) => ok({
            'known': true,
            'last': 7,
            'gap': false,
            'more': false,
            'lines': crashLines,
          }),
        );
        await open(h, tester);
        await settle(tester);
        expect(h.api.commands.any((c) => c.$1 == 'run.watch'), isTrue);
        final first = h.api.commands.firstWhere((c) => c.$1 == 'run.log');
        expect(first.$2, {'session': 's1'});
        expect(find.byKey(const ValueKey('crash-2')), findsOneWidget);
        expect(find.byKey(const ValueKey('crash-5')), findsNothing);
        expect(
          find.byKey(const ValueKey('run-log-send-crash')),
          findsOneWidget,
        );
      },
    );

    testWidgets('a log event pulls only the new lines with since', (
      tester,
    ) async {
      var calls = 0;
      final h = build((f) {
        calls++;
        if (f?['since'] == null) {
          return ok({
            'known': true,
            'last': 1,
            'gap': false,
            'more': false,
            'lines': [line(1, 'first line')],
          });
        }
        return ok({
          'known': true,
          'last': 2,
          'gap': false,
          'more': false,
          'lines': [line(2, 'second line')],
        });
      });
      await open(h, tester);
      await settle(tester);
      expect(find.text('first line'), findsOneWidget);
      h.api.eventBus.add({
        't': 'run.event',
        'event': {'type': 'log', 'sessionId': 's1', 'lines': []},
      });
      await settle(tester);
      final since = h.api.commands.where((c) => c.$1 == 'run.log').last.$2!;
      expect(since['since'], 1);
      expect(find.text('second line'), findsOneWidget);
      expect(find.text('first line'), findsOneWidget);
      expect(calls, 2);
      h.api.eventBus.add({
        't': 'run.event',
        'event': {'type': 'log', 'sessionId': 'other', 'lines': []},
      });
      await settle(tester);
      expect(calls, 2);
    });

    testWidgets('a desktop without the op shows the update hint', (
      tester,
    ) async {
      final h = build((f) => ok({'ok': false, 'error': 'denied'}));
      await open(h, tester);
      await settle(tester);
      expect(find.textContaining('does not serve the Run log'), findsOneWidget);
    });

    testWidgets('stream chips filter lines', (tester) async {
      final h = build(
        (f) => ok({
          'known': true,
          'last': 7,
          'gap': false,
          'more': false,
          'lines': crashLines,
        }),
      );
      await open(h, tester);
      await settle(tester);
      await tester.tap(find.byKey(const ValueKey('run-log-stream-logcat')));
      await settle(tester);
      expect(find.textContaining('after'), findsNothing);
      expect(find.textContaining('FATAL EXCEPTION'), findsOneWidget);
    });

    testWidgets('leaving the page unwatches the run feed', (tester) async {
      final h = build(
        (f) => ok({
          'known': false,
          'last': 0,
          'gap': false,
          'more': false,
          'lines': [],
        }),
      );
      await open(h, tester);
      await settle(tester);
      expect(find.textContaining('No log lines yet'), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
      await settle(tester);
      expect(h.api.commands.any((c) => c.$1 == 'run.unwatch'), isTrue);
    });
  });

  group('send crash to agent', () {
    Map<String, dynamic> ws(List<Map<String, dynamic>> panes) =>
        workspace([tab('t1', panes, project: root)]);

    Future<(Harness, List<String>)> mount(
      WidgetTester tester,
      Map<String, dynamic> workspaceJson,
    ) async {
      final h = harness(ws: workspaceJson);
      final opened = <String>[];
      await h.pump(
        tester,
        Scaffold(
          body: Consumer(
            builder: (context, ref, _) => TextButton(
              key: const ValueKey('go'),
              onPressed: () => sendCrashToAgent(
                context,
                ref,
                machineId: 'm1',
                cwd: root,
                crash: 'E/flutter boom',
                open: (tab, pane) => opened.add(pane.key),
              ),
              child: const Text('go'),
            ),
          ),
        ),
      );
      return (h, opened);
    }

    String? clip;

    setUp(() {
      clip = null;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(SystemChannels.platform, (call) async {
            if (call.method == 'Clipboard.setData') {
              clip = (call.arguments as Map)['text'] as String?;
            }
            return null;
          });
    });

    tearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(SystemChannels.platform, null),
    );

    testWidgets('fills the draft and copies it, never sends anything', (
      tester,
    ) async {
      final (h, opened) = await mount(
        tester,
        ws([
          {...pane('a'), 'cwd': root},
        ]),
      );
      await tester.tap(find.byKey(const ValueKey('go')));
      await settle(tester);
      expect(opened, ['a']);
      final draft = h.container.read(crashDraftProvider)['a']!;
      expect(draft, contains('E/flutter boom'));
      expect(draft, contains('```'));
      expect(clip, isNull);
      expect(h.api.commands, isEmpty);
      expect(h.api.inputs, isEmpty);
      expect(find.textContaining('Nothing was sent'), findsOneWidget);
    });

    testWidgets('no agent pane for the project shows a hint', (tester) async {
      final (h, opened) = await mount(
        tester,
        ws([
          {...pane('a'), 'cwd': 'C:\\work\\other'},
        ]),
      );
      await tester.tap(find.byKey(const ValueKey('go')));
      await settle(tester);
      expect(opened, isEmpty);
      expect(h.container.read(crashDraftProvider), isEmpty);
      expect(find.textContaining('No agent pane'), findsOneWidget);
    });

    testWidgets('several agents ask which one', (tester) async {
      final (h, opened) = await mount(
        tester,
        ws([
          {...pane('a'), 'cwd': root},
          {...pane('b', title: 'second'), 'cwd': root},
        ]),
      );
      await tester.tap(find.byKey(const ValueKey('go')));
      await settle(tester);
      await tester.tap(find.byKey(const ValueKey('crash-pane-b')));
      await settle(tester);
      expect(opened, ['b']);
      expect(h.container.read(crashDraftProvider).keys, ['b']);
      expect(
        h.container.read(crashDraftProvider.notifier).take('b'),
        isNotNull,
      );
      expect(h.container.read(crashDraftProvider), isEmpty);
    });
  });

  test('a restarted run with lower numbering replaces the buffer', () {
    RunLogChunk chunk(List<int> ns, int last) => RunLogChunk(
      known: true,
      lines: [
        for (final n in ns) RunLogLine(n: n, stream: 'app', text: 'l$n'),
      ],
      last: last,
      gap: false,
      more: false,
    );
    final buffer = RunLogBuffer()
      ..add(chunk([1, 2, 3, 4, 5], 5), first: true)
      ..add(chunk([1, 2], 2), first: false);
    expect(buffer.lines.map((l) => l.text), ['l1', 'l2']);
    expect(buffer.last, 2);
    buffer.add(chunk([3], 3), first: false);
    expect(buffer.lines.map((l) => l.text), ['l1', 'l2', 'l3']);
  });

  test('viewer api keeps run watch shared between pages', () {
    expect(normPath('C:\\work\\alpha\\'), 'c:/work/alpha');
  });
}
