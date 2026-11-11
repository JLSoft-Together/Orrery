class PaneFacts {
  const PaneFacts({
    this.tool,
    this.toolSummary,
    this.toolSince,
    this.statusSince,
  });

  static const PaneFacts none = PaneFacts();

  final String? tool;
  final String? toolSummary;
  final DateTime? toolSince;
  final DateTime? statusSince;

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
    final terminals = snapshot['terminals'];
    if (terminals is List) {
      for (final term in terminals) {
        if (term is Map && term['id'] == terminalId) {
          statusSince = _time(term['statusSince']);
          break;
        }
      }
    }
    return PaneFacts(
      tool: tool,
      toolSummary: summary,
      toolSince: toolSince,
      statusSince: statusSince,
    );
  }

  static DateTime? _time(Object? value) =>
      value is String ? DateTime.tryParse(value) : null;
}
