import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/ui/monitor/monitor_page.dart';
import 'package:orrery_remote/ui/viewers/branches_page.dart';
import 'package:orrery_remote/ui/viewers/changes_page.dart';
import 'package:orrery_remote/ui/viewers/commit_page.dart';
import 'package:orrery_remote/ui/viewers/diff_page.dart';
import 'package:orrery_remote/ui/viewers/explorer_page.dart';
import 'package:orrery_remote/ui/viewers/file_page.dart';
import 'package:orrery_remote/ui/viewers/log_page.dart';
import 'package:orrery_remote/ui/viewers/run_page.dart';
import 'package:orrery_remote/ui/viewers/viewer_menu.dart';

import 'support/fonts.dart';
import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

final bool enabled = Platform.environment['WP12_SHOTS'] != null;
final String outDir = Platform.environment['WP12_OUT'] ?? 'shots12';

const String root = 'D:\\work\\web\\claude-code-monitor';
const String code = '''import 'package:flutter/material.dart';

class Counter extends StatefulWidget {
  const Counter({super.key, this.start = 0});

  final int start;

  @override
  State<Counter> createState() => _CounterState();
}

class _CounterState extends State<Counter> {
  late int _value = widget.start;

  void _bump() => setState(() => _value++);

  @override
  Widget build(BuildContext context) {
    // shows the current count
    return FilledButton(
      onPressed: _bump,
      child: Text('Count: \$_value'),
    );
  }
}
''';

const String diffText = '''diff --git a/lib/counter.dart b/lib/counter.dart
--- a/lib/counter.dart
+++ b/lib/counter.dart
@@ -1,22 +1,24 @@
 import 'package:flutter/material.dart';

 class Counter extends StatefulWidget {
   const Counter({super.key, this.start = 0});

   final int start;

   @override
   State<Counter> createState() => _CounterState();
 }

 class _CounterState extends State<Counter> {
-  late int _value = widget.start;
+  late int _value = widget.start;
+  int _taps = 0;

-  void _bump() => setState(() => _value++);
+  void _bump() => setState(() {
+    _value++;
+    _taps++;
+  });

   @override
   Widget build(BuildContext context) {
     return FilledButton(
       onPressed: _bump,
       child: Text('Count: \$_value'),
     );
   }
 }
''';

JsonMap ok(Object? data) => {'re': 'x', 'ok': true, 'data': data};

int now() => DateTime.now().millisecondsSinceEpoch ~/ 1000;

