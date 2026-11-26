const Duration awayMinGap = Duration(minutes: 30);

enum AwayGroup { done, waiting, failed, agentsDone, agentsFailed, compact }

DateTime? _time(Object? value) {
  if (value is String) return DateTime.tryParse(value);
  if (value is num && value > 0) {
    return DateTime.fromMillisecondsSinceEpoch(value.toInt());
  }
  return null;
}

String? _str(Object? value) => value is String && value.isNotEmpty ? value : null;

class AwayMark {
  const AwayMark({
    required this.leftAt,
    this.ref,
    this.statuses = const {},
  });

  final DateTime leftAt;
  final DateTime? ref;
  final Map<String, String> statuses;

  DateTime get cutoff => ref ?? leftAt;

  bool isAfterCutoff(DateTime at) =>
      ref == null ? !at.isBefore(leftAt) : at.isAfter(ref!);

  Map<String, Object?> toJson() => {
    'leftAt': leftAt.millisecondsSinceEpoch,
    'ref': ref?.millisecondsSinceEpoch,
    'statuses': statuses,
  };

  static AwayMark? fromJson(Object? raw) {
    if (raw is! Map) return null;
    final left = _time(raw['leftAt']);
    if (left == null) return null;
    final statuses = <String, String>{};
    final rawStatuses = raw['statuses'];
    if (rawStatuses is Map) {
      for (final e in rawStatuses.entries) {
        if (e.key is String && e.value is String) {
          statuses[e.key as String] = e.value as String;
        }
      }
    }
    return AwayMark(leftAt: left, ref: _time(raw['ref']), statuses: statuses);
  }
}

AwayMark markFromSnapshot(Map<String, dynamic>? snapshot, DateTime leftAt) {
  DateTime? ref;
  void bump(DateTime? at) {
    if (at != null && (ref == null || at.isAfter(ref!))) ref = at;
  }

  final statuses = <String, String>{};
  final terminals = snapshot?['terminals'];
  if (terminals is List) {
    for (final t in terminals) {
      if (t is! Map) continue;
      final id = _str(t['id']);
      final status = _str(t['status']);
      if (id != null && status != null) statuses[id] = status;
      bump(_time(t['lastActivityAt']));
      bump(_time(t['statusSince']));
    }
  }
  final activity = snapshot?['activity'];
  if (activity is List) {
    for (final e in activity) {
      if (e is Map) bump(_time(e['at']));
    }
  }
  return AwayMark(leftAt: leftAt, ref: ref, statuses: statuses);
}

class ActivityTally {
  ActivityTally();

  int started = 0;
  int ended = 0;
  int compactions = 0;
  int failedAgents = 0;
  int completedAgents = 0;
  int jobsDone = 0;
  final Set<String> doneIds = {};
  final Set<String> failedIds = {};
  final Set<String> compactIds = {};
  final Set<String> agentDoneIds = {};
  final Set<String> agentFailedIds = {};
}

ActivityTally summarizeActivity(List<dynamic> items, AwayMark mark) {
  final out = ActivityTally();
  for (final e in items) {
    if (e is! Map) continue;
    final at = _time(e['at']);
    if (at == null || !mark.isAfterCutoff(at)) continue;
    final id = _str(e['terminalId']);
    final data = e['data'] is Map ? e['data'] as Map : const {};
    switch (e['kind']) {
      case 'terminal.started':
        out.started++;
      case 'terminal.ended':
        out.ended++;
      case 'terminal.compacted':
        out.compactions++;
        if (id != null) out.compactIds.add(id);
      case 'agent.failed':
        out.failedAgents++;
        if (id != null) out.agentFailedIds.add(id);
      case 'agent.completed':
        out.completedAgents++;
        if (id != null) out.agentDoneIds.add(id);
      case 'job.done':
        out.jobsDone++;
      case 'terminal.status':
        if (id == null) break;
        if (data['to'] == 'error') {
          out.failedIds.add(id);
        } else if (data['from'] == 'working' && data['to'] == 'idle') {
          out.doneIds.add(id);
        }
      case 'turn.completed':
        if (id != null) out.doneIds.add(id);
      case 'turn.failed':
        if (id != null) out.failedIds.add(id);
    }
  }
  return out;
}

