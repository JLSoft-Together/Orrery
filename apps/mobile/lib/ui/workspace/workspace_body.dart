import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../commands.dart';
import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../monitor/view_prefs.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'machine_scope.dart';
import 'mission_page.dart';
import 'offline_notice.dart';
import 'open_pane_page.dart';
import 'workspace_single.dart';
import 'workspace_tabs.dart';

class WorkspaceBody extends ConsumerStatefulWidget {
  const WorkspaceBody(this.machineId, {super.key});

  final String machineId;

  @override
  ConsumerState<WorkspaceBody> createState() => _WorkspaceBodyState();
}

class _WorkspaceBodyState extends ConsumerState<WorkspaceBody> {
  String? _tabKey;
  String? _paneKey;
  PageController? _pages;

  @override
  void dispose() {
    _pages?.dispose();
    super.dispose();
  }

  TabInfo? _resolveTab(WorkspaceModel model) {
    if (model.tabs.isEmpty) return null;
    final kept = _tabKey == null ? null : model.tab(_tabKey!);
    if (kept != null) return kept;
    return model.tabs.firstWhere(
      (t) => t.panes.any((p) => p.needsUser),
      orElse: () => model.tabs.first,
    );
  }

  void _selectTab(TabInfo tab) {
    if (tab.key == _tabKey) return;
    setState(() {
      _tabKey = tab.key;
      _paneKey = null;
      _pages?.dispose();
      _pages = null;
    });
  }

  void _openPanePage() => Navigator.of(context).push(
    machineRoute<void>(
      widget.machineId,
      (_) => OpenPanePage(machineId: widget.machineId),
    ),
  );

  void _goTo(int index, TabInfo tab) {
    if (index < 0 || index >= tab.panes.length) return;
    setState(() => _paneKey = tab.panes[index].key);
    final controller = _pages;
    if (controller == null || !controller.hasClients) return;
    if (reduceMotion(context)) {
      controller.jumpToPage(index);
    } else {
      controller.animateToPage(
        index,
        duration: const Duration(milliseconds: 300),
        curve: context.tokens.out,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final id = widget.machineId;
    final link = ref.watch(linkProvider(id));
    final view = ref.watch(machineViewProvider(id));
    final model = ref.watch(workspaceModelProvider(id));
    final alternate =
        ref.watch(viewPrefsProvider.select((p) => p.workspace)) ==
        ViewMode.alternate;
    final tab = _resolveTab(model);
    if (tab != null) {
      final current = _paneKey == null ? null : tab.pane(_paneKey!);
      _paneKey =
          (current ??
                  tab.panes.where((p) => p.needsUser).firstOrNull ??
                  tab.panes.firstOrNull)
              ?.key;
    }
    return Scaffold(
      backgroundColor: c.canvas,
      body: SafeArea(
        bottom: false,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 12, 8, 0),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      l.wsTitle,
                      style: TextStyle(
                        fontSize: 28,
                        height: 34 / 28,
                        fontWeight: FontWeight.w700,
                        color: c.ink,
                      ),
                    ),
                  ),
                  ViewModeButton(
                    key: const ValueKey('workspace-view-mode'),
                    alternate: alternate,
                    primaryIcon: Icons.grid_view_rounded,
                    alternateIcon: Icons.view_carousel_outlined,
                    tooltip: alternate ? l.viewModeToSingle : l.viewModeToGrid,
                    onPressed: () =>
                        ref.read(viewPrefsProvider.notifier).toggleWorkspace(),
                  ),
                  IconButton(
                    key: const ValueKey('workspace-add'),
                    tooltip: l.openPane,
                    icon: Icon(Icons.add_rounded, color: c.ink),
                    onPressed: _openPanePage,
                  ),
                  PopupMenuButton<String>(
                    tooltip: l.more,
                    onSelected: (value) {
                      if (value == 'reopen') {
                        runCommand(context, ref, id, 'ws.reopen');
                      }
                    },
                    itemBuilder: (_) => [
                      PopupMenuItem(
                        value: 'reopen',
                        enabled:
                            model.closedCount > 0 &&
                            link.status == LinkStatus.ready,
                        child: Text(l.reopenPane(model.closedCount)),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            OfflineNotice(
              link: link,
              updatedAt: view.updatedAt,
              onRetry: () => ref.read(machineApiProvider(id)).retry(),
            ),
            if (tab == null)
              Expanded(
                child: EmptyState(
                  title: view.workspace == null
                      ? l.workspaceLoading
                      : l.workspaceEmpty,
                  body: view.workspace == null ? null : l.workspaceEmptyHint,
                ),
              )
            else ...[
              TabStrip(
                tabs: model.tabs,
                selected: tab.key,
                onSelect: _selectTab,
                onAdd: _openPanePage,
              ),
              Expanded(
                child: AnimatedSwitcher(
                  duration: motion(context),
                  switchInCurve: context.tokens.out,
                  transitionBuilder: (child, animation) => FadeTransition(
                    opacity: animation,
                    child: ScaleTransition(
                      scale: Tween<double>(
                        begin: 0.98,
                        end: 1,
                      ).animate(animation),
                      child: child,
                    ),
                  ),
                  child: alternate
                      ? KeyedSubtree(
                          key: ValueKey('grid-${tab.key}'),
                          child: _GridMode(machineId: id, tab: tab),
                        )
                      : KeyedSubtree(
                          key: ValueKey('single-${tab.key}'),
                          child: SingleMode(
                            machineId: id,
                            model: model,
                            tab: tab,
                            paneKey: _paneKey,
                            pagesOf: (initial) =>
                                _pages ??= PageController(initialPage: initial),
                            onPage: (i) {
                              if (_paneKey != tab.panes[i].key) {
                                setState(() => _paneKey = tab.panes[i].key);
                              }
                            },
                            onChip: (i) => _goTo(i, tab),
                          ),
                        ),
                ),
              ),
            ],
          ],
        ),
      ),
      floatingActionButton: alternate && tab != null
          ? FloatingActionButton.extended(
              key: const ValueKey('workspace-fab'),
              onPressed: _openPanePage,
              icon: const Icon(Icons.add_rounded),
              label: Text(l.openPane),
            )
          : null,
    );
  }
}

class _GridMode extends StatelessWidget {
  const _GridMode({required this.machineId, required this.tab});

  final String machineId;
  final TabInfo tab;

  @override
  Widget build(BuildContext context) =>
      MissionBody(machineId: machineId, tabKey: tab.key);
}
