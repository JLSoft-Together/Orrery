import 'package:flutter/material.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../widgets/common.dart';
import '../theme/orrery_shape.dart';
import 'pane_preview.dart';

class PaneGridCard extends StatelessWidget {
  const PaneGridCard({
    super.key,
    required this.machineId,
    required this.pane,
    required this.onOpen,
    required this.onLongPress,
    this.caption,
  });

  final String machineId;
  final PaneInfo pane;
  final String? caption;
  final VoidCallback onOpen;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return OrreryCard(
      barColor: agentColor(context, pane.agent),
      onTap: onOpen,
      onLongPress: onLongPress,
      padding: const EdgeInsets.fromLTRB(14, 12, 12, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (caption != null)
            Text(
              caption!,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 11,
                height: 14 / 11,
                color: c.inkFaint,
              ),
            ),
          AnimatedRename(
            text: pane.title,
            maxLines: 2,
            style: TextStyle(
              fontSize: 14,
              height: 18 / 14,
              fontWeight: FontWeight.w600,
              color: c.ink,
            ),
          ),
          const SizedBox(height: 6),
          Align(
            alignment: Alignment.centerLeft,
            child: RailChip(state: pane.rail, dense: true),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: PanePreview(machineId: machineId, pane: pane, maxLines: 3),
          ),
        ],
      ),
    );
  }
}

class PaneGrid extends StatelessWidget {
  const PaneGrid({
    super.key,
    required this.entries,
    required this.builder,
    this.bottomPadding = 96,
  });

  final List<(TabInfo, PaneInfo)> entries;
  final Widget Function(BuildContext context, TabInfo tab, PaneInfo pane)
  builder;
  final double bottomPadding;

  @override
  Widget build(BuildContext context) {
    return GridView.builder(
      padding: EdgeInsets.fromLTRB(16, 4, 16, bottomPadding),
      gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
        maxCrossAxisExtent: 240,
        mainAxisExtent: 168,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
      ),
      itemCount: entries.length,
      itemBuilder: (context, i) {
        final (tab, pane) = entries[i];
        return Appear(
          key: ValueKey('card-${pane.key}'),
          delay: (i % 8) * 25,
          child: builder(context, tab, pane),
        );
      },
    );
  }
}
