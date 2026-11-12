import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../model/workspace_model.dart' show baseName;
import '../theme/orrery_theme.dart';
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import 'async_panel.dart';
import 'diff_page.dart';
import 'log_models.dart';
import 'log_page.dart' show relativeTime;
import 'viewer_api.dart';

class CommitPage extends ConsumerWidget {
  const CommitPage({
    super.key,
    required this.machineId,
    required this.root,
    required this.hash,
  });

  final String machineId;
  final String root;
  final String hash;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final api = ref.watch(viewerApiProvider(machineId));
    return Scaffold(
      appBar: AppBar(title: Text(l.vCommitDetail)),
      body: AsyncPanel<CommitDetail>(
        load: () async => CommitDetail.parse(
          await api.map('log.commit', {'root': root, 'hash': hash}),
        ),
        builder: (context, detail, reload) => _Body(
          detail: detail,
          onFile: (file) => Navigator.of(context).push(
            machineRoute<void>(
              machineId,
              (_) => DiffPage(
                machineId: machineId,
                path: file.path,
                type: 'log.diff',
                fields: {'root': root, 'hash': hash, 'path': file.path},
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Body extends StatelessWidget {
  const _Body({required this.detail, required this.onFile});

  final CommitDetail detail;
  final ValueChanged<CommitFile> onFile;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    return ListView(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SelectableText(
                detail.message,
                style: const TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                '${detail.hash.length > 7 ? detail.hash.substring(0, 7) : detail.hash} · ${detail.author} · ${relativeTime(detail.time)}',
                style: TextStyle(color: scheme.onSurfaceVariant, fontSize: 13),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
          child: Text(
            l.vFilesChanged(detail.files.length),
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w700,
              color: scheme.onSurfaceVariant,
            ),
          ),
        ),
        for (final file in detail.files)
          InkWell(
            onTap: file.binary ? null : () => onFile(file),
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 48),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Row(
                  children: [
                    Icon(
                      switch (file.kind) {
                        'added' => Icons.add_circle_outline_rounded,
                        'deleted' => Icons.remove_circle_outline_rounded,
                        'renamed' => Icons.drive_file_rename_outline_rounded,
                        _ => Icons.edit_outlined,
                      },
                      size: 20,
                      color: switch (file.kind) {
                        'added' => colors.done,
                        'deleted' => colors.failed,
                        _ => colors.warning,
                      },
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            baseName(file.path),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          if (file.path.contains('/'))
                            Text(
                              file.path.substring(
                                0,
                                file.path.lastIndexOf('/'),
                              ),
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
                    if (file.added != null || file.removed != null)
                      Text(
                        '+${file.added ?? 0} -${file.removed ?? 0}',
                        style: TextStyle(
                          fontSize: 11,
                          color: scheme.onSurfaceVariant,
                        ),
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
