import 'package:flutter/material.dart';

import '../kit/rail_chip.dart' show agentColor, agentLabel;

class ChatGlyph extends StatelessWidget {
  const ChatGlyph({
    super.key,
    required this.agent,
    this.size = 22,
    this.avatar = false,
  });

  final String? agent;
  final double size;
  final bool avatar;

  @override
  Widget build(BuildContext context) {
    final color = agentColor(context, agent);
    final markSize = avatar ? size * 0.48 : size * 0.82;
    final mark = CustomPaint(
      size: Size.square(markSize),
      painter: GlyphPainter(agent, color),
    );
    return Semantics(
      label: agentLabel(agent),
      image: true,
      child: Container(
        width: size,
        height: size,
        alignment: Alignment.center,
        decoration: avatar
            ? BoxDecoration(
                shape: BoxShape.circle,
                color: color.withValues(alpha: 0.13),
              )
            : null,
        child: mark,
      ),
    );
  }
}

class GlyphPainter extends CustomPainter {
  const GlyphPainter(this.agent, this.color);

  final String? agent;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final k = size.width / 24;
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.2 * k
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    Offset p(double x, double y) => Offset(x * k, y * k);
    switch (agent) {
      case 'claude':
        canvas
          ..drawLine(p(12, 3.5), p(12, 20.5), paint)
          ..drawLine(p(3.5, 12), p(20.5, 12), paint)
          ..drawLine(p(6, 6), p(18, 18), paint)
          ..drawLine(p(18, 6), p(6, 18), paint);
      case 'codex':
        final path = Path()
          ..moveTo(9 * k, 6 * k)
          ..lineTo(4 * k, 12 * k)
          ..lineTo(9 * k, 18 * k)
          ..moveTo(15 * k, 6 * k)
          ..lineTo(20 * k, 12 * k)
          ..lineTo(15 * k, 18 * k);
        canvas.drawPath(path, paint);
      default:
        final path = Path()
          ..moveTo(5 * k, 7 * k)
          ..lineTo(10 * k, 12 * k)
          ..lineTo(5 * k, 17 * k)
          ..moveTo(12.5 * k, 17.5 * k)
          ..lineTo(19 * k, 17.5 * k);
        canvas.drawPath(path, paint);
    }
  }

  @override
  bool shouldRepaint(GlyphPainter old) =>
      old.agent != agent || old.color != color;
}
