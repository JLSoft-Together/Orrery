import 'package:flutter/material.dart';

import '../widgets/common.dart';

class QuietSize extends StatelessWidget {
  const QuietSize({
    super.key,
    required this.child,
    this.alignment = Alignment.topCenter,
    this.curve = Curves.easeOutCubic,
    this.milliseconds = 220,
  });

  final Widget child;
  final AlignmentGeometry alignment;
  final Curve curve;
  final int milliseconds;

  @override
  Widget build(BuildContext context) {
    if (reduceMotion(context)) return child;
    return AnimatedSize(
      duration: Duration(milliseconds: milliseconds),
      curve: curve,
      alignment: alignment,
      child: child,
    );
  }
}
