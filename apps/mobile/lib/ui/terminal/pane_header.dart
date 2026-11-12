import 'package:flutter/material.dart';

import '../kit/kit.dart';
import '../model/pane_facts.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class PaneHeader extends StatelessWidget {
  const PaneHeader({
    super.key,
    required this.pane,
    required this.facts,
    required this.claimed,
    this.compact = false,
    this.onActions,
    this.onSwipe,
  });

  final PaneInfo pane;
  final PaneFacts facts;
  final bool claimed;
  final bool compact;
  final VoidCallback? onActions;
  final ValueChanged<int>? onSwipe;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Material(
      color: c.surface,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(bottom: BorderSide(color: c.line)),
        ),
        child: GestureDetector(
          onHorizontalDragEnd: onSwipe == null
              ? null
              : (d) {
                  final v = d.primaryVelocity ?? 0;
                  if (v.abs() > 200) onSwipe!(v < 0 ? 1 : -1);
                },
          child: LayoutBuilder(
            builder: (context, box) =>
                compact ? _slim(context) : _full(context, box.maxWidth < 360),
          ),
        ),
      ),
    );
  }

  Widget _typer(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return AnimatedSwitcher(
      duration: motion(context),
      child: Text(
        claimed ? l.typerYou : l.typerViewOnly,
        key: ValueKey(claimed),
        style: TextStyle(
          fontSize: 11,
          color: claimed ? c.accentInk : c.inkFaint,
          fontWeight: claimed ? FontWeight.w600 : FontWeight.w400,
        ),
      ),
    );
  }

  Widget _slim(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 6, 16, 6),
      child: Row(
        children: [
          RailChip(state: pane.rail, dense: true),
          const SizedBox(width: 10),
          Expanded(child: _subtitle(context)),
          const SizedBox(width: 8),
          _typer(context),
        ],
      ),
    );
  }

  Widget _full(BuildContext context, bool narrow) {
    final c = context.orrery;
    final l = context.l10n;
    return InkWell(
      onLongPress: onActions,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 4, 8),
        child: Row(
          children: [
            AgentTile(agent: pane.agent, size: 32),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    pane.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.w600,
                      color: c.ink,
                    ),
                  ),
                  QuietSize(
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
                RailChip(state: pane.rail, dense: true),
                if (!narrow) const SizedBox(height: 4),
                if (!narrow) _typer(context),
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
    final c = context.orrery;
    final style = TextStyle(
      fontFamily: 'OrreryMono',
      fontSize: 12,
      height: 16 / 12,
      color: c.inkMuted,
    );
    if (facts.hasTool) {
      final since = facts.toolSince;
      return Ticker1s(
        builder: (context, now) {
          final detail = facts.toolSummary;
          final time = since == null
              ? ''
              : ' · ${elapsedText(context.l10n, now.difference(since))}';
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
    if (parts.isEmpty) return const SizedBox(height: 16);
    return Text(
      parts.join(' · '),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: style,
    );
  }
}
