import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class FadeIn extends StatelessWidget {
  const FadeIn({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (reduceMotion(context)) return child;
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: 1),
      duration: context.tokens.lockIn,
      curve: Curves.easeOut,
      builder: (context, value, child) => Opacity(opacity: value, child: child),
      child: child,
    );
  }
}

class ShakeBox extends StatefulWidget {
  const ShakeBox({super.key, required this.trigger, required this.child});

  final int trigger;
  final Widget child;

  @override
  State<ShakeBox> createState() => _ShakeBoxState();
}

class _ShakeBoxState extends State<ShakeBox>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 240),
  );

  static final TweenSequence<double> _offsets = TweenSequence<double>([
    TweenSequenceItem(tween: Tween(begin: 0, end: -6), weight: 1),
    TweenSequenceItem(tween: Tween(begin: -6, end: 6), weight: 2),
    TweenSequenceItem(tween: Tween(begin: 6, end: -4), weight: 2),
    TweenSequenceItem(tween: Tween(begin: -4, end: 0), weight: 1),
  ]);

  @override
  void didUpdateWidget(ShakeBox old) {
    super.didUpdateWidget(old);
    if (widget.trigger == old.trigger) return;
    HapticFeedback.lightImpact();
    if (reduceMotion(context)) return;
    _controller.forward(from: 0);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _controller,
      child: widget.child,
      builder: (context, child) => Transform.translate(
        offset: Offset(_offsets.transform(_controller.value), 0),
        child: child,
      ),
    );
  }
}