class AwaySummary {
  const AwaySummary({
    required this.since,
    required this.until,
    this.done = const [],
    this.waiting = const [],
    this.failed = const [],
    this.started = 0,
    this.ended = 0,
    this.compactions = 0,
    this.failedAgents = 0,
    this.completedAgents = 0,
    this.jobsDone = 0,
    this.compactIds = const [],
    this.agentDoneIds = const [],
    this.agentFailedIds = const [],
    this.partial = false,
  });

  final DateTime since;
  final DateTime until;
  final List<String> done;
  final List<String> waiting;
  final List<String> failed;
  final int started;
  final int ended;
  final int compactions;
  final int failedAgents;
  final int completedAgents;
  final int jobsDone;
  final List<String> compactIds;
  final List<String> agentDoneIds;
  final List<String> agentFailedIds;
  final bool partial;

  Duration get away => until.difference(since);

  int countOf(AwayGroup group) => switch (group) {
    AwayGroup.done => done.length,
    AwayGroup.waiting => waiting.length,
    AwayGroup.failed => failed.length,
    AwayGroup.agentsDone => completedAgents,
    AwayGroup.agentsFailed => failedAgents,
    AwayGroup.compact => compactions,
  };

  List<String> idsOf(AwayGroup group) => switch (group) {
    AwayGroup.done => done,
    AwayGroup.waiting => waiting,
    AwayGroup.failed => failed,
    AwayGroup.agentsDone => agentDoneIds,
    AwayGroup.agentsFailed => agentFailedIds,
    AwayGroup.compact => compactIds,
  };

  bool get isQuiet => AwayGroup.values.every((g) => countOf(g) == 0);
}

AwaySummary? computeAway({
  required Map<String, dynamic>? snapshot,
  required AwayMark mark,
  required DateTime now,
}) {
  if (snapshot == null) return null;
  final activity = snapshot['activity'] is List
      ? snapshot['activity'] as List
      : const [];
  final terminals = snapshot['terminals'] is List
      ? snapshot['terminals'] as List
      : const [];

  DateTime? oldest;
  for (final e in activity) {
    if (e is! Map) continue;
    final at = _time(e['at']);
    if (at != null && (oldest == null || at.isBefore(oldest))) oldest = at;
  }
  final partial = oldest != null && oldest.isAfter(mark.cutoff);

  final tally = summarizeActivity(activity, mark);
  final live = <String>{};
  final waiting = <(String, DateTime?)>[];
  final done = {...tally.doneIds};
  final failed = {...tally.failedIds};
  for (final t in terminals) {
    if (t is! Map) continue;
    final id = _str(t['id']);
    if (id == null) continue;
    live.add(id);
    final status = _str(t['status']);
    final since = _time(t['statusSince']);
    if (status == 'waiting') waiting.add((id, since));
    final prev = mark.statuses[id];
    if (prev == null) continue;
    if (status == 'idle' &&
        (prev == 'working' || (since != null && mark.isAfterCutoff(since)))) {
      done.add(id);
    }
    if (status == 'error' && prev != 'error') failed.add(id);
  }
  waiting.sort((a, b) {
    final x = a.$2;
    final y = b.$2;
    if (x == null && y == null) return 0;
    if (x == null) return 1;
    if (y == null) return -1;
    return x.compareTo(y);
  });
  final waitingIds = [for (final w in waiting) w.$1];
  final doneIds = [
    for (final id in done)
      if (live.contains(id) && !waitingIds.contains(id) && !failed.contains(id))
        id,
  ]..sort();
  final failedIds = [
    for (final id in failed)
      if (live.contains(id)) id,
  ]..sort();

  final summary = AwaySummary(
    since: mark.leftAt,
    until: now,
    done: doneIds,
    waiting: waitingIds,
    failed: failedIds,
    started: tally.started,
    ended: tally.ended,
    compactions: tally.compactions,
    failedAgents: tally.failedAgents,
    completedAgents: tally.completedAgents,
    jobsDone: tally.jobsDone,
    compactIds: tally.compactIds.toList()..sort(),
    agentDoneIds: tally.agentDoneIds.toList()..sort(),
    agentFailedIds: tally.agentFailedIds.toList()..sort(),
    partial: partial,
  );
  return summary.isQuiet ? null : summary;
}

bool awayDue(AwayMark mark, DateTime now) {
  final gap = now.difference(mark.leftAt);
  return !gap.isNegative && gap >= awayMinGap;
}
