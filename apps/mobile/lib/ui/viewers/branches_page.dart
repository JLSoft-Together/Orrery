import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../commands.dart';
import '../widgets/common.dart';
import 'async_panel.dart';
import 'log_models.dart';
import 'viewer_actions.dart';
import 'viewer_api.dart';

class BranchesPage extends ConsumerStatefulWidget {
  const BranchesPage({super.key, required this.machineId, required this.root});

  final String machineId;
  final String root;

  @override
  ConsumerState<BranchesPage> createState() => _BranchesPageState();
}

class _BranchesPageState extends ConsumerState<BranchesPage> {
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));
  bool _fetching = false;

  Future<void> _fetch(Reload reload) async {
    setState(() => _fetching = true);
    await viewerAction(
      context,
      ref,
      widget.machineId,
      'branch.fetch',
      {'root': widget.root},
      timeout: slowCommand,
      success: context.l10n.vFetched,
      after: reload,
    );
    if (mounted) setState(() => _fetching = false);
  }

  Future<void> _create(String? head, Reload reload) async {
    final l = context.l10n;
    final controller = TextEditingController();
    final name = await showDialog<String>(
      context: context,
      builder: (dialog) => AlertDialog(
        title: Text(l.vNewBranch),
        content: TextField(
          key: const ValueKey('new-branch-name'),
          controller: controller,
          autofocus: true,
          autocorrect: false,
          decoration: InputDecoration(
            labelText: l.vBranchName,
            helperText: head == null ? null : l.vBranchFrom(head),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialog),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialog, controller.text.trim()),
            child: Text(l.vCreate),
          ),
        ],
      ),
    );
    if (name == null || name.isEmpty || !mounted) return;
    await viewerAction(
      context,
      ref,
      widget.machineId,
      'branch.create',
      {'root': widget.root, 'name': name, 'base': head, 'checkout': false},
      success: l.vBranchCreated(name),
      after: reload,
    );
  }

  Future<void> _push(LocalRef ref0, Reload reload) async {
    final l = context.l10n;
    PushPreview preview;
    try {
      preview = PushPreview.parse(
        await _api.map('branch.push-preview', {
          'root': widget.root,
          'ref': ref0.ref,
        }),
      );
    } on Object catch (error) {
      if (mounted) toast(context, viewerErrorText(l, error));
      return;
    }
    if (!mounted) return;
    final go = await showDialog<bool>(
      context: context,
      builder: (dialog) => AlertDialog(
        title: Text(l.vPushTitle(preview.local)),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l.vPushTarget(preview.target),
              key: const ValueKey('push-target'),
              style: const TextStyle(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 4),
            Text(
              preview.exists
                  ? l.vPushOutgoing(preview.commits.length)
                  : l.vPushNewBranch,
            ),
            const SizedBox(height: 8),
            for (final c in preview.commits.take(5))
              Text(
                '${c.shortHash}  ${c.subject}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 12.5),
              ),
            if (preview.commits.length > 5 || preview.more)
              Text(l.vAndMore, style: const TextStyle(fontSize: 12.5)),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialog, false),
            child: Text(l.cancel),
          ),
          FilledButton(
            key: const ValueKey('push-go'),
            onPressed: () => Navigator.pop(dialog, true),
            child: Text(l.vPushAction),
          ),
        ],
      ),
    );
    if (go != true || !mounted) return;
    await viewerAction(
      context,
      ref,
      widget.machineId,
      'branch.push',
      {
        'root': widget.root,
        'ref': ref0.ref,
        'remote': preview.remote,
        'branch': preview.branch,
        'setUpstream': !preview.hasUpstream,
      },
      timeout: slowCommand,
      success: l.vPushed,
      after: reload,
    );
  }

  Future<void> _delete(LocalRef item, Reload reload) => viewerAction(
    context,
    ref,
    widget.machineId,
    'branch.delete',
    {'root': widget.root, 'ref': item.ref},
    timeout: slowCommand,
    after: reload,
  );

  Future<void> _update(LocalRef item, Reload reload) => viewerAction(
    context,
    ref,
    widget.machineId,
    'branch.update',
    {'root': widget.root, 'ref': item.ref},
    timeout: slowCommand,
    success: context.l10n.vUpdated,
    after: reload,
  );

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(title: Text(l.vBranches)),
      body: AsyncPanel<RefsInfo>(
        load: () async =>
            RefsInfo.parse(await _api.map('log.refs', {'root': widget.root})),
        events: _api.events,
        reloadOn: (event) => changedUnder(event, 'git.changed', widget.root),
        builder: _list,
      ),
    );
  }

  Widget _list(BuildContext context, RefsInfo info, Reload reload) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    Widget header(String text) => Padding(
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 4),
      child: Text(
        text,
        style: TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w700,
          color: scheme.onSurfaceVariant,
        ),
      ),
    );
    return RefreshIndicator(
      onRefresh: reload,
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Row(
              children: [
                OutlinedButton.icon(
                  key: const ValueKey('branch-fetch'),
                  onPressed: _fetching ? null : () => _fetch(reload),
                  icon: _fetching
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.cloud_download_outlined),
                  label: Text(l.vFetch),
                ),
                const SizedBox(width: 8),
                FilledButton.tonalIcon(
                  key: const ValueKey('branch-create'),
                  onPressed: () => _create(info.head, reload),
                  icon: const Icon(Icons.add_rounded),
                  label: Text(l.vNewBranch),
                ),
              ],
            ),
          ),
          header(l.vLocalBranches),
          for (final b in info.local)
            ListTile(
              key: ValueKey('branch-${b.name}'),
              leading: Icon(
                b.name == info.head
                    ? Icons.radio_button_checked_rounded
                    : Icons.call_split_rounded,
                color: b.name == info.head ? scheme.primary : null,
              ),
              title: Text(b.name),
              subtitle: b.upstream == null
                  ? null
                  : Text(
                      '${b.upstream}${b.ahead > 0 || b.behind > 0 ? '  ↑${b.ahead} ↓${b.behind}' : ''}${b.gone ? '  ${l.vUpstreamGone}' : ''}',
                    ),
              trailing: PopupMenuButton<String>(
                tooltip: l.more,
                onSelected: (value) => switch (value) {
                  'push' => _push(b, reload),
                  'update' => _update(b, reload),
                  _ => _delete(b, reload),
                },
                itemBuilder: (_) => [
                  PopupMenuItem(value: 'push', child: Text(l.vPushAction)),
                  PopupMenuItem(
                    value: 'update',
                    enabled: b.upstream != null && !b.gone,
                    child: Text(l.vUpdateBranch),
                  ),
                  PopupMenuItem(
                    value: 'delete',
                    enabled: b.name != info.head,
                    child: Text(
                      l.vDeleteBranch,
                      style: TextStyle(color: scheme.error),
                    ),
                  ),
                ],
              ),
            ),
          if (info.remotes.isNotEmpty) header(l.vRemoteBranches),
          for (final r in info.remotes)
            ListTile(
              dense: true,
              leading: const Icon(Icons.cloud_outlined),
              title: Text(r.$2),
            ),
          if (info.tags.isNotEmpty) header(l.vTags),
          for (final t in info.tags)
            ListTile(
              dense: true,
              leading: const Icon(Icons.sell_outlined),
              title: Text(t.$2),
            ),
        ],
      ),
    );
  }
}
