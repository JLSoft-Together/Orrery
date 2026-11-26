import 'dart:async';

import 'away_store.dart';
import 'away_summary.dart';

class AwayFeed {
  const AwayFeed({required this.snapshot, required this.fresh});

  final Map<String, dynamic>? snapshot;
  final bool fresh;
}

class AwayTracker {
  AwayTracker({
    required this.store,
    required this.source,
    DateTime Function()? now,
  }) : _now = now ?? DateTime.now;

  final AwayStore store;
  final Map<String, AwayFeed> Function() source;
  final DateTime Function() _now;

  final Map<String, AwaySummary> summaries = {};
  final Map<String, AwayMark> _pending = {};
  Future<void> _io = Future.value();
  bool _left = false;

  Set<String> get pendingIds => _pending.keys.toSet();

  Future<T> _queued<T>(Future<T> Function() job) {
    final next = _io.then((_) => job());
    _io = next.then<void>((_) {}, onError: (_) {});
    return next;
  }

  Future<void> leave() {
    if (_left) return _io;
    _left = true;
    summaries.clear();
    final at = _now();
    final marks = {
      for (final e in source().entries) e.key: markFromSnapshot(e.value.snapshot, at),
    };
    return _queued(() => store.save(marks));
  }

  Future<bool> back() => _queued(() async {
    _left = false;
    final now = _now();
    final stored = await store.load();
    var changed = false;
    for (final e in stored.entries) {
      if (awayDue(e.value, now)) {
        _pending[e.key] = e.value;
      } else {
        changed = true;
      }
    }
    if (changed || stored.length != _pending.length) {
      await store.save(Map.of(_pending));
    }
    return _evaluate();
  });

  Future<bool> evaluate() => _queued(() async => _evaluate());

  Future<bool> _evaluate() async {
    if (_pending.isEmpty) return false;
    final feeds = source();
    final now = _now();
    var changed = false;
    for (final id in _pending.keys.toList()) {
      final feed = feeds[id];
      if (feed == null || !feed.fresh || feed.snapshot == null) continue;
      final summary = computeAway(
        snapshot: feed.snapshot,
        mark: _pending.remove(id)!,
        now: now,
      );
      if (summary != null) summaries[id] = summary;
      changed = true;
    }
    if (changed) await store.save(Map.of(_pending));
    return changed;
  }

  bool dismiss(String machineId) => summaries.remove(machineId) != null;
}
