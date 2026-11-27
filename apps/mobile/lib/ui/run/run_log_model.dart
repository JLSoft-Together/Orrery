const int runLogMax = 2000;
const int crashLinesMax = 80;
const int crashCharsMax = 6000;

final RegExp _crashStart = RegExp(
  r'FATAL EXCEPTION|Unhandled Exception|E/flutter',
);
final RegExp _crashTail = RegExp(
  r'^\s*(at\s|#\d+\s|Caused by|\.\.\. \d+ more|<asynchronous suspension>)',
);

bool isCrashStart(String text) => _crashStart.hasMatch(text);

class RunLogLine {
  const RunLogLine({
    required this.n,
    required this.stream,
    required this.text,
    this.level,
    this.tag,
    this.crash = false,
  });

  final int n;
  final String stream;
  final String text;
  final String? level;
  final String? tag;
  final bool crash;

  RunLogLine marked(bool value) => value == crash
      ? this
      : RunLogLine(
          n: n,
          stream: stream,
          text: text,
          level: level,
          tag: tag,
          crash: value,
        );

  static RunLogLine? parse(Object? raw) {
    if (raw is! Map) return null;
    final n = raw['n'];
    final text = raw['text'];
    if (n is! int || text is! String) return null;
    final stream = raw['stream'];
    final level = raw['level'];
    final tag = raw['tag'];
    return RunLogLine(
      n: n,
      stream: stream is String ? stream : 'app',
      text: text,
      level: level is String ? level : null,
      tag: tag is String ? tag : null,
    );
  }
}

class RunLogChunk {
  const RunLogChunk({
    required this.known,
    required this.lines,
    required this.last,
    required this.gap,
    required this.more,
  });

  final bool known;
  final List<RunLogLine> lines;
  final int last;
  final bool gap;
  final bool more;

  static RunLogChunk parse(Object? raw) {
    if (raw is! Map) {
      return const RunLogChunk(
        known: false,
        lines: [],
        last: 0,
        gap: false,
        more: false,
      );
    }
    final list = raw['lines'];
    final last = raw['last'];
    return RunLogChunk(
      known: raw['known'] == true,
      lines: [
        if (list is List)
          for (final item in list) ?RunLogLine.parse(item),
      ],
      last: last is int ? last : 0,
      gap: raw['gap'] == true,
      more: raw['more'] == true,
    );
  }
}

List<RunLogLine> markCrash(List<RunLogLine> lines) {
  final out = <RunLogLine>[];
  var inside = false;
  for (final line in lines) {
    final start = isCrashStart(line.text);
    if (start) {
      inside = true;
    } else if (inside && !_crashTail.hasMatch(line.text)) {
      inside = false;
    }
    out.add(line.marked(inside));
  }
  return out;
}

String? crashExcerpt(List<RunLogLine> lines) {
  var from = -1;
  for (var i = lines.length - 1; i >= 0; i--) {
    if (!lines[i].crash) continue;
    from = i;
    while (from > 0 && lines[from - 1].crash) {
      from--;
    }
    break;
  }
  if (from < 0) return null;
  final block = <String>[];
  for (var i = from; i < lines.length && lines[i].crash; i++) {
    block.add(lines[i].text);
    if (block.length >= crashLinesMax) break;
  }
  var text = block.join('\n');
  if (text.length > crashCharsMax) text = text.substring(0, crashCharsMax);
  return text;
}

class RunLogBuffer {
  RunLogBuffer();

  List<RunLogLine> lines = const [];
  int last = 0;
  bool gap = false;
  bool known = false;

  bool get hasCrash => lines.any((l) => l.crash);

  void add(RunLogChunk chunk, {required bool first}) {
    final restarted = !first && chunk.last > 0 && chunk.last < last;
    final replace = first || restarted;
    known = known || chunk.known;
    if (chunk.gap) gap = true;
    if (restarted) last = chunk.last;
    if (chunk.last > last) last = chunk.last;
    if (chunk.lines.isEmpty) {
      if (restarted) lines = const [];
      return;
    }
    final seen = lines.isEmpty ? 0 : lines.last.n;
    final fresh = [
      for (final l in chunk.lines)
        if (replace || l.n > seen) l,
    ];
    var merged = replace ? fresh : [...lines, ...fresh];
    if (merged.length > runLogMax) {
      merged = merged.sublist(merged.length - runLogMax);
    }
    lines = markCrash(merged);
  }

  void reset() {
    lines = const [];
    last = 0;
    gap = false;
    known = false;
  }
}
