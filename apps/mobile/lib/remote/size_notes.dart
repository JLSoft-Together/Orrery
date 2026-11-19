class SnapshotTrim {
  const SnapshotTrim({this.activity = 0, this.agents = 0, this.text = false});

  final int activity;
  final int agents;
  final bool text;

  static SnapshotTrim? parse(Object? raw) {
    if (raw is! Map) return null;
    final trim = SnapshotTrim(
      activity: _count(raw['activity']),
      agents: _count(raw['agents']),
      text: raw['text'] == true,
    );
    return trim.activity > 0 || trim.agents > 0 || trim.text ? trim : null;
  }

  @override
  bool operator ==(Object other) =>
      other is SnapshotTrim &&
      other.activity == activity &&
      other.agents == agents &&
      other.text == text;

  @override
  int get hashCode => Object.hash(activity, agents, text);
}

class SizeLimit {
  const SizeLimit({required this.what, this.bytes = 0, this.max = defaultMax});

  static const int defaultMax = 1 << 20;

  final String what;
  final int bytes;
  final int max;

  static SizeLimit? parse(Map<String, dynamic> event) {
    final what = event['what'];
    if (what is! String || what.isEmpty) return null;
    final max = _count(event['max']);
    return SizeLimit(
      what: what.length > 40 ? what.substring(0, 40) : what,
      bytes: _count(event['bytes']),
      max: max > 0 ? max : defaultMax,
    );
  }

  @override
  bool operator ==(Object other) =>
      other is SizeLimit &&
      other.what == what &&
      other.bytes == bytes &&
      other.max == max;

  @override
  int get hashCode => Object.hash(what, bytes, max);
}

int _count(Object? value) {
  if (value is! num || !value.isFinite || value <= 0) return 0;
  return value.floor();
}
