import 'package:flutter/material.dart';

import '../kit/rail_chip.dart' show railLook;
import '../kit/status_chip.dart';
import '../model/pane_facts.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_glyph.dart';
import 'chat_rows.dart';
import 'chat_text.dart';

String rowLastText(BuildContext context, ChatRow row, PaneFacts facts) {
  final l = context.l10n;
  final pane = row.pane;
  if (row.working) {
    final tool = facts.tool;
    if (tool == null) return l.chatWorking;
    return [liveVerb(l, catOfTool(tool), tool), ?facts.toolSummary].join(' ');
  }
  if (row.waiting) {
    final menu = pane.menu;
    if (menu != null && menu.question.isNotEmpty) return menu.question;
    return waitLabel(l, pane.waitText);
  }
  final last = row.last?.text;
  if (last != null && last.isNotEmpty) return last;
  return railLook(context, pane.rail).label;
}

class ChatInboxRow extends StatelessWidget {
  const ChatInboxRow({
    super.key,
    required this.row,
    required this.facts,
    required this.now,
    required this.onTap,
    this.onLongPress,
  });

  final ChatRow row;
  final PaneFacts facts;
  final DateTime now;
  final VoidCallback onTap;
  final VoidCallback? onLongPress;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final pane = row.pane;
    final look = railLook(context, pane.rail);
    final tone = look.tone.resolve(context);
    final at = row.at;
    final meta = TextStyle(
      fontSize: 12.5,
      color: c.inkMuted,
      fontFeatures: const [FontFeature.tabularFigures()],
    );
    final showState =
        pane.isAgent &&
        (row.working || row.waiting || pane.rail == RailState.failed);
    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        key: ValueKey('chat-row-${pane.key}'),
        borderRadius: BorderRadius.circular(22),
        onTap: onTap,
        onLongPress: onLongPress,
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 68),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: Row(
              children: [
                ChatGlyph(agent: pane.agent, size: 40, avatar: true),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.baseline,
                        textBaseline: TextBaseline.alphabetic,
                        children: [
                          Expanded(
                            child: Text(
                              pane.title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                fontSize: 16,
                                fontWeight: FontWeight.w500,
                                color: c.ink,
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          ConstrainedBox(
                            constraints: const BoxConstraints(maxWidth: 96),
                            child: Text(
                              row.project,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: meta,
                            ),
                          ),
                          if (at != null) ...[
                            const SizedBox(width: 6),
                            Text(
                              shortElapsed(l, now.difference(at)),
                              style: meta,
                            ),
                          ],
                        ],
                      ),
                      const SizedBox(height: 3),
                      Row(
                        children: [
                          if (showState) ...[
                            if (row.working)
                              SizedBox.square(
                                dimension: 11,
                                child: CircularProgressIndicator(
                                  strokeWidth: 1.75,
                                  color: tone,
                                ),
                              )
                            else
                              Icon(look.icon, size: 14, color: tone),
                            const SizedBox(width: 4),
                            if (pane.rail == RailState.failed) ...[
                              Text(
                                look.label,
                                style: TextStyle(
                                  fontSize: 14,
                                  fontWeight: FontWeight.w500,
                                  color: tone,
                                ),
                              ),
                              const SizedBox(width: 6),
                            ],
                          ],
                          Expanded(
                            child: Text(
                              rowLastText(context, row, facts),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(fontSize: 14, color: c.inkMuted),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class RowCard extends StatelessWidget {
  const RowCard({super.key, required this.children, this.accent});

  final List<Widget> children;
  final Color? accent;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Container(
      padding: const EdgeInsets.all(4),
      decoration: BoxDecoration(
        color: c.raised,
        borderRadius: BorderRadius.circular(26),
        border: accent == null
            ? null
            : Border.all(color: accent!.withValues(alpha: 0.28)),
      ),
      child: Column(children: children),
    );
  }
}
