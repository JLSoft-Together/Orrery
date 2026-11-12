import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class NavItem {
  const NavItem({
    required this.id,
    required this.icon,
    required this.selectedIcon,
    required this.label,
    this.badge = 0,
  });

  final String id;
  final IconData icon;
  final IconData selectedIcon;
  final String label;
  final int badge;
}

class ShellNav extends StatelessWidget {
  const ShellNav({
    super.key,
    required this.items,
    required this.index,
    required this.onSelect,
  });

  final List<NavItem> items;
  final int index;
  final ValueChanged<int> onSelect;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: c.surface,
        border: Border(top: BorderSide(color: c.line)),
      ),
      child: SafeArea(
        top: false,
        child: Row(
          children: [
            for (var i = 0; i < items.length; i++)
              Expanded(
                child: _NavTile(
                  item: items[i],
                  selected: i == index,
                  onTap: () => onSelect(i),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _NavTile extends StatelessWidget {
  const _NavTile({
    required this.item,
    required this.selected,
    required this.onTap,
  });

  final NavItem item;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final tint = selected ? c.accentInk : c.inkFaint;
    return Semantics(
      button: true,
      selected: selected,
      label: item.label,
      child: InkWell(
        key: ValueKey('nav-${item.id}'),
        onTap: onTap,
        child: SizedBox(
          height: 60,
          child: Stack(
            alignment: Alignment.center,
            children: [
              Positioned(
                top: 0,
                child: AnimatedContainer(
                  duration: motion(context),
                  curve: context.tokens.out,
                  width: selected ? 36 : 0,
                  height: 2,
                  color: c.accent,
                ),
              ),
              Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Badge(
                    isLabelVisible: item.badge > 0,
                    label: Text('${item.badge}'),
                    backgroundColor: c.waiting,
                    textColor: c.onAccent,
                    child: AnimatedSwitcher(
                      duration: motion(context, 150),
                      child: Icon(
                        selected ? item.selectedIcon : item.icon,
                        key: ValueKey(selected),
                        size: 22,
                        color: tint,
                      ),
                    ),
                  ),
                  const SizedBox(height: 3),
                  AnimatedDefaultTextStyle(
                    duration: motion(context),
                    style:
                        (Theme.of(context).textTheme.bodyMedium ??
                                const TextStyle())
                            .copyWith(
                              fontSize: 12,
                              height: 16 / 12,
                              fontWeight: FontWeight.w600,
                              color: tint,
                            ),
                    child: Text(item.label),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
