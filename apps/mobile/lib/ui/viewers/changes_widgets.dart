import 'package:flutter/material.dart';

import '../theme/orrery_theme.dart';
import '../widgets/common.dart';
import 'viewer_models.dart';

class ChangesBranch extends StatelessWidget {
  const ChangesBranch({super.key, required this.status});

  final ChangesStatus status;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l = context.l10n;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 8),
      child: Row(
        children: [
          Icon(Icons.call_split_rounded, size: 18, color: scheme.primary),
          const SizedBox(width: 6),
          Flexible(
            child: Text(
              status.branch ?? l.vDetachedHead,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontWeight: FontWeight.w700),
            ),
          ),
          if (status.ahead > 0 || status.behind > 0)
            Padding(
              padding: const EdgeInsets.only(left: 8),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    Icons.arrow_upward_rounded,
                    size: 14,
                    color: scheme.onSurfaceVariant,
                  ),
                  Text(
                    '${status.ahead} ',
                    style: TextStyle(
                      color: scheme.onSurfaceVariant,
                      fontSize: 13,
                    ),
                  ),
                  Icon(
                    Icons.arrow_downward_rounded,
                    size: 14,
                    color: scheme.onSurfaceVariant,
                  ),
                  Text(
                    '${status.behind}',
                    style: TextStyle(
                      color: scheme.onSurfaceVariant,
                      fontSize: 13,
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

class ChangesSection extends StatelessWidget {
  const ChangesSection({
    super.key,
    required this.title,
    required this.actionLabel,
    required this.onAction,
  });

  final String title;
  final String actionLabel;
  final VoidCallback onAction;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 4, 0),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w700,
                color: scheme.onSurfaceVariant,
              ),
            ),
          ),
          TextButton(onPressed: onAction, child: Text(actionLabel)),
        ],
      ),
    );
  }
}

class ChangesFileRow extends StatelessWidget {
  const ChangesFileRow({
    super.key,
    required this.entry,
    required this.staged,
    required this.onTap,
    required this.onToggle,
    this.onDiscard,
  });

  final ChangeEntry entry;
  final bool staged;
  final VoidCallback onTap;
  final VoidCallback onToggle;
  final VoidCallback? onDiscard;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    final color = switch (entry.mark) {
      'A' || '?' => colors.done,
      'D' || '!' => colors.failed,
      _ => colors.warning,
    };
    final slash = entry.path.lastIndexOf('/');
    final name = entry.path.substring(slash + 1);
    final dir = slash > 0 ? entry.path.substring(0, slash) : '';
    return InkWell(
      onTap: onTap,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 52),
        child: Padding(
          padding: const EdgeInsets.only(left: 16),
          child: Row(
            children: [
              Container(
                width: 22,
                height: 22,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.16),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  entry.mark,
                  style: TextStyle(
                    color: color,
                    fontWeight: FontWeight.w800,
                    fontSize: 12,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(fontSize: 15),
                    ),
                    if (dir.isNotEmpty)
                      Text(
                        dir,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 12,
                          color: scheme.onSurfaceVariant,
                        ),
                      ),
                  ],
                ),
              ),
              if (entry.added != null || entry.removed != null)
                Padding(
                  padding: const EdgeInsets.only(left: 6),
                  child: Text(
                    '+${entry.added ?? 0} -${entry.removed ?? 0}',
                    style: TextStyle(
                      fontSize: 11,
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
                ),
              if (onDiscard != null)
                IconButton(
                  tooltip: l.vDiscardChanges,
                  icon: Icon(Icons.undo_rounded, color: scheme.error, size: 20),
                  onPressed: onDiscard,
                ),
              IconButton(
                tooltip: staged ? l.vUnstage : l.vStage,
                icon: Icon(
                  staged
                      ? Icons.remove_circle_outline_rounded
                      : Icons.add_circle_outline_rounded,
                  color: scheme.primary,
                ),
                onPressed: onToggle,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
