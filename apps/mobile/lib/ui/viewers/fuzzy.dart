int? fuzzyScore(String path, String query) {
  final text = path.toLowerCase();
  final needle = query.toLowerCase().replaceAll(RegExp(r'\s+'), '');
  if (needle.isEmpty) return 0;
  var score = 0;
  var from = 0;
  var previous = -2;
  final nameStart = text.lastIndexOf('/') + 1;
  for (var i = 0; i < needle.length; i++) {
    final at = text.indexOf(needle[i], from);
    if (at < 0) return null;
    var gain = 1;
    if (at == previous + 1) gain += 5;
    if (at == 0 ||
        text[at - 1] == '/' ||
        text[at - 1] == '_' ||
        text[at - 1] == '.') {
      gain += 4;
    }
    if (at >= nameStart) gain += 2;
    score += gain;
    previous = at;
    from = at + 1;
  }
  if (text.substring(nameStart).contains(needle)) score += 20;
  return score * 4 - text.length ~/ 8;
}

List<String> fuzzyFilter(List<String> files, String query, {int limit = 60}) {
  final scored = <(int, String)>[];
  for (final file in files) {
    final score = fuzzyScore(file, query);
    if (score != null) scored.add((score, file));
  }
  scored.sort((a, b) {
    final byScore = b.$1.compareTo(a.$1);
    return byScore != 0 ? byScore : a.$2.length.compareTo(b.$2.length);
  });
  return [for (final e in scored.take(limit)) e.$2];
}
