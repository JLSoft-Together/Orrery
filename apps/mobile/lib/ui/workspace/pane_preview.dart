import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';

List<String> previewLines(
  PaneInfo pane,
  String? tool,
  String? toolSummary,
  String subagentsText,
  String statusLabel,
) {
  final lines = <String>[];
  if (pane.isAsked && pane.waitText != null) {
    lines.addAll(pane.waitText!.split('\n').where((l) => l.trim().isNotEmpty));
  }
  if (tool != null) {
    lines.add('● ${[tool, ?toolSummary].join(' · ')}');
  }
  if (pane.subagents > 0) lines.add('✻ $subagentsText');
  if (lines.isEmpty) lines.add(statusLabel);
  return lines;
}

class PanePreview extends ConsumerWidget {
  const PanePreview({
    super.key,
    required this.machineId,
    required this.pane,
    required this.maxLines,
    this.minHeight = 0,
  });

  final String machineId;
  final PaneInfo pane;
  final int maxLines;
  final double minHeight;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.orrery;
    final l = context.l10n;
    final facts = ref.watch(paneFactsProvider((machineId, pane.terminalId)));
    final lines = previewLines(
      pane,
      facts.tool,
      facts.toolSummary,
      l.subagentsCount(pane.subagents),
      pane.rail == RailState.idle ? l.statusIdle : l.statusStarting,
    );
    return SizedBox(
      width: double.infinity,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: c.well,
          borderRadius: context.shape.controlRadius,
        ),
        child: ConstrainedBox(
          constraints: BoxConstraints(minHeight: minHeight),
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Align(
              alignment: Alignment.topLeft,
              child: AnimatedSwitcher(
                duration: motion(context, 150),
                child: Column(
                  key: ValueKey(lines.join('\n')),
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (final line in lines.take(maxLines))
                      Text(
                        line,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontFamily: 'OrreryMono',
                          fontSize: 12,
                          height: 18 / 12,
                          color: c.wellInk,
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
