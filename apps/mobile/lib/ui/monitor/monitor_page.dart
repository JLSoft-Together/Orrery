import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/providers.dart';
import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import '../workspace/session_rename.dart';
import 'monitor_cards.dart';
import 'monitor_filter.dart';
import 'monitor_model.dart';
import 'session_detail_page.dart';
import 'view_prefs.dart';

final monitorProvider = Provider.family<List<MonitorProject>, String>((
  ref,
  id,
) {
  final snapshot = ref.watch(machineViewProvider(id).select((v) => v.snapshot));
  return parseMonitor(snapshot);
});

final machineNameProvider = Provider.family<String, String>(
  (ref, id) => ref.watch(
    machinesProvider.select(
      (m) => m.where((x) => x.id == id).firstOrNull?.displayName ?? '',
    ),
  ),
);

class MonitorPage extends ConsumerStatefulWidget {
  const MonitorPage({super.key, required this.machineId, required this.onOpen});

  final String machineId;
  final ValueChanged<String> onOpen;

  @override
  ConsumerState<MonitorPage> createState() => _MonitorPageState();
}

class _MonitorPageState extends ConsumerState<MonitorPage> {
  MonitorFilter _filter = MonitorFilter.all;

  void _rename(MonitorSession s) =>
      renameSession(context, ref, widget.machineId, s.terminalId, s.alias);

  void _detail(MonitorSession s) => Navigator.of(context).push(
    machineRoute<void>(
      widget.machineId,
      (_) => SessionDetailPage(
        machineId: widget.machineId,
        terminalId: s.terminalId,
        onOpen: widget.onOpen,
      ),
    ),
  );

  String _filterLabel(MonitorFilter f, int n) {
    final l = context.l10n;
    return switch (f) {
      MonitorFilter.all => l.monFilterAll(n),
      MonitorFilter.running => l.monFilterRunning(n),
      MonitorFilter.waiting => l.monFilterWaiting(n),
      MonitorFilter.idle => l.monFilterIdle(n),
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final id = widget.machineId;
    final all = flattenSessions(ref.watch(monitorProvider(id)));
    final mode = ref.watch(viewPrefsProvider.select((p) => p.monitor));
    final alternate = mode == ViewMode.alternate;
    final link = ref.watch(linkProvider(id));
    final shown = [
      for (final s in all)
        if (matchesFilter(_filter, s)) s,
    ];
    return Scaffold(
      backgroundColor: c.canvas,
      appBar: AppBar(
        title: Text(
          ref.watch(machineNameProvider(id)),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        actions: [
          Center(
            child: LinkPill(
              link: link,
              latencyMs: ref.watch(latencyProvider(id)).value,
            ),
          ),
          ViewModeButton(
            key: const ValueKey('monitor-view-mode'),
            alternate: alternate,
            primaryIcon: Icons.view_agenda_outlined,
            alternateIcon: Icons.view_list_rounded,
            tooltip: alternate ? l.viewModeToCards : l.viewModeToCompact,
            onPressed: () =>
                ref.read(viewPrefsProvider.notifier).toggleMonitor(),
          ),
          const SizedBox(width: 4),
        ],
      ),
      body: all.isEmpty
          ? EmptyState(title: l.vMonitorEmpty)
          : Column(
              children: [
                QuietSize(
                  alignment: Alignment.topCenter,
                  child: alternate
                      ? const SizedBox(width: double.infinity)
                      : _Filters(
                          all: all,
                          selected: _filter,
                          label: _filterLabel,
                          onSelect: (f) => setState(() => _filter = f),
                        ),
                ),
                Expanded(
                  child: AnimatedSwitcher(
                    duration: motion(context),
                    switchInCurve: context.tokens.out,
                    transitionBuilder: (child, animation) => FadeTransition(
                      opacity: animation,
                      child: ScaleTransition(
                        scale: Tween<double>(
                          begin: 0.98,
                          end: 1,
                        ).animate(animation),
                        child: child,
                      ),
                    ),
                    child: ListView(
                      key: ValueKey(alternate),
                      padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
                      children: [
                        if (alternate)
                          _Grouped(
                            sessions: all,
                            onTap: _detail,
                            onLongPress: _rename,
                          )
                        else if (shown.isEmpty)
                          Padding(
                            padding: const EdgeInsets.only(top: 48),
                            child: Text(
                              l.monFilterEmpty,
                              textAlign: TextAlign.center,
                              style: TextStyle(color: c.inkMuted),
                            ),
                          )
                        else
                          AnimatedItems<MonitorSession>(
                            items: shown,
                            keyOf: (s) => s.terminalId,
                            builder: (context, s) => SessionCard(
                              session: s,
                              onTap: () => _detail(s),
                              onLongPress: () => _rename(s),
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
    );
  }
}

class _Filters extends StatelessWidget {
  const _Filters({
    required this.all,
    required this.selected,
    required this.label,
    required this.onSelect,
  });

  final List<MonitorSession> all;
  final MonitorFilter selected;
  final String Function(MonitorFilter, int) label;
  final ValueChanged<MonitorFilter> onSelect;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 52,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 6, 16, 6),
        itemCount: MonitorFilter.values.length,
        separatorBuilder: (_, _) => const SizedBox(width: 8),
        itemBuilder: (context, i) {
          final f = MonitorFilter.values[i];
          final n = all.where((s) => matchesFilter(f, s)).length;
          final on = f == selected;
          return OrreryFilterChip(
            key: ValueKey('mon-filter-${f.name}'),
            label: label(f, n),
            selected: on,
            onTap: () => onSelect(f),
          );
        },
      ),
    );
  }
}

class _Grouped extends StatelessWidget {
  const _Grouped({
    required this.sessions,
    required this.onTap,
    required this.onLongPress,
  });

  final List<MonitorSession> sessions;
  final ValueChanged<MonitorSession> onTap;
  final ValueChanged<MonitorSession> onLongPress;

  @override
  Widget build(BuildContext context) {
    final groups = groupByState(sessions);
    return AnimatedItems<(RailState, List<MonitorSession>)>(
      items: groups,
      keyOf: (g) => g.$1,
      gap: 0,
      builder: (context, g) {
        final look = railLook(context, g.$1);
        final tint = look.tone.resolve(context);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(4, 16, 4, 8),
              child: Row(
                children: [
                  Icon(look.icon, size: 16, color: tint),
                  const SizedBox(width: 6),
                  Text(
                    '${look.label} · ${g.$2.length}'.toUpperCase(),
                    style: TextStyle(
                      fontSize: 12,
                      height: 16 / 12,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 0.4,
                      color: tint,
                    ),
                  ),
                ],
              ),
            ),
            OrreryCard(
              padding: EdgeInsets.zero,
              child: AnimatedItems<MonitorSession>(
                items: g.$2,
                keyOf: (s) => s.terminalId,
                gap: 0,
                builder: (context, s) => SessionRow(
                  session: s,
                  first: g.$2.first == s,
                  onTap: () => onTap(s),
                  onLongPress: () => onLongPress(s),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}
