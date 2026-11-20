import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../kit/overlays.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../viewers/viewer_menu.dart';
import '../widgets/common.dart';
import '../workspace/pane_actions.dart';
import 'chat_glyph.dart';
import 'chat_nav.dart';

class SheetRow extends StatelessWidget {
  const SheetRow({
    super.key,
    required this.icon,
    required this.label,
    required this.onTap,
    this.sub,
    this.trailing,
    this.danger = false,
  });

  final Widget icon;
  final String label;
  final String? sub;
  final Widget? trailing;
  final VoidCallback? onTap;
  final bool danger;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final color = danger ? c.error : c.ink;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(18),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 54),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          child: Row(
            children: [
              IconTheme(
                data: IconThemeData(
                  color: danger ? c.error : c.inkMuted,
                  size: 22,
                ),
                child: icon,
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(label, style: TextStyle(fontSize: 16, color: color)),
                    if (sub != null)
                      Text(
                        sub!,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(fontSize: 13, color: c.inkMuted),
                      ),
                  ],
                ),
              ),
              ?trailing,
            ],
          ),
        ),
      ),
    );
  }
}

class SheetFrame extends StatelessWidget {
  const SheetFrame({super.key, required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(6, 4, 6, 8),
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 17,
                  fontWeight: FontWeight.w600,
                  color: c.ink,
                ),
              ),
            ),
            Flexible(
              child: SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: children,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

Future<void> showChatSessionSheet(
  BuildContext context,
  WidgetRef ref, {
  required String machineId,
  required WorkspaceModel model,
  required TabInfo tab,
  required PaneInfo pane,
  required bool termMode,
  VoidCallback? onToggleTerm,
}) async {
  final l = context.l10n;
  final root = tab.project ?? pane.cwd;
  final choice = await showOrrerySheet<String>(
    context,
    builder: (sheet) => SheetFrame(
      title: pane.title,
      children: [
        if (onToggleTerm != null)
          SheetRow(
            key: const ValueKey('chat-sheet-toggle'),
            icon: Icon(
              termMode
                  ? Icons.chat_bubble_outline_rounded
                  : Icons.terminal_rounded,
            ),
            label: termMode ? l.chatViewChat : l.chatViewTerminal,
            onTap: () => Navigator.pop(sheet, 'toggle'),
          ),
        if (root != null)
          SheetRow(
            key: const ValueKey('chat-sheet-tools'),
            icon: const Icon(Icons.folder_open_rounded),
            label: l.chatTools,
            sub: baseName(root),
            onTap: () => Navigator.pop(sheet, 'tools'),
          ),
        SheetRow(
          key: const ValueKey('chat-sheet-pane'),
          icon: const Icon(Icons.tune_rounded),
          label: l.chatPaneActions,
          onTap: () => Navigator.pop(sheet, 'pane'),
        ),
      ],
    ),
  );
  if (choice == null || !context.mounted) return;
  switch (choice) {
    case 'toggle':
      onToggleTerm?.call();
    case 'tools':
      await showViewerMenu(context, machineId, root!);
    case 'pane':
      await showTabActions(context, ref, machineId, model, tab, pane);
  }
}

Future<void> showChatPlusSheet(
  BuildContext context,
  WidgetRef ref, {
  required String machineId,
  required String? cwd,
  required VoidCallback onTerminal,
}) async {
  final l = context.l10n;
  final choice = await showOrrerySheet<String>(
    context,
    builder: (sheet) => SheetFrame(
      title: l.chatPlus,
      children: [
        if (cwd != null)
          SheetRow(
            key: const ValueKey('chat-plus-shell'),
            icon: const ChatGlyph(agent: 'shell'),
            label: l.chatNewShell,
            sub: baseName(cwd),
            onTap: () => Navigator.pop(sheet, 'shell'),
          ),
        SheetRow(
          key: const ValueKey('chat-plus-terminal'),
          icon: const Icon(Icons.terminal_rounded),
          label: l.chatViewTerminal,
          onTap: () => Navigator.pop(sheet, 'terminal'),
        ),
      ],
    ),
  );
  if (choice == null || !context.mounted) return;
  switch (choice) {
    case 'shell':
      await openNewSession(context, ref, machineId, preset: 'shell', cwd: cwd!);
    case 'terminal':
      onTerminal();
  }
}
