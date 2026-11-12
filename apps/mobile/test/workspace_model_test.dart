import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/model/pane_facts.dart';
import 'package:orrery_remote/ui/model/workspace_model.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';

import 'support/ui_fakes.dart';

void main() {
  test('panes follow the order of leaves(tree), not the list order', () {
    final raw = tab(
      't1',
      [pane('b'), pane('a'), pane('c')],
      tree: {
        'dir': 'row',
        'ratio': 0.4,
        'a': {'pane': 'a'},
        'b': {
          'dir': 'col',
          'ratio': 0.5,
          'a': {'pane': 'c'},
          'b': {'pane': 'b'},
        },
      },
    );
    final parsed = TabInfo.parse(raw)!;
    expect(parsed.panes.map((p) => p.key), ['a', 'c', 'b']);
    expect(parsed.tree!.leaves, ['a', 'c', 'b']);
  });

  test('panes missing from the tree are kept at the end', () {
    final raw = tab('t1', [pane('a'), pane('x')], tree: {'pane': 'a'});
    expect(TabInfo.parse(raw)!.panes.map((p) => p.key), ['a', 'x']);
  });

  test('projects group tabs and float needs-you panes first', () {
    final model = WorkspaceModel.parse(
      workspace([
        tab('t1', [pane('a')], project: 'C:\\work\\alpha'),
        tab('t2', [pane('b', rail: 'asked')], project: 'C:\\work\\beta'),
        tab('t3', [pane('c')], project: 'C:\\work\\alpha'),
        tab('t4', [pane('d')]),
      ]),
    );
    final projects = model.projects;
    expect(projects.map((p) => p.name), ['beta', 'alpha', null]);
    expect(projects[1].tabs.map((t) => t.key), ['t1', 't3']);
    expect(projects[0].needsCount, 1);
  });

  test('inbox orders asked before waiting before done', () {
    final model = WorkspaceModel.parse(
      workspace([
        tab('t1', [
          pane('a', attention: 'done', rail: 'done'),
          pane('b', attention: 'waiting'),
          pane('c', rail: 'asked'),
          pane('d'),
        ]),
      ]),
    );
    expect(model.inbox.map((p) => p.key), ['c', 'b', 'a']);
  });

  test('bad input never throws', () {
    expect(WorkspaceModel.parse(null).tabs, isEmpty);
    expect(
      WorkspaceModel.parse({
        'tabs': [
          1,
          {'key': 5},
          {'key': 'ok', 'panes': 'nope', 'tree': 3},
        ],
      }).tabs.map((t) => t.key),
      ['ok'],
    );
    expect(LayoutNode.parse({'dir': 'row'}), isNull);
  });

  test('pane facts read the main agent tool and the terminal statusSince', () {
    final snapshot = dashboard(
      agents: [
        {'terminalId': 't-a', 'role': 'subagent', 'currentTool': 'Read'},
        {
          'terminalId': 't-a',
          'role': 'main',
          'currentTool': 'Bash',
          'currentToolSummary': 'npm test',
          'currentToolStartedAt': '2026-11-12T09:00:00.000Z',
        },
      ],
      terminals: [
        {'id': 't-a', 'statusSince': '2026-11-12T08:59:00.000Z'},
      ],
    );
    final facts = PaneFacts.of(snapshot, 't-a');
    expect(facts.tool, 'Bash');
    expect(facts.toolSummary, 'npm test');
    expect(facts.toolSince, DateTime.parse('2026-11-12T09:00:00.000Z'));
    expect(facts.statusSince, DateTime.parse('2026-11-12T08:59:00.000Z'));
    expect(PaneFacts.of(snapshot, 'other').hasTool, isFalse);
    expect(PaneFacts.of(null, 't-a').hasTool, isFalse);
  });

  test('theme follows hello mode, palette is parsed but the app keeps orrery tokens', () {
    final remote = RemoteTheme.fromHello({
      'mode': 'dark',
      'palette': {'--ccm-accent': '#ff8800', 'bad': 12},
    });
    expect(remote.mode, ThemeMode.dark);
    expect(remote.pick(['accent']), const Color(0xFFFF8800));
    expect(RemoteTheme.fromHello(null).mode, ThemeMode.system);
    expect(parseColor('#abc'), const Color(0xFFAABBCC));
    expect(parseColor('red'), isNull);
    final theme = buildOrreryTheme(remote, Brightness.dark);
    expect(theme.colorScheme.primary, const Color(0xFFE8835D));
  });
}
