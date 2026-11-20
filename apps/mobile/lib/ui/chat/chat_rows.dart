import '../../remote/chat_protocol.dart';
import '../model/workspace_model.dart';

class ChatRow {
  const ChatRow({
    required this.pane,
    required this.tab,
    required this.project,
    this.at,
    this.last,
  });

  final PaneInfo pane;
  final TabInfo tab;
  final String project;
  final DateTime? at;
  final ChatLast? last;

  bool get waiting => pane.rail == RailState.asked;

  bool get working =>
      pane.rail == RailState.working || pane.rail == RailState.subagents;

  bool matches(String query) {
    final q = query.trim().toLowerCase();
    if (q.isEmpty) return true;
    final text = '${pane.title} $project ${last?.text ?? ''}'.toLowerCase();
    return text.contains(q);
  }
}

class ChatRowGroups {
  const ChatRowGroups(this.waiting, this.rest);

  final List<ChatRow> waiting;
  final List<ChatRow> rest;

  bool get isEmpty => waiting.isEmpty && rest.isEmpty;
}

DateTime? _time(Object? value) {
  if (value is String) return DateTime.tryParse(value);
  if (value is num && value > 0) {
    return DateTime.fromMillisecondsSinceEpoch(value.toInt());
  }
  return null;
}

Map<String, DateTime> terminalTimes(Map<String, dynamic>? snapshot) {
  final terminals = snapshot?['terminals'];
  if (terminals is! List) return const {};
  final out = <String, DateTime>{};
  for (final term in terminals) {
    if (term is! Map) continue;
    final id = term['id'];
    final at = _time(term['lastActivityAt']);
    if (id is String && at != null) out[id] = at;
  }
  return out;
}

String terminalTimesKey(Map<String, dynamic>? snapshot) {
  final times = terminalTimes(snapshot);
  return [
    for (final e in times.entries) '${e.key}=${e.value.millisecondsSinceEpoch}',
  ].join(';');
}

int? contextPctOf(Map<String, dynamic>? snapshot, String? terminalId) {
  if (terminalId == null) return null;
  final agents = snapshot?['agents'];
  if (agents is! List) return null;
  for (final agent in agents) {
    if (agent is! Map || agent['terminalId'] != terminalId) continue;
    if (agent['role'] != 'main') continue;
    final pct = agent['contextPct'];
    if (pct is num && pct.isFinite) return pct.round().clamp(0, 100);
    return null;
  }
  return null;
}

String projectOf(TabInfo tab, PaneInfo pane) {
  final path = tab.project ?? pane.cwd;
  if (path != null && path.isNotEmpty) {
    final name = baseName(path);
    if (name.isNotEmpty) return name;
  }
  return tab.displayName;
}

List<ChatRow> buildChatRows(
  WorkspaceModel model,
  Map<String, dynamic>? snapshot,
  Map<String, ChatLast> lasts,
) {
  final times = terminalTimes(snapshot);
  final rows = <ChatRow>[];
  for (final tab in model.tabs) {
    for (final pane in tab.panes) {
      if (!pane.isTerm) continue;
      final last = lasts[pane.key];
      final at =
          times[pane.terminalId] ??
          last?.at ??
          (pane.inbox?.since == null
              ? null
              : DateTime.tryParse(pane.inbox!.since!));
      rows.add(
        ChatRow(
          pane: pane,
          tab: tab,
          project: projectOf(tab, pane),
          at: at,
          last: last,
        ),
      );
    }
  }
  return rows;
}

int _byTime(ChatRow a, ChatRow b) {
  final x = a.at;
  final y = b.at;
  if (x == null && y == null) return 0;
  if (x == null) return 1;
  if (y == null) return -1;
  return y.compareTo(x);
}

ChatRowGroups groupChatRows(List<ChatRow> rows, [String query = '']) {
  final hit = rows.where((r) => r.matches(query)).toList();
  final waiting = hit.where((r) => r.waiting).toList();
  final rest = hit.where((r) => !r.waiting).toList();
  return ChatRowGroups(
    List.of(waiting)..sort(_byTime),
    List.of(rest)..sort(_byTime),
  );
}
