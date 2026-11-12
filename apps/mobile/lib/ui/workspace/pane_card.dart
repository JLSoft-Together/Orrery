import 'package:flutter/material.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../widgets/common.dart';
import '../theme/orrery_shape.dart';
import 'pane_preview.dart';

class PaneCard extends StatelessWidget {
  const PaneCard({
    super.key,
    required this.machineId,
    required this.pane,
    required this.onOpen,
    required this.onLongPress,
  });

  final String machineId;
  final PaneInfo pane;
  final VoidCallback onOpen;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return OrreryCard(
      barColor: agentColor(context, pane.agent),
      onLongPress: onLongPress,
      padding: const EdgeInsets.fromLTRB(16, 14, 14, 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AgentTile(agent: pane.agent, size: 40),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    AnimatedRename(
                      text: pane.title,
                      style: TextStyle(
                        fontSize: 15,
                        height: 20 / 15,
                        fontWeight: FontWeight.w600,
                        color: c.ink,
                      ),
                    ),
                    if (pane.branch != null)
                      Row(
                        children: [
                          Icon(
                            Icons.call_split_rounded,
                            size: 14,
                            color: c.inkFaint,
                          ),
                          const SizedBox(width: 4),
                          Expanded(
                            child: Text(
                              pane.branch!,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                fontFamily: 'OrreryMono',
                                fontSize: 12,
                                height: 16 / 12,
                                color: c.inkMuted,
                              ),
                            ),
                          ),
                        ],
                      ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              RailChip(state: pane.rail),
            ],
          ),
          const SizedBox(height: 12),
          PanePreview(
            machineId: machineId,
            pane: pane,
            maxLines: 6,
            minHeight: 120,
          ),
          const SizedBox(height: 12),
          OrreryButton(
            key: const ValueKey('pane-open-full'),
            label: l.wsOpenFull,
            icon: Icons.terminal_rounded,
            onPressed: onOpen,
          ),
        ],
      ),
    );
  }
}
