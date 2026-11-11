import 'package:flutter/material.dart';

import '../../remote/pty_stream.dart';
import '../model/workspace_model.dart';
import '../terminal/terminal_pane.dart';
import '../widgets/common.dart';

class SplitView extends StatelessWidget {
  const SplitView({
    super.key,
    required this.machineId,
    required this.tab,
    required this.focusKey,
    required this.onFocus,
    required this.onActions,
  });

  final String machineId;
  final TabInfo tab;
  final String focusKey;
  final ValueChanged<String> onFocus;
  final ValueChanged<PaneInfo> onActions;

  Set<String> _active() {
    final ordered = [
      ...tab.panes.where((p) => p.key == focusKey),
      ...tab.panes.where((p) => p.key != focusKey),
    ];
    return {
      for (final p in ordered.where((p) => p.canAttach).take(maxAttach)) p.key,
      for (final p in ordered.where((p) => !p.canAttach)) p.key,
    };
  }

  @override
  Widget build(BuildContext context) {
    final active = _active();
    final tree = tab.tree;
    final node =
        tree ??
        (tab.panes.isEmpty
            ? null
            : tab.panes
                  .skip(1)
                  .fold<LayoutNode>(
                    PaneLeaf(tab.panes.first.key),
                    (acc, p) => SplitNode(Axis2.row, 0.5, acc, PaneLeaf(p.key)),
                  ));
    if (node == null) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.all(6),
      child: _build(context, node, active),
    );
  }

  Widget _build(BuildContext context, LayoutNode node, Set<String> active) {
    switch (node) {
      case PaneLeaf(:final key):
        final pane = tab.pane(key);
        if (pane == null) return const SizedBox.shrink();
        return _Leaf(
          machineId: machineId,
          tabKey: tab.key,
          pane: pane,
          active: active.contains(key),
          focused: key == focusKey,
          onFocus: () => onFocus(key),
          onActions: () => onActions(pane),
        );
      case SplitNode(:final axis, :final ratio, :final a, :final b):
        final first = Expanded(
          flex: (ratio * 1000).round(),
          child: _build(context, a, active),
        );
        final second = Expanded(
          flex: ((1 - ratio) * 1000).round(),
          child: _build(context, b, active),
        );
        const gap = SizedBox(width: 6, height: 6);
        return axis == Axis2.row
            ? Row(children: [first, gap, second])
            : Column(children: [first, gap, second]);
    }
  }
}

class _Leaf extends StatelessWidget {
  const _Leaf({
    required this.machineId,
    required this.tabKey,
    required this.pane,
    required this.active,
    required this.focused,
    required this.onFocus,
    required this.onActions,
  });

  final String machineId;
  final String tabKey;
  final PaneInfo pane;
  final bool active;
  final bool focused;
  final VoidCallback onFocus;
  final VoidCallback onActions;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return AnimatedContainer(
      duration: motion(context),
      decoration: BoxDecoration(
        color: scheme.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: focused ? scheme.primary : scheme.outlineVariant,
          width: focused ? 2 : 1,
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: (_) {
          if (!focused) onFocus();
        },
        child: LayoutBuilder(
          builder: (context, box) {
            final small =
                !focused && (box.maxWidth < 260 || box.maxHeight < 220);
            if (active && !small) {
              return TerminalPane(
                key: ValueKey('split-${pane.key}'),
                machineId: machineId,
                tabKey: tabKey,
                pane: pane,
                showInput: focused,
                onActions: onActions,
              );
            }
            return InkWell(
              onTap: onFocus,
              child: ClipRect(
                child: Center(
                  child: FittedBox(
                    fit: BoxFit.scaleDown,
                    child: Padding(
                      padding: const EdgeInsets.all(8),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          StatusBadge(state: pane.rail),
                          const SizedBox(height: 8),
                          Text(
                            pane.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          const SizedBox(height: 4),
                          Text(
                            context.l10n.tapToView,
                            style: TextStyle(color: scheme.onSurfaceVariant),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}
