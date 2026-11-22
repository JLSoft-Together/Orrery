import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class UnsentStrip extends StatelessWidget {
  const UnsentStrip({
    super.key,
    required this.queued,
    required this.onSend,
    required this.onDrop,
  });

  final int queued;
  final VoidCallback? onSend;
  final VoidCallback? onDrop;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final style = TextButton.styleFrom(
      minimumSize: const Size(48, 40),
      foregroundColor: c.ink,
      disabledForegroundColor: c.inkFaint,
      textStyle: const TextStyle(fontWeight: FontWeight.w700),
    );
    return Container(
      key: const ValueKey('term-unsent'),
      color: c.waiting.withValues(alpha: 0.12),
      padding: const EdgeInsets.only(left: 12, right: 4),
      child: Row(
        children: [
          Icon(Icons.link_off_rounded, size: 16, color: c.waiting),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              l.termUnsent(queued),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: c.waiting,
              ),
            ),
          ),
          TextButton(
            key: const ValueKey('term-unsent-send'),
            style: style,
            onPressed: onSend,
            child: Text(l.termSendUnsent),
          ),
          TextButton(
            key: const ValueKey('term-unsent-drop'),
            style: style,
            onPressed: onDrop,
            child: Text(l.termDropUnsent),
          ),
        ],
      ),
    );
  }
}
