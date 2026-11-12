import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';

class OrbitMark extends StatelessWidget {
  const OrbitMark({super.key, this.size = 96});

  final double size;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return RepaintBoundary(
      child: CustomPaint(
        size: Size.square(size),
        painter: _OrbitPainter(
          ring: c.accent,
          core: c.accent,
          moonA: c.agentCodex,
          moonB: c.agentShell,
        ),
      ),
    );
  }
}

class _OrbitPainter extends CustomPainter {
  const _OrbitPainter({
    required this.ring,
    required this.core,
    required this.moonA,
    required this.moonB,
  });

  final Color ring;
  final Color core;
  final Color moonA;
  final Color moonB;

  @override
  void paint(Canvas canvas, Size size) {
    final s = size.width / 96;
    final center = Offset(size.width / 2, size.height / 2);
    final stroke = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 3 * s;
    canvas.drawCircle(
      center,
      34 * s,
      stroke..color = ring.withValues(alpha: 0.28),
    );
    canvas.drawCircle(
      center,
      23 * s,
      stroke..color = ring.withValues(alpha: 0.55),
    );
    canvas.drawCircle(center, 14 * s, stroke..color = ring);
    canvas.drawCircle(center, 6 * s, Paint()..color = core);
    canvas.drawCircle(
      center.translate(0, -34 * s),
      6 * s,
      Paint()..color = moonA,
    );
    canvas.drawCircle(
      center.translate(34 * s * 0.94, -34 * s * 0.34),
      4.5 * s,
      Paint()..color = moonB,
    );
  }

  @override
  bool shouldRepaint(_OrbitPainter old) =>
      old.ring != ring ||
      old.core != core ||
      old.moonA != moonA ||
      old.moonB != moonB;
}
