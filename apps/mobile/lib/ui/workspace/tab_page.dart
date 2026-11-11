import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/providers.dart';
import '../commands.dart';
import '../model/workspace_model.dart';
import '../terminal/terminal_pane.dart';
import '../theme/orrery_theme.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import '../widgets/link_widgets.dart';
import 'pane_actions.dart';
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
  late String _tabKey = widget.tabKey;
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
        _pages!.animateToPage(
          index,
          duration: motion(context, 260),
          curve: Curves.easeOutCubic,
        );
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
    final siblings = model.tabs.where((t) => t.project == tab.project).toList();
    final wide = MediaQuery.sizeOf(context).width >= 700;
    final current = tab.panes[index];
    return Scaffold(
      resizeToAvoidBottomInset: true,
      appBar: AppBar(
        titleSpacing: Navigator.canPop(context) ? 0 : 16,
        title: Row(
          children: [
            Flexible(
              child: Text(
                tab.displayName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const SizedBox(width: 8),
            LinkChip(link: link),
          ],
        ),
        actions: [
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
            OfflineBanner(
              link: link,
              updatedAt: view.updatedAt,
              onRetry: () => ref.read(machineApiProvider(id)).retry(),
            ),
            if (siblings.length > 1)
              _TabChips(
                tabs: siblings,
                selected: _tabKey,
                onSelect: (key) => setState(() {
                  _tabKey = key;
                  _paneKey = null;
                  _index = 0;
                  _pages?.dispose();
                  _pages = null;
                }),
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
            if (!wide && tab.panes.length > 1)
              _Dots(tab: tab, index: index, onSelect: _goTo, onSwipe: _swipe),
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

class _TabChips extends StatelessWidget {
  const _TabChips({
    required this.tabs,
    required this.selected,
    required this.onSelect,
  });

  final List<TabInfo> tabs;
  final String selected;
  final ValueChanged<String> onSelect;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 48,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        itemCount: tabs.length,
        separatorBuilder: (_, _) => const SizedBox(width: 8),
        itemBuilder: (context, i) {
          final tab = tabs[i];
          final needs = tab.panes.any((p) => p.needsUser);
          return ChoiceChip(
            key: ValueKey('tab-${tab.key}'),
            selected: tab.key == selected,
            onSelected: (_) => onSelect(tab.key),
            avatar: needs
                ? Icon(
                    Icons.front_hand_outlined,
                    size: 16,
                    color: context.statusColors.asked,
                  )
                : null,
            label: Text(
              '${i + 1}. ${tab.displayName}',
              overflow: TextOverflow.ellipsis,
            ),
          );
        },
      ),
    );
  }
}

class _Dots extends StatelessWidget {
  const _Dots({
    required this.tab,
    required this.index,
    required this.onSelect,
    required this.onSwipe,
  });

  final TabInfo tab;
  final int index;
  final ValueChanged<int> onSelect;
  final ValueChanged<int> onSwipe;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onHorizontalDragEnd: (d) {
        final v = d.primaryVelocity ?? 0;
        if (v.abs() > 200) onSwipe(v < 0 ? 1 : -1);
      },
      child: SizedBox(
        height: 40,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            for (var i = 0; i < tab.panes.length; i++)
              Semantics(
                button: true,
                selected: i == index,
                label: tab.panes[i].title,
                child: InkResponse(
                  key: ValueKey('dot-$i'),
                  onTap: () => onSelect(i),
                  radius: 20,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 6,
                      vertical: 14,
                    ),
                    child: AnimatedContainer(
                      duration: motion(context),
                      curve: Curves.easeOutCubic,
                      width: i == index ? 20 : 8,
                      height: 8,
                      decoration: BoxDecoration(
                        color: tab.panes[i].needsUser
                            ? context.statusColors.asked
                            : (i == index
                                  ? scheme.primary
                                  : scheme.outlineVariant),
                        borderRadius: BorderRadius.circular(4),
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
