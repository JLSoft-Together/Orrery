import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class ViewModeButton extends StatelessWidget {
  const ViewModeButton({
    super.key,
    required this.alternate,
    required this.primaryIcon,
    required this.alternateIcon,
    required this.tooltip,
    required this.onPressed,
  });

  final bool alternate;
  final IconData primaryIcon;
  final IconData alternateIcon;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return IconButton(
      tooltip: tooltip,
      onPressed: onPressed,
      icon: AnimatedSwitcher(
        duration: motion(context, 150),
        transitionBuilder: (child, animation) => FadeTransition(
          opacity: animation,
          child: ScaleTransition(
            scale: Tween<double>(begin: 0.7, end: 1).animate(animation),
            child: child,
          ),
        ),
        child: Icon(
          alternate ? alternateIcon : primaryIcon,
          key: ValueKey(alternate),
          color: c.ink,
        ),
      ),
    );
  }
}
