import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../commands.dart';
import '../kit/kit.dart'
    show ButtonKind, ChipTone, NoticeCard, NoticeTone, OrreryButton, OrreryChip;
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'viewer_models.dart';

class FilePathLine extends StatelessWidget {
  const FilePathLine({super.key, required this.path});

  final String path;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 6),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Text(
          path,
          key: const ValueKey('file-path'),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            fontFamily: 'monospace',
            fontFamilyFallback: monoFallback,
            fontSize: 12,
            color: c.inkFaint,
          ),
        ),
      ),
    );
  }
}

class FileFooter extends StatelessWidget {
  const FileFooter({
    super.key,
    required this.lang,
    required this.lines,
    required this.editing,
    required this.dirty,
  });

  final String? lang;
  final int lines;
  final bool editing;
  final bool dirty;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final lang = this.lang;
    final parts = [
      if (lang != null && lang.isNotEmpty) lang,
      l.fileLines(lines),
      editing ? l.fileEditing : l.fileReadOnly,
    ];
    return DecoratedBox(
      decoration: BoxDecoration(
        color: c.surface,
        border: Border(top: BorderSide(color: c.line)),
      ),
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  parts.join(' · '),
                  key: const ValueKey('file-footer'),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(fontSize: 13, color: c.inkMuted),
                ),
              ),
              OrreryChip(
                key: const ValueKey('file-state'),
                tone: dirty ? ChipTone.waiting : ChipTone.done,
                icon: dirty ? Icons.edit_outlined : Icons.check_rounded,
                label: dirty ? l.fileUnsaved : l.vSaved,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class FileConflictBanner extends StatelessWidget {
  const FileConflictBanner({
    super.key,
    required this.mine,
    required this.onReload,
  });

  final String Function() mine;
  final VoidCallback onReload;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      child: Column(
        key: const ValueKey('conflict-banner'),
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          NoticeCard(tone: NoticeTone.warning, title: l.vFileChangedOnDisk),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(
                child: OrreryButton(
                  label: l.vCopyMine,
                  kind: ButtonKind.secondary,
                  onPressed: () =>
                      Clipboard.setData(ClipboardData(text: mine())).then((_) {
                        if (context.mounted) toast(context, l.vCopied);
                      }),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OrreryButton(
                  key: const ValueKey('conflict-reload'),
                  label: l.vReloadDiscard,
                  kind: ButtonKind.danger,
                  onPressed: onReload,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class FileEditor extends StatelessWidget {
  const FileEditor({
    super.key,
    required this.controller,
    required this.onChanged,
  });

  final TextEditingController controller;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return ColoredBox(
      color: c.well,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(8, 4, 8, 0),
        child: TextField(
          key: const ValueKey('file-editor'),
          controller: controller,
          maxLines: null,
          expands: true,
          textAlignVertical: TextAlignVertical.top,
          keyboardType: TextInputType.multiline,
          autocorrect: false,
          enableSuggestions: false,
          onChanged: (_) => onChanged(),
          style: TextStyle(
            fontFamily: 'monospace',
            fontFamilyFallback: monoFallback,
            fontSize: 13,
            height: 1.4,
            color: c.wellInk,
          ),
          cursorColor: c.accent,
          decoration: const InputDecoration(border: InputBorder.none),
        ),
      ),
    );
  }
}
