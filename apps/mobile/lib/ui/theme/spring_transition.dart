import 'package:flutter/material.dart';

class OrrerySpringTransitionsBuilder extends PageTransitionsBuilder {
  const OrrerySpringTransitionsBuilder();

  static const Curve _in = Cubic(0.215, 0.61, 0.355, 1);
  static const Curve _out = Cubic(0.55, 0.055, 0.675, 0.19);

  @override
  Duration get transitionDuration => const Duration(milliseconds: 300);

  @override
  Duration get reverseTransitionDuration => const Duration(milliseconds: 210);

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    if (MediaQuery.maybeOf(context)?.disableAnimations ?? false) {
      return FadeTransition(opacity: animation, child: child);
    }
    final enter = CurvedAnimation(
      parent: animation,
      curve: _in,
      reverseCurve: _out,
    );
    final back = CurvedAnimation(
      parent: secondaryAnimation,
      curve: _in,
      reverseCurve: _out,
    );
    return SlideTransition(
      position: Tween<Offset>(
        begin: Offset.zero,
        end: const Offset(-0.24, 0),
      ).animate(back),
      child: FadeTransition(
        opacity: Tween<double>(begin: 1, end: 0.6).animate(back),
        child: SlideTransition(
          position: Tween<Offset>(
            begin: const Offset(1, 0),
            end: Offset.zero,
          ).animate(enter),
          child: child,
        ),
      ),
    );
  }
}
