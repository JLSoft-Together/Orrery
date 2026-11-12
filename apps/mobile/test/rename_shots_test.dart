import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/ui/monitor/monitor_page.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';

import 'support/fonts.dart';
import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

final bool enabled = Platform.environment['WP23_SHOTS'] != null;
final String outDir = Platform.environment['WP23_OUT'] ?? 'shots';

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

Map<String, dynamic> snap() => dashboard(
  terminals: [
    {
      'id': 't-a',
      'title': 'refactor',
      'alias': 'Ads work',
      'status': 'working',
      'cwd': 'C:\\work\\alpha',
    },
    {
      'id': 't-b',
      'title': 'tests',
      'status': 'waiting',
      'cwd': 'C:\\work\\alpha',
    },
  ],
);

void main() {
  setUpAll(loadFonts);

  Future<GlobalKey> open(
    WidgetTester tester,
    Harness h,
    Widget home,
    Locale locale,
    Brightness brightness,
  ) async {
    final key = GlobalKey();
    await h.pump(
      tester,
      home,
      boundary: key,
      locale: locale,
      brightness: brightness,
      fontFamily: 'Roboto',
    );
    return key;
  }

  for (final dark in [false, true]) {
    final tag = dark ? 'dark' : 'light';
    final b = dark ? Brightness.dark : Brightness.light;
    final vi = dark ? const Locale('vi') : const Locale('en');

    testWidgets('home rename $tag', (tester) async {
      final h = Harness(
        api: FakeMachineApi(),
        viewState: view(ws: sample(), snapshot: snap()),
        linkState: linkReady,
        machines: [machine('m1', 'DESKTOP-JIN').withAlias('Office PC')],
      );
      final key = await open(tester, h, const HomePage(), vi, b);
      await shot(tester, key, '01-home-alias-$tag');
      await tester.tap(find.byKey(const ValueKey('machine-menu-m1')));
      await tester.pumpAndSettle();
      await shot(tester, key, '02-home-menu-$tag');
      await tester.tap(find.byKey(const ValueKey('machine-rename')));
      await tester.pumpAndSettle();
      await shot(tester, key, '03-home-dialog-$tag');
    }, skip: !enabled);

    testWidgets('pane rename $tag', (tester) async {
      final h = Harness(
        api: FakeMachineApi(),
        viewState: view(ws: sample(), snapshot: snap()),
        linkState: linkReady,
      );
      final key = await open(
        tester,
        h,
        const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
        vi,
        b,
      );
      await shot(tester, key, '04-pane-header-$tag');
      await tester.tap(find.byTooltip(dark ? 'Thao tác tab' : 'Tab actions'));
      await tester.pumpAndSettle();
      await shot(tester, key, '05-pane-menu-$tag');
      await tester.tap(find.byKey(const ValueKey('action-rename-session')));
      await tester.pumpAndSettle();
      await shot(tester, key, '06-pane-dialog-$tag');
    }, skip: !enabled);

    testWidgets('monitor rename $tag', (tester) async {
      final h = Harness(
        api: FakeMachineApi(),
        viewState: view(ws: sample(), snapshot: snap()),
        linkState: linkReady,
      );
      final key = await open(
        tester,
        h,
        MonitorPage(machineId: 'm1', onOpen: (_) {}),
        vi,
        b,
      );
      await tester.tap(find.byKey(const ValueKey('mon-t-a')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('mon-menu-t-a')));
      await tester.pumpAndSettle();
      await shot(tester, key, '07-monitor-menu-$tag');
    }, skip: !enabled);
  }
}
