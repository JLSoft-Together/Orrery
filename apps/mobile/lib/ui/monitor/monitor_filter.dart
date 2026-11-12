import '../model/workspace_model.dart';
import 'monitor_model.dart';

enum MonitorFilter { all, running, waiting, idle }

bool isRunning(RailState r) =>
    r == RailState.working || r == RailState.subagents;

bool isWaiting(RailState r) => r == RailState.asked || r == RailState.failed;

bool matchesFilter(MonitorFilter filter, MonitorSession s) => switch (filter) {
  MonitorFilter.all => true,
  MonitorFilter.running => isRunning(s.rail),
  MonitorFilter.waiting => isWaiting(s.rail),
  MonitorFilter.idle => !isRunning(s.rail) && !isWaiting(s.rail),
};

int _rank(RailState r) => isWaiting(r)
    ? 0
    : isRunning(r)
    ? 1
    : 2;

List<MonitorSession> flattenSessions(List<MonitorProject> projects) {
  final all = [for (final p in projects) ...p.sessions];
  final indexed = [for (var i = 0; i < all.length; i++) (i, all[i])];
  indexed.sort((a, b) {
    final r = _rank(a.$2.rail).compareTo(_rank(b.$2.rail));
    return r != 0 ? r : a.$1.compareTo(b.$1);
  });
  return [for (final e in indexed) e.$2];
}

List<(RailState, List<MonitorSession>)> groupByState(
  List<MonitorSession> sessions,
) {
  final out = <(RailState, List<MonitorSession>)>[];
  for (final r in const [
    RailState.failed,
    RailState.asked,
    RailState.working,
    RailState.done,
    RailState.idle,
  ]) {
    final mine = [
      for (final s in sessions)
        if (_bucket(s.rail) == r) s,
    ];
    if (mine.isNotEmpty) out.add((r, mine));
  }
  return out;
}

RailState _bucket(RailState r) => switch (r) {
  RailState.subagents => RailState.working,
  RailState.starting || RailState.ended || RailState.parked => RailState.idle,
  _ => r,
};
