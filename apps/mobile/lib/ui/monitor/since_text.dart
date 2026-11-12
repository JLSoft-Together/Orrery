import 'dart:async';

import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';

String shortElapsed(Duration d) {
  if (d.isNegative || d.inSeconds < 60) return '<1m';
  if (d.inHours >= 1) return '${d.inHours}h';
  return '${d.inMinutes}m';
}

class SinceText extends StatefulWidget {
  const SinceText({super.key, required this.since});

  final DateTime? since;

  @override
  State<SinceText> createState() => _SinceTextState();
}

class _SinceTextState extends State<SinceText> {
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 20), (_) {
      if (mounted) setState(() {});
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final since = widget.since;
    if (since == null) return const SizedBox.shrink();
    return Text(
      shortElapsed(DateTime.now().difference(since)),
      style: TextStyle(
        fontSize: 12,
        height: 16 / 12,
        fontFamily: 'OrreryMono',
        color: context.orrery.inkFaint,
      ),
    );
  }
}
