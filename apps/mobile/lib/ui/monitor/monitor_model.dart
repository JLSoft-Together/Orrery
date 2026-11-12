import '../model/workspace_model.dart';
import '../viewers/viewer_models.dart';

class MonitorAgent {
  const MonitorAgent({
    required this.id,
    required this.rail,
    required this.depth,
    this.name,
    this.model,
    this.tool,
    this.toolSummary,
    this.contextPct,
    this.totalTokens,
  });

  final String id;
  final RailState rail;
  final int depth;
  final String? name;
  final String? model;
  final String? tool;
  final String? toolSummary;
  final int? contextPct;
  final int? totalTokens;
}

class MonitorSession {
  const MonitorSession({
    required this.terminalId,
    required this.title,
    required this.rail,
    required this.subagents,
    this.alias,
    this.provider,
    this.branch,
    this.model,
    this.tool,
    this.toolSummary,
    this.contextPct,
    this.totalTokens,
    this.waitingFor,
    this.statusSince,
  });

  final String? alias;
  final String terminalId;
  final String title;
  final RailState rail;
  final List<MonitorAgent> subagents;
  final String? provider;
  final String? branch;
  final String? model;
  final String? tool;
  final String? toolSummary;
  final int? contextPct;
  final int? totalTokens;
  final String? waitingFor;
  final DateTime? statusSince;
}

class MonitorProject {
  const MonitorProject(this.name, this.cwd, this.sessions);

  final String name;
  final String? cwd;
  final List<MonitorSession> sessions;
}

RailState _rail(String? status) => switch (status) {
  'working' => RailState.working,
  'waiting' => RailState.asked,
  'error' || 'failed' => RailState.failed,
  'completed' || 'done' => RailState.done,
  _ => RailState.idle,
};

int _railRank(RailState r) => switch (r) {
  RailState.failed => 0,
  RailState.asked => 1,
  RailState.working => 2,
  _ => 3,
};

String? _modelName(Object? names, String? model) {
  if (model == null) return null;
  if (names is Map && names[model] is String) return names[model] as String;
  return model;
}

int? _pct(Object? v) => v is num ? v.round().clamp(0, 100) : null;

List<MonitorProject> parseMonitor(Map<String, dynamic>? snapshot) {
  if (snapshot == null) return const [];
  final names = snapshot['modelNames'];
  final agents = maps(snapshot['agents']);
  final grouped = <String, List<MonitorSession>>{};
  final cwdOf = <String, String?>{};
  for (final t in maps(snapshot['terminals'])) {
    final id = str(t['id']);
    if (id == null) continue;
    final mine = [
      for (final a in agents)
        if (a['terminalId'] == id) a,
    ];
    final main = mine.where((a) => a['role'] == 'main').firstOrNull;
    final subs = mine.where((a) => a['role'] == 'subagent').toList();
    final byParent = <String?, List<Map<String, dynamic>>>{};
    for (final s in subs) {
      (byParent[str(s['parentId'])] ??= []).add(s);
    }
    final known = {for (final s in subs) str(s['id'])};
    final ordered = <MonitorAgent>[];
    void walk(String? parent, int depth) {
      for (final s in byParent[parent] ?? const []) {
        ordered.add(
          MonitorAgent(
            id: str(s['id']) ?? '',
            rail: _rail(str(s['status'])),
            depth: depth,
            name: str(s['name']) ?? str(s['description']) ?? str(s['type']),
            model: _modelName(names, str(s['model'])),
            tool: str(s['currentTool']),
            toolSummary: str(s['currentToolSummary']),
            contextPct: _pct(s['contextPct']),
            totalTokens: intOrNull(s['totalTokens']),
          ),
        );
        walk(str(s['id']), depth + 1);
      }
    }

    walk(str(main?['id']), 1);
    for (final parent in byParent.keys.toList()) {
      if (parent != null &&
          parent != str(main?['id']) &&
          !known.contains(parent)) {
        walk(parent, 1);
      }
    }
    walk(null, 1);
    final cwd = str(t['cwd']);
    final key = cwd == null ? '' : cwd.replaceAll('\\', '/').toLowerCase();
    cwdOf[key] = cwd;
    (grouped[key] ??= []).add(
      MonitorSession(
        terminalId: id,
        title: str(t['alias']) ?? str(t['title']) ?? id,
        alias: str(t['alias']),
        rail: _rail(str(t['status'])),
        subagents: ordered,
        provider: str(t['provider']),
        branch: str(t['gitBranch']),
        model: _modelName(names, str(main?['model'])),
        tool: str(main?['currentTool']),
        toolSummary: str(main?['currentToolSummary']),
        contextPct: _pct(main?['contextPct']),
        totalTokens: intOrNull(main?['totalTokens']),
        waitingFor: str(t['waitingFor']),
        statusSince: DateTime.tryParse(str(t['statusSince']) ?? ''),
      ),
    );
  }
  final projects = [
    for (final entry in grouped.entries)
      MonitorProject(
        entry.key.isEmpty ? '' : baseName(cwdOf[entry.key]!),
        cwdOf[entry.key],
        entry.value
          ..sort((a, b) => _railRank(a.rail).compareTo(_railRank(b.rail))),
      ),
  ];
  projects.sort((a, b) {
    final ra = a.sessions
        .map((s) => _railRank(s.rail))
        .reduce((x, y) => x < y ? x : y);
    final rb = b.sessions
        .map((s) => _railRank(s.rail))
        .reduce((x, y) => x < y ? x : y);
    return ra.compareTo(rb);
  });
  return projects;
}
