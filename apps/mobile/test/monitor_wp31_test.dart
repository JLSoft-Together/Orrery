import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/monitor/monitor_page.dart';
import 'package:orrery_remote/ui/monitor/view_prefs.dart';

import 'support/ui_fakes.dart';
import 'support/wp31_scenario.dart';

void main() {
  testWidgets('monitor A shows cards, filters by chip and opens detail', (
    tester,
  ) async {
    final opened = <String>[];
    final h = scenarioHarness();
    await h.pump(tester, MonitorPage(machineId: 'm1', onOpen: opened.add));
    await tester.pump();
    expect(find.text('Sửa lỗi pty.overflow'), findsOneWidget);
    expect(find.text('Review PR #482'), findsOneWidget);
    expect(find.text('Tất cả 5'), findsNothing);
    await tester.tap(find.byKey(const ValueKey('mon-filter-running')));
    await tester.pumpAndSettle();
    expect(find.text('Review PR #482'), findsNothing);
    expect(find.text('Refactor AdsGate'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('mon-t-c')));
    await tester.pumpAndSettle();
    expect(find.text('SUB-AGENTS · 3'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('detail-open-terminal')));
    expect(opened, ['t-c']);
  });

  testWidgets('view mode toggle groups by state and is remembered', (
    tester,
  ) async {
    final h = scenarioHarness();
    await h.pump(tester, MonitorPage(machineId: 'm1', onOpen: (_) {}));
    await tester.pump();
    expect(find.byKey(const ValueKey('mon-filter-all')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('monitor-view-mode')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('mon-filter-all')), findsNothing);
    expect(find.text('CHỜ BẠN · 1'), findsNothing);
    expect(find.text('WAITING FOR YOU · 1'), findsOneWidget);
    expect(find.text('RUNNING · 2'), findsOneWidget);
    expect(find.text('IDLE · 2'), findsOneWidget);
    expect((h.prefs as MemoryViewPrefsStore).value.monitor, ViewMode.alternate);
  });

  testWidgets('saved view mode is applied on start', (tester) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(ws: scenarioWorkspace(), snapshot: scenarioSnapshot()),
      linkState: linkReady,
      prefsStore: MemoryViewPrefsStore(
        const ViewPrefs(monitor: ViewMode.alternate),
      ),
    );
    await h.pump(tester, MonitorPage(machineId: 'm1', onOpen: (_) {}));
    await tester.pumpAndSettle();
    expect(find.text('RUNNING · 2'), findsOneWidget);
  });

  testWidgets('prefs round trip through json', (tester) async {
    const prefs = ViewPrefs(
      monitor: ViewMode.alternate,
      workspace: ViewMode.alternate,
    );
    final back = ViewPrefs.fromJson(prefs.toJson());
    expect(back.monitor, ViewMode.alternate);
    expect(back.workspace, ViewMode.alternate);
    expect(ViewPrefs.fromJson('junk').monitor, ViewMode.primary);
  });

  testWidgets(
    'removed session slides out when motion is on, instantly when reduced',
    (tester) async {
      for (final quiet in [false, true]) {
        final h = scenarioHarness();
        await h.pump(
          tester,
          MonitorPage(machineId: 'm1', onOpen: (_) {}),
          quiet: quiet,
        );
        await tester.pump(const Duration(milliseconds: 400));
        expect(find.text('npm run dev'), findsOneWidget);
        final snap = scenarioSnapshot();
        (snap['terminals'] as List).removeWhere(
          (t) => (t as Map)['id'] == 't-d',
        );
        h.viewNotifier.emit(view(ws: scenarioWorkspace(), snapshot: snap));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 40));
        expect(find.text('npm run dev'), quiet ? findsNothing : findsOneWidget);
        await tester.pump(const Duration(milliseconds: 400));
        expect(find.text('npm run dev'), findsNothing);
        await tester.pumpWidget(const SizedBox());
      }
    },
  );
}
