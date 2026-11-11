import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../model/workspace_model.dart';
import '../theme/orrery_theme.dart';
import '../widgets/common.dart';

class PermissionBar extends StatelessWidget {
  const PermissionBar({
    super.key,
    required this.pane,
    required this.enabled,
    required this.onKeys,
  });

  final PaneInfo pane;
  final bool enabled;
  final void Function(String keys) onKeys;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final color = context.statusColors.asked;
    final visible = pane.isAsked;
    return AnimatedSize(
      duration: motion(context),
      curve: Curves.easeOutCubic,
      alignment: Alignment.topCenter,
      child: !visible
          ? const SizedBox(width: double.infinity)
          : Container(
              width: double.infinity,
              margin: const EdgeInsets.fromLTRB(8, 8, 8, 4),
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: color.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(14),
                border: Border.all(color: color.withValues(alpha: 0.5)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.front_hand_outlined, size: 18, color: color),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          pane.waitText ?? l.statusAsked,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(fontWeight: FontWeight.w600),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      _button(l.answerYes, 'approve', filled: true),
                      _button(l.answerYesAlways, '2'),
                      _button(l.answerNo, 'deny'),
                      _button(l.answerEsc, 'esc'),
                    ],
                  ),
                ],
              ),
            ),
    );
  }

  Widget _button(String label, String keys, {bool filled = false}) {
    void tap() {
      HapticFeedback.mediumImpact();
      onKeys(keys);
    }

    final key = ValueKey('answer-$keys');
    return filled
        ? FilledButton(
            key: key,
            onPressed: enabled ? tap : null,
            child: Text(label),
          )
        : OutlinedButton(
            key: key,
            onPressed: enabled ? tap : null,
            child: Text(label),
          );
  }
}
