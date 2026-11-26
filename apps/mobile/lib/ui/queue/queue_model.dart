import '../model/workspace_model.dart';

enum QueuePhase { ready, sending, pendingUndo, snoozed, sent, stale, gone }

bool isQueueCandidate(PaneInfo pane) =>
    pane.isTerm && pane.rail == RailState.asked && !pane.snoozed;

int queueCount(WorkspaceModel model) =>
    model.panes.where(isQueueCandidate).length;

DateTime? queueSince(PaneInfo pane) {
  final raw = pane.inbox?.since;
  final parsed = raw == null ? null : DateTime.tryParse(raw);
  return parsed ?? pane.menu?.since;
}

String queueSignature(PaneInfo pane) {
  final menu = pane.menu;
  if (menu != null) return 'm:${menu.sig}';
  final since = queueSince(pane)?.millisecondsSinceEpoch ?? 0;
  return 'q:$since|${pane.question ?? pane.waitText ?? ''}';
}

class QueueSlot {
  QueueSlot({required this.pane, required this.tab, required this.firstSeen})
    : signature = queueSignature(pane),
      since = queueSince(pane);

  PaneInfo pane;
  TabInfo tab;
  String signature;
  DateTime? since;
  final DateTime firstSeen;
  QueuePhase phase = QueuePhase.ready;
  bool skipped = false;
  bool removed = false;
  bool ignoreSnoozed = false;
  String? pickedLabel;
  int snoozeMinutes = 0;

  String get key => pane.key;

  DateTime get waitingSince => since ?? firstSeen;

  bool get open => !removed && !skipped;

  bool get busy => phase == QueuePhase.sending || phase == QueuePhase.pendingUndo;

  bool get actionable => phase == QueuePhase.ready;
}

class QueueFlow {
  QueueFlow({DateTime Function()? now}) : _now = now ?? DateTime.now;

  final DateTime Function() _now;
  final Map<String, QueueSlot> _slots = {};
  int _retired = 0;

  List<QueueSlot> get slots => List.unmodifiable(_slots.values);

  List<QueueSlot> get open => [
    for (final slot in _slots.values)
      if (slot.open) slot,
  ];

  QueueSlot? get current => open.isEmpty ? null : open.first;

  int get total => _slots.length + _retired;

  int get position => (total - open.length + 1).clamp(1, total < 1 ? 1 : total);

  int get skippedCount => _slots.values
      .where((s) => s.skipped && !s.removed)
      .length;

  QueueSlot? slot(String paneKey) => _slots[paneKey];

  bool sync(WorkspaceModel model, {required bool live}) {
    if (!live) return false;
    var changed = false;
    final seen = <String>{};
    final fresh = <(TabInfo, PaneInfo)>[];
    for (final tab in model.tabs) {
      for (final pane in tab.panes) {
        final slot = _slots[pane.key];
        if (slot == null) {
          if (isQueueCandidate(pane)) fresh.add((tab, pane));
          continue;
        }
        seen.add(pane.key);
        if (_update(slot, tab, pane, fresh)) changed = true;
      }
    }
    for (final slot in _slots.values) {
      if (seen.contains(slot.key) || slot.removed) continue;
      if (slot.phase == QueuePhase.ready || slot.phase == QueuePhase.stale) {
        slot.phase = QueuePhase.gone;
        changed = true;
      }
    }
    final order = {for (var i = 0; i < fresh.length; i++) fresh[i].$2.key: i};
    fresh.sort((a, b) {
      final x = queueSince(a.$2);
      final y = queueSince(b.$2);
      if (x != null && y != null) {
        final byTime = x.compareTo(y);
        if (byTime != 0) return byTime;
      } else if (x != null) {
        return -1;
      } else if (y != null) {
        return 1;
      }
      return order[a.$2.key]!.compareTo(order[b.$2.key]!);
    });
    for (final (tab, pane) in fresh) {
      _slots[pane.key] = QueueSlot(pane: pane, tab: tab, firstSeen: _now());
      changed = true;
    }
    return changed;
  }

  bool _update(
    QueueSlot slot,
    TabInfo tab,
    PaneInfo pane,
    List<(TabInfo, PaneInfo)> fresh,
  ) {
    slot.tab = tab;
    slot.pane = pane;
    final asked = pane.rail == RailState.asked;
    switch (slot.phase) {
      case QueuePhase.ready:
        if (!pane.snoozed) slot.ignoreSnoozed = false;
        if (!asked || (pane.snoozed && !slot.ignoreSnoozed)) {
          slot.phase = QueuePhase.gone;
          return true;
        }
        if (!pane.snoozed && queueSignature(pane) != slot.signature) {
          slot.phase = QueuePhase.stale;
          return true;
        }
        return false;
      case QueuePhase.stale:
        if (!asked) {
          slot.phase = QueuePhase.gone;
          return true;
        }
        return false;
      case QueuePhase.sent:
      case QueuePhase.gone:
        if (slot.removed &&
            isQueueCandidate(pane) &&
            queueSignature(pane) != slot.signature) {
          _slots.remove(slot.key);
          _retired++;
          fresh.add((tab, pane));
          return true;
        }
        return false;
      case QueuePhase.sending:
      case QueuePhase.pendingUndo:
      case QueuePhase.snoozed:
        return false;
    }
  }

  void begin(String paneKey, QueuePhase phase) {
    final slot = _slots[paneKey];
    if (slot == null) return;
    slot.phase = phase;
    if (phase == QueuePhase.snoozed) slot.ignoreSnoozed = true;
  }

  void resume(String paneKey, {bool afterUnsnooze = false}) {
    final slot = _slots[paneKey];
    if (slot == null) return;
    slot.ignoreSnoozed = afterUnsnooze;
    final pane = slot.pane;
    if (pane.rail != RailState.asked) {
      slot.phase = QueuePhase.gone;
      return;
    }
    slot.phase = queueSignature(pane) != slot.signature && !pane.snoozed
        ? QueuePhase.stale
        : QueuePhase.ready;
  }

  void refresh(String paneKey) {
    final slot = _slots[paneKey];
    if (slot == null) return;
    final pane = slot.pane;
    if (pane.rail != RailState.asked) {
      slot.phase = QueuePhase.gone;
      return;
    }
    slot.signature = queueSignature(pane);
    slot.since = queueSince(pane);
    slot.phase = QueuePhase.ready;
  }

  void skip(String paneKey) => _slots[paneKey]?.skipped = true;

  void remove(String paneKey) => _slots[paneKey]?.removed = true;

  void reviewSkipped() {
    for (final slot in _slots.values) {
      if (slot.skipped) slot.skipped = false;
    }
  }
}
