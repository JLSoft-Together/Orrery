import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_ask.dart' show AskFrame;

class PendingCard extends StatelessWidget {
  const PendingCard({
    super.key,
    required this.label,
    required this.undo,
    required this.busy,
    required this.onUndo,
  });

  final String label;
  final Duration undo;
  final bool busy;
  final VoidCallback? onUndo;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return ClipRRect(
      key: const ValueKey('chat-menu-pending'),
      borderRadius: BorderRadius.circular(22),
      child: ColoredBox(
        color: c.raised,
        child: Column(
          children: [
            SizedBox(
              height: 54,
              child: Row(
                children: [
                  const SizedBox(width: 18),
                  Icon(Icons.check_rounded, size: 18, color: c.done),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      l.chatPicked(label),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(fontSize: 15, color: c.ink),
                    ),
                  ),
                  TextButton.icon(
                    key: const ValueKey('chat-menu-undo'),
                    onPressed: onUndo,
                    style: TextButton.styleFrom(
                      minimumSize: const Size(44, 44),
                      foregroundColor: c.ink,
                      shape: const StadiumBorder(),
                    ),
                    icon: const Icon(Icons.undo_rounded, size: 16),
                    label: Text(
                      l.chatUndo,
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                  ),
                  const SizedBox(width: 6),
                ],
              ),
            ),
            SizedBox(
              height: 3,
              child: busy
                  ? LinearProgressIndicator(
                      color: c.accent,
                      backgroundColor: c.line,
                    )
                  : TweenAnimationBuilder<double>(
                      tween: Tween(begin: 1, end: 0),
                      duration: undo,
                      builder: (context, value, _) => Align(
                        alignment: Alignment.centerLeft,
                        child: FractionallySizedBox(
                          widthFactor: value,
                          child: ColoredBox(color: c.accent),
                        ),
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class SentLine extends StatelessWidget {
  const SentLine({super.key, required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Icon(Icons.check_rounded, size: 14, color: c.done),
        const SizedBox(width: 6),
        Flexible(
          child: Text(
            context.l10n.chatSent(label),
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 13, color: c.inkMuted),
          ),
        ),
      ],
    );
  }
}

class WaitCard extends StatelessWidget {
  const WaitCard({super.key, required this.title, required this.onTerminal});

  final String title;
  final VoidCallback onTerminal;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return AskFrame(
      icon: Icons.schedule_rounded,
      title: title,
      children: [
        const SizedBox(height: 12),
        Material(
          color: c.surface,
          shape: StadiumBorder(side: BorderSide(color: c.lineStrong)),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            key: const ValueKey('chat-wait-terminal'),
            onTap: onTerminal,
            child: SizedBox(
              height: 46,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.terminal_rounded, size: 18, color: c.ink),
                  const SizedBox(width: 8),
                  Text(
                    context.l10n.chatOpenTerminal,
                    style: TextStyle(fontSize: 15, color: c.ink),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}
