import '../../l10n/app_localizations.dart';
import '../../remote/chat_protocol.dart';

ChatToolCat catOfTool(String? tool) {
  final name = tool ?? '';
  return switch (name) {
    'Edit' ||
    'Write' ||
    'MultiEdit' ||
    'NotebookEdit' ||
    'apply_patch' => ChatToolCat.edit,
    'Bash' ||
    'PowerShell' ||
    'shell' ||
    'exec_command' ||
    'local_shell' => ChatToolCat.run,
    'Read' || 'NotebookRead' => ChatToolCat.read,
    'Grep' || 'Glob' || 'LS' || 'ToolSearch' => ChatToolCat.search,
    'WebFetch' || 'WebSearch' => ChatToolCat.web,
    'Task' || 'Agent' => ChatToolCat.agent,
    _ => ChatToolCat.other,
  };
}

String toolVerb(AppLocalizations l, ChatToolCat cat, String tool) =>
    switch (cat) {
      ChatToolCat.edit => l.chatVerbEdit,
      ChatToolCat.run => l.chatVerbRun,
      ChatToolCat.read => l.chatVerbRead,
      ChatToolCat.search => l.chatVerbSearch,
      ChatToolCat.web => l.chatVerbWeb,
      ChatToolCat.agent => l.chatVerbAgent,
      ChatToolCat.other => tool,
    };

String liveVerb(AppLocalizations l, ChatToolCat cat, String tool) =>
    switch (cat) {
      ChatToolCat.edit => l.chatLiveEdit,
      ChatToolCat.run => l.chatLiveRun,
      ChatToolCat.read => l.chatLiveRead,
      ChatToolCat.search => l.chatLiveSearch,
      ChatToolCat.web => l.chatLiveWeb,
      ChatToolCat.agent => l.chatLiveAgent,
      ChatToolCat.other => tool,
    };

const List<ChatToolCat> _order = [
  ChatToolCat.edit,
  ChatToolCat.run,
  ChatToolCat.read,
  ChatToolCat.search,
  ChatToolCat.web,
  ChatToolCat.agent,
  ChatToolCat.other,
];

String toolSummary(AppLocalizations l, Map<ChatToolCat, int> sum) {
  final parts = <String>[];
  for (final cat in _order) {
    final n = sum[cat] ?? 0;
    if (n <= 0) continue;
    parts.add(switch (cat) {
      ChatToolCat.edit => l.chatSumEdit(n),
      ChatToolCat.run => l.chatSumRun(n),
      ChatToolCat.read => l.chatSumRead(n),
      ChatToolCat.search => l.chatSumSearch(n),
      ChatToolCat.web => l.chatSumWeb(n),
      ChatToolCat.agent => l.chatSumAgent(n),
      ChatToolCat.other => l.chatSumOther(n),
    });
  }
  if (parts.isEmpty) return l.chatSumNone;
  final text = parts.join(', ');
  return text[0].toUpperCase() + text.substring(1);
}

String sysText(AppLocalizations l, ChatSys item) {
  final base = switch (item.kind) {
    ChatSysKind.stop => l.chatSysStop,
    ChatSysKind.error => l.chatSysError,
    ChatSysKind.limit => l.chatSysLimit,
    ChatSysKind.compact => l.chatSysCompact,
  };
  final extra = item.text;
  if (extra == null || extra.isEmpty) return base;
  return '$base: $extra';
}

String mmss(Duration d) {
  final s = d.isNegative ? 0 : d.inSeconds;
  final m = s ~/ 60;
  return '$m:${(s % 60).toString().padLeft(2, '0')}';
}
