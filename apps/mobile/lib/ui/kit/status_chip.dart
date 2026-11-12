import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'pulse_dot.dart';

enum ChipTone {
  working,
  waiting,
  done,
  error,
  idle,
  info,
  accent,
  agentClaude,
  agentCodex,
  agentShell,
}

extension ChipToneColor on ChipTone {
  Color resolve(BuildContext context) {
    final c = context.orrery;
    return switch (this) {
      ChipTone.working => c.working,
      ChipTone.waiting => c.waiting,
      ChipTone.done => c.done,
      ChipTone.error => c.error,
      ChipTone.idle => c.idle,
      ChipTone.info => c.info,
      ChipTone.accent => c.accentInk,
      ChipTone.agentClaude => c.agentClaude,
      ChipTone.agentCodex => c.agentCodex,
      ChipTone.agentShell => c.agentShell,
    };
  }
}

class OrreryChip extends StatelessWidget {
  const OrreryChip({
    super.key,
    required this.tone,
    required this.label,
    this.icon,
    this.pulse = false,
    this.dense = false,
  });

  final ChipTone tone;
  final String label;
  final IconData? icon;
  final bool pulse;
  final bool dense;

  @override
  Widget build(BuildContext context) {
    final target = tone.resolve(context);
    final fast = motion(context, 150);
    final base = motion(context);
    return TweenAnimationBuilder<Color?>(
      tween: ColorTween(end: target),
      duration: base,
      builder: (context, color, _) {
        final tint = color ?? target;
        return DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(999),
            border: Border.all(color: tint.withValues(alpha: 0.38)),
          ),
          child: Padding(
            padding: EdgeInsets.symmetric(
              horizontal: dense ? 7 : 9,
              vertical: dense ? 2 : 4,
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                AnimatedSwitcher(
                  duration: base,
                  reverseDuration: fast,
                  transitionBuilder: (child, animation) => FadeTransition(
                    opacity: animation,
                    child: ScaleTransition(
                      scale: Tween<double>(
                        begin: 0.7,
                        end: 1,
                      ).animate(animation),
                      child: child,
                    ),
                  ),
                  child: pulse
                      ? PulseDot(key: const ValueKey('pulse'), color: tint)
                      : Icon(
                          icon ?? Icons.circle,
                          key: ValueKey(icon),
                          size: 14,
                          color: tint,
                        ),
                ),
                const SizedBox(width: 5),
                Flexible(
                  child: AnimatedSwitcher(
                    duration: base,
                    reverseDuration: fast,
                    child: Text(
                      label,
                      key: ValueKey(label),
                      maxLines: 1,
                      softWrap: false,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        height: 16 / 12,
                        fontWeight: FontWeight.w600,
                        color: tint,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
