import 'package:flutter/material.dart';

import 'orrery_colors.dart';

@immutable
class OrreryShape extends ThemeExtension<OrreryShape> {
  const OrreryShape({
    this.card = 10,
    this.control = 8,
    this.inner = 6,
    this.touch = 44,
    this.gap = 8,
    this.pad = 16,
  });

  final double card;
  final double control;
  final double inner;
  final double touch;
  final double gap;
  final double pad;

  BorderRadius get cardRadius => BorderRadius.circular(card);
  BorderRadius get controlRadius => BorderRadius.circular(control);
  BorderRadius get innerRadius => BorderRadius.circular(inner);

  @override
  OrreryShape copyWith({double? card, double? control, double? inner}) =>
      OrreryShape(
        card: card ?? this.card,
        control: control ?? this.control,
        inner: inner ?? this.inner,
        touch: touch,
        gap: gap,
        pad: pad,
      );

  @override
  OrreryShape lerp(OrreryShape? other, double t) => other ?? this;
}

@immutable
class OrreryMotion extends ThemeExtension<OrreryMotion> {
  const OrreryMotion();

  Duration get fast => const Duration(milliseconds: 150);
  Duration get base => const Duration(milliseconds: 220);
  Duration get slow => const Duration(milliseconds: 300);
  Duration get pop => const Duration(milliseconds: 210);
  Duration get pulse => const Duration(milliseconds: 1200);
  Duration get lockIn => const Duration(milliseconds: 250);
  Duration get shake => const Duration(milliseconds: 240);

  Curve get out => const Cubic(0.215, 0.61, 0.355, 1);
  Curve get inCurve => const Cubic(0.55, 0.055, 0.675, 0.19);
  Curve get spring => const Cubic(0.3, 1.25, 0.5, 1);

  @override
  OrreryMotion copyWith() => this;

  @override
  OrreryMotion lerp(OrreryMotion? other, double t) => this;
}

extension OrreryThemeX on BuildContext {
  OrreryColors get orrery =>
      Theme.of(this).extension<OrreryColors>() ??
      (Theme.of(this).brightness == Brightness.dark
          ? OrreryColors.dark
          : OrreryColors.light);

  OrreryShape get shape =>
      Theme.of(this).extension<OrreryShape>() ?? const OrreryShape();

  OrreryMotion get tokens =>
      Theme.of(this).extension<OrreryMotion>() ?? const OrreryMotion();
}
