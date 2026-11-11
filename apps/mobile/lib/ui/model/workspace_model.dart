enum RailState {
  failed,
  asked,
  ended,
  working,
  subagents,
  done,
  starting,
  idle,
  parked,
}

RailState railOf(String? value) => switch (value) {
  'failed' => RailState.failed,
  'asked' => RailState.asked,
  'ended' => RailState.ended,
  'working' => RailState.working,
  'subagents' => RailState.subagents,
  'done' => RailState.done,
  'starting' => RailState.starting,
  'parked' => RailState.parked,
  _ => RailState.idle,
};

sealed class LayoutNode {
  const LayoutNode();

  static LayoutNode? parse(Object? raw) {
    if (raw is! Map) return null;
    final pane = raw['pane'];
    if (pane is String) return PaneLeaf(pane);
    final dir = raw['dir'];
    final a = parse(raw['a']);
    final b = parse(raw['b']);
    if ((dir != 'row' && dir != 'col') || a == null || b == null) return null;
    final ratio = raw['ratio'];
    final value = ratio is num ? ratio.toDouble() : 0.5;
    return SplitNode(
      dir == 'row' ? Axis2.row : Axis2.col,
      value.clamp(0.1, 0.9),
      a,
      b,
    );
  }

  List<String> get leaves;
}

enum Axis2 { row, col }

class PaneLeaf extends LayoutNode {
  const PaneLeaf(this.key);

  final String key;

  @override
  List<String> get leaves => [key];
}

class SplitNode extends LayoutNode {
  const SplitNode(this.axis, this.ratio, this.a, this.b);

  final Axis2 axis;
  final double ratio;
  final LayoutNode a;
  final LayoutNode b;

  @override
  List<String> get leaves => [...a.leaves, ...b.leaves];
}

class PaneInfo {
  const PaneInfo({
    required this.key,
    required this.kind,
    required this.title,
    required this.rail,
    required this.subagents,
    required this.dirty,
    required this.busy,
    this.preset,
    this.agent,
    this.cwd,
    this.branch,
    this.waitText,
    this.attention,
    this.sessionId,
    this.terminalId,
  });

  final String key;
  final String kind;
  final String title;
  final RailState rail;
  final int subagents;
  final bool dirty;
  final bool busy;
  final String? preset;
  final String? agent;
  final String? cwd;
  final String? branch;
  final String? waitText;
  final String? attention;
  final String? sessionId;
  final String? terminalId;

  bool get isTerm => kind == 'term';
  bool get isAsked => rail == RailState.asked;
  bool get needsUser => rail == RailState.asked || rail == RailState.failed;
  bool get isDone => attention == 'done' || rail == RailState.done;
  bool get isWaiting => attention == 'waiting';
  bool get canAttach => isTerm && sessionId != null;

  static PaneInfo? parse(Object? raw) {
    if (raw is! Map) return null;
    final key = raw['key'];
    if (key is! String) return null;
    String? text(String name) {
      final value = raw[name];
      return value is String && value.isNotEmpty ? value : null;
    }

    final count = raw['subagents'];
    return PaneInfo(
      key: key,
      kind: text('kind') ?? 'term',
      title: text('title') ?? key,
      rail: railOf(text('rail')),
      subagents: count is num ? count.toInt() : 0,
      dirty: raw['dirty'] == true,
      busy: raw['busy'] == true,
      preset: text('preset'),
      agent: text('agent'),
      cwd: text('cwd'),
      branch: text('branch'),
      waitText: text('waitText'),
      attention: text('attention'),
      sessionId: text('sessionId'),
      terminalId: text('terminalId'),
    );
  }
}

class TabInfo {
  const TabInfo({
    required this.key,
    required this.tree,
    required this.panes,
    this.name,
    this.project,
    this.zoom,
    this.folded = false,
  });

  final String key;
  final String? name;
  final String? project;
  final LayoutNode? tree;
  final String? zoom;
  final bool folded;
  final List<PaneInfo> panes;

  PaneInfo? pane(String paneKey) {
    for (final item in panes) {
      if (item.key == paneKey) return item;
    }
    return null;
  }

  String get displayName {
    final named = name;
    if (named != null && named.isNotEmpty) return named;
    final path = project;
    if (path != null) return baseName(path);
    return panes.isEmpty ? key : panes.first.title;
  }

