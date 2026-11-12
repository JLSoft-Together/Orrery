import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class OrreryFilterChip extends StatelessWidget {
  const OrreryFilterChip({
    super.key,
    required this.label,
    required this.selected,
    required this.onTap,
    this.leading,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;
  final Widget? leading;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final base = Theme.of(context).textTheme.bodyMedium ?? const TextStyle();
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: AnimatedContainer(
        duration: motion(context),
        curve: context.tokens.out,
        constraints: const BoxConstraints(minHeight: 40),
        padding: const EdgeInsets.symmetric(horizontal: 14),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: selected ? c.accent.withValues(alpha: 0.12) : c.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(
            color: selected ? c.accent.withValues(alpha: 0.5) : c.lineStrong,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (leading != null) ...[leading!, const SizedBox(width: 6)],
            AnimatedDefaultTextStyle(
              duration: motion(context),
              style: base.copyWith(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: selected ? c.accentInk : c.ink,
              ),
              child: Text(label),
            ),
          ],
        ),
      ),
    );
  }
}
