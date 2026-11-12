import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/providers.dart';
import '../commands.dart';
import '../model/workspace_model.dart';
import '../terminal/terminal_pane.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import '../viewers/viewer_menu.dart';
import 'offline_notice.dart';
import 'pane_actions.dart';
import 'pane_picker.dart';
import 'split_view.dart';

class TabPage extends ConsumerStatefulWidget {
  const TabPage({
    super.key,
    required this.machineId,
    required this.tabKey,
    this.paneKey,
  });

  final String machineId;
  final String tabKey;
  final String? paneKey;

  @override
  ConsumerState<TabPage> createState() => _TabPageState();
}

class _TabPageState extends ConsumerState<TabPage> {
  late final String _tabKey = widget.tabKey;
  String? _paneKey;
  int _index = 0;
  PageController? _pages;
  bool _leaving = false;

  @override
  void initState() {
    super.initState();
    _paneKey = widget.paneKey;
  }

  @override
  void dispose() {
    _pages?.dispose();
    super.dispose();
  }

  int _resolveIndex(TabInfo tab) {
    final key = _paneKey;
    if (key != null) {
      final found = tab.panes.indexWhere((p) => p.key == key);
      if (found >= 0) return found;
      if (tab.panes.isNotEmpty) {
        final fallback = _index.clamp(0, tab.panes.length - 1);
        return fallback;
      }
    }
    return 0;
  }

  void _swipe(int delta) => _goTo(_index + delta);

  void _goTo(int target) {
    final tab = ref.read(workspaceModelProvider(widget.machineId)).tab(_tabKey);
    if (tab == null || target < 0 || target >= tab.panes.length) return;
    setState(() {
      _index = target;
      _paneKey = tab.panes[target].key;
    });
  }

  void _leave(String message) {
    if (_leaving) return;
    _leaving = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      toast(context, message);
      Navigator.of(context).maybePop();
    });
  }

  void _syncPages(int index) {
    final controller = _pages;
    if (controller == null || !controller.hasClients) return;
    if (controller.position.isScrollingNotifier.value) return;
    final current = controller.page?.round();
    if (current == index) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && (_pages?.hasClients ?? false)) {
        if (reduceMotion(context)) {
          _pages!.jumpToPage(index);
        } else {
          _pages!.animateToPage(
            index,
            duration: motion(context, 260),
            curve: Curves.easeOutCubic,
          );
        }
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final id = widget.machineId;
    final l = context.l10n;
    final model = ref.watch(workspaceModelProvider(id));
    final link = ref.watch(linkProvider(id));
    final view = ref.watch(machineViewProvider(id));
    final tab = model.tab(_tabKey);
    if (tab == null) {
      if (view.workspace != null) _leave(l.tabClosed);
      return Scaffold(appBar: AppBar(), body: const SizedBox.shrink());
    }
    if (tab.panes.isEmpty) {
      _leave(l.tabClosed);
      return Scaffold(appBar: AppBar(), body: const SizedBox.shrink());
    }
    final index = _resolveIndex(tab);
    _index = index;
    if (_paneKey != null && tab.pane(_paneKey!) == null) {
      _paneKey = tab.panes[index].key;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) toast(context, l.paneClosed);
      });
    }
    _paneKey ??= tab.panes[index].key;
    _pages ??= PageController(initialPage: index);
    _syncPages(index);
    final wide = MediaQuery.sizeOf(context).width >= 700;
    final current = tab.panes[index];
    return Scaffold(
      resizeToAvoidBottomInset: true,
      appBar: AppBar(
        titleSpacing: 0,
        title: Align(
          alignment: Alignment.centerLeft,
          child: PanePickerButton(
            pane: current,
            tabName: tab.displayName,
            onTap: () => showPaneSheet(
              context,
              tab: tab,
              currentKey: current.key,
              onPick: _goTo,
            ),
          ),
        ),
        actions: [
          if ((current.cwd ?? tab.project) != null)
            IconButton(
              key: const ValueKey('viewer-menu'),
              tooltip: l.vViewerMenu,
              icon: const Icon(Icons.folder_open_rounded),
              onPressed: () =>
                  showViewerMenu(context, id, (current.cwd ?? tab.project)!),
            ),
          IconButton(
            tooltip: l.tabActions,
            icon: const Icon(Icons.tune_rounded),
            onPressed: () =>
                showTabActions(context, ref, id, model, tab, current),
          ),
        ],
      ),
      body: SafeArea(
        top: false,
        child: Column(
          children: [
            OfflineNotice(
              link: link,
              updatedAt: view.updatedAt,
              onRetry: () => ref.read(machineApiProvider(id)).retry(),
            ),
            Expanded(
              child: wide
                  ? SplitView(
                      machineId: id,
                      tab: tab,
                      focusKey: current.key,
                      onFocus: (key) => setState(() => _paneKey = key),
                      onActions: (pane) =>
                          showTabActions(context, ref, id, model, tab, pane),
                    )
                  : _phonePages(tab, model),
            ),
          ],
        ),
      ),
    );
  }

  Widget _phonePages(TabInfo tab, WorkspaceModel model) {
    return PageView.builder(
      controller: _pages,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: tab.panes.length,
      onPageChanged: (i) {
        if (i < tab.panes.length) {
          setState(() {
            _index = i;
            _paneKey = tab.panes[i].key;
          });
        }
      },
      findChildIndexCallback: (key) {
        if (key is! ValueKey<String>) return null;
        final found = tab.panes.indexWhere((p) => 'pane-${p.key}' == key.value);
        return found < 0 ? null : found;
      },
      itemBuilder: (context, i) {
        final pane = tab.panes[i];
        return KeyedSubtree(
          key: ValueKey('pane-${pane.key}'),
          child: TerminalPane(
            machineId: widget.machineId,
            tabKey: tab.key,
            pane: pane,
            onSwipe: _swipe,
            compactHeader: true,
            onActions: () => showTabActions(
              context,
              ref,
              widget.machineId,
              model,
              tab,
              pane,
            ),
          ),
        );
      },
    );
  }
}
