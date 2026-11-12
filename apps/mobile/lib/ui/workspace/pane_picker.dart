import 'package:flutter/material.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class PanePickerButton extends StatelessWidget {
  const PanePickerButton({
    super.key,
    required this.pane,
    required this.tabName,
    required this.onTap,
  });

  final PaneInfo pane;
  final String tabName;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Semantics(
      button: true,
      label: context.l10n.paneSheetOpen,
      child: InkWell(
        key: const ValueKey('pane-picker'),
        onTap: onTap,
        borderRadius: BorderRadius.circular(999),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: c.control,
            borderRadius: BorderRadius.circular(999),
            border: Border.all(color: c.lineStrong),
          ),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(12, 8, 8, 8),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                DecoratedBox(
                  decoration: BoxDecoration(
                    color: agentColor(context, pane.agent),
                    shape: BoxShape.circle,
                  ),
                  child: const SizedBox.square(dimension: 8),
                ),
                const SizedBox(width: 8),
                Flexible(
                  child: AnimatedRename(
                    text: '${agentLabel(pane.agent)} · $tabName',
                    style: TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.w600,
                      color: c.ink,
                    ),
                  ),
                ),
                Icon(
                  Icons.keyboard_arrow_down_rounded,
                  size: 20,
                  color: c.inkMuted,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

Future<void> showPaneSheet(
  BuildContext context, {
  required TabInfo tab,
  required String currentKey,
  required ValueChanged<int> onPick,
}) {
  return showOrrerySheet<void>(
    context,
    builder: (sheet) {
      final c = sheet.orrery;
      return SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(0, 4, 0, 12),
                child: Text(
                  sheet.l10n.paneSheetTitle(tab.displayName),
                  style: TextStyle(
                    fontSize: 17,
                    fontWeight: FontWeight.w700,
                    color: c.ink,
                  ),
                ),
              ),
              Flexible(
                child: ListView(
                  shrinkWrap: true,
                  children: [
                    for (var i = 0; i < tab.panes.length; i++)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: _PaneRow(
                          pane: tab.panes[i],
                          selected: tab.panes[i].key == currentKey,
                          onTap: () {
                            Navigator.of(sheet).pop();
                            onPick(i);
                          },
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      );
    },
  );
}

class _PaneRow extends StatelessWidget {
  const _PaneRow({
    required this.pane,
    required this.selected,
    required this.onTap,
  });

  final PaneInfo pane;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return InkWell(
      key: ValueKey('pane-row-${pane.key}'),
      onTap: onTap,
      borderRadius: context.shape.cardRadius,
      child: AnimatedContainer(
        duration: motion(context),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: c.surface,
          borderRadius: context.shape.cardRadius,
          border: Border.all(color: selected ? c.accent : c.line),
        ),
        child: Row(
          children: [
            AgentTile(agent: pane.agent, size: 40),
            const SizedBox(width: 12),
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
                  Text(
                    agentLabel(pane.agent),
                    style: TextStyle(fontSize: 12, color: c.inkMuted),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            RailChip(state: pane.rail),
          ],
        ),
      ),
    );
  }
}
