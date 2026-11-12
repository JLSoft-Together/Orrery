import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/app_lock_scope.dart';
import 'package:orrery_remote/ui/viewers/async_panel.dart';
import 'package:orrery_remote/ui/viewers/diff_page.dart';
import 'package:orrery_remote/ui/viewers/file_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/fonts.dart';
import 'support/lock_fakes.dart';
import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';
import 'support/wp31_scenario.dart';

final bool enabled = Platform.environment['WP32_SHOTS'] != null;
final String outDir = Platform.environment['WP32_OUT'] ?? 'shots32';

const String root = 'D:/work/web/claude-code-monitor';
const String filePath = '$root/apps/mobile/lib/ui/terminal/key_bar.dart';

const String code = '''import 'package:flutter/material.dart';

class KeyBar extends StatelessWidget {
  const KeyBar({super.key, required this.onKey});
  final ValueChanged<String> onKey;

  Widget build(BuildContext context) {
    final c = Theme.of(context);
    return SizedBox(
      height: 48,
      child: ListView(
        scrollDirection: Axis.horizontal,
        children: [
          for (final k in _keys) _Key(k, onKey),
        ],
      ),
    );
  }
}
''';

const String diffText = '''diff --git a/key_bar.dart b/key_bar.dart
--- a/key_bar.dart
+++ b/key_bar.dart
@@ -12,6 +12,9 @@ class KeyBar {
     return SizedBox(
       height: 48,
-      child: Row(
+      child: ListView(
+        scrollDirection: Axis.horizontal,
         children: [
-          _Key('Esc', onKey),
+          for (final k in _keys) _Key(k, onKey),
         ],
       ),
@@ -40,3 +43,8 @@
+  static const _keys = [
+    'Esc', 'Tab', 'Ctrl', 'Alt',
+  ];
 }
''';

Map<String, dynamic> _ok(Object? data) => {'re': 'x', 'ok': true, 'data': data};

Future<void> shot(WidgetTester tester, GlobalKey key, String name) async {
  await tester.pump(const Duration(milliseconds: 700));
  await tester.pump(const Duration(milliseconds: 700));
  await tester.runAsync(() async {
    final boundary =
        key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final image = await boundary.toImage(pixelRatio: 2);
    final data = await image.toByteData(format: ui.ImageByteFormat.png);
    final file = File('$outDir/$name.png');
    await file.create(recursive: true);
    await file.writeAsBytes(data!.buffer.asUint8List());
  });
}

Harness _viewerHarness() {
  final h = harness();
  h.api.responder = (type, fields) {
    if (type == 'fs.read') {
      return _ok({
        'kind': 'text',
        'path': filePath,
        'size': code.length,
        'mtimeMs': 10,
        'text': code,
        'lang': 'dart',
      });
    }
    if (type == 'git.diff') {
      return _ok({'ok': true, 'text': diffText});
    }
    return _ok(null);
  };
  return h;
}

Future<void> scene(
  WidgetTester tester,
  Harness h,
  Widget home,
  String name,
  Brightness b, {
  Future<void> Function()? after,
}) async {
  final key = GlobalKey();
  await h.pump(
    tester,
    home,
    size: const Size(360, 800),
    boundary: key,
    locale: const Locale('vi'),
    brightness: b,
    fontFamily: 'Roboto',
  );
  for (var i = 0; i < 8; i++) {
    await tester.pump(const Duration(milliseconds: 150));
  }
  await after?.call();
  await shot(tester, key, name);
}

void main() {
  setUpAll(loadFonts);

  for (final dark in [false, true]) {
    final tag = dark ? 'dark' : 'light';
    final b = dark ? Brightness.dark : Brightness.light;

    testWidgets('file $tag', (tester) async {
      await scene(
        tester,
        _viewerHarness(),
        const FilePage(machineId: 'm1', root: root, path: filePath),
        'file-$tag',
        b,
      );
    }, skip: !enabled);

    testWidgets('diff $tag', (tester) async {
      await scene(
        tester,
        _viewerHarness(),
        const DiffPage(
          machineId: 'm1',
          path: 'apps/mobile/lib/ui/terminal/key_bar.dart',
          type: 'git.diff',
          fields: {'root': root, 'path': 'x'},
        ),
        'diff-$tag',
        b,
      );
    }, skip: !enabled);

    testWidgets('settings $tag', (tester) async {
      final lock = makeLock(FakeAuth());
      await tester.runAsync(lock.load);
      await scene(
        tester,
        scenarioHarness(),
        AppLockScope(
          lock: lock,
          child: const WorkspacePage(machineId: 'm1'),
        ),
        'settings-$tag',
        b,
        after: () async {
          await tester.tap(find.byKey(const ValueKey('nav-settings')));
          await tester.pump(const Duration(milliseconds: 400));
        },
      );
      lock.dispose();
    }, skip: !enabled);

    testWidgets('error $tag', (tester) async {
      await scene(
        tester,
        harness(),
        Scaffold(
          appBar: AppBar(title: const Text('Changes')),
          body: ErrorState(
            message: 'Không đọc được thay đổi của repo (Relay không phản hồi, timeout 5s)',
            onRetry: () {},
          ),
        ),
        'error-$tag',
        b,
      );
    }, skip: !enabled);

    testWidgets('empty $tag', (tester) async {
      await scene(
        tester,
        harness(),
        Scaffold(
          appBar: AppBar(title: const Text('Git Log')),
          body: const EmptyState(
            icon: Icons.history_rounded,
            text: 'Chưa có commit nào',
          ),
        ),
        'empty-$tag',
        b,
      );
    }, skip: !enabled);
  }
}
