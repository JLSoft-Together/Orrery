import '../../remote/protocol.dart';

enum CheckView { none, idle, running, pass, fail, stale }

const int maxChecklistItems = 50;

class ChecklistItem {
  const ChecklistItem({
    required this.id,
    required this.text,
    required this.done,
    required this.byAgent,
  });

  final String id;
  final String text;
  final bool done;
  final bool byAgent;
}

class ChecklistData {
  const ChecklistData({
    required this.items,
    required this.script,
    required this.state,
    required this.checkHead,
    required this.head,
  });

  final List<ChecklistItem> items;
  final String script;
  final String state;
  final String checkHead;
  final String head;

  int get open => items.where((i) => !i.done).length;

  CheckView get view {
    if (script.isEmpty) return CheckView.none;
    switch (state) {
      case 'running':
        return CheckView.running;
      case 'pass':
      case 'fail':
        if (head.isNotEmpty && checkHead.isNotEmpty && head != checkHead) {
          return CheckView.stale;
        }
        return state == 'pass' ? CheckView.pass : CheckView.fail;
      default:
        return CheckView.idle;
    }
  }

  bool get ready => open == 0 && view == CheckView.pass;

  bool get canRun => script.isNotEmpty;

  static ChecklistData parse(JsonMap m) {
    final rawItems = m['items'];
    final check = m['check'];
    final checkMap = check is Map ? check.cast<String, dynamic>() : const {};
    return ChecklistData(
      items: [
        if (rawItems is List)
          for (final e in rawItems.take(maxChecklistItems))
            if (e is Map && e['id'] is String && e['text'] is String)
              ChecklistItem(
                id: e['id'] as String,
                text: e['text'] as String,
                done: e['done'] == true,
                byAgent: e['by'] == 'agent',
              ),
      ],
      script: _text(checkMap['script']),
      state: _text(checkMap['state']),
      checkHead: _text(checkMap['head']),
      head: _text(m['head']),
    );
  }

  static String _text(Object? v) => v is String ? v : '';
}