Map<String, JsonMap Function(JsonMap?)> handlers() => {
  'fs.list': (f) => ok([
    if (f!['dir'] == root) ...[
      {'name': 'apps', 'path': '$root\\apps', 'directory': true, 'outOfRoot': false},
      {'name': 'docs', 'path': '$root\\docs', 'directory': true, 'outOfRoot': false},
      {'name': 'package.json', 'path': '$root\\package.json', 'directory': false, 'outOfRoot': false},
      {'name': 'README.md', 'path': '$root\\README.md', 'directory': false, 'outOfRoot': false},
    ] else ...[
      {'name': 'mobile', 'path': '$root\\apps\\mobile', 'directory': true, 'outOfRoot': false},
      {'name': 'collector', 'path': '$root\\apps\\collector', 'directory': true, 'outOfRoot': false},
    ],
  ]),
  'git.status': (f) => ok({
    'ok': true,
    'root': root,
    'branch': 'feat/v97-mobile',
    'upstream': 'origin/feat/v97-mobile',
    'ahead': 2,
    'behind': 0,
    'truncated': false,
    'entries': [
      {'path': 'apps/mobile/lib/counter.dart', 'x': 'M', 'y': '.', 'kind': 'modified', 'staged': true, 'added': 12, 'removed': 3},
      {'path': 'apps/mobile/lib/ui/viewers/file_page.dart', 'x': '.', 'y': 'M', 'kind': 'modified', 'staged': false, 'added': 40, 'removed': 8},
      {'path': 'docs/qa/v97/notes.md', 'x': '?', 'y': '?', 'kind': 'untracked', 'staged': false},
      {'path': 'old.txt', 'x': '.', 'y': 'D', 'kind': 'deleted', 'staged': false, 'removed': 20},
    ],
  }),
  'fs.read': (f) => ok({
    'kind': 'text',
    'path': '$root\\apps\\mobile\\lib\\counter.dart',
    'size': code.length,
    'mtimeMs': 10,
    'text': code,
    'lang': 'dart',
  }),
  'git.diff': (f) => ok({'ok': true, 'path': 'lib/counter.dart', 'text': diffText, 'binary': false, 'tooLarge': false, 'untracked': false}),
  'log.page': (f) => ok({
    'ok': true,
    'more': false,
    'commits': [
      for (var i = 0; i < 9; i++)
        {
          'hash': 'e${i}f3a9c01d2b',
          'parents': i == 3 ? ['a', 'b'] : ['a'],
          'author': i.isEven ? 'Jin Vu' : 'Claude',
          'time': now() - 600 * (i + 1) * (i + 1),
          'refs': i == 0
              ? 'HEAD -> feat/v97-mobile, origin/feat/v97-mobile'
              : i == 4
              ? 'tag: v2.3.0'
              : '',
          'subject': const [
            'feat(mobile): viewer screens for explorer, changes and git log',
            'fix(shell): refuse UNC paths before canonicalize',
            'feat(remote): open a pane from Telegram and ntfy notifications',
            'Merge branch master into feat/v97-mobile',
            'docs(planning): v97 WP-8 and WP-13 briefs',
            'feat(mobile): claim flag on user input',
            'chore: bump relay',
            'test(mobile): fake gateway',
            'init',
          ][i],
        },
    ],
  }),
  'log.commit': (f) => ok({
    'ok': true,
    'hash': 'e0f3a9c01d2b',
    'parents': ['a'],
    'author': 'Jin Vu',
    'authorTime': now() - 600,
    'message': 'feat(mobile): viewer screens for explorer, changes and git log\n\nAdds the phone viewers.',
    'files': [
      {'path': 'apps/mobile/lib/ui/viewers/explorer_page.dart', 'kind': 'added', 'added': 320, 'removed': 0},
      {'path': 'apps/mobile/lib/ui/workspace/tab_page.dart', 'kind': 'modified', 'added': 9, 'removed': 1},
      {'path': 'apps/mobile/lib/old.dart', 'kind': 'deleted', 'added': 0, 'removed': 44},
    ],
  }),
  'log.diff': (f) => ok({'ok': true, 'path': 'x', 'text': diffText, 'binary': false, 'tooLarge': false}),
  'log.refs': (f) => ok({
    'ok': true,
    'head': {'branch': 'feat/v97-mobile', 'hash': 'h'},
    'local': [
      {'ref': 'refs/heads/feat/v97-mobile', 'name': 'feat/v97-mobile', 'hash': 'h', 'upstream': 'origin/feat/v97-mobile', 'ahead': 2, 'behind': 0, 'gone': false},
      {'ref': 'refs/heads/master', 'name': 'master', 'hash': 'h', 'upstream': 'origin/master', 'ahead': 0, 'behind': 3, 'gone': false},
      {'ref': 'refs/heads/spike/relay', 'name': 'spike/relay', 'hash': 'h', 'upstream': null, 'ahead': 0, 'behind': 0, 'gone': false},
    ],
    'remotes': [
      {'name': 'origin', 'refs': [{'ref': 'refs/remotes/origin/master', 'name': 'origin/master', 'hash': 'h'}]},
    ],
    'tags': [
      {'ref': 'refs/tags/v2.3.0', 'name': 'v2.3.0', 'hash': 'h'},
    ],
  }),
  'branch.push-preview': (f) => ok({
    'ok': true,
    'local': 'spike/relay',
    'remotes': ['origin'],
    'remote': 'origin',
    'branch': 'spike/relay',
    'upstream': false,
    'hasUpstream': false,
    'exists': false,
    'commits': [
      {'hash': 'abc1234def', 'parents': ['a'], 'author': 'x', 'time': 1, 'refs': '', 'subject': 'spike: relay probe'},
      {'hash': 'abc5678def', 'parents': ['a'], 'author': 'x', 'time': 1, 'refs': '', 'subject': 'spike: close codes'},
    ],
    'more': false,
  }),
  'run.configs': (f) => ok([
    {'id': 'c1', 'name': 'orrery_remote', 'type': 'flutter'},
    {'id': 'c2', 'name': 'Android debug', 'type': 'android'},
  ]),
  'run.devices': (f) => ok([
    {'id': 'd1', 'name': 'Galaxy A21s', 'state': 'ready'},
    {'id': 'd2', 'name': 'Pixel 7 emulator', 'state': 'offline'},
  ]),
  'run.sessions': (f) => ok([
    {'id': 's1', 'configName': 'orrery_remote', 'state': 'running', 'cwd': root, 'deviceName': 'Galaxy A21s', 'progress': null},
  ]),
};

