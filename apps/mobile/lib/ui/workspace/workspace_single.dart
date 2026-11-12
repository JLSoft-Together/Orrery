import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'pane_actions.dart';
import 'pane_card.dart';
import 'workspace_page.dart';

class SingleMode extends ConsumerWidget {
  const SingleMode({
    super.key,
    required this.machineId,
    required this.model,
    required this.tab,
    required this.paneKey,
    required this.pagesOf,
    required this.onPage,
    required this.onChip,
  });

  final String machineId;
  final WorkspaceModel model;
  final TabInfo tab;
  final String? paneKey;
  final PageController Function(int initial) pagesOf;
  final ValueChanged<int> onPage;
  final ValueChanged<int> onChip;

  int get _index {
    final key = paneKey;
    if (key != null) {
      final found = tab.panes.indexWhere((p) => p.key == key);
      if (found >= 0) return found;
    }
    final needs = tab.panes.indexWhere((p) => p.needsUser);
    return needs >= 0 ? needs : 0;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final c = context.orrery;
    final panes = tab.panes;
    if (panes.isEmpty) return EmptyState(title: l.missionEmpty);
    final index = _index.clamp(0, panes.length - 1);
    final controller = pagesOf(index);
    final totals = <String, int>{};
    for (final p in panes) {
      totals.update(agentLabel(p.agent), (v) => v + 1, ifAbsent: () => 1);
    }
    final seen = <String, int>{};
    final labels = [
      for (final p in panes)
        () {
          final base = agentLabel(p.agent);
          final n = seen.update(base, (v) => v + 1, ifAbsent: () => 1);
          return totals[base]! > 1 ? '$base $n' : base;
        }(),
    ];
    return Column(
      children: [
        SizedBox(
          height: 56,
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
            itemCount: panes.length,
            separatorBuilder: (_, _) => const SizedBox(width: 8),
            itemBuilder: (context, i) => OrreryFilterChip(
              key: ValueKey('pane-chip-${panes[i].key}'),
              label: labels[i],
              selected: i == index,
              leading: DecoratedBox(
                decoration: BoxDecoration(
                  color: agentColor(context, panes[i].agent),
                  shape: BoxShape.circle,
                ),
                child: const SizedBox.square(dimension: 8),
              ),
              onTap: () => onChip(i),
            ),
          ),
        ),
        Expanded(
          child: PageView.builder(
            controller: controller,
            itemCount: panes.length,
            onPageChanged: onPage,
            itemBuilder: (context, i) {
              final pane = panes[i];
              return RefreshIndicator(
                onRefresh: () async {
                  try {
                    await ref
                        .read(machineApiProvider(machineId))
                        .command('ws.get');
                  } on Object {
                    return;
                  }
                },
                child: ListView(
                  physics: const AlwaysScrollableScrollPhysics(),
                  padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
                  children: [
                    PaneCard(
                      key: ValueKey('pane-card-${pane.key}'),
                      machineId: machineId,
                      pane: pane,
                      onOpen: () => openPane(context, machineId, tab, pane.key),
                      onLongPress: () => showTabActions(
                        context,
                        ref,
                        machineId,
                        model,
                        tab,
                        pane,
                      ),
                    ),
                    Padding(
                      padding: const EdgeInsets.only(top: 12),
                      child: Text(
                        l.wsPaneHint(panes.length),
                        textAlign: TextAlign.center,
                        style: TextStyle(fontSize: 13, color: c.inkFaint),
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}
