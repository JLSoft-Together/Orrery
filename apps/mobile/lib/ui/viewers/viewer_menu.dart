import 'package:flutter/material.dart';

import '../model/workspace_model.dart' show baseName;
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import 'changes_page.dart';
import 'explorer_page.dart';
import 'log_page.dart';
import 'run_page.dart';

Future<void> showViewerMenu(
  BuildContext context,
  String machineId,
  String root,
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
              baseName(root),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            subtitle: Text(root, maxLines: 1, overflow: TextOverflow.ellipsis),
          ),
          const Divider(height: 1),
          ListTile(
            key: const ValueKey('viewer-explorer'),
            leading: const Icon(Icons.folder_open_rounded),
            title: Text(l.vExplorer),
            onTap: () => Navigator.pop(sheet, 'explorer'),
          ),
          ListTile(
            key: const ValueKey('viewer-changes'),
            leading: const Icon(Icons.difference_outlined),
            title: Text(l.vChanges),
            onTap: () => Navigator.pop(sheet, 'changes'),
          ),
          ListTile(
            key: const ValueKey('viewer-log'),
            leading: const Icon(Icons.history_rounded),
            title: Text(l.vGitLog),
            onTap: () => Navigator.pop(sheet, 'log'),
          ),
          ListTile(
            key: const ValueKey('viewer-run'),
            leading: const Icon(Icons.play_circle_outline_rounded),
            title: Text(l.vRun),
            onTap: () => Navigator.pop(sheet, 'run'),
          ),
        ],
      ),
    ),
  );
  if (choice == null || !context.mounted) return;
  final page = switch (choice) {
    'explorer' => ExplorerPage(machineId: machineId, root: root),
    'changes' => ChangesPage(machineId: machineId, root: root),
    'log' => LogPage(machineId: machineId, root: root),
    _ => RunPage(machineId: machineId, root: root),
  };
  await Navigator.of(context).push(machineRoute<void>(machineId, (_) => page));
}
