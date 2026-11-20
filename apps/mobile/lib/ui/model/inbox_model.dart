import 'workspace_model.dart';

const List<String> inboxGroups = [
  'menu',
  'permission',
  'question',
  'clash',
  'error',
  'note',
  'done',
];

const List<String> inboxReasons = [
  'menu',
  'permission',
  'question',
  'clash',
  'failed',
  'loop',
  'checks',
  'stuck',
  'note',
  'done',
];

const int inboxTextMax = 120;

String groupOf(String reason) => switch (reason) {
  'failed' || 'loop' || 'checks' || 'stuck' => 'error',
  _ => reason,
};

class InboxInfo {
  const InboxInfo({required this.reason, this.since, this.file, this.peer});

  final String reason;
  final String? since;
  final String? file;
  final String? peer;

  static InboxInfo? parse(Object? raw) {
    if (raw is! Map) return null;
    final reason = raw['reason'];
    if (reason is! String || !inboxReasons.contains(reason)) return null;
    String? text(String name) {
      final value = raw[name];
      if (value is! String || value.isEmpty) return null;
      return value.length > inboxTextMax
          ? value.substring(0, inboxTextMax)
          : value;
    }

    return InboxInfo(
      reason: reason,
      since: text('since'),
      file: text('file'),
      peer: text('peer'),
    );
  }
}

class InboxRow {
  const InboxRow({
    required this.tab,
    required this.pane,
    required this.reason,
    this.since,
    this.file,
    this.peer,
    this.text,
  });

  final TabInfo tab;
  final PaneInfo pane;
  final String reason;
  final DateTime? since;
  final String? file;
  final String? peer;
  final String? text;

  String get group => groupOf(reason);
}

List<InboxRow> buildInboxRows(WorkspaceModel model) {
  final rows = <InboxRow>[];
  for (final tab in model.tabs) {
    for (final pane in tab.panes) {
      final row = model.inboxMode
          ? _fromInbox(tab, pane)
          : _fromAttention(tab, pane);
      if (row != null) rows.add(row);
    }
  }
  final indexed = rows.asMap().entries.toList()
    ..sort((x, y) {
      final byGroup = inboxGroups
          .indexOf(x.value.group)
          .compareTo(inboxGroups.indexOf(y.value.group));
      if (byGroup != 0) return byGroup;
      final a = x.value.since;
      final b = y.value.since;
      if (a != null && b != null) {
        final byTime = a.compareTo(b);
        if (byTime != 0) return byTime;
      } else if (a != null) {
        return -1;
      } else if (b != null) {
        return 1;
      }
      return x.key.compareTo(y.key);
    });
  return [for (final e in indexed) e.value];
}

List<(String, List<InboxRow>)> groupInboxRows(List<InboxRow> rows) => [
  for (final group in inboxGroups)
    if (rows.any((r) => r.group == group))
      (
        group,
        [
          for (final r in rows)
            if (r.group == group) r,
        ],
      ),
];

InboxRow? _fromInbox(TabInfo tab, PaneInfo pane) {
  final info = pane.inbox;
  if (info == null) return null;
  final since = info.since == null ? null : DateTime.tryParse(info.since!);
  return InboxRow(
    tab: tab,
    pane: pane,
    reason: info.reason,
    since: since,
    file: info.file,
    peer: info.peer,
  );
}

InboxRow? _fromAttention(TabInfo tab, PaneInfo pane) {
  final String reason;
  if (pane.rail == RailState.failed) {
    reason = 'failed';
  } else if (pane.rail == RailState.asked) {
    reason = 'permission';
  } else if (pane.isWaiting) {
    reason = 'question';
  } else if (pane.isDone) {
    reason = 'done';
  } else {
    return null;
  }
  return InboxRow(
    tab: tab,
    pane: pane,
    reason: reason,
    text: pane.isAsked ? pane.waitText : null,
  );
}
