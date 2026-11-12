import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../kit/kit.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import '../workspace/session_rename.dart';
import 'monitor_model.dart';
import 'monitor_page.dart';
import 'session_activity.dart';
import 'session_detail_parts.dart';

class SessionDetailPage extends ConsumerWidget {
  const SessionDetailPage({
    super.key,
    required this.machineId,
    required this.terminalId,
    required this.onOpen,
  });

  final String machineId;
  final String terminalId;
  final ValueChanged<String> onOpen;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final c = context.orrery;
    MonitorSession? session;
    String? project;
    for (final p in ref.watch(monitorProvider(machineId))) {
      for (final s in p.sessions) {
        if (s.terminalId == terminalId) {
          session = s;
          project = p.name;
        }
      }
    }
    final s = session;
    return Scaffold(
      backgroundColor: c.canvas,
      appBar: AppBar(
        title: s == null
            ? const SizedBox.shrink()
            : AnimatedRename(text: s.title),
        actions: [
          if (s != null)
            PopupMenuButton<String>(
              key: ValueKey('mon-menu-$terminalId'),
              tooltip: l.more,
              onSelected: (_) =>
                  renameSession(context, ref, machineId, terminalId, s.alias),
              itemBuilder: (_) => [
                PopupMenuItem<String>(
                  key: const ValueKey('mon-rename-session'),
                  value: 'rename',
                  child: Text(l.renameSession),
                ),
              ],
            ),
        ],
      ),
      body: s == null
          ? EmptyState(title: l.vMonitorEmpty)
          : Column(
              children: [
                Expanded(
                  child: ListView(
                    padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
                    children: [
                      SessionHead(
                        session: s,
                        project: project ?? '',
                        machine: ref.watch(machineNameProvider(machineId)),
                      ),
                      SessionFacts(session: s),
                      SectionLabel(
                        l.detailSubagents,
                        count: s.subagents.length,
                      ),
                      SessionSubagents(session: s),
                      SectionLabel(l.actTitle),
                      SessionActivity(
                        machineId: machineId,
                        terminalId: terminalId,
                      ),
                    ],
                  ),
                ),
                SafeArea(
                  top: false,
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
                    child: OrreryButton(
                      key: const ValueKey('detail-open-terminal'),
                      label: l.detailOpenTerminal,
                      icon: Icons.terminal_rounded,
                      onPressed: () => onOpen(terminalId),
                    ),
                  ),
                ),
              ],
            ),
    );
  }
}
