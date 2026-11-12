import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class OrreryCard extends StatefulWidget {
  const OrreryCard({
    super.key,
    required this.child,
    this.onTap,
    this.onLongPress,
    this.barColor,
    this.padding = const EdgeInsets.all(14),
    this.muted = false,
  });

  final Widget child;
  final VoidCallback? onTap;
  final VoidCallback? onLongPress;
  final Color? barColor;
  final EdgeInsetsGeometry padding;
  final bool muted;

  @override
  State<OrreryCard> createState() => _OrreryCardState();
}

class _OrreryCardState extends State<OrreryCard> {
  bool _down = false;

  void _set(bool value) {
    if (_down != value) setState(() => _down = value);
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final radius = context.shape.cardRadius;
    final interactive = widget.onTap != null || widget.onLongPress != null;
    final body = Padding(padding: widget.padding, child: widget.child);
    final bar = widget.barColor;
    final content = bar == null
        ? body
        : Stack(
            children: [
              body,
              Positioned(
                left: 0,
                top: 0,
                bottom: 0,
                width: 3,
                child: ColoredBox(color: bar),
              ),
            ],
          );
    final card = DecoratedBox(
      decoration: BoxDecoration(
        color: widget.muted ? c.raised : c.surface,
        borderRadius: radius,
        border: Border.all(color: c.line),
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: Material(
          type: MaterialType.transparency,
          child: interactive
              ? InkWell(
                  onTap: widget.onTap,
                  onLongPress: widget.onLongPress,
                  onHighlightChanged: _set,
                  child: content,
                )
              : content,
        ),
      ),
    );
    if (!interactive) return card;
    return AnimatedScale(
      scale: _down && !reduceMotion(context) ? 0.98 : 1,
      duration: const Duration(milliseconds: 90),
      child: card,
    );
  }
}

class SectionLabel extends StatelessWidget {
  const SectionLabel(this.text, {super.key, this.count});

  final String text;
  final int? count;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final label = count == null ? text : '$text · $count';
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 16, 4, 8),
      child: Text(
        label.toUpperCase(),
        style: TextStyle(
          fontSize: 12,
          height: 16 / 12,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.4,
          color: c.inkFaint,
        ),
      ),
    );
  }
}
