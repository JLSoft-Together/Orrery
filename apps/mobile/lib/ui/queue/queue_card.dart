import 'package:flutter/material.dart';

import '../../app/menu_action.dart';
import '../../remote/chat_protocol.dart';
import '../kit/buttons.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'queue_model.dart';

String queueQuestionText(BuildContext context, PaneInfo pane) {
  final l = context.l10n;
  final menu = pane.menu;
  if (menu != null && menu.question.isNotEmpty) return menu.question;
  final question = pane.question;
  if (question != null && question.isNotEmpty) return question;
  return waitLabel(l, pane.waitText ?? 'question');
}

List<RemoteMenuItem> queueChoices(PaneInfo pane) {
  final menu = pane.menu;
  if (menu == null) return const [];
  return [
    for (final item in menu.items)
      if (menuItemFor(pane, menu.sig, item.index) != null) item,
  ];
}

class QueueCard extends StatelessWidget {
  const QueueCard({
    super.key,
    required this.slot,
    required this.now,
    required this.enabled,
    required this.canReply,
    required this.draft,
    required this.onPick,
    required this.onSend,
    required this.onReload,
    required this.onUndoSnooze,
  });

  final QueueSlot slot;
  final DateTime now;
  final bool enabled;
  final bool canReply;
  final TextEditingController draft;
  final ValueChanged<RemoteMenuItem> onPick;
  final VoidCallback onSend;
  final VoidCallback onReload;
  final VoidCallback onUndoSnooze;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final pane = slot.pane;
    final waited = shortElapsed(l, now.difference(slot.waitingSince));
    return Container(
      key: const ValueKey('queue-card'),
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        color: c.raised,
        borderRadius: BorderRadius.circular(24),
        border: Border.all(color: c.line),
      ),
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                AgentGlyph(agent: pane.agent, kind: pane.kind, size: 28),
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
                          fontSize: 17,
                          fontWeight: FontWeight.w600,
                          color: c.ink,
                        ),
                      ),
                      Text(
                        '${slot.tab.displayName}, ${l.queueWaited(waited)}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(fontSize: 13, color: c.inkMuted),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            AnimatedSwitcher(
              duration: motion(context, 160),
              child: KeyedSubtree(
                key: ValueKey(slot.phase),
                child: _body(context),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _body(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    switch (slot.phase) {
      case QueuePhase.sent:
        return _note(
          context,
          Icons.check_circle_outline_rounded,
          l.queueSent,
          c.done,
          const ValueKey('queue-sent'),
        );
      case QueuePhase.gone:
        return _note(
          context,
          Icons.desktop_windows_outlined,
          l.queueGone,
          c.inkMuted,
          const ValueKey('queue-gone'),
        );
      case QueuePhase.snoozed:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _note(
              context,
              Icons.snooze_rounded,
              l.queueSnoozed(slot.snoozeMinutes),
              c.waiting,
              const ValueKey('queue-snoozed'),
            ),
            const SizedBox(height: 12),
            OrreryButton(
              key: const ValueKey('queue-undo-snooze'),
              label: l.chatUndo,
              kind: ButtonKind.secondary,
              icon: Icons.undo_rounded,
              onPressed: enabled ? onUndoSnooze : null,
            ),
          ],
        );
      case QueuePhase.stale:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _note(
              context,
              Icons.sync_problem_rounded,
              l.queueStale,
              c.waiting,
              const ValueKey('queue-stale'),
            ),
            const SizedBox(height: 12),
            OrreryButton(
              key: const ValueKey('queue-reload'),
              label: l.queueStaleReload,
              kind: ButtonKind.secondary,
              onPressed: onReload,
            ),
          ],
        );
      case QueuePhase.pendingUndo:
        return _note(
          context,
          Icons.schedule_send_rounded,
          l.queuePending(slot.pickedLabel ?? ''),
          c.accentInk,
          const ValueKey('queue-pending'),
        );
      case QueuePhase.ready:
      case QueuePhase.sending:
        return _ask(context);
    }
  }

  Widget _note(
    BuildContext context,
    IconData icon,
    String text,
    Color color,
    Key key,
  ) => Padding(
    key: key,
    padding: const EdgeInsets.symmetric(vertical: 8),
    child: Row(
      children: [
        Icon(icon, size: 22, color: color),
        const SizedBox(width: 10),
        Expanded(
          child: Text(
            text,
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: color,
            ),
          ),
        ),
      ],
    ),
  );

  Widget _ask(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final pane = slot.pane;
    final busy = slot.phase == QueuePhase.sending;
    final menu = pane.menu;
    final choices = queueChoices(pane);
    final permission =
        menu != null &&
        (menu.kind == RemoteMenuKind.permission ||
            menu.kind == RemoteMenuKind.codexApproval);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          queueQuestionText(context, pane),
          key: const ValueKey('queue-question'),
          style: TextStyle(fontSize: 16, height: 1.35, color: c.ink),
        ),
        const SizedBox(height: 16),
        if (menu != null && choices.isNotEmpty)
          for (var k = 0; k < choices.length; k++)
            Padding(
              padding: EdgeInsets.only(top: k == 0 ? 0 : 8),
              child: OrreryButton(
                key: ValueKey('queue-menu-${choices[k].index}'),
                label: choices[k].label,
                kind: permission && k == 0
                    ? ButtonKind.primary
                    : ButtonKind.secondary,
                onPressed: enabled && !busy ? () => onPick(choices[k]) : null,
              ),
            )
        else if (menu != null || !canReply)
          Text(
            l.queueNoChoice,
            key: const ValueKey('queue-no-choice'),
            style: TextStyle(color: c.inkMuted),
          )
        else
          _reply(context, busy),
      ],
    );
  }

  Widget _reply(BuildContext context, bool busy) {
    final c = context.orrery;
    final l = context.l10n;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(
          key: const ValueKey('queue-reply'),
          controller: draft,
          enabled: enabled && !busy,
          minLines: 1,
          maxLines: 4,
          textInputAction: TextInputAction.newline,
          decoration: InputDecoration(
            hintText: l.queueReplyHint,
            filled: true,
            fillColor: c.surface,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(18),
              borderSide: BorderSide.none,
            ),
          ),
        ),
        const SizedBox(height: 10),
        ValueListenableBuilder<TextEditingValue>(
          valueListenable: draft,
          builder: (context, value, _) => OrreryButton(
            key: const ValueKey('queue-send'),
            label: l.queueSend,
            icon: Icons.send_rounded,
            busy: busy,
            onPressed: enabled && value.text.trim().isNotEmpty ? onSend : null,
          ),
        ),
      ],
    );
  }
}
