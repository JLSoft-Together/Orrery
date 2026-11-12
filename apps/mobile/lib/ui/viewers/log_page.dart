import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../theme/orrery_theme.dart';
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import 'async_panel.dart';
import 'branches_page.dart';
import 'commit_page.dart';
import 'log_models.dart';
import 'viewer_api.dart';

String relativeTime(int seconds) {
  final then = DateTime.fromMillisecondsSinceEpoch(seconds * 1000);
  final diff = DateTime.now().difference(then);
  if (diff.inMinutes < 1) return '<1m';
  if (diff.inHours < 1) return '${diff.inMinutes}m';
  if (diff.inDays < 1) return '${diff.inHours}h';
  if (diff.inDays < 14) return '${diff.inDays}d';
  return '${then.year}-${then.month.toString().padLeft(2, '0')}-${then.day.toString().padLeft(2, '0')}';
}

class LogPage extends ConsumerStatefulWidget {
  const LogPage({super.key, required this.machineId, required this.root});

  final String machineId;
  final String root;

  @override
  ConsumerState<LogPage> createState() => _LogPageState();
}

class _LogPageState extends ConsumerState<LogPage> {
  final List<LogCommit> _commits = [];
  final ScrollController _scroll = ScrollController();
  bool _more = false;
  bool _loading = true;
  Object? _error;
  StreamSubscription<JsonMap>? _sub;
  Timer? _debounce;
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));
  int _gen = 0;

  @override
  void initState() {
    super.initState();
    _api.watch('git', widget.root);
    _sub = _api.events.listen((event) {
      if (changedUnder(event, 'git.changed', widget.root)) {
        _debounce?.cancel();
        _debounce = Timer(const Duration(milliseconds: 500), _reload);
      }
    });
    _scroll.addListener(() {
      if (_more && !_loading && _scroll.position.extentAfter < 600) {
        _load(append: true);
      }
    });
    _reload();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _sub?.cancel();
    _scroll.dispose();
    _api.unwatch('git', widget.root);
    super.dispose();
  }

  Future<void> _reload() => _load(append: false);

  Future<void> _load({required bool append}) async {
    final gen = ++_gen;
    setState(() => _loading = true);
    try {
      final reply = await _api.call('log.page', {
        'root': widget.root,
        'query': {
          'branch': null,
          'notRef': null,
          'author': null,
          'since': null,
          'until': null,
          'text': null,
          'regex': false,
          'matchCase': false,
          'paths': <String>[],
          'skip': append ? _commits.length : 0,
        },
      });
      if (!mounted || gen != _gen) return;
      final page = LogPageData.parse(reply.map);
      setState(() {
        if (!append) _commits.clear();
        _commits.addAll(page.commits);
        _more = page.more;
        _error = null;
        _loading = false;
      });
    } on Object catch (error) {
      if (!mounted || gen != _gen) return;
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  void _openCommit(LogCommit commit) {
    Navigator.of(context).push(
      machineRoute<void>(
        widget.machineId,
        (_) => CommitPage(
          machineId: widget.machineId,
          root: widget.root,
          hash: commit.hash,
        ),
      ),
    );
  }

  void _openBranches() {
    Navigator.of(context).push(
      machineRoute<void>(
        widget.machineId,
        (_) => BranchesPage(machineId: widget.machineId, root: widget.root),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(
        title: Text(l.vGitLog),
        actions: [
          IconButton(
            key: const ValueKey('open-branches'),
            tooltip: l.vBranches,
            icon: const Icon(Icons.call_split_rounded),
            onPressed: _openBranches,
          ),
        ],
      ),
      body: _body(context),
    );
  }

  Widget _body(BuildContext context) {
    final l = context.l10n;
    if (_commits.isEmpty) {
      if (_loading) return const Center(child: CircularProgressIndicator());
      if (_error != null) {
        return ErrorState(
          message: viewerErrorText(l, _error!),
          onRetry: _reload,
        );
      }
      return EmptyState(icon: Icons.history_rounded, text: l.vNoCommits);
    }
    return RefreshIndicator(
      onRefresh: _reload,
      child: ListView.builder(
        key: const ValueKey('log-list'),
        controller: _scroll,
        physics: const AlwaysScrollableScrollPhysics(),
        itemCount: _commits.length + (_more || _error != null ? 1 : 0),
        itemBuilder: (context, index) {
          if (index == _commits.length) {
            return Padding(
              padding: const EdgeInsets.all(16),
              child: _error != null
                  ? TextButton(
                      onPressed: () => _load(append: true),
                      child: Text(l.vRetry),
                    )
                  : const Center(
                      child: SizedBox(
                        width: 20,
                        height: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                    ),
            );
          }
          return CommitRow(
            commit: _commits[index],
            first: index == 0,
            last: index == _commits.length - 1 && !_more,
            onTap: () => _openCommit(_commits[index]),
          );
        },
      ),
    );
  }
}

class CommitRow extends StatelessWidget {
  const CommitRow({
    super.key,
    required this.commit,
    required this.first,
    required this.last,
    required this.onTap,
  });

  final LogCommit commit;
  final bool first;
  final bool last;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final merge = commit.parents.length > 1;
    final decorations = commit.decorations;
    return InkWell(
      onTap: onTap,
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(
              width: 36,
              child: CustomPaint(
                painter: _GraphPainter(
                  color: scheme.primary,
                  background: scheme.surface,
                  first: first,
                  last: last,
                  merge: merge,
                ),
              ),
            ),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(0, 10, 12, 10),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      commit.subject,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    if (decorations.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 4),
                        child: Wrap(
                          spacing: 4,
                          runSpacing: 4,
                          children: [
                            for (final d in decorations.take(4))
                              _RefChip(label: d),
                          ],
                        ),
                      ),
                    const SizedBox(height: 4),
                    Text(
                      '${commit.shortHash} · ${commit.author} · ${relativeTime(commit.time)}',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _RefChip extends StatelessWidget {
  const _RefChip({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final head = label.startsWith('HEAD');
    final tag = label.startsWith('tag:');
    final color = head
        ? scheme.primary
        : tag
        ? context.statusColors.warning
        : scheme.onSurfaceVariant;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label.replaceFirst('HEAD -> ', ''),
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: color,
        ),
      ),
    );
  }
}

class _GraphPainter extends CustomPainter {
  const _GraphPainter({
    required this.color,
    required this.background,
    required this.first,
    required this.last,
    required this.merge,
  });

  final Color color;
  final Color background;
  final bool first;
  final bool last;
  final bool merge;

  @override
  void paint(Canvas canvas, Size size) {
    final x = size.width / 2;
    const dotY = 22.0;
    final line = Paint()
      ..color = color.withValues(alpha: 0.5)
      ..strokeWidth = 2;
    if (!first) canvas.drawLine(Offset(x, 0), Offset(x, dotY), line);
    if (!last) canvas.drawLine(Offset(x, dotY), Offset(x, size.height), line);
    if (merge) {
      canvas.drawCircle(Offset(x, dotY), 6, Paint()..color = background);
      canvas.drawCircle(
        Offset(x, dotY),
        6,
        Paint()
          ..color = color
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2.5,
      );
    } else {
      canvas.drawCircle(Offset(x, dotY), 5, Paint()..color = color);
    }
  }

  @override
  bool shouldRepaint(_GraphPainter old) =>
      old.color != color ||
      old.first != first ||
      old.last != last ||
      old.merge != merge ||
      old.background != background;
}
