import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

enum ButtonKind { primary, secondary, text, danger }

class OrreryButton extends StatefulWidget {
  const OrreryButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.icon,
    this.kind = ButtonKind.primary,
    this.busy = false,
    this.expand = true,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final ButtonKind kind;
  final bool busy;
  final bool expand;

  @override
  State<OrreryButton> createState() => _OrreryButtonState();
}

class _OrreryButtonState extends State<OrreryButton> {
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final shape = context.shape;
    final enabled = widget.onPressed != null && !widget.busy;
    final (bg, fg, side) = switch (widget.kind) {
      ButtonKind.primary => (c.accent, c.onAccent, null),
      ButtonKind.secondary => (c.control, c.ink, c.lineStrong),
      ButtonKind.text => (Colors.transparent, c.accentInk, null),
      ButtonKind.danger => (
        Colors.transparent,
        c.error,
        c.error.withValues(alpha: 0.38),
      ),
    };
    final child = Row(
      mainAxisSize: widget.expand ? MainAxisSize.max : MainAxisSize.min,
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        AnimatedSwitcher(
          duration: motion(context, 150),
          child: widget.busy
              ? SizedBox.square(
                  key: const ValueKey('busy'),
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2, color: fg),
                )
              : widget.icon == null
              ? const SizedBox.shrink(key: ValueKey('none'))
              : Icon(
                  widget.icon,
                  key: ValueKey(widget.icon),
                  size: 18,
                  color: fg,
                ),
        ),
        if (widget.busy || widget.icon != null) const SizedBox(width: 8),
        Flexible(
          child: Text(
            widget.label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontSize: 15,
              height: 20 / 15,
              fontWeight: FontWeight.w600,
              color: fg,
            ),
          ),
        ),
      ],
    );
    return AnimatedOpacity(
      opacity: widget.onPressed == null ? 0.45 : 1,
      duration: motion(context, 150),
      child: AnimatedScale(
        scale: _down && enabled && !reduceMotion(context) ? 0.97 : 1,
        duration: const Duration(milliseconds: 90),
        child: Material(
          color: bg,
          shape: RoundedRectangleBorder(
            borderRadius: shape.controlRadius,
            side: side == null ? BorderSide.none : BorderSide(color: side),
          ),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            onTap: enabled ? widget.onPressed : null,
            onHighlightChanged: (v) => setState(() => _down = v),
            child: ConstrainedBox(
              constraints: BoxConstraints(
                minHeight: shape.touch + 4,
                minWidth: shape.touch,
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: child,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
