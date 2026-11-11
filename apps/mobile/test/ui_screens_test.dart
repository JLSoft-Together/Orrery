import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/ui/terminal/terminal_pane.dart';
import 'package:orrery_remote/ui/workspace/inbox_page.dart';
import 'package:orrery_remote/ui/workspace/mission_page.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

void main() {
  testWidgets('home lists the machine with link, needs count and pane count', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const HomePage());
    expect(find.text('Dev PC'), findsOneWidget);
    expect(find.text('LAN'), findsOneWidget);
    expect(find.text('3 panes'), findsOneWidget);
    expect(find.text('1 need you'), findsOneWidget);
  });

  testWidgets('home shows old data time when offline', (tester) async {
    final h = harness(link: linkOffline, stale: true);
    await h.pump(tester, const HomePage());
    expect(find.text('Offline'), findsOneWidget);
    expect(find.text('Old data, 09:30'), findsOneWidget);
  });

  testWidgets('home empty state invites pairing', (tester) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(),
      linkState: linkReady,
      machines: const [],
    );
    await h.pump(tester, const HomePage());
    expect(find.text('No computer paired yet'), findsOneWidget);
    expect(find.text('Pair a computer'), findsOneWidget);
  });

  testWidgets('swipe to remove asks first, then forgets the machine', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const HomePage());
    await tester.drag(find.text('Dev PC'), const Offset(-400, 0));
    await tester.pumpAndSettle();
    expect(find.text('Remove Dev PC?'), findsOneWidget);
    await tester.tap(find.text('Remove'));
    await tester.pumpAndSettle();
    final machines =
        h.container.read(machinesProvider.notifier) as FakeMachines;
    expect(machines.forgotten, ['m1']);
    expect(find.text('Dev PC'), findsNothing);
  });

  testWidgets('workspace floats the project that needs you first', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    final alpha = tester.getTopLeft(find.text('alpha')).dy;
    final beta = tester.getTopLeft(find.text('beta')).dy;
    expect(alpha < beta, isTrue);
    expect(find.text('1 need you'), findsOneWidget);
    expect(find.text('1 running'), findsOneWidget);
    expect(find.text('1 done'), findsOneWidget);
    expect(find.text('Mission Control'), findsNothing);
  });

  testWidgets('tapping a project opens the pane that needs you', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    await tester.tap(find.text('alpha'));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(find.text('tests'), findsOneWidget);
    expect(find.text('Needs permission'), findsWidgets);
    expect(h.api.attached, ['b']);
  });

  testWidgets('offline banner shows last data time and retries', (
    tester,
  ) async {
    final h = harness(link: linkOffline, stale: true);
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    expect(find.text('Computer offline, data from 09:30'), findsOneWidget);
    await tester.tap(find.text('Retry'));
    expect(h.api.retries, 1);
    h.linkNotifier.emit(linkReady);
    await tester.pumpAndSettle();
    expect(find.text('Computer offline, data from 09:30'), findsNothing);
  });

  testWidgets('revoked link shows the revoked page and clears the cache', (
    tester,
  ) async {
    final h = harness(link: const LinkState(status: LinkStatus.revoked));
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    await tester.pump();
    expect(find.text('The computer revoked this phone'), findsOneWidget);
    expect(h.api.cleared, isTrue);
    expect(find.text('alpha'), findsNothing);
  });

  testWidgets('protocol too old asks to update', (tester) async {
    final h = harness(link: const LinkState(status: LinkStatus.protoTooOld));
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    expect(find.text('Update the app or the desktop'), findsOneWidget);
  });

  testWidgets('vietnamese strings are used when the desktop locale is vi', (
    tester,
  ) async {
    final h = harness();
    await h.pump(
      tester,
      const WorkspacePage(machineId: 'm1'),
      locale: const Locale('vi'),
    );
    expect(find.text('1 cần bạn'), findsOneWidget);
    expect(find.text('Mở pane'), findsOneWidget);
  });

  testWidgets('tab page swipes between panes and follows a closed pane', (
    tester,
  ) async {
    final h = harness();
    await h.pump(
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
    );
    expect(find.text('refactor'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('dot-1')));
    await tester.pumpAndSettle();
    expect(find.text('tests'), findsOneWidget);
    h.viewNotifier.emit(
      view(
        ws: workspace([
          tab('t1', [
            pane('a', title: 'refactor', rail: 'working'),
          ], project: 'C:\\work\\alpha'),
        ]),
        snapshot: dashboard(),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('The pane you were viewing was closed'), findsOneWidget);
    expect(find.text('refactor'), findsOneWidget);
  });

  testWidgets('tab page leaves when its tab disappears', (tester) async {
    final h = harness();
    await h.pump(
      tester,
      Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute<void>(
                builder: (_) =>
                    const TabPage(machineId: 'm1', tabKey: 't2', paneKey: 'c'),
              ),
            ),
            child: const Text('go'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('go'));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    h.viewNotifier.emit(
      view(ws: workspace([sample()['tabs'][0] as Map<String, dynamic>])),
    );
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsNothing);
    expect(find.text('This tab was closed on the computer'), findsOneWidget);
  });

  testWidgets('wide screens draw the split tree with every pane attached', (
    tester,
  ) async {
    final h = harness();
    await h.pump(
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
      size: const Size(1000, 700),
    );
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.byType(TerminalPane), findsNWidgets(2));
    expect(find.byType(PageView), findsNothing);
    expect(h.api.attached.toSet(), {'a', 'b'});
  });

  testWidgets('split view attaches at most four terminals', (tester) async {
    final panes = [for (var i = 0; i < 6; i++) pane('p$i', title: 'pane$i')];
    Map<String, dynamic> leaf(int i) => {'pane': 'p$i'};
    Map<String, dynamic> rowOf(int a) => {
      'dir': 'row',
      'ratio': 0.33,
      'a': leaf(a),
      'b': {'dir': 'row', 'ratio': 0.5, 'a': leaf(a + 1), 'b': leaf(a + 2)},
    };
    final h = harness(
      ws: workspace([
        tab(
          't1',
          panes,
          project: 'C:\\work\\alpha',
          tree: {'dir': 'col', 'ratio': 0.5, 'a': rowOf(0), 'b': rowOf(3)},
        ),
      ]),
    );
    await h.pump(
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'p0'),
      size: const Size(1200, 800),
    );
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.byType(TerminalPane), findsNWidgets(4));
    expect(find.text('Tap to view'), findsNWidgets(2));
  });

  testWidgets('mission control filters panes and opens one', (tester) async {
    final h = harness();
    await h.pump(tester, const MissionPage(machineId: 'm1'));
    expect(find.text('refactor'), findsOneWidget);
    expect(find.text('tests'), findsOneWidget);
    expect(find.text('docs'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('filter-needs')));
    await tester.pumpAndSettle();
    expect(find.text('refactor'), findsNothing);
    expect(find.text('tests'), findsOneWidget);
    await tester.drag(find.byType(ListView).first, const Offset(-300, 0));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('filter-done')));
    await tester.pumpAndSettle();
    expect(find.text('docs'), findsOneWidget);
    await tester.tap(find.text('docs'));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
  });

  testWidgets('inbox lists what needs you and jumps to it', (tester) async {
    final h = harness();
    await h.pump(tester, const InboxPage(machineId: 'm1'));
    expect(find.text('tests'), findsOneWidget);
    expect(find.text('docs'), findsOneWidget);
    expect(find.text('refactor'), findsNothing);
    await tester.tap(find.text('tests'));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(h.api.attached, ['b']);
  });

  testWidgets('inbox is friendly when empty', (tester) async {
    final h = harness(
      ws: workspace([
        tab('t1', [pane('a')], project: 'C:\\work\\alpha'),
      ]),
    );
    await h.pump(tester, const InboxPage(machineId: 'm1'));
    expect(find.text('Nothing needs you'), findsOneWidget);
  });
}
