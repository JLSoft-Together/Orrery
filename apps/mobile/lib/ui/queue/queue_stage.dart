import 'package:flutter/material.dart';

import '../kit/buttons.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import '../chat/chat_live.dart' show MinuteTicker;
import 'queue_card.dart';
import 'queue_driver.dart';
import 'queue_model.dart';
import '../../remote/queue_cap.dart';

class QueueStage extends StatelessWidget {
  const QueueStage({
    super.key,
    required this.slot,
    required this.enabled,
    required this.cap,
    required this.driver,
    required this.showHint,
    required this.onSeen,
    required this.onOpen,
  });

  final QueueSlot slot;
  final bool enabled;
  final bool cap;
  final QueueDriver driver;
  final bool showHint;
  final VoidCallback onSeen;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final ready = slot.actionable;
    final swipeable = (ready || slot.phase == QueuePhase.stale) && enabled;
    final canSnooze = ready && cap && enabled;
    Widget card = MinuteTicker(
      builder: (context, now) => QueueCard(
        slot: slot,
        now: now,
        enabled: enabled,
        canReply: cap,
        draft: driver.draftOf(slot.key),
        onPick: (item) {
          onSeen();
          driver.pick(slot, item);
        },
        onSend: () {
          onSeen();
          driver.reply(slot);
        },
        onReload: () => driver.reload(slot),
        onUndoSnooze: () => driver.undoSnooze(slot),
      ),
    );
    card = Dismissible(
      key: ValueKey('queue-swipe-${slot.key}'),
      direction: !swipeable
          ? DismissDirection.none
          : canSnooze
          ? DismissDirection.horizontal
          : DismissDirection.endToStart,
      confirmDismiss: (direction) async {
        onSeen();
        if (direction == DismissDirection.endToStart) {
          driver.skip(slot);
        } else {
          driver.snooze(slot, queueSnoozeMinutes.first);
        }
        return false;
      },
      background: _SwipeBack(
        icon: Icons.snooze_rounded,
        label: l.queueSnooze,
        color: c.waiting,
        alignment: Alignment.centerLeft,
      ),
      secondaryBackground: _SwipeBack(
        icon: Icons.skip_next_rounded,
        label: l.queueSkip,
        color: c.inkMuted,
        alignment: Alignment.centerRight,
      ),
      child: card,
    );
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (showHint && swipeable)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Text(
                canSnooze ? l.queueHint : l.queueHintSkip,
                key: const ValueKey('queue-hint'),
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 13, color: c.inkMuted),
              ),
            ),
          Expanded(child: card),
          const SizedBox(height: 12),
          Row(
            children: [
              if (canSnooze) ...[
                Expanded(child: _SnoozeButton(onPick: (m) => driver.snooze(slot, m))),
                const SizedBox(width: 8),
              ],
              Expanded(
                child: OrreryButton(
                  key: const ValueKey('queue-skip'),
                  label: l.queueSkip,
                  kind: ButtonKind.secondary,
                  onPressed: swipeable ? () => driver.skip(slot) : null,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OrreryButton(
                  key: const ValueKey('queue-open-pane'),
                  label: l.queueOpenPane,
                  kind: ButtonKind.secondary,
                  onPressed: slot.canResend || slot.phase == QueuePhase.stale
                      ? onOpen
                      : null,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _SwipeBack extends StatelessWidget {
  const _SwipeBack({
    required this.icon,
    required this.label,
    required this.color,
    required this.alignment,
  });

  final IconData icon;
  final String label;
  final Color color;
  final Alignment alignment;

  @override
  Widget build(BuildContext context) => Container(
    alignment: alignment,
    padding: const EdgeInsets.symmetric(horizontal: 24),
    decoration: BoxDecoration(
      color: color.withValues(alpha: 0.14),
      borderRadius: BorderRadius.circular(24),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, color: color),
        const SizedBox(width: 8),
        Text(
          label,
          style: TextStyle(fontWeight: FontWeight.w600, color: color),
        ),
      ],
    ),
  );
}

class _SnoozeButton extends StatelessWidget {
  const _SnoozeButton({required this.onPick});

  final ValueChanged<int> onPick;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return PopupMenuButton<int>(
      key: const ValueKey('queue-snooze'),
      tooltip: l.queueSnooze,
      onSelected: onPick,
      itemBuilder: (_) => [
        PopupMenuItem(
          key: const ValueKey('queue-snooze-15'),
          value: 15,
          child: Text(l.queueSnoozeMinutes(15)),
        ),
        PopupMenuItem(
          key: const ValueKey('queue-snooze-60'),
          value: 60,
          child: Text(l.queueSnoozeHour),
        ),
      ],
      child: Container(
        constraints: const BoxConstraints(minHeight: 52),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: c.control,
          borderRadius: context.shape.controlRadius,
          border: Border.all(color: c.lineStrong),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.snooze_rounded, size: 18, color: c.ink),
            const SizedBox(width: 6),
            Flexible(
              child: Text(
                l.queueSnooze,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  color: c.ink,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

