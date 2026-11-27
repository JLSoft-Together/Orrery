import 'dart:convert';

const int waitingLabelMax = 24;
const int waitingLabelCount = 2;
const int waitingCountMax = 999;
const String clearKind = 'clear';

class WaitingUpdate {
  const WaitingUpdate({required this.count, this.labels = const []});

  final int count;
  final List<String> labels;

  @override
  bool operator ==(Object other) =>
      other is WaitingUpdate &&
      other.count == count &&
      _sameLabels(other.labels, labels);

  @override
  int get hashCode => Object.hash(count, Object.hashAll(labels));

  @override
  String toString() => 'WaitingUpdate($count, $labels)';
}

bool _sameLabels(List<String> a, List<String> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

bool isClearPush(Map<String, String> data) => data['k'] == clearKind;

WaitingUpdate? parseWaiting(Map<String, String> data) {
  if (isClearPush(data)) return const WaitingUpdate(count: 0);
  final raw = data['w']?.trim();
  if (raw == null || !RegExp(r'^[0-9]{1,6}$').hasMatch(raw)) return null;
  final parsed = int.parse(raw);
  final count = parsed > waitingCountMax ? waitingCountMax : parsed;
  if (count == 0) return const WaitingUpdate(count: 0);
  return WaitingUpdate(count: count, labels: parseWaitingLabels(data['wl']));
}

List<String> parseWaitingLabels(String? raw) {
  if (raw == null || raw.trim().isEmpty) return const [];
  var items = <String>[];
  try {
    final value = jsonDecode(raw);
    if (value is List) items = value.whereType<String>().toList();
  } on FormatException {
    items = raw.split(RegExp(r'[\r\n]+'));
  }
  return [
    for (final item in items)
      if (item.trim().isNotEmpty) _cut(item.trim()),
  ].take(waitingLabelCount).toList();
}

String _cut(String text) {
  final runes = text.runes.toList();
  if (runes.length <= waitingLabelMax) return text;
  return String.fromCharCodes(runes.take(waitingLabelMax));
}
