import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
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
    final c = context.orrery;
    final visible = pane.isAsked;
    return QuietSize(
      alignment: Alignment.topCenter,
      child: !visible
          ? const SizedBox(width: double.infinity)
          : Container(
              width: double.infinity,
              margin: const EdgeInsets.fromLTRB(12, 8, 12, 4),
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: c.waiting.withValues(alpha: 0.1),
                borderRadius: context.shape.cardRadius,
                border: Border.all(color: c.waiting.withValues(alpha: 0.38)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.schedule_rounded, size: 18, color: c.waiting),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          pane.waitText ?? l.statusAsked,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontWeight: FontWeight.w600,
                            color: c.ink,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      _button(l.answerYes, 'approve', ButtonKind.primary),
                      _button(l.answerYesAlways, '2', ButtonKind.secondary),
                      _button(l.answerNo, 'deny', ButtonKind.secondary),
                      _button(l.answerEsc, 'esc', ButtonKind.secondary),
                    ],
                  ),
                ],
              ),
            ),
    );
  }

  Widget _button(String label, String keys, ButtonKind kind) {
    return OrreryButton(
      key: ValueKey('answer-$keys'),
      label: label,
      kind: kind,
      expand: false,
      onPressed: enabled
          ? () {
              HapticFeedback.mediumImpact();
              onKeys(keys);
            }
          : null,
    );
  }
}