  static TabInfo? parse(Object? raw) {
    if (raw is! Map) return null;
    final key = raw['key'];
    if (key is! String) return null;
    final byKey = <String, PaneInfo>{};
    final order = <String>[];
    final list = raw['panes'];
    if (list is List) {
      for (final item in list) {
        final pane = PaneInfo.parse(item);
        if (pane == null) continue;
        byKey[pane.key] = pane;
        order.add(pane.key);
      }
    }
    final tree = LayoutNode.parse(raw['tree']);
    final leaves = tree?.leaves ?? const <String>[];
    final ordered = <PaneInfo>[
      for (final k in leaves) ?byKey.remove(k),
      for (final k in order) ?byKey.remove(k),
    ];
    String? text(String name) {
      final value = raw[name];
      return value is String && value.isNotEmpty ? value : null;
    }

    return TabInfo(
      key: key,
      name: text('name'),
      project: text('project'),
      tree: tree,
      zoom: text('zoom'),
      folded: raw['folded'] == true,
      panes: ordered,
    );
  }
}

String baseName(String path) {
  final trimmed = path.replaceAll(RegExp(r'[\\/]+$'), '');
  final index = trimmed.lastIndexOf(RegExp(r'[\\/]'));
  return index < 0 ? trimmed : trimmed.substring(index + 1);
}

class ProjectInfo {
  const ProjectInfo({required this.id, required this.path, required this.tabs});

  final String id;
  final String? path;
  final List<TabInfo> tabs;

  String? get name {
    final value = path;
    if (value != null) return baseName(value);
    return null;
  }

  Iterable<PaneInfo> get panes => tabs.expand((t) => t.panes);

  int get paneCount => tabs.fold(0, (sum, t) => sum + t.panes.length);

  int get needsCount => panes.where((p) => p.needsUser).length;

  int get workingCount => panes
      .where(
        (p) => p.rail == RailState.working || p.rail == RailState.subagents,
      )
      .length;

  int get doneCount => panes.where((p) => p.isDone && !p.needsUser).length;
}

class WorkspaceModel {
  const WorkspaceModel({required this.tabs, required this.closedCount});

  static const WorkspaceModel empty = WorkspaceModel(tabs: [], closedCount: 0);

  final List<TabInfo> tabs;
  final int closedCount;

  static WorkspaceModel parse(Map<String, dynamic>? raw) {
    if (raw == null) return empty;
    final list = raw['tabs'];
    final tabs = <TabInfo>[
      if (list is List)
        for (final item in list) ?TabInfo.parse(item),
    ];
    final closed = raw['closedCount'];
    return WorkspaceModel(
      tabs: tabs,
      closedCount: closed is num ? closed.toInt() : 0,
    );
  }

  TabInfo? tab(String key) {
    for (final item in tabs) {
      if (item.key == key) return item;
    }
    return null;
  }

  TabInfo? tabOfPane(String paneKey) {
    for (final item in tabs) {
      if (item.pane(paneKey) != null) return item;
    }
    return null;
  }

  Iterable<PaneInfo> get panes => tabs.expand((t) => t.panes);

  List<ProjectInfo> get projects {
    final order = <String>[];
    final grouped = <String, List<TabInfo>>{};
    for (final tab in tabs) {
      final id = tab.project ?? '';
      if (!grouped.containsKey(id)) order.add(id);
      (grouped[id] ??= []).add(tab);
    }
    final list = [
      for (final id in order)
        ProjectInfo(id: id, path: id.isEmpty ? null : id, tabs: grouped[id]!),
    ];
    final indexed = list.asMap().entries.toList()
      ..sort((x, y) {
        final needs = y.value.needsCount.compareTo(x.value.needsCount);
        return needs != 0 ? needs : x.key.compareTo(y.key);
      });
    return [for (final e in indexed) e.value];
  }

  List<PaneInfo> get inbox {
    final entries = panes
        .where((p) => p.needsUser || p.isWaiting || p.isDone)
        .toList();
    int rank(PaneInfo p) => p.needsUser ? 0 : (p.isWaiting ? 1 : 2);
    entries.sort((a, b) => rank(a).compareTo(rank(b)));
    return entries;
  }
}
