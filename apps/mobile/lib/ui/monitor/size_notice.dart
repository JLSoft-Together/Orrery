import 'package:flutter/material.dart';

import '../../remote/size_notes.dart';
import '../kit/kit.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

String megabytes(int bytes) => '${(bytes / (1 << 20)).toStringAsFixed(1)} MB';

class SizeNotice extends StatelessWidget {
  const SizeNotice({
    super.key,
    required this.limit,
    required this.trimmed,
    required this.onRetry,
  });

  final SizeLimit? limit;
  final SnapshotTrim? trimmed;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final current = limit;
    if (current != null) {
      return Padding(
        key: const ValueKey('monitor-limit'),
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 8),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: c.error.withValues(alpha: 0.1),
            borderRadius: context.shape.cardRadius,
            border: Border.all(color: c.error.withValues(alpha: 0.38)),
          ),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(12, 10, 8, 10),
            child: Row(
              children: [
                Icon(Icons.warning_amber_rounded, size: 20, color: c.error),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        l.monLimitTitle,
                        style: TextStyle(
                          fontSize: 14,
                          height: 20 / 14,
                          fontWeight: FontWeight.w600,
                          color: c.ink,
                        ),
                      ),
                      Text(
                        l.monLimitBody(
                          current.bytes > 0
                              ? megabytes(current.bytes)
                              : '> 1 MB',
                          megabytes(current.max),
                        ),
                        style: TextStyle(
                          fontSize: 12,
                          height: 16 / 12,
                          color: c.inkMuted,
                        ),
                      ),
                    ],
                  ),
                ),
                OrreryButton(
                  key: const ValueKey('monitor-limit-retry'),
                  label: l.retry,
                  kind: ButtonKind.text,
                  expand: false,
                  onPressed: onRetry,
                ),
              ],
            ),
          ),
        ),
      );
    }
    if (trimmed == null) return const SizedBox.shrink();
    return Padding(
      key: const ValueKey('monitor-trimmed'),
      padding: const EdgeInsets.fromLTRB(20, 2, 20, 6),
      child: Row(
        children: [
          Icon(Icons.info_outline_rounded, size: 16, color: c.inkMuted),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              l.monTrimmed,
              style: TextStyle(
                fontSize: 12,
                height: 16 / 12,
                color: c.inkMuted,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
