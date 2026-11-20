import 'package:flutter/material.dart';

import '../../remote/chat_protocol.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_markdown_view.dart' show chatMono;

class FilesCard extends StatelessWidget {
  const FilesCard({super.key, required this.item, required this.onOpen});

  final ChatFiles item;
  final ValueChanged<ChatFile> onOpen;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final count = item.files.length + item.more;
    return ClipRRect(
      borderRadius: BorderRadius.circular(22),
      child: ColoredBox(
        color: c.raised,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(
              height: 50,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 18),
                child: Row(
                  children: [
                    Icon(Icons.edit_outlined, size: 17, color: c.inkMuted),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        l.chatFilesEdited(count),
                        style: TextStyle(
                          fontSize: 14.5,
                          fontWeight: FontWeight.w500,
                          color: c.ink,
                        ),
                      ),
                    ),
                    DiffStat(add: item.added, del: item.removed),
                  ],
                ),
              ),
            ),
            for (final file in item.files)
              _FileRow(
                file: file,
                onTap: file.outside ? null : () => onOpen(file),
              ),
            if (item.more > 0)
              Padding(
                padding: const EdgeInsets.fromLTRB(18, 10, 18, 12),
                child: Text(
                  l.chatMoreFiles(item.more),
                  style: TextStyle(fontSize: 13, color: c.inkMuted),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _FileRow extends StatelessWidget {
  const _FileRow({required this.file, this.onTap});

  final ChatFile file;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final path = file.path;
    final cut = path.lastIndexOf('/');
    final dir = cut < 0 ? '' : path.substring(0, cut + 1);
    final name = cut < 0 ? path : path.substring(cut + 1);
    return InkWell(
      key: ValueKey('chat-file-$path'),
      onTap: onTap,
      child: Container(
        constraints: const BoxConstraints(minHeight: 46),
        padding: const EdgeInsets.fromLTRB(18, 0, 14, 0),
        decoration: BoxDecoration(
          border: Border(top: BorderSide(color: c.line)),
        ),
        child: Row(
          children: [
            Expanded(
              child: Text.rich(
                TextSpan(
                  style: TextStyle(
                    fontFamily: chatMono,
                    fontSize: 13,
                    color: c.ink,
                  ),
                  children: [
                    TextSpan(
                      text: dir,
                      style: TextStyle(color: c.inkMuted),
                    ),
                    TextSpan(text: name),
                  ],
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const SizedBox(width: 10),
            DiffStat(add: file.add, del: file.del),
            const SizedBox(width: 6),
            Icon(
              Icons.chevron_right_rounded,
              size: 18,
              color: onTap == null ? c.line : c.inkMuted,
            ),
          ],
        ),
      ),
    );
  }
}

class DiffStat extends StatelessWidget {
  const DiffStat({super.key, this.add, this.del});

  final int? add;
  final int? del;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    const base = TextStyle(fontFamily: chatMono, fontSize: 12.5);
    return Text.rich(
      TextSpan(
        children: [
          if (add != null && add! > 0)
            TextSpan(
              text: '+$add',
              style: base.copyWith(color: c.diffAdd),
            ),
          if (add != null && add! > 0 && del != null && del! > 0)
            const TextSpan(text: ' '),
          if (del != null && del! > 0)
            TextSpan(
              text: '-$del',
              style: base.copyWith(color: c.diffDel),
            ),
        ],
      ),
    );
  }
}
