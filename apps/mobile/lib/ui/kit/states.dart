import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import 'orbit_mark.dart';

class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.title,
    this.body,
    this.actions = const [],
    this.art,
  });

  final String title;
  final String? body;
  final List<Widget> actions;
  final Widget? art;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final text = Theme.of(context).textTheme;
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            art ?? const OrbitMark(size: 112),
            const SizedBox(height: 20),
            Text(title, textAlign: TextAlign.center, style: text.titleMedium),
            if (body != null) ...[
              const SizedBox(height: 8),
              Text(
                body!,
                textAlign: TextAlign.center,
                style: text.bodyMedium?.copyWith(color: c.inkMuted),
              ),
            ],
            for (final action in actions) ...[
              const SizedBox(height: 12),
              action,
            ],
          ],
        ),
      ),
    );
  }
}

enum NoticeTone { error, warning, info }

class NoticeCard extends StatelessWidget {
  const NoticeCard({
    super.key,
    required this.tone,
    required this.title,
    this.body,
    this.icon,
    this.trailing,
  });

  final NoticeTone tone;
  final String title;
  final String? body;
  final IconData? icon;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final color = switch (tone) {
      NoticeTone.error => c.error,
      NoticeTone.warning => c.waiting,
      NoticeTone.info => c.info,
    };
    final glyph =
        icon ??
        switch (tone) {
          NoticeTone.error => Icons.error_outline_rounded,
          NoticeTone.warning => Icons.warning_amber_rounded,
          NoticeTone.info => Icons.shield_outlined,
        };
    return TweenAnimationBuilder<Color?>(
      tween: ColorTween(end: color),
      duration: const Duration(milliseconds: 220),
      builder: (context, tint, _) {
        final t = tint ?? color;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: t.withValues(alpha: 0.1),
            borderRadius: context.shape.cardRadius,
            border: Border.all(color: t.withValues(alpha: 0.38)),
          ),
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(glyph, size: 20, color: t),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        style: TextStyle(
                          fontSize: 14,
                          height: 20 / 14,
                          fontWeight: FontWeight.w600,
                          color: c.ink,
                        ),
                      ),
                      if (body != null)
                        Text(
                          body!,
                          style: TextStyle(
                            fontSize: 12,
                            height: 16 / 12,
                            color: c.inkMuted,
                          ),
                        ),
                    ],
                  ),
                ),
                ?trailing,
              ],
            ),
          ),
        );
      },
    );
  }
}
