import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';
import 'package:orrery_remote/ui/workspace/inbox_page.dart';
import 'package:orrery_remote/ui/workspace/mission_page.dart';
import 'package:orrery_remote/ui/workspace/open_pane_page.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/ui_fakes.dart';

final bool enabled = Platform.environment['WP11_SHOTS'] != null;
final String outDir = Platform.environment['WP11_OUT'] ?? 'shots';

Future<void> loadFonts() async {
  final root =
      Platform.environment['FLUTTER_ROOT'] ?? 'D:/Flutter/develop/flutter';
  final dir = '$root/bin/cache/artifacts/material_fonts';
  Future<ByteData> bytes(String path) async =>
      ByteData.sublistView(await File(path).readAsBytes());
  final roboto = FontLoader('Roboto')
    ..addFont(bytes('$dir/Roboto-Regular.ttf'))
    ..addFont(bytes('$dir/Roboto-Medium.ttf'))
    ..addFont(bytes('$dir/Roboto-Bold.ttf'));
  await roboto.load();
  final icons = FontLoader('MaterialIcons')
    ..addFont(bytes('$dir/MaterialIcons-Regular.otf'));
  await icons.load();
  for (final family in ['monospace', 'Consolas']) {
    final mono = FontLoader(family)
      ..addFont(bytes('C:/Windows/Fonts/consola.ttf'))
      ..addFont(bytes('C:/Windows/Fonts/consolab.ttf'));
    await mono.load();
  }
}

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

Uint8List ansi(String text) => Uint8List.fromList(utf8.encode(text));

const String demo =
    '\x1b[1;38;5;208m> Orrery Remote\x1b[0m \x1b[2mv1.0\x1b[0m\r\n'
    '\x1b[32m✔\x1b[0m Đọc 12 file trong \x1b[36mapps/mobile/lib\x1b[0m\r\n'
    '\x1b[32m✔\x1b[0m Sửa \x1b[36mterminal_pane.dart\x1b[0m (+48 -12)\r\n'
    '\x1b[33m●\x1b[0m Chạy \x1b[1mflutter test\x1b[0m\r\n'
    '  00:07 +46: All tests passed!\r\n\r\n'
    '\x1b[1m Do you want to run this command?\x1b[0m\r\n'
    '  \x1b[7m 1. Yes \x1b[0m\r\n  2. Yes, and don\'t ask again\r\n  3. No\r\n';

Map<String, dynamic> sampleWs() => workspace([
  tab('t1', [
    pane(
      'a',
      title: 'Refactor remote layer',
      rail: 'working',
      terminalId: 't-a',
    ),
    pane(
      'b',
      title: 'Fix PTY resize',
      agent: 'codex',
      rail: 'asked',
      waitText: 'Bash: flutter test --concurrency=4',
      terminalId: 't-b',
    ),
  ], project: 'D:\\work\\web\\claude-code-monitor'),
  tab('t2', [
    pane('c', title: 'Release notes', rail: 'done', attention: 'done'),
  ], project: 'D:\\work\\android\\WeJoy\\Dumpling-5'),
  tab('t3', [
    pane('d', title: 'shell', agent: null, rail: 'idle'),
    pane('e', title: 'Idle agent', rail: 'idle', attention: 'waiting'),
  ], project: 'D:\\work\\web\\orrery-relay'),
], closed: 2);

Map<String, dynamic> sampleSnapshot() => dashboard(
  agents: [
    {
      'terminalId': 't-a',
      'role': 'main',
      'currentTool': 'Bash',
      'currentToolSummary': 'flutter analyze',
      'currentToolStartedAt': DateTime.now()
          .subtract(const Duration(seconds: 73))
          .toUtc()
          .toIso8601String(),
    },
  ],
  favorites: [
    {
      'dir': 'D:\\work\\web\\claude-code-monitor',
      'label': 'Orrery',
      'addedAt': 'x',
    },
    {
      'dir': 'D:\\work\\android\\WeJoy\\Dumpling-5',
      'label': 'Dumpling 5',
      'addedAt': 'x',
    },
  ],
);

Harness make({
  Map<String, dynamic>? ws,
  LinkState link = linkReady,
  bool stale = false,
  List<ConfirmAsk> confirms = const [],
  List<dynamic>? machines,
}) => Harness(
  api: FakeMachineApi(),
  viewState: view(
    ws: ws ?? sampleWs(),
    snapshot: sampleSnapshot(),
    stale: stale,
    confirms: confirms,
  ),
  linkState: link,
  machines: machines?.cast(),
);

