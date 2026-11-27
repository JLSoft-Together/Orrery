import 'package:flutter/material.dart';

import '../../remote/away_summary.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

extension AwayGroupX on AwayGroup {
  IconData get icon => switch (this) {
    AwayGroup.done => Icons.check_circle_outline_rounded,
    AwayGroup.waiting => Icons.schedule_rounded,
    AwayGroup.failed => Icons.error_outline_rounded,
    AwayGroup.agentsDone => Icons.call_split_rounded,
    AwayGroup.agentsFailed => Icons.call_split_rounded,
    AwayGroup.compact => Icons.compress_rounded,
  };

  Color tone(BuildContext context) {
    final c = context.orrery;
    return switch (this) {
      AwayGroup.done => c.done,
      AwayGroup.waiting => c.waiting,
      AwayGroup.failed || AwayGroup.agentsFailed => c.error,
      AwayGroup.agentsDone => c.working,
      AwayGroup.compact => c.inkMuted,
    };
  }

  String label(BuildContext context, int n) {
    final l = context.l10n;
    return switch (this) {
      AwayGroup.done => l.awayDone(n),
      AwayGroup.waiting => l.awayWaiting(n),
      AwayGroup.failed => l.awayFailed(n),
      AwayGroup.agentsDone => l.awayAgentsDone(n),
      AwayGroup.agentsFailed => l.awayAgentsFailed(n),
      AwayGroup.compact => l.awayCompact(n),
    };
  }
}

class AwayChip extends StatelessWidget {
  const AwayChip({
    super.key,
    required this.group,
    required this.count,
    required this.selected,
    required this.onTap,
  });

  final AwayGroup group;
  final int count;
  final bool selected;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final tone = group.tone(context);
    final text = group.label(context, count);
    return Semantics(
      button: onTap != null,
      selected: selected,
      label: text,
      excludeSemantics: true,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onTap,
        child: AnimatedContainer(
          duration: motion(context, 160),
          curve: context.tokens.out,
          constraints: const BoxConstraints(minHeight: 48),
          padding: const EdgeInsets.symmetric(horizontal: 14),
          decoration: BoxDecoration(
            color: selected ? tone.withValues(alpha: 0.14) : c.surface,
            borderRadius: BorderRadius.circular(999),
            border: Border.all(
              color: selected ? tone.withValues(alpha: 0.6) : c.lineStrong,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(group.icon, size: 18, color: tone),
              const SizedBox(width: 6),
              Flexible(
                child: Text(
                  text,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: c.ink,
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
