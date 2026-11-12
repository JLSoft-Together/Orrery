import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/providers.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_theme.dart';
import '../viewers/async_panel.dart';
import '../widgets/common.dart';
import 'monitor_model.dart';

final monitorProvider = Provider.family<List<MonitorProject>, String>((
  ref,
  id,
) {
  final snapshot = ref.watch(machineViewProvider(id).select((v) => v.snapshot));
  return parseMonitor(snapshot);
});

class MonitorPage extends ConsumerWidget {
  const MonitorPage({super.key, required this.machineId, required this.onOpen});

  final String machineId;
  final ValueChanged<String> onOpen;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final projects = ref.watch(monitorProvider(machineId));
    return Scaffold(
      appBar: AppBar(title: Text(l.vMonitor)),
      body: projects.isEmpty
          ? EmptyState(icon: Icons.public_rounded, text: l.vMonitorEmpty)
          : ListView.builder(
              key: const ValueKey('monitor-list'),
              padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
              itemCount: projects.length,
              itemBuilder: (context, index) => Appear(
                key: ValueKey('mp-${projects[index].cwd}'),
                delay: index * 30,
                child: _ProjectBlock(project: projects[index], onOpen: onOpen),
              ),
            ),
    );
  }
}

class _ProjectBlock extends StatelessWidget {
  const _ProjectBlock({required this.project, required this.onOpen});

  final MonitorProject project;
  final ValueChanged<String> onOpen;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(4, 0, 4, 6),
            child: Row(
              children: [
                Icon(Icons.folder_outlined, size: 18, color: scheme.primary),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    project.name.isEmpty ? l.otherProject : project.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                Text(
                  l.panesCount(project.sessions.length),
                  style: TextStyle(
                    color: scheme.onSurfaceVariant,
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
          for (final s in project.sessions)
            _SessionCard(session: s, onTap: () => onOpen(s.terminalId)),
        ],
      ),
    );
  }
}

String _compact(int n) {
  if (n >= 1000000) return '${(n / 1000000).toStringAsFixed(1)}M';
  if (n >= 1000) return '${(n / 1000).toStringAsFixed(1)}k';
  return '$n';
}

class _SessionCard extends StatelessWidget {
  const _SessionCard({required this.session, required this.onTap});

  final MonitorSession session;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final meta = [
      ?session.model,
      ?session.branch,
      if (session.totalTokens != null)
        l.vTokens(_compact(session.totalTokens!)),
    ];
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Material(
        color: scheme.surface,
        borderRadius: BorderRadius.circular(14),
        child: InkWell(
          key: ValueKey('mon-${session.terminalId}'),
          borderRadius: BorderRadius.circular(14),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    AgentGlyph(agent: session.provider),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        session.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          fontSize: 15,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    StatusBadge(state: session.rail, compact: true),
                  ],
                ),
                if (session.tool != null)
                  Padding(
                    padding: const EdgeInsets.only(top: 6),
                    child: Row(
                      children: [
                        Icon(
                          Icons.build_outlined,
                          size: 14,
                          color: scheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            [session.tool!, ?session.toolSummary].join(' · '),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              fontSize: 12.5,
                              color: scheme.onSurfaceVariant,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                if (session.waitingFor != null &&
                    session.rail == RailState.asked)
                  Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Text(
                      session.waitingFor!,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12.5,
                        color: context.statusColors.asked,
                      ),
                    ),
                  ),
                if (meta.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Text(
                      meta.join(' · '),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                if (session.contextPct != null)
                  _ContextBar(percent: session.contextPct!),
                for (final a in session.subagents) _AgentRow(agent: a),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ContextBar extends StatelessWidget {
  const _ContextBar({required this.percent});

  final int percent;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    final color = percent >= 85
        ? colors.failed
        : percent >= 65
        ? colors.warning
        : colors.working;
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Row(
        children: [
          Expanded(
            child: ClipRRect(
              borderRadius: BorderRadius.circular(3),
              child: LinearProgressIndicator(
                value: percent / 100,
                minHeight: 5,
                color: color,
                backgroundColor: scheme.surfaceContainerHighest,
              ),
            ),
          ),
          const SizedBox(width: 8),
          Text(
            context.l10n.vContextPct(percent),
            style: TextStyle(fontSize: 11.5, color: scheme.onSurfaceVariant),
          ),
        ],
      ),
    );
  }
}

class _AgentRow extends StatelessWidget {
  const _AgentRow({required this.agent});

  final MonitorAgent agent;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final style = statusStyle(context, agent.rail);
    final detail = [
      ?agent.tool,
      ?agent.toolSummary,
      if (agent.contextPct != null) context.l10n.vContextPct(agent.contextPct!),
      if (agent.totalTokens != null)
        context.l10n.vTokens(_compact(agent.totalTokens!)),
    ].join(' · ');
    return Padding(
      padding: EdgeInsets.only(left: agent.depth * 16.0, top: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            Icons.subdirectory_arrow_right_rounded,
            size: 16,
            color: scheme.outline,
          ),
          const SizedBox(width: 4),
          Icon(style.icon, size: 15, color: style.color),
          const SizedBox(width: 6),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  agent.name ?? agent.id,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    fontSize: 13.5,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                if (detail.isNotEmpty)
                  Text(
                    detail,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 11.5,
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
