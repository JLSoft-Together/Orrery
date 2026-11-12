import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import 'async_panel.dart';
import 'changes_widgets.dart';
import 'diff_page.dart';
import 'viewer_actions.dart';
import 'viewer_api.dart';
import 'viewer_models.dart';

class ChangesPage extends ConsumerStatefulWidget {
  const ChangesPage({super.key, required this.machineId, required this.root});

  final String machineId;
  final String root;

  @override
  ConsumerState<ChangesPage> createState() => _ChangesPageState();
}

class _ChangesPageState extends ConsumerState<ChangesPage> {
  final TextEditingController _message = TextEditingController();
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));
  bool _committing = false;

  @override
  void initState() {
    super.initState();
    _api.watch('git', widget.root);
  }

  @override
  void dispose() {
    _api.unwatch('git', widget.root);
    _message.dispose();
    super.dispose();
  }

  Future<void> _stage(String type, List<String> paths, Reload reload) =>
      viewerAction(context, ref, widget.machineId, type, {
        'root': widget.root,
        'paths': paths,
      }, after: reload);

  Future<void> _discard(ChangeEntry entry, Reload reload) => viewerAction(
    context,
    ref,
    widget.machineId,
    'git.discard',
    {'root': widget.root, 'path': entry.path},
    after: reload,
  );

  Future<void> _commit(Reload reload) async {
    final text = _message.text.trim();
    if (text.isEmpty) return;
    setState(() => _committing = true);
    await viewerAction(
      context,
      ref,
      widget.machineId,
      'git.commit',
      {'root': widget.root, 'message': text},
      success: context.l10n.vCommitted,
      after: () {
        _message.clear();
        reload();
      },
    );
    if (mounted) setState(() => _committing = false);
  }

  void _openDiff(ChangeEntry entry) {
    Navigator.of(context).push(
      machineRoute<void>(
        widget.machineId,
        (_) => DiffPage(
          machineId: widget.machineId,
          path: entry.path,
          type: 'git.diff',
          fields: {'root': widget.root, 'path': entry.path},
          root: widget.root,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(title: Text(l.vChanges)),
      body: AsyncPanel<ChangesStatus>(
        load: () async => ChangesStatus.parse(
          await _api.map('git.status', {'root': widget.root}),
        ),
        events: _api.events,
        reloadOn: (event) => changedUnder(event, 'git.changed', widget.root),
        builder: (context, status, reload) => _content(context, status, reload),
      ),
    );
  }

  Widget _content(BuildContext context, ChangesStatus status, Reload reload) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final staged = status.staged;
    final unstaged = status.unstaged;
    final canCommit = staged.isNotEmpty && _message.text.trim().isNotEmpty;
    return Column(
      children: [
        Expanded(
          child: RefreshIndicator(
            onRefresh: reload,
            child: staged.isEmpty && unstaged.isEmpty
                ? ListView(
                    physics: const AlwaysScrollableScrollPhysics(),
                    children: [
                      const SizedBox(height: 40),
                      ChangesBranch(status: status),
                      const SizedBox(height: 60),
                      EmptyState(
                        icon: Icons.check_circle_outline_rounded,
                        text: l.vChangesClean,
                      ),
                    ],
                  )
                : ListView(
                    key: const ValueKey('changes-list'),
                    physics: const AlwaysScrollableScrollPhysics(),
                    padding: const EdgeInsets.only(bottom: 12),
                    children: [
                      ChangesBranch(status: status),
                      if (staged.isNotEmpty)
                        ChangesSection(
                          title: l.vStaged(staged.length),
                          actionLabel: l.vUnstageAll,
                          onAction: () => _stage('git.unstage', [
                            for (final e in staged) e.path,
                          ], reload),
                        ),
                      for (final e in staged)
                        ChangesFileRow(
                          entry: e,
                          staged: true,
                          onTap: () => _openDiff(e),
                          onToggle: () =>
                              _stage('git.unstage', [e.path], reload),
                        ),
                      if (unstaged.isNotEmpty)
                        ChangesSection(
                          title: l.vUnstaged(unstaged.length),
                          actionLabel: l.vStageAll,
                          onAction: () => _stage('git.stage', [
                            for (final e in unstaged) e.path,
                          ], reload),
                        ),
                      for (final e in unstaged)
                        ChangesFileRow(
                          entry: e,
                          staged: false,
                          onTap: () => _openDiff(e),
                          onToggle: () => _stage('git.stage', [e.path], reload),
                          onDiscard: () => _discard(e, reload),
                        ),
                      if (status.truncated)
                        Padding(
                          padding: const EdgeInsets.all(16),
                          child: Text(l.vListTruncated),
                        ),
                    ],
                  ),
          ),
        ),
        Material(
          color: scheme.surface,
          elevation: 2,
          child: SafeArea(
            top: false,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  Expanded(
                    child: TextField(
                      key: const ValueKey('commit-message'),
                      controller: _message,
                      minLines: 1,
                      maxLines: 4,
                      onChanged: (_) => setState(() {}),
                      decoration: InputDecoration(
                        hintText: l.vCommitMessage,
                        isDense: true,
                        filled: true,
                        fillColor: scheme.surfaceContainerHigh,
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: BorderSide.none,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  FilledButton(
                    key: const ValueKey('commit-button'),
                    onPressed: canCommit && !_committing
                        ? () => _commit(reload)
                        : null,
                    child: Text(l.vCommit),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}
