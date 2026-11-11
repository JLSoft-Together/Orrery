import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../model/workspace_model.dart' show baseName;
import '../theme/orrery_theme.dart';
import '../widgets/common.dart';
import 'async_panel.dart';
import '../workspace/machine_scope.dart';
import 'file_page.dart';
import 'fuzzy.dart';
import 'viewer_api.dart';
import 'viewer_models.dart';

class ExplorerPage extends ConsumerStatefulWidget {
  const ExplorerPage({super.key, required this.machineId, required this.root});

  final String machineId;
  final String root;

  @override
  ConsumerState<ExplorerPage> createState() => _ExplorerPageState();
}

class _ExplorerPageState extends ConsumerState<ExplorerPage> {
  final Map<String, List<DirEntry>> _children = {};
  final Set<String> _expanded = {};
  final Set<String> _busy = {};
  Object? _rootError;
  Map<String, String> _marks = const {};
  Set<String> _dirtyDirs = const {};
  bool _searching = false;
  final TextEditingController _query = TextEditingController();
  List<String>? _files;
  bool _filesTruncated = false;
  StreamSubscription<JsonMap>? _sub;
  Timer? _debounce;
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));

  @override
  void initState() {
    super.initState();
    _api.watch('fs', widget.root);
    _api.watch('git', widget.root);
    _sub = _api.events.listen((event) {
      if (changedUnder(event, 'fs.changed', widget.root) ||
          changedUnder(event, 'git.changed', widget.root)) {
        _debounce?.cancel();
        _debounce = Timer(const Duration(milliseconds: 400), _refresh);
      }
    });
    _loadDir(widget.root);
    _loadStatus();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _sub?.cancel();
    _api.unwatch('fs', widget.root);
    _api.unwatch('git', widget.root);
    _query.dispose();
    super.dispose();
  }

  Future<void> _refresh() async {
    _files = null;
    if (_searching) _loadFiles();
    for (final dir in [widget.root, ..._expanded]) {
      await _loadDir(dir);
    }
    await _loadStatus();
  }

  Future<void> _loadDir(String dir) async {
    if (mounted) setState(() => _busy.add(dir));
    try {
      final reply = await _api.call('fs.list', {
        'root': widget.root,
        'dir': dir,
      });
      if (!mounted) return;
      setState(() {
        _children[dir] = DirEntry.parseAll(reply.data);
        if (dir == widget.root) _rootError = null;
      });
    } on Object catch (error) {
      if (!mounted) return;
      if (dir == widget.root) setState(() => _rootError = error);
    } finally {
      if (mounted) setState(() => _busy.remove(dir));
    }
  }

  Future<void> _loadStatus() async {
    try {
      final reply = await _api.call('git.status', {'root': widget.root});
      final status = ChangesStatus.parse(reply.map);
      final marks = <String, String>{};
      final dirs = <String>{};
      final rootKey = normPath(widget.root);
      for (final entry in status.entries) {
        final full = normPath(joinPath(status.root, entry.path));
        marks[full] = entry.mark;
        var at = full.lastIndexOf('/');
        while (at > rootKey.length) {
          final dir = full.substring(0, at);
          if (!dirs.add(dir)) break;
          at = dir.lastIndexOf('/');
        }
      }
      if (mounted) {
        setState(() {
          _marks = marks;
          _dirtyDirs = dirs;
        });
      }
    } on Object {
      if (mounted) {
        setState(() {
          _marks = const {};
          _dirtyDirs = const {};
        });
      }
    }
  }

  Future<void> _loadFiles() async {
    try {
      final reply = await _api.call('fs.files', {'root': widget.root});
      final raw = reply.map['files'];
      if (!mounted) return;
      setState(() {
        _files = [
          if (raw is List)
            for (final f in raw)
              if (f is String) f,
        ];
        _filesTruncated = reply.map['truncated'] == true;
      });
    } on Object {
      if (mounted) setState(() => _files = const []);
    }
  }

  void _toggle(DirEntry entry) {
    if (_expanded.contains(entry.path)) {
      setState(() => _expanded.remove(entry.path));
      return;
    }
    setState(() => _expanded.add(entry.path));
    if (!_children.containsKey(entry.path)) _loadDir(entry.path);
  }

  void _open(String path) {
    Navigator.of(context).push(
      machineRoute<void>(
        widget.machineId,
        (_) => FilePage(
          machineId: widget.machineId,
          root: widget.root,
          path: path,
        ),
      ),
    );
  }

  void _startSearch() {
    setState(() => _searching = true);
    if (_files == null) _loadFiles();
  }

  void _stopSearch() {
    _query.clear();
    setState(() => _searching = false);
  }

  List<(DirEntry, int)> _visible(String dir, int depth) {
    final rows = <(DirEntry, int)>[];
    for (final entry in _children[dir] ?? const <DirEntry>[]) {
      rows.add((entry, depth));
      if (entry.directory && _expanded.contains(entry.path)) {
        rows.addAll(_visible(entry.path, depth + 1));
      }
    }
    return rows;
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(
        titleSpacing: _searching ? 0 : null,
        leading: _searching
            ? IconButton(
                icon: const Icon(Icons.arrow_back_rounded),
                onPressed: _stopSearch,
              )
            : null,
        title: _searching
            ? TextField(
                key: const ValueKey('explorer-search'),
                controller: _query,
                autofocus: true,
                onChanged: (_) => setState(() {}),
                decoration: InputDecoration(
                  hintText: l.vFindFile,
                  border: InputBorder.none,
                ),
              )
            : Text(baseName(widget.root)),
        actions: [
          if (!_searching)
            IconButton(
              key: const ValueKey('explorer-search-button'),
              tooltip: l.vFindFile,
              icon: const Icon(Icons.search_rounded),
              onPressed: _startSearch,
            ),
        ],
      ),
      body: _searching ? _searchBody(context) : _treeBody(context),
    );
  }

  Widget _searchBody(BuildContext context) {
    final l = context.l10n;
    final files = _files;
    if (files == null) return const Center(child: CircularProgressIndicator());
    final query = _query.text.trim();
    if (query.isEmpty) {
      return EmptyState(icon: Icons.search_rounded, text: l.vFindFileHint);
    }
    final hits = fuzzyFilter(files, query);
    if (hits.isEmpty) {
      return EmptyState(icon: Icons.search_off_rounded, text: l.vNoMatches);
    }
    return ListView.builder(
      itemCount: hits.length + (_filesTruncated ? 1 : 0),
      itemBuilder: (context, index) {
        if (index == hits.length) {
          return Padding(
            padding: const EdgeInsets.all(16),
            child: Text(l.vListTruncated),
          );
        }
        final rel = hits[index];
        final slash = rel.lastIndexOf('/');
        return ListTile(
          dense: true,
          leading: fileIcon(rel, Theme.of(context).colorScheme),
          title: Text(rel.substring(slash + 1)),
          subtitle: slash > 0 ? Text(rel.substring(0, slash)) : null,
          onTap: () => _open(joinPath(widget.root, rel)),
        );
      },
    );
  }

  Widget _treeBody(BuildContext context) {
    final l = context.l10n;
    if (_rootError != null && !_children.containsKey(widget.root)) {
      return ErrorState(
        message: viewerErrorText(l, _rootError!),
        onRetry: () => _loadDir(widget.root),
      );
    }
    if (!_children.containsKey(widget.root)) {
      return const Center(child: CircularProgressIndicator());
    }
    final rows = _visible(widget.root, 0);
    if (rows.isEmpty) {
      return EmptyState(icon: Icons.folder_open_rounded, text: l.vFolderEmpty);
    }
    return RefreshIndicator(
      onRefresh: _refresh,
      child: ListView.builder(
        key: const ValueKey('explorer-list'),
        physics: const AlwaysScrollableScrollPhysics(),
        itemCount: rows.length,
        itemBuilder: (context, index) {
          final (entry, depth) = rows[index];
          return _Row(
            entry: entry,
            depth: depth,
            expanded: _expanded.contains(entry.path),
            loading: _busy.contains(entry.path),
            mark: _marks[normPath(entry.path)],
            dirty: entry.directory && _dirtyDirs.contains(normPath(entry.path)),
            onTap: () => entry.directory ? _toggle(entry) : _open(entry.path),
          );
        },
      ),
    );
  }
}

