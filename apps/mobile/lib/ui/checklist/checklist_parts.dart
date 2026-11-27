import 'package:flutter/material.dart';

import '../../l10n/app_localizations.dart';
import '../theme/orrery_shape.dart';
import 'checklist_model.dart';

String checkViewText(AppLocalizations l, CheckView view) => switch (view) {
  CheckView.none => l.vClNone,
  CheckView.idle => l.vClIdle,
  CheckView.running => l.vClRunning,
  CheckView.pass => l.vClPass,
  CheckView.fail => l.vClFail,
  CheckView.stale => l.vClStale,
};

IconData checkViewIcon(CheckView view) => switch (view) {
  CheckView.none || CheckView.idle => Icons.radio_button_unchecked_rounded,
  CheckView.running => Icons.sync_rounded,
  CheckView.pass => Icons.check_circle_rounded,
  CheckView.fail => Icons.cancel_rounded,
  CheckView.stale => Icons.schedule_rounded,
};

class ChecklistSummary extends StatelessWidget {
  const ChecklistSummary({super.key, required this.data});

  final ChecklistData data;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.orrery;
    final ready = data.ready;
    final color = ready ? c.done : c.inkMuted;
    return Row(
      children: [
        Icon(
          ready ? Icons.verified_rounded : Icons.checklist_rounded,
          size: 18,
          color: color,
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            ready ? l.vClReady : l.vClOpen(data.open),
            key: const ValueKey('cl-summary'),
            style: Theme.of(context).textTheme.titleSmall
                ?.copyWith(color: color, fontWeight: FontWeight.w600),
          ),
        ),
      ],
    );
  }
}

class ChecklistRow extends StatelessWidget {
  const ChecklistRow({super.key, required this.item, required this.onChanged});

  final ChecklistItem item;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.orrery;
    return CheckboxListTile(
      key: ValueKey('cl-item-${item.id}'),
      value: item.done,
      onChanged: (v) => onChanged(v ?? !item.done),
      controlAffinity: ListTileControlAffinity.leading,
      contentPadding: EdgeInsets.zero,
      title: Text(
        item.text,
        style: item.done
            ? TextStyle(
                color: c.inkFaint,
                decoration: TextDecoration.lineThrough,
              )
            : null,
      ),
      secondary: item.byAgent
          ? Tooltip(
              message: l.vClByAgent,
              child: Icon(
                Icons.smart_toy_outlined,
                size: 16,
                color: c.inkFaint,
              ),
            )
          : null,
    );
  }
}

class CheckStateRow extends StatelessWidget {
  const CheckStateRow({super.key, required this.view});

  final CheckView view;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.orrery;
    final color = switch (view) {
      CheckView.pass => c.done,
      CheckView.fail => c.error,
      CheckView.running => c.working,
      _ => c.inkMuted,
    };
    return Row(
      children: [
        Icon(checkViewIcon(view), size: 16, color: color),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            checkViewText(l, view),
            key: const ValueKey('cl-state'),
            style: TextStyle(color: color),
          ),
        ),
      ],
    );
  }
}
