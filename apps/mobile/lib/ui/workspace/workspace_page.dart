import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../commands.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_theme.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import '../widgets/link_widgets.dart';
import '../monitor/monitor_page.dart';
import 'inbox_page.dart';
import 'machine_scope.dart';
import 'mission_page.dart';
import 'open_pane_page.dart';
import 'tab_page.dart';

class WorkspacePage extends StatelessWidget {
  const WorkspacePage({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context) =>
      MachineScope(machineId: machineId, child: _Shell(machineId));
}

class _Shell extends ConsumerStatefulWidget {
  const _Shell(this.machineId);

  final String machineId;

  @override
  ConsumerState<_Shell> createState() => _ShellState();
}

class _ShellState extends ConsumerState<_Shell> {
  int _index = 0;

  void _openTerminal(String terminalId) {
    final id = widget.machineId;
    final model = ref.read(workspaceModelProvider(id));
    final pane = model.panes
        .where((p) => p.terminalId == terminalId)
        .firstOrNull;
    final tab = pane == null ? null : model.tabOfPane(pane.key);
    if (pane == null || tab == null) {
      toast(context, context.l10n.vNotInWorkspace);
      return;
    }
    openPane(context, id, tab, pane.key);
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final id = widget.machineId;
    final inbox = ref.watch(workspaceModelProvider(id)).inbox.length;
    return Scaffold(
      body: IndexedStack(
        index: _index,
        children: [
          _WorkspaceBody(id),
          MonitorPage(machineId: id, onOpen: _openTerminal),
          InboxPage(machineId: id),
        ],
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _index,
        onDestinationSelected: (i) => setState(() => _index = i),
        destinations: [
          NavigationDestination(
            key: const ValueKey('nav-workspace'),
            icon: const Icon(Icons.space_dashboard_outlined),
            selectedIcon: const Icon(Icons.space_dashboard_rounded),
            label: l.navWorkspace,
          ),
          NavigationDestination(
            key: const ValueKey('nav-monitor'),
            icon: const Icon(Icons.public_outlined),
            selectedIcon: const Icon(Icons.public_rounded),
            label: l.vMonitor,
          ),
          NavigationDestination(
            key: const ValueKey('nav-inbox'),
            icon: Badge(
              isLabelVisible: inbox > 0,
              label: Text('$inbox'),
              child: const Icon(Icons.inbox_outlined),
            ),
            selectedIcon: const Icon(Icons.inbox_rounded),
            label: l.inbox,
          ),
        ],
      ),
    );
  }
}

void openPane(
  BuildContext context,
  String machineId,
  TabInfo tab,
  String? paneKey,
) {
  Navigator.of(context).push(
    machineRoute<void>(
      machineId,
      (_) => TabPage(machineId: machineId, tabKey: tab.key, paneKey: paneKey),
    ),
  );
}

class _WorkspaceBody extends ConsumerWidget {
  const _WorkspaceBody(this.machineId);

  final String machineId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final link = ref.watch(linkProvider(machineId));
    final view = ref.watch(machineViewProvider(machineId));
    final model = ref.watch(workspaceModelProvider(machineId));
    final projects = model.projects;
    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              ref.watch(
                machinesProvider.select(
                  (m) =>
                      m
                          .where((x) => x.id == machineId)
                          .firstOrNull
                          ?.displayName ??
                      '',
                ),
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
            ),
            LinkChip(link: link),
          ],
        ),
        toolbarHeight: 64,
        actions: [
          IconButton(
            tooltip: l.missionControl,
            icon: const Icon(Icons.grid_view_rounded),
            onPressed: () => Navigator.of(context).push(
              machineRoute<void>(
                machineId,
                (_) => MissionPage(machineId: machineId),
              ),
            ),
          ),
          PopupMenuButton<String>(
            tooltip: l.more,
            onSelected: (value) {
              if (value == 'reopen') {
                runCommand(context, ref, machineId, 'ws.reopen');
              }
            },
            itemBuilder: (_) => [
              PopupMenuItem(
                value: 'reopen',
                enabled:
                    model.closedCount > 0 && link.status == LinkStatus.ready,
                child: Text(l.reopenPane(model.closedCount)),
              ),
            ],
          ),
        ],
      ),
      body: Column(
        children: [
          OfflineBanner(
            link: link,
            updatedAt: view.updatedAt,
            onRetry: () => ref.read(machineApiProvider(machineId)).retry(),
          ),
          Expanded(
            child: RefreshIndicator(
              onRefresh: () async {
                try {
                  await ref
                      .read(machineApiProvider(machineId))
                      .command('ws.get');
                } on Object {
                  return;
                }
              },
              child: projects.isEmpty
                  ? _Empty(
                      machineId: machineId,
                      loading: view.workspace == null,
                    )
                  : ListView.separated(
                      physics: const AlwaysScrollableScrollPhysics(),
                      padding: const EdgeInsets.fromLTRB(16, 12, 16, 96),
                      itemCount: projects.length,
                      separatorBuilder: (_, _) => const SizedBox(height: 10),
                      itemBuilder: (context, index) => Appear(
                        key: ValueKey('project-${projects[index].id}'),
                        delay: index * 30,
                        child: _ProjectCard(
                          machineId: machineId,
                          project: projects[index],
                        ),
                      ),
                    ),
            ),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => Navigator.of(context).push(
          machineRoute<void>(
            machineId,
            (_) => OpenPanePage(machineId: machineId),
          ),
        ),
        icon: const Icon(Icons.add_rounded),
        label: Text(l.openPane),
      ),
    );
  }
}