Widget fileIcon(String name, ColorScheme scheme) {
  final ext = name.contains('.') ? name.split('.').last.toLowerCase() : '';
  final icon = switch (ext) {
    'dart' || 'kt' || 'rs' || 'ts' || 'js' || 'py' || 'java' || 'swift' => Icons.code_rounded,
    'json' || 'yaml' || 'yml' || 'toml' || 'xml' => Icons.data_object_rounded,
    'md' || 'txt' => Icons.article_outlined,
    'png' || 'jpg' || 'jpeg' || 'webp' || 'svg' || 'gif' => Icons.image_outlined,
    _ => Icons.insert_drive_file_outlined,
  };
  return Icon(icon, size: 22, color: scheme.onSurfaceVariant);
}

class _Row extends StatelessWidget {
  const _Row({
    required this.entry,
    required this.depth,
    required this.expanded,
    required this.loading,
    required this.mark,
    required this.dirty,
    required this.onTap,
  });

  final DirEntry entry;
  final int depth;
  final bool expanded;
  final bool loading;
  final String? mark;
  final bool dirty;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    final markColor = switch (mark) {
      'A' || '?' => colors.done,
      'D' || '!' => colors.failed,
      _ => colors.warning,
    };
    final dim = entry.outOfRoot;
    return InkWell(
      onTap: dim ? null : onTap,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 44),
        child: Padding(
          padding: EdgeInsets.fromLTRB(12 + depth * 18.0, 0, 12, 0),
          child: Row(
            children: [
              SizedBox(
                width: 22,
                child: entry.directory
                    ? AnimatedRotation(
                        turns: expanded ? 0.25 : 0,
                        duration: motion(context, 160),
                        child: Icon(
                          Icons.chevron_right_rounded,
                          size: 20,
                          color: scheme.onSurfaceVariant,
                        ),
                      )
                    : null,
              ),
              entry.directory
                  ? Icon(
                      expanded
                          ? Icons.folder_open_rounded
                          : Icons.folder_rounded,
                      size: 22,
                      color: scheme.primary,
                    )
                  : fileIcon(entry.name, scheme),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  entry.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: 15,
                    color: dim ? scheme.onSurfaceVariant : null,
                    fontStyle: dim ? FontStyle.italic : null,
                  ),
                ),
              ),
              if (loading)
                const SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              else if (mark != null)
                Semantics(
                  label: mark,
                  child: Text(
                    mark!,
                    style: TextStyle(
                      color: markColor,
                      fontWeight: FontWeight.w800,
                      fontSize: 13,
                    ),
                  ),
                )
              else if (dirty)
                Container(
                  width: 7,
                  height: 7,
                  decoration: BoxDecoration(
                    color: colors.warning,
                    shape: BoxShape.circle,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
