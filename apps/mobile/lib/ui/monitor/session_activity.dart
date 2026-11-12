import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../../remote/providers.dart';
import '../kit/kit.dart';
import '../theme/orrery_shape.dart';
import '../viewers/viewer_models.dart';
import '../widgets/common.dart';
import 'monitor_cards.dart';

const int activityShown = 12;

class ActivityItem {
  const ActivityItem({
    required this.id,
    required this.kind,
    required this.at,
    this.detail,
  });

  final String id;
  final String kind;
  final DateTime at;
  final String? detail;
}

String? _detail(String kind, Map<String, dynamic> data) {
  final parts = switch (kind) {
    'tool.started' || 'tool.failed' => [str(data['tool']), str(data['error'])],
    'agent.started' ||
    'agent.completed' ||
    'agent.failed' ||
    'agent.cancelled' => [
      str(data['agentType']),
      str(data['description']),
      str(data['error']),
    ],
    'terminal.renamed' => [str(data['to']) ?? str(data['title'])],
    'terminal.status' => [str(data['to']), str(data['waitingFor'])],
    _ => [str(data['title']), str(data['description']), str(data['error'])],
  };
  final text = parts.whereType<String>().where((s) => s.isNotEmpty).join(' · ');
  if (text.isEmpty) return null;
  final line = text.replaceAll(RegExp(r'\s+'), ' ');
  return line.length > 160 ? '${line.substring(0, 160)}…' : line;
}

List<ActivityItem> parseActivity(Object? raw, String terminalId) {
  final out = <ActivityItem>[];
  final list = raw is List ? raw : const [];
  for (var i = list.length - 1; i >= 0 && out.length < activityShown; i--) {
    final e = list[i];
    if (e is! Map || e['terminalId'] != terminalId) continue;
    final at = DateTime.tryParse(str(e['at']) ?? '');
    final kind = str(e['kind']);
    if (at == null || kind == null) continue;
    final data = e['data'];
    out.add(
      ActivityItem(
        id: str(e['id']) ?? '$i',
        kind: kind,
        at: at.toLocal(),
        detail: data is Map
            ? _detail(kind, data.cast<String, dynamic>())
            : null,
      ),
    );
  }
  return out;
}

final sessionActivityProvider =
    Provider.family<List<ActivityItem>, (String, String)>((ref, args) {
      final activity = ref.watch(
        machineViewProvider(args.$1).select((v) => v.snapshot?['activity']),
      );
      return parseActivity(activity, args.$2);
    });

(IconData, String) _look(AppLocalizations l, String kind) => switch (kind) {
  'terminal.started' => (Icons.play_arrow_rounded, l.actStarted),
  'terminal.ended' => (Icons.stop_rounded, l.actEnded),
  'terminal.renamed' => (Icons.edit_rounded, l.actRenamed),
  'terminal.status' => (Icons.swap_horiz_rounded, l.actStatus),
  'session.cleared' => (Icons.cleaning_services_rounded, l.actCleared),
  'terminal.compacted' => (Icons.compress_rounded, l.actCompacted),
  'agent.started' => (Icons.account_tree_outlined, l.actAgentStarted),
  'agent.completed' => (Icons.check_circle_outline_rounded, l.actAgentDone),
  'agent.failed' => (Icons.error_outline_rounded, l.actAgentFailed),
  'agent.cancelled' => (Icons.cancel_outlined, l.actAgentCancelled),
  'tool.started' => (Icons.build_outlined, l.actTool),
  'tool.failed' => (Icons.error_outline_rounded, l.actToolFailed),
  'turn.started' => (Icons.play_arrow_rounded, l.actTurnStarted),
  'turn.completed' => (Icons.check_circle_outline_rounded, l.actTurnDone),
  'turn.failed' => (Icons.error_outline_rounded, l.actTurnFailed),
  'turn.interrupted' => (Icons.pause_circle_outline_rounded, l.actTurnStopped),
  'sensitive' => (Icons.shield_outlined, l.actSensitive),
  _ => (Icons.circle_outlined, kind),
};

bool _bad(String kind) =>
    kind.endsWith('.failed') || kind == 'sensitive' || kind == 'launch.failed';

class SessionActivity extends ConsumerWidget {
  const SessionActivity({
    super.key,
    required this.machineId,
    required this.terminalId,
  });

  final String machineId;
  final String terminalId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final c = context.orrery;
    final items = ref.watch(sessionActivityProvider((machineId, terminalId)));
    if (items.isEmpty) {
      return Padding(
        padding: const EdgeInsets.fromLTRB(4, 4, 4, 0),
        child: Text(l.actEmpty, style: TextStyle(color: c.inkMuted)),
      );
    }
    return OrreryCard(
      padding: EdgeInsets.zero,
      child: AnimatedItems<ActivityItem>(
        items: items,
        keyOf: (a) => a.id,
        gap: 0,
        builder: (context, a) {
          final (icon, label) = _look(l, a.kind);
          final tone = _bad(a.kind) ? c.error : c.inkFaint;
          return DecoratedBox(
            key: ValueKey('act-${a.id}'),
            decoration: BoxDecoration(
              border: a == items.first
                  ? null
                  : Border(top: BorderSide(color: c.line)),
            ),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(14, 10, 14, 10),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Padding(
                    padding: const EdgeInsets.only(top: 1),
                    child: Icon(icon, size: 16, color: tone),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          label,
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            color: _bad(a.kind) ? c.error : c.ink,
                          ),
                        ),
                        if (a.detail != null)
                          Text(
                            a.detail!,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: monoStyle(context),
                          ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(clock(a.at), style: monoStyle(context)),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}
