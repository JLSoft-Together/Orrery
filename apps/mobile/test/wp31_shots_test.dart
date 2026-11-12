import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/monitor/view_prefs.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/fonts.dart';
import 'support/ui_fakes.dart';
import 'support/wp31_scenario.dart';

final bool enabled = Platform.environment['WP31_SHOTS'] != null;
final String outDir = Platform.environment['WP31_OUT'] ?? 'shots';

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

Future<GlobalKey> openShell(
  WidgetTester tester,
  Harness h,
  Brightness b,
) async {
  final key = GlobalKey();
  await h.pump(
    tester,
    const WorkspacePage(machineId: 'm1'),
    size: const Size(360, 800),
    boundary: key,
    locale: const Locale('vi'),
    brightness: b,
    fontFamily: 'Roboto',
  );
  await tester.pump(const Duration(milliseconds: 300));
  return key;
}

void main() {
  setUpAll(loadFonts);

  for (final dark in [false, true]) {
    final tag = dark ? 'dark' : 'light';
    final b = dark ? Brightness.dark : Brightness.light;

    for (final alt in [false, true]) {
      final name = alt ? 'mon-b' : 'mon-a';
      testWidgets('$name $tag', (tester) async {
        final h = Harness(
          api: FakeMachineApi(),
          viewState: view(
            ws: scenarioWorkspace(),
            snapshot: scenarioSnapshot(),
          ),
          linkState: linkReady,
          machines: [machine('m1', 'DESKTOP-DAUPH')],
          prefsStore: MemoryViewPrefsStore(
            ViewPrefs(monitor: alt ? ViewMode.alternate : ViewMode.primary),
          ),
        );
        final key = await openShell(tester, h, b);
        await tester.tap(find.byKey(const ValueKey('nav-monitor')));
        await tester.pump(const Duration(milliseconds: 100));
        await shot(tester, key, '$name-$tag');
      }, skip: !enabled);
    }

    testWidgets('ws-a $tag', (tester) async {
      final h = scenarioHarness();
      final key = await openShell(tester, h, b);
      await tester.tap(find.byKey(const ValueKey('pane-chip-a')));
      await tester.pump(const Duration(milliseconds: 100));
      await shot(tester, key, 'ws-a-$tag');
    }, skip: !enabled);

    testWidgets('ws-b $tag', (tester) async {
      final h = Harness(
        api: FakeMachineApi(),
        viewState: view(ws: scenarioWorkspace(), snapshot: scenarioSnapshot()),
        linkState: linkReady,
        machines: [machine('m1', 'DESKTOP-DAUPH')],
        prefsStore: MemoryViewPrefsStore(
          const ViewPrefs(workspace: ViewMode.alternate),
        ),
      );
      final key = await openShell(tester, h, b);
      await shot(tester, key, 'ws-b-$tag');
    }, skip: !enabled);

    testWidgets('term $tag', (tester) async {
      final h = scenarioHarness();
      final key = GlobalKey();
      await h.pump(
        tester,
        const TabPage(machineId: 'm1', tabKey: 'tab1', paneKey: 'a'),
        size: const Size(360, 800),
        boundary: key,
        locale: const Locale('vi'),
        brightness: b,
        fontFamily: 'Roboto',
      );
      await tester.pump(const Duration(milliseconds: 100));
      h.api.handles.single.out.add(Uint8List.fromList(utf8.encode(termDemo)));
      await tester.pump(const Duration(milliseconds: 100));
      await shot(tester, key, 'term-$tag');
    }, skip: !enabled);

    testWidgets('term-pick $tag', (tester) async {
      final h = scenarioHarness();
      final key = GlobalKey();
      await h.pump(
        tester,
        const TabPage(machineId: 'm1', tabKey: 'tab1', paneKey: 'a'),
        size: const Size(360, 800),
        boundary: key,
        locale: const Locale('vi'),
        brightness: b,
        fontFamily: 'Roboto',
      );
      await tester.pump(const Duration(milliseconds: 100));
      h.api.handles.single.out.add(Uint8List.fromList(utf8.encode(termDemo)));
      await tester.tap(find.byKey(const ValueKey('pane-picker')));
      await tester.pump(const Duration(milliseconds: 100));
      await shot(tester, key, 'term-pick-$tag');
    }, skip: !enabled);

    testWidgets('detail $tag', (tester) async {
      final h = scenarioHarness();
      final key = await openShell(tester, h, b);
      await tester.tap(find.byKey(const ValueKey('nav-monitor')));
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(find.byKey(const ValueKey('mon-t-a')));
      await tester.pump(const Duration(milliseconds: 100));
      await shot(tester, key, 'detail-$tag');
    }, skip: !enabled);
  }
}
