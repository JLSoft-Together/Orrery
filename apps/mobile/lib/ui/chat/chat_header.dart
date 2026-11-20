import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../kit/rail_chip.dart' show railLook;
import '../kit/status_chip.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_glyph.dart';
import 'reconnect_strip.dart' show slowLatencyMs;

class ChatHeader extends ConsumerWidget {
  const ChatHeader({
    super.key,
    required this.machineId,
    required this.pane,
    required this.project,
    required this.termMode,
    required this.onToggleTerm,
    required this.onMore,
    this.contextPct,
  });

  final String machineId;
  final PaneInfo pane;
  final String project;
  final bool termMode;
  final VoidCallback? onToggleTerm;
  final VoidCallback onMore;
  final int? contextPct;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.orrery;
    final l = context.l10n;
    final link = ref.watch(linkProvider(machineId));
    final latency = ref.watch(latencyProvider(machineId)).value;
    final online = link.status == LinkStatus.ready;
    final look = railLook(context, pane.rail);
    final tone = look.tone.resolve(context);
    final sub = TextStyle(fontSize: 12.5, color: c.inkMuted);
    final pct = contextPct;
    return SizedBox(
      height: 56,
      child: Row(
        children: [
          SizedBox(
            width: 96,
            child: Row(
              children: [
                IconButton(
                  key: const ValueKey('chat-back'),
                  tooltip: MaterialLocalizations.of(context).backButtonTooltip,
                  onPressed: () => Navigator.of(context).maybePop(),
                  icon: const Icon(Icons.chevron_left_rounded, size: 26),
                ),
                if (onToggleTerm != null)
                  IconButton(
                    key: const ValueKey('chat-term-toggle'),
                    tooltip: termMode ? l.chatViewChat : l.chatViewTerminal,
                    isSelected: termMode,
                    onPressed: onToggleTerm,
                    icon: Icon(
                      termMode
                          ? Icons.chat_bubble_outline_rounded
                          : Icons.terminal_rounded,
                      size: 22,
                    ),
                  ),
              ],
            ),
          ),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    ChatGlyph(agent: pane.agent, size: 16),
                    const SizedBox(width: 6),
                    Flexible(
                      child: Text(
                        pane.title,
                        key: const ValueKey('chat-title'),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 15.5,
                          fontWeight: FontWeight.w600,
                          color: c.ink,
                        ),
                      ),
                    ),
                  ],
                ),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (pane.isAgent) ...[
                      Icon(look.icon, size: 12, color: tone),
                      const SizedBox(width: 3),
                      Flexible(
                        child: Text(
                          look.label,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: sub.copyWith(
                            color: tone,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                    ],
                    Flexible(
                      child: Text(
                        project,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: sub,
                      ),
                    ),
                    if (!online) ...[
                      const SizedBox(width: 8),
                      Icon(Icons.cloud_off_rounded, size: 12, color: c.waiting),
                      const SizedBox(width: 3),
                      Flexible(
                        child: Text(
                          l.chatNoNet,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: sub.copyWith(color: c.waiting),
                        ),
                      ),
                    ] else if (latency != null && latency > slowLatencyMs) ...[
                      const SizedBox(width: 8),
                      Icon(
                        Icons.network_check_rounded,
                        size: 12,
                        color: c.waiting,
                      ),
                      const SizedBox(width: 3),
                      Semantics(
                        label: l.chatSlow(latency),
                        excludeSemantics: true,
                        child: Text(
                          '$latency ms',
                          key: const ValueKey('chat-latency'),
                          maxLines: 1,
                          softWrap: false,
                          style: sub.copyWith(color: c.waiting),
                        ),
                      ),
                    ],
                  ],
                ),
              ],
            ),
          ),
          SizedBox(
            width: 96,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                if (pct != null)
                  Semantics(
                    label: l.chatContext(pct),
                    child: SizedBox.square(
                      dimension: 44,
                      child: Center(child: ContextRing(pct: pct)),
                    ),
                  ),
                IconButton(
                  key: const ValueKey('chat-more'),
                  tooltip: l.chatSession,
                  onPressed: onMore,
                  icon: const Icon(Icons.more_horiz_rounded),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class ContextRing extends StatelessWidget {
  const ContextRing({super.key, required this.pct, this.size = 30});

  final int pct;
  final double size;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final warn = pct >= 80;
    final color = warn ? c.waiting : c.inkMuted;
    return SizedBox.square(
      dimension: size,
      child: CustomPaint(
        painter: _RingPainter(pct / 100, c.lineStrong, color),
        child: Center(
          child: Text(
            '$pct',
            style: TextStyle(
              fontSize: 10.5,
              fontWeight: FontWeight.w600,
              color: warn ? c.waiting : c.ink,
            ),
          ),
        ),
      ),
    );
  }
}

class _RingPainter extends CustomPainter {
  const _RingPainter(this.value, this.track, this.color);

  final double value;
  final Color track;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final stroke = size.width * 0.087;
    final rect = Offset.zero & size;
    final ring = rect.deflate(stroke / 2);
    final base = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..color = track;
    canvas.drawArc(ring, 0, math.pi * 2, false, base);
    final fg = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = StrokeCap.round
      ..color = color;
    canvas.drawArc(
      ring,
      -math.pi / 2,
      math.pi * 2 * value.clamp(0, 1),
      false,
      fg,
    );
  }

  @override
  bool shouldRepaint(_RingPainter old) =>
      old.value != value || old.color != color || old.track != track;
}