Future<void> shot(WidgetTester tester, GlobalKey key, String name) async {
  await tester.pump(const Duration(milliseconds: 700));
  await tester.pump(const Duration(milliseconds: 700));
  await tester.runAsync(() async {
    final boundary = key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final image = await boundary.toImage(pixelRatio: 2);
    final data = await image.toByteData(format: ui.ImageByteFormat.png);
    final file = File('$outDir/$name.png');
    await file.create(recursive: true);
    await file.writeAsBytes(data!.buffer.asUint8List());
  });
}

void main() {
  setUpAll(loadFonts);

  Future<void> scene(
    String name,
    WidgetTester tester,
    Widget home, {
    Brightness brightness = Brightness.light,
    Locale locale = const Locale('en'),
    Size size = const Size(390, 844),
    Harness? harness,
    Future<void> Function(Harness h)? after,
  }) async {
    debugDisableShadows = false;
    final h = harness ?? Harness(
      api: FakeMachineApi()..responder = (t, f) => (handlers()[t] ?? (_) => ok(null))(f),
      viewState: view(ws: sample(), snapshot: dashboard()),
      linkState: linkReady,
    );
    final key = GlobalKey();
    await h.pump(tester, home, size: size, brightness: brightness, locale: locale, boundary: key, fontFamily: 'Roboto');
    for (var i = 0; i < 8; i++) {
      await tester.pump(const Duration(milliseconds: 150));
    }
    if (after != null) await after(h);
    await shot(tester, key, name);
    debugDisableShadows = true;
  }

  void sTest(String name, Future<void> Function(WidgetTester) body) {
    testWidgets(name, body, skip: !enabled);
  }

  sTest('explorer', (tester) async {
    await scene('01-explorer', tester, const ExplorerPage(machineId: 'm1', root: root), after: (h) async {
      await tester.tap(find.text('apps'));
      for (var i = 0; i < 6; i++) {
        await tester.pump(const Duration(milliseconds: 150));
      }
    });
  });

  sTest('explorer dark vi', (tester) async {
    await scene('01b-explorer-dark', tester, const ExplorerPage(machineId: 'm1', root: root), brightness: Brightness.dark, locale: const Locale('vi'));
  });

  sTest('file', (tester) async {
    await scene('02-file', tester, FilePage(machineId: 'm1', root: root, path: '$root\\apps\\mobile\\lib\\counter.dart'));
  });

  sTest('file dark', (tester) async {
    await scene('02b-file-dark', tester, FilePage(machineId: 'm1', root: root, path: '$root\\apps\\mobile\\lib\\counter.dart'), brightness: Brightness.dark);
  });

  sTest('file conflict', (tester) async {
    await scene('02c-file-conflict', tester, FilePage(machineId: 'm1', root: root, path: '$root\\apps\\mobile\\lib\\counter.dart'), after: (h) async {
      await tester.tap(find.byKey(const ValueKey('file-edit')));
      await tester.pump(const Duration(milliseconds: 300));
      h.api.eventBus.add({'t': 'fs.changed', 'root': root, 'paths': ['$root\\apps\\mobile\\lib\\counter.dart']});
      await tester.pump(const Duration(milliseconds: 300));
    });
  });

  sTest('changes', (tester) async {
    await scene('03-changes', tester, const ChangesPage(machineId: 'm1', root: root));
  });

  sTest('changes dark vi', (tester) async {
    await scene('03b-changes-dark-vi', tester, const ChangesPage(machineId: 'm1', root: root), brightness: Brightness.dark, locale: const Locale('vi'));
  });

  sTest('diff', (tester) async {
    await scene('04-diff', tester, const DiffPage(machineId: 'm1', path: 'apps/mobile/lib/counter.dart', type: 'git.diff', fields: {'root': root, 'path': 'x'}));
  });

  sTest('diff dark', (tester) async {
    await scene('04b-diff-dark', tester, const DiffPage(machineId: 'm1', path: 'apps/mobile/lib/counter.dart', type: 'git.diff', fields: {'root': root, 'path': 'x'}), brightness: Brightness.dark);
  });

  sTest('log', (tester) async {
    await scene('05-log', tester, const LogPage(machineId: 'm1', root: root));
  });

  sTest('commit', (tester) async {
    await scene('05b-commit', tester, const CommitPage(machineId: 'm1', root: root, hash: 'e0f3a9c01d2b'));
  });

  sTest('branches', (tester) async {
    await scene('06-branches', tester, const BranchesPage(machineId: 'm1', root: root));
  });

  sTest('push dialog', (tester) async {
    await scene('06b-push', tester, const BranchesPage(machineId: 'm1', root: root), after: (h) async {
      await tester.tap(find.descendant(of: find.byKey(const ValueKey('branch-spike/relay')), matching: find.byType(PopupMenuButton<String>)));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Push...').last);
      await tester.pumpAndSettle();
      for (var i = 0; i < 6; i++) {
        await tester.pump(const Duration(milliseconds: 150));
      }
    });
  });

  sTest('run', (tester) async {
    await scene('07-run', tester, const RunPage(machineId: 'm1', root: root), after: (h) async {
      h.api.eventBus.add({
        't': 'run.event',
        'event': {
          'type': 'log',
          'sessionId': 's1',
          'lines': [
            {'stream': 'app', 'text': 'Launching lib/main.dart on Galaxy A21s in debug mode...', 'level': 'I', 'at': 1},
            {'stream': 'build', 'text': 'Running Gradle task assembleDebug...', 'at': 2},
            {'stream': 'app', 'text': 'W/Orrery: relay token expires soon', 'level': 'W', 'at': 3},
            {'stream': 'app', 'text': 'E/Orrery: handshake timeout, retry 2', 'level': 'E', 'at': 4},
            {'stream': 'app', 'text': 'Syncing files to device Galaxy A21s...', 'at': 5},
          ],
        },
      });
      await tester.pump(const Duration(milliseconds: 300));
    });
  });

  sTest('monitor', (tester) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(ws: sample(), snapshot: monitorSnapshot()),
      linkState: linkReady,
    );
    await scene('08-monitor', tester, MonitorPage(machineId: 'm1', onOpen: (_) {}), harness: h);
  });

  sTest('monitor dark vi', (tester) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(ws: sample(), snapshot: monitorSnapshot()),
      linkState: linkReady,
    );
    await scene('08b-monitor-dark-vi', tester, MonitorPage(machineId: 'm1', onOpen: (_) {}), harness: h, brightness: Brightness.dark, locale: const Locale('vi'));
  });

  sTest('viewer menu', (tester) async {
    await scene('09-menu', tester, Builder(builder: (context) => Scaffold(body: Center(child: FilledButton(onPressed: () => showViewerMenu(context, 'm1', root), child: const Text('menu'))))), after: (h) async {
      await tester.tap(find.text('menu'));
      for (var i = 0; i < 6; i++) {
        await tester.pump(const Duration(milliseconds: 150));
      }
    });
  });
}

