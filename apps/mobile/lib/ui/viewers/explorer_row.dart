import 'package:flutter/material.dart';

import '../theme/orrery_theme.dart';
import '../widgets/common.dart';
import 'viewer_models.dart';

Widget fileIcon(String name, ColorScheme scheme) {
  final ext = name.contains('.') ? name.split('.').last.toLowerCase() : '';
  final icon = switch (ext) {
    'dart' ||
    'kt' ||
    'rs' ||
    'ts' ||
    'js' ||
    'py' ||
    'java' ||
    'swift' => Icons.code_rounded,
    'json' || 'yaml' || 'yml' || 'toml' || 'xml' => Icons.data_object_rounded,
    'md' || 'txt' => Icons.article_outlined,
    'png' ||
    'jpg' ||
    'jpeg' ||
    'webp' ||
    'svg' ||
    'gif' => Icons.image_outlined,
    _ => Icons.insert_drive_file_outlined,
  };
  return Icon(icon, size: 22, color: scheme.onSurfaceVariant);
}

class ExplorerRow extends StatelessWidget {
  const ExplorerRow({
    super.key,
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
