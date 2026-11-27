import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'queue_model.dart';
import 'queue_page.dart';

({int others, bool mine}) queueNudgeCounts(
  WidgetRef ref,
  String machineId,
  String paneKey,
) => ref.watch(
  workspaceModelProvider(machineId).select((model) {
    var others = 0;
    var mine = false;
    for (final pane in model.panes) {
      if (!isQueueCandidate(pane)) continue;
      if (pane.key == paneKey) {
        mine = true;
      } else {
        others++;
      }
    }
    return (others: others, mine: mine);
  }),
);

class QueueNudge extends ConsumerStatefulWidget {
  const QueueNudge({
    super.key,
    required this.machineId,
    required this.paneKey,
    required this.child,
  });

  final String machineId;
  final String paneKey;
  final Widget child;

  @override
  ConsumerState<QueueNudge> createState() => _QueueNudgeState();
}

class _QueueNudgeState extends ConsumerState<QueueNudge> {
  bool _hidden = false;

  @override
  Widget build(BuildContext context) {
    final counts = queueNudgeCounts(ref, widget.machineId, widget.paneKey);
    final total = counts.others + (counts.mine ? 1 : 0);
    final show = !_hidden && counts.others >= 1 && total >= 2;
    final top = MediaQuery.paddingOf(context).top + 60;
    return Stack(
      children: [
        widget.child,
        Positioned(
          top: top,
          right: 12,
          child: AnimatedSwitcher(
            duration: motion(context, 160),
            child: show
                ? _Pill(
                    key: const ValueKey('queue-nudge'),
                    count: counts.others,
                    onOpen: () => openQueue(context, widget.machineId),
                    onHide: () => setState(() => _hidden = true),
                  )
                : const SizedBox.shrink(key: ValueKey('queue-nudge-off')),
          ),
        ),
      ],
    );
  }
}

class _Pill extends StatelessWidget {
  const _Pill({
    super.key,
    required this.count,
    required this.onOpen,
    required this.onHide,
  });

  final int count;
  final VoidCallback onOpen;
  final VoidCallback onHide;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return Material(
      color: c.ink,
      elevation: 3,
      shape: const StadiumBorder(),
      clipBehavior: Clip.antiAlias,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          InkWell(
            onTap: onOpen,
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 48),
              child: Padding(
                padding: const EdgeInsets.only(left: 16, right: 8),
                child: Center(
                  child: Text(
                    l.queueMore(count),
                    style: TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                      color: c.surface,
                    ),
                  ),
                ),
              ),
            ),
          ),
          InkWell(
            key: const ValueKey('queue-nudge-hide'),
            onTap: onHide,
            child: SizedBox(
              width: 48,
              height: 48,
              child: Tooltip(
                message: l.queueHide,
                child: Icon(Icons.close_rounded, size: 18, color: c.surface),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