Map<String, dynamic> monitorSnapshot() => dashboard(
  terminals: [
    {'id': 't1', 'title': 'Refactor remote layer', 'cwd': root, 'status': 'working', 'provider': 'claude', 'gitBranch': 'feat/v97-mobile'},
    {'id': 't2', 'title': 'Fix PTY resize', 'cwd': root, 'status': 'waiting', 'provider': 'codex', 'waitingFor': 'Bash: flutter test --concurrency=4'},
    {'id': 't3', 'title': 'Release notes', 'cwd': 'D:\\work\\android\\WeJoy\\Dumpling-5', 'status': 'idle', 'provider': 'claude'},
  ],
  agents: [
    {'id': 'a1', 'terminalId': 't1', 'role': 'main', 'status': 'working', 'model': 'claude-opus', 'currentTool': 'Bash', 'currentToolSummary': 'flutter analyze', 'contextPct': 58},
    {'id': 'a2', 'terminalId': 't1', 'role': 'subagent', 'parentId': 'a1', 'status': 'working', 'name': 'flutter-reviewer', 'currentTool': 'Read', 'totalTokens': 48200},
    {'id': 'a3', 'terminalId': 't1', 'role': 'subagent', 'parentId': 'a2', 'status': 'idle', 'name': 'explorer'},
    {'id': 'a4', 'terminalId': 't2', 'role': 'main', 'status': 'waiting', 'model': 'gpt-5', 'contextPct': 91},
    {'id': 'a5', 'terminalId': 't3', 'role': 'main', 'status': 'idle', 'model': 'claude-sonnet', 'contextPct': 12},
  ],
);
