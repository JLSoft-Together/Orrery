import 'package:flutter/material.dart';

import '../model/pane_facts.dart';
import '../model/workspace_model.dart';
import '../widgets/common.dart';

class PaneHeader extends StatelessWidget {
  const PaneHeader({
    super.key,
    required this.pane,
    required this.facts,
    required this.claimed,
    this.onActions,
    this.onSwipe,
  });

  final PaneInfo pane;
  final PaneFacts facts;
  final bool claimed;
  final VoidCallback? onActions;
  final ValueChanged<int>? onSwipe;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      color: scheme.surface,
      child: GestureDetector(
        onHorizontalDragEnd: onSwipe == null
            ? null
            : (d) {
                final v = d.primaryVelocity ?? 0;
                if (v.abs() > 200) onSwipe!(v < 0 ? 1 : -1);
              },
        child: LayoutBuilder(
          builder: (context, box) => _row(context, box.maxWidth < 360),
        ),
      ),
    );
  }

  Widget _row(BuildContext context, bool narrow) {
    final scheme = Theme.of(context).colorScheme;
    final l = context.l10n;
    return InkWell(
      onLongPress: onActions,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 4, 8),
        child: Row(
          children: [
            AgentGlyph(agent: pane.agent, kind: pane.kind),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    pane.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  AnimatedSize(
                    duration: motion(context),
                    alignment: Alignment.topLeft,
                    child: _subtitle(context),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 6),
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                StatusBadge(state: pane.rail, iconOnly: narrow),
                if (!narrow) const SizedBox(height: 4),
                if (!narrow)
                  AnimatedSwitcher(
                    duration: motion(context),
                    child: Text(
                      claimed ? l.typerYou : l.typerViewOnly,
                      key: ValueKey(claimed),
                      style: TextStyle(
                        fontSize: 11,
                        color: claimed
                            ? scheme.primary
                            : scheme.onSurfaceVariant,
                        fontWeight: claimed ? FontWeight.w600 : FontWeight.w400,
                      ),
                    ),
                  ),
              ],
            ),
            if (onActions != null)
              IconButton(
                tooltip: l.more,
                onPressed: onActions,
                icon: const Icon(Icons.more_vert_rounded),
              )
            else
              const SizedBox(width: 8),
          ],
        ),
      ),
    );
  }

  Widget _subtitle(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final style = TextStyle(fontSize: 12, color: scheme.onSurfaceVariant);
    if (facts.hasTool) {
      final since = facts.toolSince;
      return Ticker1s(
        builder: (context, now) {
          final detail = facts.toolSummary;
          final time = since == null
              ? ''
              : ' · ${elapsedText(now.difference(since))}';
          return Text(
            '${facts.tool}${detail == null ? '' : ' · $detail'}$time',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: style,
          );
        },
      );
    }
    final parts = [
      if (pane.branch != null) pane.branch!,
      if (pane.subagents > 0) context.l10n.subagentsCount(pane.subagents),
    ];
    if (parts.isEmpty) return const SizedBox(height: 0);
    return Text(
      parts.join(' · '),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: style,
    );
  }
}
