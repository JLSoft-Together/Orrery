import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/monitor/monitor_page.dart';
import 'package:orrery_remote/ui/viewers/branches_page.dart';
import 'package:orrery_remote/ui/viewers/changes_page.dart';
import 'package:orrery_remote/ui/viewers/diff_view.dart';
import 'package:orrery_remote/ui/viewers/explorer_page.dart';
import 'package:orrery_remote/ui/viewers/file_page.dart';
import 'package:orrery_remote/ui/viewers/fuzzy.dart';
import 'package:orrery_remote/ui/viewers/log_page.dart';
import 'package:orrery_remote/ui/viewers/run_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

const String root = 'C:\\work\\alpha';

JsonMap ok(Object? data) => {'re': 'x', 'ok': true, 'data': data};

Future<void> settle(WidgetTester tester) async {
  for (var i = 0; i < 6; i++) {
    await tester.pump(const Duration(milliseconds: 150));
  }
}

Harness build(Map<String, JsonMap Function(JsonMap? f)> handlers) {
  final h = harness();
  h.api.responder = (type, fields) {
    final handler = handlers[type];
    return handler == null ? ok(null) : handler(fields);
  };
  return h;
}

Map<String, dynamic> status(List<Map<String, dynamic>> entries) => {
  'ok': true,
  'root': root,
  'branch': 'main',
  'upstream': 'origin/main',
  'ahead': 1,
  'behind': 0,
  'entries': entries,
  'truncated': false,
};

Map<String, dynamic> entry(
  String path,
  String x,
  String y, {
  String kind = 'modified',
}) => {
  'path': path,
  'x': x,
  'y': y,
  'kind': kind,
  'staged': x != '.',
  'added': 3,
  'removed': 1,
};

Map<String, dynamic> file(
  String text, {
  num mtime = 100,
  String kind = 'text',
}) => {
  'kind': kind,
  'path': '$root\\main.dart',
  'size': text.length,
  'mtimeMs': mtime,
  'text': text,
  'lang': 'dart',
};

