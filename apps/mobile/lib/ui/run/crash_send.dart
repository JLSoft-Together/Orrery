import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../chat/chat_nav.dart';
import '../commands.dart';
import '../model/workspace_model.dart';
import '../ui_providers.dart';
import '../viewers/viewer_api.dart' show samePath;
import '../widgets/common.dart';

class CrashDraft extends Notifier<Map<String, String>> {
  @override
  Map<String, String> build() => const {};

  void put(String paneKey, String text) => state = {...state, paneKey: text};

  String? take(String paneKey) {
    final text = state[paneKey];
    if (text == null) return null;
    state = {...state}..remove(paneKey);
    return text;
  }
}

final crashDraftProvider = NotifierProvider<CrashDraft, Map<String, String>>(
  CrashDraft.new,
);

List<(TabInfo, PaneInfo)> agentPanesFor(WorkspaceModel model, String cwd) => [
  for (final tab in model.tabs)
    for (final pane in tab.panes)
      if (pane.isAgent &&
          (cwd.isEmpty || pane.cwd == null || samePath(pane.cwd!, cwd)))
        (tab, pane),
];

Future<(TabInfo, PaneInfo)?> _pick(
  BuildContext context,
  List<(TabInfo, PaneInfo)> panes,
) {
  final l = context.l10n;
  return showModalBottomSheet<(TabInfo, PaneInfo)>(
    context: context,
    showDragHandle: true,
    builder: (sheet) => SafeArea(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 8),
            child: Align(
              alignment: Alignment.centerLeft,
              child: Text(
                l.runLogPickAgent,
                style: const TextStyle(fontWeight: FontWeight.w700),
              ),
            ),
          ),
          for (final item in panes)
            ListTile(
              key: ValueKey('crash-pane-${item.$2.key}'),
              leading: const Icon(Icons.smart_toy_outlined),
              title: Text(item.$2.title, maxLines: 1),
              subtitle: item.$2.branch == null ? null : Text(item.$2.branch!),
              onTap: () => Navigator.pop(sheet, item),
            ),
        ],
      ),
    ),
  );
}

Future<bool> sendCrashToAgent(
  BuildContext context,
  WidgetRef ref, {
  required String machineId,
  required String cwd,
  required String crash,
  void Function(TabInfo tab, PaneInfo pane)? open,
}) async {
  final l = context.l10n;
  final model = ref.read(workspaceModelProvider(machineId));
  final panes = agentPanesFor(model, cwd);
  if (panes.isEmpty) {
    toast(context, l.runLogNoAgent);
    return false;
  }
  final target = panes.length == 1 ? panes.first : await _pick(context, panes);
  if (target == null || !context.mounted) return false;
  final text = '${l.runLogCrashPrompt}\n\n```\n$crash\n```';
  ref.read(crashDraftProvider.notifier).put(target.$2.key, text);
  await Clipboard.setData(ClipboardData(text: text));
  if (!context.mounted) return false;
  if (open != null) {
    open(target.$1, target.$2);
  } else {
    openSession(context, ref, machineId, target.$1, target.$2);
  }
  toast(context, l.runLogCrashCopied);
  return true;
}
