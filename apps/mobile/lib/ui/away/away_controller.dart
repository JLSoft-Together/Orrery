import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/away_store.dart';
import '../../remote/away_summary.dart';
import '../../remote/away_tracker.dart';
import '../../remote/providers.dart';
import '../../remote/remote_host.dart';

final awayStoreProvider = Provider<AwayStore>((ref) => FileAwayStore());

class AwayNotifier extends Notifier<Map<String, AwaySummary>> {
  late AwayTracker _tracker;
  final Map<String, StreamSubscription<Object?>> _subs = {};

  @override
  Map<String, AwaySummary> build() {
    final RemoteHost host;
    try {
      host = ref.read(remoteHostProvider);
    } on Object {
      _tracker = AwayTracker(store: MemoryAwayStore(), source: () => {});
      return const {};
    }
    _tracker = AwayTracker(
      store: ref.read(awayStoreProvider),
      source: () {
        final out = <String, AwayFeed>{};
        for (final m in host.machines.all) {
          final view = host.bundle(m.id).store.view;
          out[m.id] = AwayFeed(
            snapshot: view.snapshot,
            fresh: !view.stale && !view.fromCache,
          );
        }
        return out;
      },
    );
    ref.onDispose(() {
      for (final s in _subs.values) {
        s.cancel();
      }
    });
    unawaited(back());
    return const {};
  }

  void _watchMachines() {
    final RemoteHost host;
    try {
      host = ref.read(remoteHostProvider);
    } on Object {
      return;
    }
    for (final m in host.machines.all) {
      if (_subs.containsKey(m.id)) continue;
      _subs[m.id] = host.bundle(m.id).store.changes.listen((_) {
        if (_tracker.pendingIds.isNotEmpty) unawaited(_evaluate());
      });
    }
  }

  void _sync() {
    if (ref.mounted) state = Map.of(_tracker.summaries);
  }

  Future<void> _evaluate() async {
    if (await _tracker.evaluate()) _sync();
  }

  Future<void> leave() async {
    final done = _tracker.leave();
    _sync();
    await done;
  }

  Future<void> back() async {
    await _tracker.back();
    _watchMachines();
    _sync();
  }

  void dismiss(String machineId) {
    if (_tracker.dismiss(machineId)) _sync();
    ref.read(awayFilterProvider(machineId).notifier).clear();
  }
}

final awayProvider =
    NotifierProvider<AwayNotifier, Map<String, AwaySummary>>(AwayNotifier.new);

class AwayFilterNotifier extends Notifier<AwayGroup?> {
  AwayFilterNotifier(this.machineId);

  final String machineId;

  @override
  AwayGroup? build() => null;

  void toggle(AwayGroup group) => state = state == group ? null : group;

  void clear() => state = null;
}

final awayFilterProvider =
    NotifierProvider.family<AwayFilterNotifier, AwayGroup?, String>(
      AwayFilterNotifier.new,
    );

final awayIdsProvider = Provider.family<Set<String>?, String>((ref, id) {
  final summary = ref.watch(awayProvider.select((m) => m[id]));
  final group = ref.watch(awayFilterProvider(id));
  if (summary == null || group == null) return null;
  return summary.idsOf(group).toSet();
});
