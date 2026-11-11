import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../model/workspace_model.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'workspace_page.dart';

class InboxPage extends ConsumerWidget {
  const InboxPage({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final model = ref.watch(workspaceModelProvider(machineId));
    final entries = [
      for (final pane in model.inbox)
        if (model.tabOfPane(pane.key) case final tab?) (tab, pane),
    ];
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: Text(l.inbox)),
      body: AnimatedSwitcher(
        duration: motion(context),
        child: entries.isEmpty
            ? Center(
                key: const ValueKey('empty'),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.inbox_outlined, size: 56, color: scheme.primary),
                    const SizedBox(height: 12),
                    Text(
                      l.inboxEmpty,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 4),
                    Text(
                      l.inboxEmptyHint,
                      style: TextStyle(color: scheme.onSurfaceVariant),
                    ),
                  ],
                ),
              )
            : ListView.separated(
                key: const ValueKey('list'),
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
                itemCount: entries.length,
                separatorBuilder: (_, _) => const SizedBox(height: 8),
                itemBuilder: (context, i) {
                  final (tab, pane) = entries[i];
                  return Appear(
                    key: ValueKey('inbox-${pane.key}'),
                    delay: i * 30,
                    child: _InboxTile(
                      machineId: machineId,
                      tab: tab,
                      pane: pane,
                    ),
                  );
                },
              ),
      ),
    );
  }
}

class _InboxTile extends StatelessWidget {
  const _InboxTile({
    required this.machineId,
    required this.tab,
    required this.pane,
  });

  final String machineId;
  final TabInfo tab;
  final PaneInfo pane;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l = context.l10n;
    final detail = pane.isAsked && pane.waitText != null
        ? pane.waitText!
        : (pane.isWaiting ? l.inboxWaiting : l.inboxDone);
    return Card(
      child: PressScale(
        onTap: () => openPane(context, machineId, tab, pane.key),
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Row(
            children: [
              AgentGlyph(agent: pane.agent, kind: pane.kind, size: 24),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      pane.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '${tab.displayName} · $detail',
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(color: scheme.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              StatusBadge(state: pane.rail),
            ],
          ),
        ),
      ),
    );
  }
}
