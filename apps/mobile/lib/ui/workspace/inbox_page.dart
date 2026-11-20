import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../model/inbox_model.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'workspace_page.dart';

String inboxGroupLabel(AppLocalizations l, String group) => switch (group) {
  'menu' => l.inboxGroupMenu,
  'permission' => l.inboxGroupPermission,
  'question' => l.inboxGroupQuestion,
  'clash' => l.inboxGroupClash,
  'error' => l.inboxGroupError,
  'note' => l.inboxGroupNote,
  _ => l.inboxGroupDone,
};

String inboxReasonLabel(AppLocalizations l, InboxRow row) {
  final text = row.text;
  if (text != null) return waitLabel(l, text);
  return switch (row.reason) {
    'menu' => l.inboxReasonMenu,
    'permission' => l.inboxReasonPermission,
    'question' => l.inboxReasonQuestion,
    'clash' => l.inboxReasonClash(row.file ?? '?', row.peer ?? '?'),
    'failed' => l.inboxReasonFailed,
    'loop' => l.inboxReasonLoop,
    'checks' => l.inboxReasonChecks,
    'stuck' => l.inboxReasonStuck,
    'note' => l.inboxReasonNote,
    _ => l.inboxReasonDone,
  };
}

class InboxPage extends ConsumerWidget {
  const InboxPage({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final model = ref.watch(workspaceModelProvider(machineId));
    final groups = groupInboxRows(model.inboxRows);
    final scheme = Theme.of(context).colorScheme;
    final items = <Object>[
      for (final (group, rows) in groups) ...[group, ...rows],
    ];
    return Scaffold(
      appBar: AppBar(title: Text(l.inbox)),
      body: AnimatedSwitcher(
        duration: motion(context),
        child: items.isEmpty
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
            : ListView.builder(
                key: const ValueKey('list'),
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
                itemCount: items.length,
                itemBuilder: (context, i) {
                  final item = items[i];
                  if (item is String) {
                    return _GroupHeader(group: item);
                  }
                  final row = item as InboxRow;
                  return Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Appear(
                      key: ValueKey('inbox-${row.pane.key}'),
                      delay: i * 30,
                      child: _InboxTile(machineId: machineId, row: row),
                    ),
                  );
                },
              ),
      ),
    );
  }
}

class _GroupHeader extends StatelessWidget {
  const _GroupHeader({required this.group});

  final String group;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      key: ValueKey('inbox-group-$group'),
      padding: const EdgeInsets.fromLTRB(4, 12, 4, 6),
      child: Text(
        inboxGroupLabel(context.l10n, group),
        style: TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w700,
          color: scheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _InboxTile extends StatelessWidget {
  const _InboxTile({required this.machineId, required this.row});

  final String machineId;
  final InboxRow row;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l = context.l10n;
    final pane = row.pane;
    final since = row.since;
    final age = since == null
        ? ''
        : ', ${shortElapsed(l, DateTime.now().difference(since))}';
    return Card(
      child: PressScale(
        onTap: () => openPane(context, machineId, row.tab, pane.key),
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
                      '${row.tab.displayName}, ${inboxReasonLabel(l, row)}$age',
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
