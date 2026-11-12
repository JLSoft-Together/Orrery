import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/app_lock_scope.dart';
import 'package:orrery_remote/ui/viewers/async_panel.dart';
import 'package:orrery_remote/ui/viewers/diff_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/lock_fakes.dart';
import 'support/ui_samples.dart';

const String diffText = '''diff --git a/a.dart b/a.dart
--- a/a.dart
+++ b/a.dart
@@ -1,3 +1,4 @@
 keep
-old
+new
+more
 tail
''';

void main() {
  testWidgets('settings tab shows lock controls and the machine', (
    tester,
  ) async {
    final auth = FakeAuth();
    final lock = makeLock(auth);
    await tester.runAsync(lock.load);
    final h = harness();
    await h.pump(
      tester,
      AppLockScope(
        lock: lock,
        child: const WorkspacePage(machineId: 'm1'),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byKey(const ValueKey('lock-switch')), findsOneWidget);
    expect(find.byKey(const ValueKey('lock-timeout-0')), findsOneWidget);
    expect(find.byKey(const ValueKey('lock-timeout-3')), findsOneWidget);
    expect(find.text('Follows the computer'), findsNWidgets(2));
    lock.dispose();
  });

  testWidgets('settings tab without a lock hides the security section', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.byKey(const ValueKey('lock-switch')), findsNothing);
    expect(find.byKey(const ValueKey('settings-list')), findsOneWidget);
  });

  testWidgets('diff page counts added and removed lines', (tester) async {
    final h = harness();
    h.api.responder = (type, fields) => {
      're': 'x',
      'ok': true,
      'data': {'ok': true, 'text': diffText},
    };
    await h.pump(
      tester,
      const DiffPage(
        machineId: 'm1',
        path: 'lib/a.dart',
        type: 'git.diff',
        fields: {'root': 'r', 'path': 'a'},
      ),
    );
    for (var i = 0; i < 6; i++) {
      await tester.pump(const Duration(milliseconds: 150));
    }
    expect(find.text('2 added'), findsOneWidget);
    expect(find.text('1 removed'), findsOneWidget);
  });

  testWidgets('error state offers a retry', (tester) async {
    var retried = 0;
    final h = harness();
    await h.pump(
      tester,
      Scaffold(
        body: ErrorState(message: 'boom', onRetry: () => retried++),
      ),
    );
    expect(find.text('boom'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('error-retry')));
    expect(retried, 1);
  });
}
