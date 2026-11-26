import 'package:flutter/material.dart';

import '../../remote/away_summary.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'away_chips.dart';

String awayAgo(BuildContext context, Duration d) {
  final l = context.l10n;
  if (d.inHours > 0) {
    return '${l.durationHours(d.inHours)} ${l.durationMinutes(d.inMinutes % 60)}';
  }
  return l.durationMinutes(d.inMinutes);
}

class AwayCard extends StatelessWidget {
  const AwayCard({
    super.key,
    required this.summary,
    required this.selected,
    required this.onSelect,
    required this.onSeen,
  });

  final AwaySummary summary;
  final AwayGroup? selected;
  final ValueChanged<AwayGroup> onSelect;
  final VoidCallback onSeen;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final groups = [
      for (final g in AwayGroup.values)
        if (summary.countOf(g) > 0) g,
    ];
    return Container(
      key: const ValueKey('away-card'),
      margin: const EdgeInsets.only(top: 8, bottom: 4),
      padding: const EdgeInsets.fromLTRB(16, 14, 8, 6),
      decoration: BoxDecoration(
        color: c.raised,
        borderRadius: BorderRadius.circular(26),
        border: Border.all(color: c.lineStrong),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(right: 8),
            child: Text(
              l.awayTitle(clock(summary.since), awayAgo(context, summary.away)),
              key: const ValueKey('away-title'),
              style: TextStyle(
                fontSize: 15,
                fontWeight: FontWeight.w600,
                color: c.ink,
              ),
            ),
          ),
          const SizedBox(height: 10),
          Padding(
            padding: const EdgeInsets.only(right: 8),
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final g in groups)
                  AwayChip(
                    key: ValueKey('away-chip-${g.name}'),
                    group: g,
                    count: summary.countOf(g),
                    selected: selected == g,
                    onTap: summary.idsOf(g).isEmpty ? null : () => onSelect(g),
                  ),
              ],
            ),
          ),
          if (summary.partial)
            Padding(
              padding: const EdgeInsets.fromLTRB(0, 10, 8, 0),
              child: Text(
                l.awayPartial,
                key: const ValueKey('away-partial'),
                style: TextStyle(fontSize: 12, color: c.inkMuted),
              ),
            ),
          Align(
            alignment: Alignment.centerRight,
            child: TextButton(
              key: const ValueKey('away-seen'),
              style: TextButton.styleFrom(
                minimumSize: const Size(48, 48),
                tapTargetSize: MaterialTapTargetSize.padded,
              ),
              onPressed: onSeen,
              child: Text(l.awaySeen),
            ),
          ),
        ],
      ),
    );
  }
}
