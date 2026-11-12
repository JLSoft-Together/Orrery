import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../commands.dart';
import '../model/workspace_model.dart';
import '../monitor/monitor_page.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'inbox_page.dart';
import 'machine_scope.dart';
import 'settings_body.dart';
import 'shell_nav.dart';
import 'tab_page.dart';
import 'workspace_body.dart';

class WorkspacePage extends StatelessWidget {
  const WorkspacePage({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context) =>
      MachineScope(machineId: machineId, child: _Shell(machineId));
}

class _Shell extends ConsumerStatefulWidget {
  const _Shell(this.machineId);

  final String machineId;

  @override
  ConsumerState<_Shell> createState() => _ShellState();
}

class _ShellState extends ConsumerState<_Shell> {
  int _index = 0;

  void _openTerminal(String terminalId) {
    final id = widget.machineId;
    final model = ref.read(workspaceModelProvider(id));
    final pane = model.panes
        .where((p) => p.terminalId == terminalId)
        .firstOrNull;
    final tab = pane == null ? null : model.tabOfPane(pane.key);
    if (pane == null || tab == null) {
      toast(context, context.l10n.vNotInWorkspace);
      return;
    }
    openPane(context, id, tab, pane.key);
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final id = widget.machineId;
    final model = ref.watch(workspaceModelProvider(id));
    final inbox = model.inbox.length;
    final needs = model.panes.where((p) => p.needsUser).length;
    return Scaffold(
      body: IndexedStack(
        index: _index,
        children: [
          WorkspaceBody(id),
          MonitorPage(machineId: id, onOpen: _openTerminal),
          InboxPage(machineId: id),
          SettingsBody(machineId: id),
        ],
      ),
      bottomNavigationBar: ShellNav(
        index: _index,
        onSelect: (i) => setState(() => _index = i),
        items: [
          NavItem(
            id: 'workspace',
            icon: Icons.space_dashboard_outlined,
            selectedIcon: Icons.space_dashboard_rounded,
            label: l.navWorkspace,
          ),
          NavItem(
            id: 'monitor',
            icon: Icons.public_outlined,
            selectedIcon: Icons.public_rounded,
            label: l.vMonitor,
            badge: needs,
          ),
          NavItem(
            id: 'inbox',
            icon: Icons.inbox_outlined,
            selectedIcon: Icons.inbox_rounded,
            label: l.inbox,
            badge: inbox,
          ),
          NavItem(
            id: 'settings',
            icon: Icons.tune_rounded,
            selectedIcon: Icons.tune_rounded,
            label: l.navSettings,
          ),
        ],
      ),
    );
  }
}

void openPane(
  BuildContext context,
  String machineId,
  TabInfo tab,
  String? paneKey,
) {
  Navigator.of(context).push(
    machineRoute<void>(
      machineId,
      (_) => TabPage(machineId: machineId, tabKey: tab.key, paneKey: paneKey),
    ),
  );
}
