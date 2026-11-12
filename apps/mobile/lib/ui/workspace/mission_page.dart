import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'pane_actions.dart';
import 'pane_grid.dart';
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

class MissionPage extends StatelessWidget {
  const MissionPage({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: context.orrery.canvas,
      appBar: AppBar(title: Text(context.l10n.missionControl)),
      body: MissionBody(machineId: machineId),
    );
  }
}

class MissionBody extends ConsumerStatefulWidget {
  const MissionBody({super.key, required this.machineId, this.tabKey});

  final String machineId;
  final String? tabKey;

  @override
  ConsumerState<MissionBody> createState() => _MissionBodyState();
}

class _MissionBodyState extends ConsumerState<MissionBody> {
  MissionFilter _filter = MissionFilter.all;

  String _label(MissionFilter f, int n) {
    final l = context.l10n;
    final name = switch (f) {
      MissionFilter.all => l.filterAll,
      MissionFilter.needs => l.filterNeeds,
      MissionFilter.running => l.filterRunning,
      MissionFilter.done => l.filterDone,
      MissionFilter.idle => l.filterIdle,
    };
    return '$name · $n';
  }

  Widget? _leading(MissionFilter f) {
    final c = context.orrery;
    return switch (f) {
      MissionFilter.running => Icon(
        Icons.autorenew_rounded,
        size: 16,
        color: c.working,
      ),
      MissionFilter.needs => Icon(
        Icons.schedule_rounded,
        size: 16,
        color: c.waiting,
      ),
      _ => null,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final model = ref.watch(workspaceModelProvider(widget.machineId));
    final scoped = [
      for (final tab in model.tabs)
        if (widget.tabKey == null || tab.key == widget.tabKey)
          for (final pane in tab.panes) (tab, pane),
    ];
    final entries = [
      for (final e in scoped)
        if (matches(_filter, e.$2)) e,
    ];
    return Column(
      children: [
        SizedBox(
          height: 56,
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            itemCount: MissionFilter.values.length,
            separatorBuilder: (_, _) => const SizedBox(width: 8),
            itemBuilder: (context, i) {
              final f = MissionFilter.values[i];
              final count = scoped.where((e) => matches(f, e.$2)).length;
              return OrreryFilterChip(
                key: ValueKey('filter-${f.name}'),
                label: _label(f, count),
                leading: _leading(f),
                selected: _filter == f,
                onTap: () => setState(() => _filter = f),
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
                      style: TextStyle(color: c.inkMuted),
                    ),
                  )
                : PaneGrid(
                    key: ValueKey(_filter),
                    entries: entries,
                    builder: (context, tab, pane) => PaneGridCard(
                      machineId: widget.machineId,
                      pane: pane,
                      caption: widget.tabKey == null ? tab.displayName : null,
                      onOpen: () =>
                          openPane(context, widget.machineId, tab, pane.key),
                      onLongPress: () => showTabActions(
                        context,
                        ref,
                        widget.machineId,
                        model,
                        tab,
                        pane,
                      ),
                    ),
                  ),
          ),
        ),
      ],
    );
  }
}