class _Empty extends StatelessWidget {
  const _Empty({required this.machineId, required this.loading});

  final String machineId;
  final bool loading;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        const SizedBox(height: 120),
        Icon(
          loading
              ? Icons.hourglass_top_rounded
              : Icons.space_dashboard_outlined,
          size: 56,
          color: Theme.of(context).colorScheme.primary,
        ),
        const SizedBox(height: 16),
        Center(
          child: Text(
            loading ? l.workspaceLoading : l.workspaceEmpty,
            style: Theme.of(context).textTheme.titleMedium,
          ),
        ),
        const SizedBox(height: 6),
        Center(child: Text(loading ? '' : l.workspaceEmptyHint)),
      ],
    );
  }
}

class _ProjectCard extends ConsumerWidget {
  const _ProjectCard({required this.machineId, required this.project});

  final String machineId;
  final ProjectInfo project;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    final needs = project.needsCount;
    final working = project.workingCount;
    final done = project.doneCount;
    final target = project.tabs.firstWhere(
      (t) => t.panes.any((p) => p.needsUser),
      orElse: () => project.tabs.first,
    );
    final focus = target.panes.where((p) => p.needsUser).firstOrNull?.key;
    final shape = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(14),
      side: BorderSide(
        color: needs > 0 ? colors.asked : scheme.outlineVariant,
        width: needs > 0 ? 1.5 : 1,
      ),
    );
    return AnimatedContainer(
      duration: motion(context),
      decoration: ShapeDecoration(color: scheme.surface, shape: shape),
      child: PressScale(
        onTap: () => openPane(context, machineId, target, focus),
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(Icons.folder_outlined, color: scheme.primary),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      project.name ?? l.otherProject,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 17,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  Text(
                    l.panesCount(project.paneCount),
                    style: TextStyle(color: scheme.onSurfaceVariant),
                  ),
                  Icon(
                    Icons.chevron_right_rounded,
                    color: scheme.onSurfaceVariant,
                  ),
                ],
              ),
              AnimatedSize(
                duration: motion(context),
                alignment: Alignment.topLeft,
                child: Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children:
                      [
                            if (needs > 0)
                              _CountPill(
                                state: RailState.asked,
                                count: needs,
                                label: l.needsYouCount(needs),
                              ),
                            if (working > 0)
                              _CountPill(
                                state: RailState.working,
                                count: working,
                                label: l.runningCount(working),
                              ),
                            if (done > 0)
                              _CountPill(
                                state: RailState.done,
                                count: done,
                                label: l.doneCount(done),
                              ),
                          ]
                          .map(
                            (w) => Padding(
                              padding: const EdgeInsets.only(top: 10),
                              child: w,
                            ),
                          )
                          .toList(),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CountPill extends StatelessWidget {
  const _CountPill({
    required this.state,
    required this.count,
    required this.label,
  });

  final RailState state;
  final int count;
  final String label;

  @override
  Widget build(BuildContext context) {
    final style = statusStyle(context, state);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: style.color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(style.icon, size: 14, color: style.color),
          const SizedBox(width: 4),
          Text(
            label,
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: style.color,
            ),
          ),
        ],
      ),
    );
  }
}
