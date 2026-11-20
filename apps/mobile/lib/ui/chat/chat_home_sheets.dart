import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/providers.dart';
import '../kit/overlays.dart';
import '../model/workspace_model.dart';
import '../monitor/monitor_page.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../viewers/viewer_menu.dart';
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import '../workspace/open_pane_page.dart';
import '../workspace/settings_body.dart';
import '../workspace/workspace_page.dart';
import 'chat_glyph.dart';
import 'chat_nav.dart';
import 'chat_rows.dart';
import 'chat_sheets.dart';

String? recentProject(WidgetRef ref, String machineId) {
  final model = ref.read(workspaceModelProvider(machineId));
  final view = ref.read(machineViewProvider(machineId));
  final rows = buildChatRows(model, view.snapshot, view.chatLast);
  final sorted = groupChatRows(rows);
  for (final row in [...sorted.waiting, ...sorted.rest]) {
    final dir = row.tab.project ?? row.pane.cwd;
    if (dir != null && dir.isNotEmpty) return dir;
  }
  final choices = projectChoices(view.snapshot, model);
  return choices.isEmpty ? null : choices.first.dir;
}

Future<void> newShell(
  BuildContext context,
  WidgetRef ref,
  String machineId,
) async {
  final dir = recentProject(ref, machineId);
  if (dir == null) {
    await showNewSessionSheet(context, ref, machineId);
    return;
  }
  await openNewSession(context, ref, machineId, preset: 'shell', cwd: dir);
}

void _push(BuildContext context, String machineId, WidgetBuilder builder) {
  Navigator.of(context).push(machineRoute<void>(machineId, builder));
}

void _openTerminalById(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  String terminalId,
) {
  final model = ref.read(workspaceModelProvider(machineId));
  for (final tab in model.tabs) {
    for (final pane in tab.panes) {
      if (pane.terminalId == terminalId) {
        openSession(context, ref, machineId, tab, pane);
        return;
      }
    }
  }
}

Future<void> showHomeMoreSheet(
  BuildContext context,
  WidgetRef ref,
  String machineId,
) async {
  final l = context.l10n;
  final root = recentProject(ref, machineId);
  final chevron = Icon(
    Icons.chevron_right_rounded,
    color: context.orrery.inkMuted,
  );
  final choice = await showOrrerySheet<String>(
    context,
    builder: (sheet) => SheetFrame(
      title: l.chatMoreTitle,
      children: [
        SheetRow(
          key: const ValueKey('home-more-workspace'),
          icon: const Icon(Icons.space_dashboard_outlined),
          label: l.navWorkspace,
          trailing: chevron,
          onTap: () => Navigator.pop(sheet, 'workspace'),
        ),
        SheetRow(
          key: const ValueKey('home-more-monitor'),
          icon: const Icon(Icons.public_outlined),
          label: l.vMonitor,
          trailing: chevron,
          onTap: () => Navigator.pop(sheet, 'monitor'),
        ),
        if (root != null)
          SheetRow(
            key: const ValueKey('home-more-tools'),
            icon: const Icon(Icons.folder_open_rounded),
            label: l.chatTools,
            sub: baseName(root),
            trailing: chevron,
            onTap: () => Navigator.pop(sheet, 'tools'),
          ),
        SheetRow(
          key: const ValueKey('home-more-settings'),
          icon: const Icon(Icons.tune_rounded),
          label: l.navSettings,
          trailing: chevron,
          onTap: () => Navigator.pop(sheet, 'settings'),
        ),
      ],
    ),
  );
  if (choice == null || !context.mounted) return;
  switch (choice) {
    case 'workspace':
      _push(context, machineId, (_) => WorkspaceShell(machineId: machineId));
    case 'monitor':
      _push(
        context,
        machineId,
        (inner) => MonitorPage(
          machineId: machineId,
          onOpen: (terminalId) =>
              _openTerminalById(inner, ref, machineId, terminalId),
        ),
      );
    case 'tools':
      await showViewerMenu(context, machineId, root!);
    case 'settings':
      _push(context, machineId, (_) => SettingsBody(machineId: machineId));
  }
}

Future<void> showNewSessionSheet(
  BuildContext context,
  WidgetRef ref,
  String machineId,
) async {
  final model = ref.read(workspaceModelProvider(machineId));
  final snapshot = ref.read(machineViewProvider(machineId)).snapshot;
  final choices = projectChoices(snapshot, model);
  final picked = await showOrrerySheet<(String, String)>(
    context,
    builder: (sheet) => _NewSessionBody(
      choices: choices,
      initial: recentProject(ref, machineId),
      onPick: (preset, dir) => Navigator.pop(sheet, (preset, dir)),
      onAdvanced: () => Navigator.pop(sheet, ('advanced', '')),
    ),
  );
  if (picked == null || !context.mounted) return;
  final (preset, dir) = picked;
  if (preset == 'advanced') {
    _push(context, machineId, (_) => OpenPanePage(machineId: machineId));
    return;
  }
  await openNewSession(context, ref, machineId, preset: preset, cwd: dir);
}

class _NewSessionBody extends StatefulWidget {
  const _NewSessionBody({
    required this.choices,
    required this.initial,
    required this.onPick,
    required this.onAdvanced,
  });

  final List<ProjectChoice> choices;
  final String? initial;
  final void Function(String preset, String dir) onPick;
  final VoidCallback onAdvanced;

  @override
  State<_NewSessionBody> createState() => _NewSessionBodyState();
}

class _NewSessionBodyState extends State<_NewSessionBody> {
  late String? _dir = widget.initial ?? widget.choices.firstOrNull?.dir;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final dir = _dir;
    Widget option(String preset, String label) => Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Material(
        color: c.raised,
        borderRadius: BorderRadius.circular(22),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          key: ValueKey('new-$preset'),
          onTap: dir == null ? null : () => widget.onPick(preset, dir),
          child: SizedBox(
            height: 60,
            child: Row(
              children: [
                const SizedBox(width: 14),
                ChatGlyph(agent: preset, size: 36, avatar: true),
                const SizedBox(width: 14),
                Text(
                  label,
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w500,
                    color: dir == null ? c.inkFaint : c.ink,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    return SheetFrame(
      title: l.chatNewSession,
      children: [
        option('claude', 'Claude'),
        option('codex', 'Codex'),
        option('shell', l.chatNewShell),
        Padding(
          padding: const EdgeInsets.fromLTRB(6, 10, 6, 6),
          child: Text(
            l.chatWhere,
            style: TextStyle(fontSize: 14, color: c.inkMuted),
          ),
        ),
        if (widget.choices.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 6),
            child: Text(l.noProjects, style: TextStyle(color: c.inkMuted)),
          ),
        Wrap(
          spacing: 6,
          runSpacing: 6,
          children: [
            for (final choice in widget.choices)
              ChoiceChip(
                key: ValueKey('new-project-${choice.dir}'),
                label: Text(choice.label),
                selected: choice.dir == dir,
                showCheckmark: false,
                avatar: Icon(
                  choice.favorite ? Icons.star_rounded : Icons.folder_outlined,
                  size: 16,
                ),
                shape: const StadiumBorder(),
                materialTapTargetSize: MaterialTapTargetSize.padded,
                onSelected: (_) => setState(() => _dir = choice.dir),
              ),
          ],
        ),
        const SizedBox(height: 8),
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton(
            key: const ValueKey('new-advanced'),
            onPressed: widget.onAdvanced,
            child: Text(l.chatNewAdvanced),
          ),
        ),
      ],
    );
  }
}
