class PaneFacts {
  const PaneFacts({
    this.tool,
    this.toolSummary,
    this.toolSince,
    this.statusSince,
    this.alias,
  });

  static const PaneFacts none = PaneFacts();

  final String? tool;
  final String? toolSummary;
  final DateTime? toolSince;
  final DateTime? statusSince;
  final String? alias;

  bool get hasTool => tool != null && tool!.isNotEmpty;

  static PaneFacts of(Map<String, dynamic>? snapshot, String? terminalId) {
    if (snapshot == null || terminalId == null) return none;
    String? tool;
    String? summary;
    DateTime? toolSince;
    final agents = snapshot['agents'];
    if (agents is List) {
      for (final agent in agents) {
        if (agent is! Map || agent['terminalId'] != terminalId) continue;
        if (agent['role'] != 'main') continue;
        final name = agent['currentTool'];
        if (name is String && name.isNotEmpty) {
          tool = name;
          final detail = agent['currentToolSummary'];
          if (detail is String && detail.isNotEmpty) summary = detail;
          toolSince = _time(agent['currentToolStartedAt']);
        }
        break;
      }
    }
    DateTime? statusSince;
    String? alias;
    final terminals = snapshot['terminals'];
    if (terminals is List) {
      for (final term in terminals) {
        if (term is Map && term['id'] == terminalId) {
          statusSince = _time(term['statusSince']);
          final raw = term['alias'];
          if (raw is String && raw.isNotEmpty) alias = raw;
          break;
        }
      }
    }
    return PaneFacts(
      tool: tool,
      toolSummary: summary,
      toolSince: toolSince,
      statusSince: statusSince,
      alias: alias,
    );
  }

  static DateTime? _time(Object? value) =>
      value is String ? DateTime.tryParse(value) : null;
}

Map<String, String> sessionAliases(Map<String, dynamic>? snapshot) {
  final terminals = snapshot?['terminals'];
  if (terminals is! List) return const {};
  final out = <String, String>{};
  for (final term in terminals) {
    if (term is! Map) continue;
    final id = term['id'];
    final alias = term['alias'];
    if (id is String && alias is String && alias.isNotEmpty) out[id] = alias;
  }
  return out;
}

String sessionAliasKey(Map<String, String> aliases) =>
    aliases.entries.map((e) => '${e.key}|${e.value}').join(';');
