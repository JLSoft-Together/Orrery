import 'package:flutter/material.dart';

import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class TabStrip extends StatelessWidget {
  const TabStrip({
    super.key,
    required this.tabs,
    required this.selected,
    required this.onSelect,
    required this.onAdd,
  });

  final List<TabInfo> tabs;
  final String selected;
  final ValueChanged<TabInfo> onSelect;
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return SizedBox(
      height: 48,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(bottom: BorderSide(color: c.line)),
        ),
        child: ListView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(horizontal: 12),
          children: [
            for (final tab in tabs)
              InkWell(
                key: ValueKey('tab-${tab.key}'),
                onTap: () => onSelect(tab),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  child: Stack(
                    alignment: Alignment.center,
                    children: [
                      Row(
                        children: [
                          AnimatedDefaultTextStyle(
                            duration: motion(context),
                            style:
                                (Theme.of(context).textTheme.bodyMedium ??
                                        const TextStyle())
                                    .copyWith(
                                      fontSize: 15,
                                      fontWeight: FontWeight.w600,
                                      color: tab.key == selected
                                          ? c.ink
                                          : c.inkMuted,
                                    ),
                            child: Text(tab.displayName),
                          ),
                          if (tab.panes.any((p) => p.needsUser))
                            Padding(
                              padding: const EdgeInsets.only(left: 6),
                              child: DecoratedBox(
                                decoration: BoxDecoration(
                                  color: c.waiting,
                                  shape: BoxShape.circle,
                                ),
                                child: const SizedBox.square(dimension: 7),
                              ),
                            ),
                        ],
                      ),
                      Positioned(
                        left: 0,
                        right: 0,
                        bottom: 0,
                        child: AnimatedContainer(
                          duration: motion(context),
                          curve: context.tokens.out,
                          height: 2,
                          color: tab.key == selected
                              ? c.accent
                              : Colors.transparent,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            IconButton(
              key: const ValueKey('workspace-tab-add'),
              tooltip: context.l10n.openPane,
              icon: Icon(Icons.add_rounded, color: c.inkMuted),
              onPressed: onAdd,
            ),
          ],
        ),
      ),
    );
  }
}
