const int pushMenuMax = 3;
const int pushMenuLabelMax = 24;
const String menuActionPrefix = 'menu:';
const String openActionId = 'open';
final RegExp pushSigPattern = RegExp(r'^[A-Za-z0-9_-]{1,64}$');

class PushMenuItem {
  const PushMenuItem(this.index, this.label);

  final int index;
  final String label;

  @override
  bool operator ==(Object other) =>
      other is PushMenuItem && other.index == index && other.label == label;

  @override
  int get hashCode => Object.hash(index, label);
}

class PushMenu {
  const PushMenu({required this.sig, required this.items});

  final String sig;
  final List<PushMenuItem> items;
}

bool _drop(int r) =>
    r < 0x20 ||
    r == 0x7f ||
    (r >= 0x202a && r <= 0x202e) ||
    (r >= 0x2066 && r <= 0x2069);

String cutMenuLabel(String text) {
  final clean = String.fromCharCodes(text.runes.map((r) => _drop(r) ? 0x20 : r))
      .replaceAll(RegExp(r'\s+'), ' ')
      .trim();
  final runes = clean.runes.toList();
  if (runes.length <= pushMenuLabelMax) return clean;
  return String.fromCharCodes(runes.take(pushMenuLabelMax));
}

PushMenu? parsePushMenu(Object? raw) {
  if (raw is! Map) return null;
  final sig = raw['sig'];
  final items = raw['items'];
  if (sig is! String || !pushSigPattern.hasMatch(sig) || items is! List) {
    return null;
  }
  final seen = <int>{};
  final out = <PushMenuItem>[];
  for (final item in items) {
    if (item is! Map) continue;
    final index = item['index'];
    final label = item['label'];
    if (index is! int || index < 1 || index > 99 || label is! String) continue;
    final cut = cutMenuLabel(label);
    if (cut.isEmpty || !seen.add(index)) continue;
    out.add(PushMenuItem(index, cut));
    if (out.length == pushMenuMax) break;
  }
  return out.isEmpty ? null : PushMenu(sig: sig, items: out);
}

String menuActionId(int index) => '$menuActionPrefix$index';

int? menuActionIndex(String? actionId) {
  if (actionId == null || !actionId.startsWith(menuActionPrefix)) return null;
  final index = int.tryParse(actionId.substring(menuActionPrefix.length));
  return index != null && index >= 1 && index <= 99 ? index : null;
}

Map<String, String> tapDataWithAction(
  Map<String, String> data,
  String? actionId,
) {
  final index = menuActionIndex(actionId);
  if (index == null) return data;
  final sig = data['sig'];
  if (sig == null || !pushSigPattern.hasMatch(sig)) return data;
  return {...data, 'a': 'menu', 'i': '$index'};
}
