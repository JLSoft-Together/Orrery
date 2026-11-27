import 'package:flutter/material.dart';

import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'prompt_model.dart';
import 'prompt_sheet.dart';
import 'prompts_controller.dart';

class PromptStrip extends StatelessWidget {
  const PromptStrip({
    super.key,
    required this.controller,
    required this.enabled,
    required this.onInsert,
    this.projectPath,
  });

  final PromptsController controller;
  final bool enabled;
  final void Function(SavedPrompt prompt) onInsert;
  final String? projectPath;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: controller,
      builder: (context, _) {
        final l = context.l10n;
        final c = context.orrery;
        if (controller.status == PromptsStatus.loading) {
          return const SizedBox.shrink();
        }
        final failed = controller.status == PromptsStatus.failed;
        final list = controller.list;
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: SizedBox(
            height: 44,
            child: ListView(
              key: const ValueKey('prompt-strip'),
              scrollDirection: Axis.horizontal,
              children: [
                _Pill(
                  key: const ValueKey('prompt-add'),
                  icon: Icons.add_rounded,
                  label: l.promptsAdd,
                  tint: c.accentInk,
                  onTap: enabled && !failed
                      ? () => showPromptEditor(
                          context,
                          controller,
                          projectPath: projectPath,
                        )
                      : null,
                ),
                if (failed)
                  _Pill(
                    key: const ValueKey('prompt-retry'),
                    icon: Icons.refresh_rounded,
                    label: l.promptsRetry,
                    tint: c.inkMuted,
                    onTap: controller.load,
                  ),
                for (final prompt in list.items)
                  _Pill(
                    key: ValueKey('prompt-chip-${prompt.id}'),
                    label: prompt.title,
                    tint: c.ink,
                    emphasis: prompt.local,
                    onTap: enabled ? () => onInsert(prompt) : null,
                    onLongPress: () => showPromptActions(
                      context,
                      controller,
                      prompt,
                      projectPath,
                    ),
                  ),
                if (list.hidden > 0) _Note(text: l.promptsHidden(list.hidden)),
                if (list.more > 0) _Note(text: l.promptsMore(list.more)),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _Pill extends StatelessWidget {
  const _Pill({
    super.key,
    required this.label,
    required this.tint,
    required this.onTap,
    this.icon,
    this.onLongPress,
    this.emphasis = false,
  });

  final String label;
  final Color tint;
  final IconData? icon;
  final bool emphasis;
  final VoidCallback? onTap;
  final VoidCallback? onLongPress;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Padding(
      padding: const EdgeInsets.only(right: 8),
      child: Material(
        color: c.raised,
        shape: StadiumBorder(
          side: BorderSide(color: emphasis ? c.accent : c.line),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          onLongPress: onLongPress,
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 44, maxWidth: 220),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 14),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (icon != null) ...[
                    Icon(icon, size: 18, color: tint),
                    const SizedBox(width: 6),
                  ],
                  Flexible(
                    child: Text(
                      label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w500,
                        color: onTap == null ? c.inkFaint : tint,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Note extends StatelessWidget {
  const _Note({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.only(right: 8, left: 4),
        child: Text(
          text,
          style: TextStyle(fontSize: 12, color: context.orrery.inkFaint),
        ),
      ),
    );
  }
}
