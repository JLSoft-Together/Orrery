import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../model/workspace_model.dart';
import '../theme/orrery_theme.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'workspace_page.dart';

enum MissionFilter { all, needs, running, done, idle }

bool matches(MissionFilter filter, PaneInfo pane) => switch (filter) {
  MissionFilter.all => true,
  MissionFilter.needs => pane.needsUser,
  MissionFilter.running =>
    pane.rail == RailState.working || pane.rail == RailState.subagents,
  MissionFilter.done => pane.isDone && !pane.needsUser,
  MissionFilter.idle =>
    !pane.needsUser &&
        !pane.isDone &&
        pane.rail != RailState.working &&
        pane.rail != RailState.subagents,
};

class MissionPage extends ConsumerStatefulWidget {
  const MissionPage({super.key, required this.machineId});

  final String machineId;

  @override
  ConsumerState<MissionPage> createState() => _MissionPageState();
}

class _MissionPageState extends ConsumerState<MissionPage> {
  MissionFilter _filter = MissionFilter.all;

  String _label(MissionFilter f) {
    final l = context.l10n;
    return switch (f) {
      MissionFilter.all => l.filterAll,
      MissionFilter.needs => l.filterNeeds,
      MissionFilter.running => l.filterRunning,
      MissionFilter.done => l.filterDone,
      MissionFilter.idle => l.filterIdle,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final model = ref.watch(workspaceModelProvider(widget.machineId));
    final entries = <(TabInfo, PaneInfo)>[
      for (final tab in model.tabs)
        for (final pane in tab.panes)
          if (matches(_filter, pane)) (tab, pane),
    ];
    return Scaffold(
      appBar: AppBar(title: Text(l.missionControl)),
      body: Column(
        children: [
          SizedBox(
            height: 52,
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              itemCount: MissionFilter.values.length,
              separatorBuilder: (_, _) => const SizedBox(width: 8),
              itemBuilder: (context, i) {
                final f = MissionFilter.values[i];
                final count = model.panes.where((p) => matches(f, p)).length;
                return FilterChip(
                  key: ValueKey('filter-${f.name}'),
                  selected: _filter == f,
                  onSelected: (_) => setState(() => _filter = f),
                  label: Text('${_label(f)} · $count'),
                );
              },
            ),
          ),
          Expanded(
            child: AnimatedSwitcher(
              duration: motion(context),
              child: entries.isEmpty
                  ? Center(
                      key: const ValueKey('none'),
                      child: Text(
                        l.missionEmpty,
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    )
                  : GridView.builder(
                      key: ValueKey(_filter),
                      padding: const EdgeInsets.fromLTRB(12, 4, 12, 24),
                      gridDelegate:
                          const SliverGridDelegateWithMaxCrossAxisExtent(
                            maxCrossAxisExtent: 240,
                            mainAxisExtent: 132,
                            crossAxisSpacing: 10,
                            mainAxisSpacing: 10,
                          ),
                      itemCount: entries.length,
                      itemBuilder: (context, i) {
                        final (tab, pane) = entries[i];
                        return Appear(
                          key: ValueKey('card-${pane.key}'),
                          delay: (i % 8) * 25,
                          child: MissionCard(
                            machineId: widget.machineId,
                            tab: tab,
                            pane: pane,
                          ),
                        );
                      },
                    ),
            ),
          ),
        ],
      ),
    );
  }
}

class MissionCard extends ConsumerWidget {
  const MissionCard({
    super.key,
    required this.machineId,
    required this.tab,
    required this.pane,
  });

  final String machineId;
  final TabInfo tab;
  final PaneInfo pane;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scheme = Theme.of(context).colorScheme;
    final facts = ref.watch(paneFactsProvider((machineId, pane.terminalId)));
    final needs = pane.needsUser;
    return DecoratedBox(
      decoration: ShapeDecoration(
        color: scheme.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(14),
          side: BorderSide(
            color: needs ? context.statusColors.asked : scheme.outlineVariant,
            width: needs ? 1.5 : 1,
          ),
        ),
      ),
      child: PressScale(
        onTap: () => openPane(context, machineId, tab, pane.key),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  AgentGlyph(agent: pane.agent, kind: pane.kind, size: 18),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      tab.displayName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Text(
                pane.title,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const Spacer(),
              StatusBadge(state: pane.rail, compact: true),
              if (facts.hasTool)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: Text(
                    facts.tool!,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 11,
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