void main() {
  test('fuzzy ranks name matches first', () {
    final hits = fuzzyFilter([
      'lib/ui/workspace/tab_page.dart',
      'lib/remote/protocol.dart',
      'test/tab_test.dart',
    ], 'tabpage');
    expect(hits.first, 'lib/ui/workspace/tab_page.dart');
    expect(fuzzyFilter(['a.dart'], 'zzz'), isEmpty);
  });

  test('diff parse numbers lines and folds long context', () {
    final text = StringBuffer(
      'diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,30 +1,31 @@\n',
    );
    for (var i = 0; i < 20; i++) {
      text.write(' keep $i\n');
    }
    text.write('-old\n+new\n+extra\n');
    for (var i = 0; i < 8; i++) {
      text.write(' tail $i\n');
    }
    final lines = parseDiff(text.toString());
    expect(lines.where((l) => l.kind == DiffKind.add).length, 2);
    expect(lines.where((l) => l.kind == DiffKind.del).single.oldNo, 21);
    final rows = foldRows(lines, {});
    expect(rows.whereType<FoldRow>().length, 1);
    expect(
      foldRows(lines, {for (var i = 0; i < 40; i++) i}).whereType<FoldRow>(),
      isEmpty,
    );
  });

  testWidgets('explorer lists lazily, marks git state, opens file', (
    tester,
  ) async {
    final h = build({
      'fs.list': (f) => ok([
        if (f!['dir'] == root) ...[
          {
            'name': 'lib',
            'path': '$root\\lib',
            'directory': true,
            'outOfRoot': false,
          },
          {
            'name': 'main.dart',
            'path': '$root\\main.dart',
            'directory': false,
            'outOfRoot': false,
          },
        ] else
          {
            'name': 'a.dart',
            'path': '$root\\lib\\a.dart',
            'directory': false,
            'outOfRoot': false,
          },
      ]),
      'git.status': (f) => ok(
        status([
          entry('lib/a.dart', '.', 'M'),
          entry('main.dart', '.', 'M', kind: 'untracked'),
        ]),
      ),
      'fs.read': (f) => ok(file('void main() {}\n')),
    });
    await h.pump(tester, const ExplorerPage(machineId: 'm1', root: root));
    await settle(tester);
    expect(find.text('lib'), findsOneWidget);
    expect(find.text('a.dart'), findsNothing);
    expect(
      h.api.commands.map((c) => c.$1),
      containsAll(['fs.watch', 'git.watch']),
    );
    await tester.tap(find.text('lib'));
    await settle(tester);
    expect(find.text('a.dart'), findsOneWidget);
    expect(find.text('M'), findsOneWidget);
    await tester.tap(find.text('main.dart'));
    await settle(tester);
    expect(find.byType(FilePage), findsOneWidget);
    expect(find.text('void'), findsNothing);
    expect(find.textContaining('main()', findRichText: true), findsOneWidget);
    await tester.pageBack();
    await settle(tester);
    await tester.pumpWidget(const SizedBox());
    await tester.pump();
    expect(
      h.api.commands.map((c) => c.$1),
      containsAll(['fs.unwatch', 'git.unwatch']),
    );
  });

  testWidgets('file edit saves with base and conflict keeps the edit', (
    tester,
  ) async {
    var conflict = false;
    final h = build({
      'fs.read': (f) => ok(file('one\ntwo\n', mtime: 100)),
      'fs.write': (f) => conflict
          ? ok({'ok': false, 'error': 'conflict'})
          : ok({'ok': true, 'path': f!['path'], 'mtimeMs': 200, 'size': 5}),
    });
    await h.pump(
      tester,
      const FilePage(machineId: 'm1', root: root, path: '$root\\main.dart'),
    );
    await settle(tester);
    await tester.tap(find.byKey(const ValueKey('file-edit')));
    await settle(tester);
    await tester.enterText(
      find.byKey(const ValueKey('file-editor')),
      'changed',
    );
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('file-save')));
    await settle(tester);
    final write = h.api.commands.firstWhere((c) => c.$1 == 'fs.write');
    expect(write.$2!['base'], 100);
    expect(write.$2!['text'], 'changed');
    expect(find.text('Saved'), findsOneWidget);

    conflict = true;
    await tester.tap(find.byKey(const ValueKey('file-edit')));
    await settle(tester);
    await tester.enterText(find.byKey(const ValueKey('file-editor')), 'again');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('file-save')));
    await settle(tester);
    expect(find.byKey(const ValueKey('conflict-banner')), findsOneWidget);
    expect(find.byKey(const ValueKey('file-editor')), findsOneWidget);
    expect(h.api.commands.where((c) => c.$1 == 'fs.write').length, 2);
  });

  testWidgets('file changed on disk while editing shows the conflict banner', (
    tester,
  ) async {
    final h = build({'fs.read': (f) => ok(file('one\n'))});
    await h.pump(
      tester,
      const FilePage(machineId: 'm1', root: root, path: '$root\\main.dart'),
    );
    await settle(tester);
    await tester.tap(find.byKey(const ValueKey('file-edit')));
    await settle(tester);
    h.api.eventBus.add({
      't': 'fs.changed',
      'root': root,
      'paths': ['$root\\main.dart'],
    });
    await settle(tester);
    expect(find.byKey(const ValueKey('conflict-banner')), findsOneWidget);
    expect(h.api.commands.where((c) => c.$1 == 'fs.read').length, 1);
  });

  testWidgets('too large and binary files show a notice', (tester) async {
    final h = build({
      'fs.read': (f) =>
          ok({'kind': 'too-large', 'path': 'x', 'size': 3000000, 'mtimeMs': 1}),
    });
    await h.pump(
      tester,
      const FilePage(machineId: 'm1', root: root, path: '$root\\big.bin'),
    );
    await settle(tester);
    expect(find.textContaining('too large to open'), findsOneWidget);
    expect(find.byKey(const ValueKey('file-edit')), findsNothing);
  });

  testWidgets(
    'changes: stage, discard waits for confirm, commit needs staged and message',
    (tester) async {
      final h = build({
        'git.status': (f) => ok(
          status([
            entry('a.dart', 'M', '.'),
            entry('b.dart', '.', 'M'),
            entry('c.txt', '?', '?', kind: 'untracked'),
          ]),
        ),
        'git.discard': (f) => {'re': 'x', 'ok': true, 'confirm': 'cid1'},
        'git.commit': (f) => ok({'ok': true, 'hash': 'abc', 'subject': 's'}),
      });
      await h.pump(tester, const ChangesPage(machineId: 'm1', root: root));
      await settle(tester);
      expect(find.text('Staged (1)'), findsOneWidget);
      expect(find.text('Changes (2)'), findsOneWidget);
      final commit = find.byKey(const ValueKey('commit-button'));
      expect(tester.widget<FilledButton>(commit).onPressed, isNull);
      await tester.enterText(
        find.byKey(const ValueKey('commit-message')),
        'fix it',
      );
      await tester.pump();
      expect(tester.widget<FilledButton>(commit).onPressed, isNotNull);
      await tester.tap(find.byTooltip('Stage').first);
      await settle(tester);
      final stage = h.api.commands.firstWhere((c) => c.$1 == 'git.stage');
      expect(stage.$2, {
        'root': root,
        'paths': ['b.dart'],
      });
      await tester.tap(find.byTooltip('Discard changes').first);
      await settle(tester);
      expect(find.text('Confirm on this phone to continue.'), findsOneWidget);
      expect(h.api.commands.where((c) => c.$1 == 'git.discard').single.$2, {
        'root': root,
        'path': 'b.dart',
      });
      final before = h.api.commands.where((c) => c.$1 == 'git.status').length;
      h.viewNotifier.emit(
        h.viewState.copyWith(
          confirms: [
            ConfirmAsk(
              cid: 'cid1',
              title: 'Discard?',
              ok: 'Discard',
              cancel: 'Keep',
              receivedAt: DateTime.now(),
            ),
          ],
        ),
      );
      await tester.pump();
      h.viewNotifier.emit(h.viewState.copyWith(confirms: const []));
      await tester.pump(const Duration(seconds: 2));
      await settle(tester);
      expect(
        h.api.commands.where((c) => c.$1 == 'git.status').length,
        greaterThan(before),
      );
      await tester.pump(const Duration(seconds: 6));
      await tester.pump(const Duration(seconds: 1));
      await tester.tap(commit);
      await settle(tester);
      expect(h.api.commands.firstWhere((c) => c.$1 == 'git.commit').$2, {
        'root': root,
        'message': 'fix it',
      });
    },
  );

  testWidgets('git changed event reloads the changes list', (tester) async {
    final h = build({
      'git.status': (f) => ok(status([entry('a.dart', '.', 'M')])),
    });
    await h.pump(tester, const ChangesPage(machineId: 'm1', root: root));
    await settle(tester);
    final before = h.api.commands.where((c) => c.$1 == 'git.status').length;
    h.api.eventBus.add({'t': 'git.changed', 'root': root});
    await settle(tester);
    expect(
      h.api.commands.where((c) => c.$1 == 'git.status').length,
      before + 1,
    );
  });

  testWidgets('git log lists commits and paginates', (tester) async {
    var skips = <int>[];
    final h = build({
      'log.page': (f) {
        final skip = (f!['query'] as Map)['skip'] as int;
        skips.add(skip);
        return ok({
          'ok': true,
          'more': skip == 0,
          'commits': [
            for (var i = 0; i < 12; i++)
              {
                'hash': 'a${skip + i}bcdef1234',
                'parents': skip == 0 && i == 3 ? ['p1', 'p2'] : ['p'],
                'author': 'Jin',
                'time': DateTime.now().millisecondsSinceEpoch ~/ 1000 - 3600,
                'refs': i == 0 && skip == 0 ? 'HEAD -> main, origin/main' : '',
                'subject': 'commit ${skip + i}',
              },
          ],
        });
      },
    });
    await h.pump(
      tester,
      const LogPage(machineId: 'm1', root: root),
      size: const Size(390, 700),
    );
    await settle(tester);
    expect(find.text('commit 0'), findsOneWidget);
    expect(find.text('main'), findsOneWidget);
    await tester.drag(
      find.byKey(const ValueKey('log-list')),
      const Offset(0, -2000),
    );
    await settle(tester);
    expect(skips, [0, 12]);
  });

  testWidgets(
    'push shows the full target, then pushes with remote and branch',
    (tester) async {
      final h = build({
        'log.refs': (f) => ok({
          'ok': true,
          'head': {'branch': 'feat', 'hash': 'h'},
          'local': [
            {
              'ref': 'refs/heads/feat',
              'name': 'feat',
              'hash': 'h',
              'upstream': null,
              'ahead': 0,
              'behind': 0,
              'gone': false,
            },
            {
              'ref': 'refs/heads/main',
              'name': 'main',
              'hash': 'h',
              'upstream': 'origin/main',
              'ahead': 0,
              'behind': 0,
              'gone': false,
            },
          ],
          'remotes': [],
          'tags': [],
        }),
        'branch.push-preview': (f) => ok({
          'ok': true,
          'local': 'feat',
          'remotes': ['origin'],
          'remote': 'origin',
          'branch': 'feat',
          'upstream': false,
          'hasUpstream': false,
          'exists': false,
          'commits': [],
          'more': false,
        }),
        'branch.push': (f) => {'re': 'x', 'ok': true, 'confirm': 'c9'},
        'branch.delete': (f) => {'re': 'x', 'ok': true, 'confirm': 'c10'},
      });
      await h.pump(tester, const BranchesPage(machineId: 'm1', root: root));
      await settle(tester);
      await tester.tap(
        find.descendant(
          of: find.byKey(const ValueKey('branch-feat')),
          matching: find.byType(PopupMenuButton<String>),
        ),
      );
      await settle(tester);
      await tester.tap(find.text('Push...').last);
      await settle(tester);
      expect(find.byKey(const ValueKey('push-target')), findsOneWidget);
      expect(find.text('To origin/feat'), findsOneWidget);
      expect(h.api.commands.any((c) => c.$1 == 'branch.push'), isFalse);
      await tester.tap(find.byKey(const ValueKey('push-go')));
      await settle(tester);
      expect(h.api.commands.firstWhere((c) => c.$1 == 'branch.push').$2, {
        'root': root,
        'ref': 'refs/heads/feat',
        'remote': 'origin',
        'branch': 'feat',
        'setUpstream': true,
      });
      expect(find.text('Confirm on this phone to continue.'), findsOneWidget);
      await tester.tap(
        find.descendant(
          of: find.byKey(const ValueKey('branch-main')),
          matching: find.byType(PopupMenuButton<String>),
        ),
      );
      await settle(tester);
      await tester.tap(find.text('Delete').last);
      await settle(tester);
      expect(h.api.commands.firstWhere((c) => c.$1 == 'branch.delete').$2, {
        'root': root,
        'ref': 'refs/heads/main',
      });
    },
  );

  testWidgets('run starts a config and shows state and log from run.event', (
    tester,
  ) async {
    final h = build({
      'run.configs': (f) => ok([
        {'id': 'c1', 'name': 'App', 'type': 'flutter'},
      ]),
      'run.devices': (f) => ok([
        {'id': 'd1', 'name': 'Pixel', 'state': 'ready'},
      ]),
      'run.sessions': (f) => ok([]),
      'run.start': (f) => ok({'ok': true, 'sessionId': 's1'}),
    });
    await h.pump(tester, const RunPage(machineId: 'm1', root: root));
    await settle(tester);
    await tester.tap(find.byKey(const ValueKey('run-start')));
    await settle(tester);
    expect(h.api.commands.firstWhere((c) => c.$1 == 'run.start').$2, {
      'root': root,
      'configId': 'c1',
      'deviceId': 'd1',
    });
    h.api.eventBus.add({
      't': 'run.event',
      'event': {
        'type': 'session',
        'session': {
          'id': 's1',
          'configName': 'App',
          'state': 'running',
          'cwd': root,
        },
      },
    });
    h.api.eventBus.add({
      't': 'run.event',
      'event': {
        'type': 'log',
        'sessionId': 's1',
        'lines': [
          {'stream': 'app', 'text': 'hello run', 'at': 1},
        ],
      },
    });
    await settle(tester);
    expect(find.text('running'), findsOneWidget);
    expect(find.text('hello run'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('run-stop')));
    await settle(tester);
    expect(h.api.commands.any((c) => c.$1 == 'run.stop'), isTrue);
  });

  testWidgets(
    'monitor shows sessions with subagent tree and no invented tokens',
    (tester) async {
      final h = Harness(
        api: FakeMachineApi(),
        viewState: view(
          ws: sample(),
          snapshot: dashboard(
            terminals: [
              {
                'id': 't1',
                'title': 'Refactor',
                'cwd': root,
                'status': 'working',
                'provider': 'claude',
                'gitBranch': 'main',
              },
            ],
            agents: [
              {
                'id': 'a1',
                'terminalId': 't1',
                'role': 'main',
                'status': 'working',
                'model': 'opus',
                'currentTool': 'Bash',
                'currentToolSummary': 'flutter test',
                'contextPct': 42,
              },
              {
                'id': 'a2',
                'terminalId': 't1',
                'role': 'subagent',
                'parentId': 'a1',
                'status': 'working',
                'name': 'explorer',
                'totalTokens': 12500,
              },
              {
                'id': 'a3',
                'terminalId': 't1',
                'role': 'subagent',
                'parentId': 'a2',
                'status': 'idle',
                'name': 'grandchild',
              },
            ],
          ),
        ),
        linkState: linkReady,
      );
      await h.pump(tester, MonitorPage(machineId: 'm1', onOpen: (_) {}));
      await tester.pump();
      expect(find.text('Refactor'), findsOneWidget);
      expect(find.textContaining('2 subagents'), findsNothing);
      await tester.tap(find.byKey(const ValueKey('mon-t1')));
      await tester.pumpAndSettle();
      expect(find.text('explorer'), findsOneWidget);
      expect(find.text('grandchild'), findsOneWidget);
      expect(find.textContaining('context 42%'), findsOneWidget);
      expect(find.textContaining('12.5k tokens'), findsOneWidget);
      expect(find.textContaining('tokens'), findsOneWidget);
    },
  );

  testWidgets('workspace bottom nav switches to Monitor', (tester) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    await tester.pump(const Duration(milliseconds: 600));
    await tester.tap(find.byKey(const ValueKey('nav-monitor')));
    await tester.pump(const Duration(milliseconds: 600));
    expect(find.text('No sessions on the computer right now.'), findsOneWidget);
  });
}
