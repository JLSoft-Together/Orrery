import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class PulseDot extends StatefulWidget {
  const PulseDot({super.key, required this.color, this.size = 8});

  final Color color;
  final double size;

  @override
  State<PulseDot> createState() => _PulseDotState();
}

class _PulseDotState extends State<PulseDot>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(vsync: this);
  bool _running = false;

  void _sync(BuildContext context) {
    final quiet = reduceMotion(context);
    if (quiet && _running) {
      _controller.stop();
      _controller.value = 0;
      _running = false;
    } else if (!quiet && !_running) {
      _controller.duration = context.tokens.pulse;
      _controller.repeat(reverse: true);
      _running = true;
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _sync(context);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final dot = DecoratedBox(
      decoration: BoxDecoration(color: widget.color, shape: BoxShape.circle),
      child: SizedBox.square(dimension: widget.size),
    );
    return SizedBox.square(
      dimension: 14,
      child: Center(
        child: AnimatedBuilder(
          animation: _controller,
          child: dot,
          builder: (context, child) {
            final t = Curves.easeInOut.transform(_controller.value);
            return Opacity(
              opacity: 1 - 0.65 * t,
              child: Transform.scale(scale: 1 - 0.3 * t, child: child),
            );
          },
        ),
      ),
    );
  }
}
