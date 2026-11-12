import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../commands.dart';
import '../model/workspace_model.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import '../widgets/name_dialog.dart';
import 'session_rename.dart';

Future<void> showTabActions(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  WorkspaceModel model,
  TabInfo tab,
  PaneInfo pane,
) async {
  final l = context.l10n;
  final choice = await showModalBottomSheet<String>(
    context: context,
    builder: (sheet) => SafeArea(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          ListTile(
            title: Text(
              pane.title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            subtitle: Text(tab.displayName),
          ),
          const Divider(),
          ListTile(
            key: const ValueKey('action-rename'),
            leading: const Icon(Icons.edit_outlined),
            title: Text(l.renameTab),
            onTap: () => Navigator.pop(sheet, 'rename'),
          ),
          if (pane.isTerm && pane.terminalId != null)
            ListTile(
              key: const ValueKey('action-rename-session'),
              leading: const Icon(Icons.drive_file_rename_outline_rounded),
              title: Text(l.renameSession),
              onTap: () => Navigator.pop(sheet, 'rename-session'),
            ),
          ListTile(
            key: const ValueKey('action-move'),
            leading: const Icon(Icons.drive_file_move_outline),
            title: Text(l.movePane),
            enabled: model.tabs.length > 1,
            onTap: () => Navigator.pop(sheet, 'move'),
          ),
          ListTile(
            key: const ValueKey('action-reopen'),
            leading: const Icon(Icons.undo_rounded),
            title: Text(l.reopenPane(model.closedCount)),
            enabled: model.closedCount > 0,
            onTap: () => Navigator.pop(sheet, 'reopen'),
          ),
          if (pane.isTerm && pane.agent != null)
            ListTile(
              key: const ValueKey('action-compact'),
              leading: const Icon(Icons.compress_rounded),
              title: Text(l.compact),
              onTap: () => Navigator.pop(sheet, 'compact'),
            ),
          ListTile(
            key: const ValueKey('action-close'),
            leading: Icon(
              Icons.close_rounded,
              color: Theme.of(sheet).colorScheme.error,
            ),
            title: Text(
              l.closePane,
              style: TextStyle(color: Theme.of(sheet).colorScheme.error),
            ),
            onTap: () => Navigator.pop(sheet, 'close'),
          ),
        ],
      ),
    ),
  );
  if (choice == null || !context.mounted) return;
  switch (choice) {
    case 'rename':
      await _rename(context, ref, machineId, tab);
    case 'rename-session':
      await renameSession(
        context,
        ref,
        machineId,
        pane.terminalId!,
        ref.read(paneFactsProvider((machineId, pane.terminalId))).alias,
      );
    case 'move':
      await _move(context, ref, machineId, model, tab, pane);
    case 'reopen':
      await runCommand(context, ref, machineId, 'ws.reopen');
    case 'compact':
      await _compact(context, ref, machineId, pane);
    case 'close':
      await _close(context, ref, machineId, pane);
  }
}

Future<void> _rename(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  TabInfo tab,
) async {
  final l = context.l10n;
  final name = await showNameDialog(
    context,
    title: l.renameTab,
    initial: tab.name ?? '',
    maxLength: 100,
    hint: l.renameHint,
  );
  if (name == null || !context.mounted) return;
  await runCommand(context, ref, machineId, 'ws.rename', {
    'tabKey': tab.key,
    'name': name.isEmpty ? null : name,
  });
}

Future<void> _move(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  WorkspaceModel model,
  TabInfo tab,
  PaneInfo pane,
) async {
  final l = context.l10n;
  final others = model.tabs.where((t) => t.key != tab.key).toList();
  final target = await showModalBottomSheet<String>(
    context: context,
    builder: (sheet) => SafeArea(
      child: ListView(
        shrinkWrap: true,
        children: [
          ListTile(title: Text(l.movePane)),
          for (final t in others)
            ListTile(
              leading: const Icon(Icons.folder_outlined),
              title: Text(t.displayName),
              onTap: () => Navigator.pop(sheet, t.key),
            ),
        ],
      ),
    ),
  );
  if (target == null || !context.mounted) return;
  await runCommand(context, ref, machineId, 'ws.move', {
    'paneKey': pane.key,
    'tabKey': target,
  });
}

Future<void> _compact(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  PaneInfo pane,
) async {
  final l = context.l10n;
  final ok = await showDialog<bool>(
    context: context,
    builder: (dialog) => AlertDialog(
      title: Text(l.compactTitle),
      content: Text(l.compactBody),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(dialog, false),
          child: Text(l.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(dialog, true),
          child: Text(l.compact),
        ),
      ],
    ),
  );
  if (ok != true || !context.mounted) return;
  await runCommand(context, ref, machineId, 'ws.compact', {
    'paneKey': pane.key,
  });
}

Future<void> _close(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  PaneInfo pane,
) async {
  final l = context.l10n;
  final ok = await showDialog<bool>(
    context: context,
    builder: (dialog) => AlertDialog(
      title: Text(l.closePaneTitle(pane.title)),
      content: Text(l.closePaneBody),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(dialog, false),
          child: Text(l.cancel),
        ),
        FilledButton(
          style: FilledButton.styleFrom(
            backgroundColor: Theme.of(dialog).colorScheme.error,
            foregroundColor: Theme.of(dialog).colorScheme.onError,
          ),
          onPressed: () => Navigator.pop(dialog, true),
          child: Text(l.closePane),
        ),
      ],
    ),
  );
  if (ok != true || !context.mounted) return;
  await runCommand(context, ref, machineId, 'ws.close', {'paneKey': pane.key});
}