void main() {
  setUpAll(loadFonts);

  void sTest(String name, Future<void> Function(WidgetTester) body) {
    testWidgets(name, (tester) async {
      debugDisableShadows = false;
      try {
        await body(tester);
      } finally {
        debugDisableShadows = true;
      }
    });
  }

  Future<void> run(
    String name,
    WidgetTester tester,
    Widget home, {
    Harness? harness,
    Size size = const Size(390, 844),
    Brightness brightness = Brightness.light,
    Locale locale = const Locale('en'),
    Future<void> Function(Harness h)? after,
  }) async {
    final h = harness ?? make();
    final key = GlobalKey();
    await h.pump(
      tester,
      home,
      size: size,
      brightness: brightness,
      locale: locale,
      boundary: key,
      fontFamily: 'Roboto',
    );
    if (after != null) await after(h);
    await shot(tester, key, name);
  }

  Future<void> feed(WidgetTester tester, Harness h) async {
    await tester.pump(const Duration(milliseconds: 200));
    for (final handle in h.api.handles) {
      handle.out.add(ansi(demo));
    }
    await tester.pump(const Duration(milliseconds: 200));
  }

  sTest('01 machines', (tester) async {
    await run('01-machines', tester, const HomePage(), harness: make());
  });

  sTest('01b machines dark', (tester) async {
    await run(
      '01b-machines-dark',
      tester,
      const HomePage(),
      brightness: Brightness.dark,
    );
  });

  sTest('02 machines empty', (tester) async {
    await run(
      '02-machines-empty',
      tester,
      const HomePage(),
      harness: make(machines: const []),
    );
  });

  sTest('03 workspace', (tester) async {
    await run('03-workspace', tester, const WorkspacePage(machineId: 'm1'));
  });

  sTest('03b workspace dark', (tester) async {
    await run(
      '03b-workspace-dark',
      tester,
      const WorkspacePage(machineId: 'm1'),
      brightness: Brightness.dark,
    );
  });

  sTest('03c workspace vi', (tester) async {
    await run(
      '03c-workspace-vi',
      tester,
      const WorkspacePage(machineId: 'm1'),
      locale: const Locale('vi'),
    );
  });

  sTest('04 workspace offline', (tester) async {
    await run(
      '04-workspace-offline',
      tester,
      const WorkspacePage(machineId: 'm1'),
      harness: make(link: linkOffline, stale: true),
    );
  });

  sTest('05 pane asked', (tester) async {
    final h = make();
    await run(
      '05-pane-asked',
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'b'),
      harness: h,
      after: (h) => feed(tester, h),
    );
  });

  sTest('05b pane working dark', (tester) async {
    await run(
      '05b-pane-working-dark',
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
      brightness: Brightness.dark,
      after: (h) => feed(tester, h),
    );
  });

  sTest('06 tablet split', (tester) async {
    await run(
      '06-tablet-split',
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'b'),
      size: const Size(1000, 700),
      after: (h) => feed(tester, h),
    );
  });

  sTest('06b landscape phone', (tester) async {
    await run(
      '06b-landscape',
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
      size: const Size(844, 390),
      after: (h) => feed(tester, h),
    );
  });

  sTest('07 mission control', (tester) async {
    await run('07-mission', tester, const MissionPage(machineId: 'm1'));
  });

  sTest('08 inbox', (tester) async {
    await run('08-inbox', tester, const InboxPage(machineId: 'm1'));
  });

  sTest('09 open pane', (tester) async {
    await run(
      '09-open-pane',
      tester,
      const OpenPanePage(machineId: 'm1'),
      after: (h) async {
        await tester.tap(find.text('Orrery'));
        await tester.pump(const Duration(milliseconds: 300));
      },
    );
  });

  sTest('10 confirm sheet', (tester) async {
    final h = make(
      confirms: [
        ConfirmAsk(
          cid: 'c1',
          title: 'Close a busy pane?',
          detail: 'Fix PTY resize is still running a command.',
          ok: 'Close pane',
          cancel: 'Keep it',
          receivedAt: DateTime.now(),
        ),
      ],
    );
    await run(
      '10-confirm',
      tester,
      const WorkspacePage(machineId: 'm1'),
      harness: h,
    );
  });

  sTest('11 pane actions', (tester) async {
    await run(
      '11-pane-actions',
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
      after: (h) async {
        await feed(tester, h);
        await tester.tap(find.byTooltip('Tab actions'));
        await tester.pump(const Duration(milliseconds: 500));
      },
    );
  });

  sTest('12 revoked', (tester) async {
    await run(
      '12-revoked',
      tester,
      const WorkspacePage(machineId: 'm1'),
      harness: make(link: const LinkState(status: LinkStatus.revoked)),
    );
  });

  sTest('13 proto too old', (tester) async {
    await run(
      '13-proto',
      tester,
      const WorkspacePage(machineId: 'm1'),
      harness: make(link: const LinkState(status: LinkStatus.protoTooOld)),
    );
  });

  sTest('14 desktop palette', (tester) async {
    final key = GlobalKey();
    final h = make();
    await h.pump(
      tester,
      const WorkspacePage(machineId: 'm1'),
      boundary: key,
      fontFamily: 'Roboto',
      brightness: Brightness.dark,
      remote: RemoteTheme.fromHello({
        'mode': 'dark',
        'palette': {
          'accent': '#ff7a45',
          'bg': '#101418',
          'surface': '#1a2027',
          'ink': '#e8edf2',
        },
      }),
    );
    await shot(tester, key, '14-desktop-palette');
  });
}
