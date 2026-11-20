import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/model/inbox_model.dart';
import 'package:orrery_remote/ui/model/workspace_model.dart';
import 'package:orrery_remote/ui/workspace/inbox_page.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

Map<String, dynamic> withInbox(Map<String, dynamic> pane, Object? inbox) => {
  ...pane,
  'inbox': inbox,
};

Map<String, dynamic> flagged(List<Map<String, dynamic>> tabs) => {
  ...workspace(tabs),
  'inbox': true,
};

List<String> tsList(String source, String name) {
  final match = RegExp('export const $name = \\[([^\\]]*)\\]')
      .firstMatch(source);
  return RegExp("'([a-z]+)'")
      .allMatches(match!.group(1)!)
      .map((m) => m.group(1)!)
      .toList();
}

void main() {
  test('group order is the hard-coded shared order', () {
    expect(inboxGroups, [
      'menu',
      'permission',
      'question',
      'clash',
      'error',
      'note',
      'done',
    ]);
    expect(
      inboxReasons.map(groupOf).toSet().toList()..sort(),
      [...inboxGroups]..sort(),
    );
    expect(groupOf('failed'), 'error');
    expect(groupOf('loop'), 'error');
    expect(groupOf('checks'), 'error');
    expect(groupOf('stuck'), 'error');
    expect(groupOf('clash'), 'clash');
  });

  test('constants match packages/shared/src/inbox.ts', () {
    final file = File('../../packages/shared/src/inbox.ts');
    if (!file.existsSync()) return;
    final source = file.readAsStringSync();
    expect(tsList(source, 'INBOX_REASONS'), inboxReasons);
    expect(tsList(source, 'INBOX_GROUPS'), inboxGroups);
  });

  test('without the flag the inbox falls back to attention', () {
    final model = WorkspaceModel.parse(
      workspace([
        tab('t1', [
          withInbox(pane('a'), {'reason': 'note'}),
          pane('b', attention: 'done', rail: 'done'),
          pane('c', rail: 'asked', waitText: 'Allow?'),
        ]),
      ]),
    );
    expect(model.inboxMode, isFalse);
    final rows = model.inboxRows;
    expect(rows.map((r) => r.pane.key), ['c', 'b']);
    expect(rows.first.text, 'Allow?');
  });

  test('with the flag only panes[].inbox count, unknown reason skipped', () {
    final model = WorkspaceModel.parse(
      flagged([
        tab('t1', [
          pane('a', attention: 'done', rail: 'done'),
          withInbox(pane('b'), {'reason': 'weird'}),
          withInbox(pane('c'), {'reason': 'done'}),
          withInbox(pane('d'), {'reason': 'menu'}),
          withInbox(pane('e'), {
            'reason': 'clash',
            'file': 'lib/x.dart',
            'peer': 'codex',
          }),
          withInbox(pane('f'), {'reason': 'stuck'}),
          withInbox(pane('g'), {'reason': 'note'}),
        ]),
      ]),
    );
    expect(model.inboxMode, isTrue);
    final rows = model.inboxRows;
    expect(rows.map((r) => r.pane.key), ['d', 'e', 'f', 'g', 'c']);
    expect(rows.map((r) => r.group), [
      'menu',
      'clash',
      'error',
      'note',
      'done',
    ]);
    expect(rows[1].file, 'lib/x.dart');
    expect(rows[1].peer, 'codex');
    expect(model.inbox.length, 5);
  });

  test('oldest since first inside a group, strings cut to 120', () {
    final long = 'x' * 300;
    final model = WorkspaceModel.parse(
      flagged([
        tab('t1', [
          withInbox(pane('a'), {
            'reason': 'failed',
            'since': '2026-11-20T10:05:00Z',
          }),
          withInbox(pane('b'), {
            'reason': 'loop',
            'since': '2026-11-20T10:01:00Z',
          }),
          withInbox(pane('c'), {'reason': 'checks'}),
          withInbox(pane('d'), {'reason': 'clash', 'file': long}),
        ]),
      ]),
    );
    final rows = model.inboxRows;
    expect(rows.map((r) => r.pane.key), ['d', 'b', 'a', 'c']);
    expect(rows.first.file!.length, 120);
  });

  test('groupInboxRows omits empty groups in order', () {
    final model = WorkspaceModel.parse(
      flagged([
        tab('t1', [
          withInbox(pane('a'), {'reason': 'done'}),
          withInbox(pane('b'), {'reason': 'question'}),
        ]),
      ]),
    );
    final groups = groupInboxRows(model.inboxRows);
    expect(groups.map((g) => g.$1), ['question', 'done']);
  });

  testWidgets('inbox page shows a section per group with clash detail', (
    tester,
  ) async {
    final h = harness(
      ws: flagged([
        tab('t1', [
          withInbox(pane('a', title: 'alpha'), {'reason': 'done'}),
          withInbox(pane('b', title: 'beta'), {
            'reason': 'clash',
            'file': 'lib/x.dart',
            'peer': 'codex',
          }),
          withInbox(pane('c', title: 'gamma'), {'reason': 'menu'}),
        ], project: 'C:\\work\\alpha'),
      ]),
    );
    await h.pump(tester, const InboxPage(machineId: 'm1'));
    expect(find.text('Waiting on a menu'), findsOneWidget);
    expect(find.text('Same file'), findsOneWidget);
    expect(find.text('Done, not seen'), findsOneWidget);
    expect(find.text('Errors'), findsNothing);
    expect(
      find.textContaining('Editing lib/x.dart with codex'),
      findsOneWidget,
    );
    final menuY = tester.getTopLeft(find.text('Waiting on a menu')).dy;
    final clashY = tester.getTopLeft(find.text('Same file')).dy;
    final doneY = tester.getTopLeft(find.text('Done, not seen')).dy;
    expect(menuY < clashY && clashY < doneY, isTrue);
    await tester.tap(find.text('gamma'));
    await tester.pumpAndSettle();
    expect(h.api.attached, ['c']);
  });
}
