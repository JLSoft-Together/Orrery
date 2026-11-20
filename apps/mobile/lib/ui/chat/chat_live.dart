import 'dart:async';

import 'package:flutter/material.dart';

import '../../remote/chat_protocol.dart';
import '../kit/pulse_dot.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_text.dart';

class SysLine extends StatelessWidget {
  const SysLine({super.key, required this.item});

  final ChatSys item;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final (icon, color) = switch (item.kind) {
      ChatSysKind.stop => (Icons.stop_rounded, c.error),
      ChatSysKind.error => (Icons.error_outline_rounded, c.error),
      ChatSysKind.limit => (Icons.speed_rounded, c.waiting),
      ChatSysKind.compact => (Icons.compress_rounded, c.inkMuted),
    };
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Icon(icon, size: 14, color: color),
        const SizedBox(width: 6),
        Flexible(
          child: Text(
            sysText(context.l10n, item),
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 13, color: c.inkMuted),
          ),
        ),
      ],
    );
  }
}

class LiveLine extends StatelessWidget {
  const LiveLine({super.key, required this.text, this.since});

  final String text;
  final DateTime? since;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Semantics(
      liveRegion: true,
      child: SizedBox(
        height: 28,
        child: Row(
          children: [
            const SizedBox(width: 6),
            PulseDot(color: c.working, size: 8),
            const SizedBox(width: 16),
            Expanded(
              child: Text(
                text,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 14.5, color: c.inkMuted),
              ),
            ),
            if (since != null)
              MinuteTicker(
                builder: (context, now) => Text(
                  shortElapsed(context.l10n, now.difference(since!)),
                  style: TextStyle(
                    fontSize: 13,
                    color: c.inkMuted,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class MinuteTicker extends StatefulWidget {
  const MinuteTicker({super.key, required this.builder});

  final Widget Function(BuildContext context, DateTime now) builder;

  @override
  State<MinuteTicker> createState() => _MinuteTickerState();
}

class _MinuteTickerState extends State<MinuteTicker> {
  Timer? _timer;
  DateTime _now = DateTime.now();

  @override
  void initState() {
    super.initState();
    _schedule();
  }

  void _schedule() {
    final wait = 60 - _now.second;
    _timer = Timer(Duration(seconds: wait <= 0 ? 60 : wait), () {
      if (!mounted) return;
      setState(() => _now = DateTime.now());
      _schedule();
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, _now);
}
