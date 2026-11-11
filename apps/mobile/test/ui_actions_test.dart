import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/workspace/open_pane_page.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

void main() {
  testWidgets('open pane sends ws.open with agent, prompt and submit', (
    tester,
  ) async {
    final h = harness(
      snapshot: dashboard(
        favorites: [
          {'dir': 'C:\\work\\fav', 'label': 'Fav app', 'addedAt': 'x'},
        ],
      ),
    );
    await h.pump(tester, const OpenPanePage(machineId: 'm1'));
    expect(find.text('Fav app'), findsOneWidget);
    expect(find.text('alpha'), findsOneWidget);
    await tester.tap(find.text('Fav app'));
    await tester.tap(find.text('Codex'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const ValueKey('prompt')), 'fix the bug');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('submit')));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('open')));
    await tester.pumpAndSettle();
    expect(h.api.commands.single.$1, 'ws.open');
    expect(h.api.commands.single.$2, {
      'cwd': 'C:\\work\\fav',
      'preset': 'codex',
      'prompt': 'fix the bug',
      'submit': true,
    });
  });

  testWidgets('open pane reports command errors and stays', (tester) async {
    final h = harness();
    h.api.commandError = const RemoteCommandException('limit');
    await h.pump(tester, const OpenPanePage(machineId: 'm1'));
    await tester.tap(find.text('alpha'));
    await tester.tap(find.text('Shell'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('open')));
    await tester.pumpAndSettle();
    expect(find.text('Limit reached.'), findsOneWidget);
    expect(find.byType(OpenPanePage), findsOneWidget);
    expect(h.api.commands.single.$2, {
      'cwd': 'C:\\work\\alpha',
      'preset': 'shell',
    });
  });

  testWidgets('open pane is disabled while the computer is offline', (
    tester,
  ) async {
    final h = harness(link: linkOffline);
    await h.pump(tester, const OpenPanePage(machineId: 'm1'));
    await tester.tap(find.text('alpha'));
    await tester.pump();
    final button = tester.widget<ButtonStyleButton>(
      find.byKey(const ValueKey('open')),
    );
    expect(button.onPressed, isNull);
  });

  testWidgets('confirm.ask opens a sheet, answers, and closes when answered', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    h.viewNotifier.emit(
      view(
        ws: sample(),
        snapshot: dashboard(),
        confirms: [
          ConfirmAsk(
            cid: 'c1',
            title: 'Close busy pane?',
            detail: 'tests is running',
            ok: 'Close',
            cancel: 'Keep',
            receivedAt: DateTime.now(),
          ),
        ],
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Close busy pane?'), findsOneWidget);
    expect(find.text('tests is running'), findsOneWidget);
    await tester.tap(find.text('Close').last);
    await tester.pump();
    expect(h.api.answers, [('c1', true)]);
    h.viewNotifier.emit(view(ws: sample(), snapshot: dashboard()));
    await tester.pumpAndSettle();
    expect(find.text('Close busy pane?'), findsNothing);
  });

  testWidgets('confirm sheet closes by itself when the ask expires', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const WorkspacePage(machineId: 'm1'));
    h.viewNotifier.emit(
      view(
        ws: sample(),
        confirms: [
          ConfirmAsk(
            cid: 'c2',
            title: 'Sure?',
            ok: 'OK',
            cancel: 'No',
            receivedAt: DateTime.now(),
          ),
        ],
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Sure?'), findsOneWidget);
    h.viewNotifier.emit(view(ws: sample()));
    await tester.pumpAndSettle();
    expect(find.text('Sure?'), findsNothing);
  });

  testWidgets('tab actions rename the tab and close a pane after confirming', (
    tester,
  ) async {
    final h = harness();
    await h.pump(
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
    );
    await tester.tap(find.byTooltip('Tab actions'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('action-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-field')),
      'Backend',
    );
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(h.api.commands.last.$1, 'ws.rename');
    expect(h.api.commands.last.$2, {'tabKey': 't1', 'name': 'Backend'});
    await tester.tap(find.byTooltip('Tab actions'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('action-close')));
    await tester.pumpAndSettle();
    expect(find.text('Close refactor?'), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, 'Close pane'));
    await tester.pumpAndSettle();
    expect(h.api.commands.last.$1, 'ws.close');
    expect(h.api.commands.last.$2, {'paneKey': 'a'});
  });
}
