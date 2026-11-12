import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../kit/kit.dart' show ChipTone, OrreryChip;
import '../model/workspace_model.dart' show baseName;
import '../widgets/common.dart';
import 'async_panel.dart';
import 'diff_view.dart';
import 'viewer_api.dart';

class DiffPage extends ConsumerWidget {
  const DiffPage({
    super.key,
    required this.machineId,
    required this.path,
    required this.type,
    required this.fields,
    this.root,
  });

  final String machineId;
  final String path;
  final String type;
  final JsonMap fields;
  final String? root;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final api = ref.watch(viewerApiProvider(machineId));
    return Scaffold(
      appBar: AppBar(
        titleSpacing: 0,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              baseName(path),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600),
            ),
            if (path.contains('/'))
              Text(
                path.substring(0, path.lastIndexOf('/')),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 12,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
          ],
        ),
        toolbarHeight: 60,
      ),
      body: AsyncPanel<JsonMap>(
        load: () => api.map(type, fields),
        events: root == null ? null : api.events,
        reloadOn: (event) =>
            root != null && changedUnder(event, 'git.changed', root!),
        builder: (context, data, reload) {
          if (data['binary'] == true) {
            return EmptyState(
              icon: Icons.image_not_supported_outlined,
              text: l.vDiffBinary,
            );
          }
          if (data['tooLarge'] == true) {
            return EmptyState(
              icon: Icons.unfold_more_rounded,
              text: l.vDiffTooLarge,
            );
          }
          final text = data['text'];
          if (text is! String || text.trim().isEmpty) {
            return EmptyState(
              icon: Icons.check_circle_outline_rounded,
              text: l.vDiffEmpty,
            );
          }
          final lines = parseDiff(text);
          final added = lines.where((e) => e.kind == DiffKind.add).length;
          final removed = lines.where((e) => e.kind == DiffKind.del).length;
          return Column(
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 4, 16, 10),
                child: Row(
                  children: [
                    OrreryChip(
                      key: const ValueKey('diff-added'),
                      tone: ChipTone.done,
                      icon: Icons.add_rounded,
                      label: l.diffAdded(added),
                    ),
                    const SizedBox(width: 8),
                    OrreryChip(
                      key: const ValueKey('diff-removed'),
                      tone: ChipTone.error,
                      icon: Icons.remove_rounded,
                      label: l.diffRemoved(removed),
                    ),
                  ],
                ),
              ),
              Expanded(child: DiffView(text: text)),
            ],
          );
        },
      ),
    );
  }
}
