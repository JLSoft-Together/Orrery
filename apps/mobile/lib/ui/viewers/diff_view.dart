import 'package:flutter/material.dart';

import '../theme/orrery_colors.dart';
import 'viewer_models.dart' show monoFallback;
import '../widgets/common.dart';

enum DiffKind { add, del, ctx, hunk }

class DiffLine {
  const DiffLine(this.kind, this.text, this.oldNo, this.newNo);

  final DiffKind kind;
  final String text;
  final int? oldNo;
  final int? newNo;
}

List<DiffLine> parseDiff(String text) {
  final lines = <DiffLine>[];
  var oldNo = 0;
  var newNo = 0;
  var inHunk = false;
  final header = RegExp(r'^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@');
  final source = text.endsWith('\n')
      ? text.substring(0, text.length - 1)
      : text;
  for (final raw in source.split('\n')) {
    final line = raw.endsWith('\r') ? raw.substring(0, raw.length - 1) : raw;
    final match = header.firstMatch(line);
    if (match != null) {
      oldNo = int.parse(match.group(1)!);
      newNo = int.parse(match.group(2)!);
      inHunk = true;
      lines.add(DiffLine(DiffKind.hunk, line, null, null));
      continue;
    }
    if (!inHunk || line.startsWith(r'\')) continue;
    if (line.startsWith('+')) {
      lines.add(DiffLine(DiffKind.add, line.substring(1), null, newNo++));
    } else if (line.startsWith('-')) {
      lines.add(DiffLine(DiffKind.del, line.substring(1), oldNo++, null));
    } else if (line.isNotEmpty || oldNo > 0) {
      final body = line.isEmpty ? '' : line.substring(1);
      lines.add(DiffLine(DiffKind.ctx, body, oldNo++, newNo++));
    }
  }
  return lines;
}

sealed class DiffRow {
  const DiffRow();
}

class LineRow extends DiffRow {
  const LineRow(this.line);

  final DiffLine line;
}

class FoldRow extends DiffRow {
  const FoldRow(this.start, this.count);

  final int start;
  final int count;
}

const int foldAbove = 12;
const int foldKeep = 3;

List<DiffRow> foldRows(List<DiffLine> lines, Set<int> open) {
  final rows = <DiffRow>[];
  var i = 0;
  while (i < lines.length) {
    if (lines[i].kind != DiffKind.ctx) {
      rows.add(LineRow(lines[i]));
      i++;
      continue;
    }
    var j = i;
    while (j < lines.length && lines[j].kind == DiffKind.ctx) {
      j++;
    }
    final run = j - i;
    if (run > foldAbove && !open.contains(i)) {
      final leading = i == 0 || lines[i - 1].kind == DiffKind.hunk;
      final trailing = j == lines.length;
      final head = leading ? 0 : foldKeep;
      final tail = trailing ? 0 : foldKeep;
      if (run - head - tail > 4) {
        for (var k = i; k < i + head; k++) {
          rows.add(LineRow(lines[k]));
        }
        rows.add(FoldRow(i, run - head - tail));
        for (var k = j - tail; k < j; k++) {
          rows.add(LineRow(lines[k]));
        }
        i = j;
        continue;
      }
    }
    for (var k = i; k < j; k++) {
      rows.add(LineRow(lines[k]));
    }
    i = j;
  }
  return rows;
}

class DiffView extends StatefulWidget {
  const DiffView({super.key, required this.text});

  final String text;

  @override
  State<DiffView> createState() => _DiffViewState();
}

class _DiffViewState extends State<DiffView> {
  late List<DiffLine> _lines = parseDiff(widget.text);
  final Set<int> _open = {};

  @override
  void didUpdateWidget(DiffView old) {
    super.didUpdateWidget(old);
    if (old.text != widget.text) {
      _lines = parseDiff(widget.text);
      _open.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    final rows = foldRows(_lines, _open);
    const c = OrreryColors.dark;
    final mono = TextStyle(
      fontFamily: 'monospace',
      fontFamilyFallback: monoFallback,
      fontSize: 12.5,
      height: 1.35,
      color: c.wellInk,
    );
    return ColoredBox(
      color: c.well,
      child: ListView.builder(
        key: const ValueKey('diff-list'),
        padding: const EdgeInsets.only(bottom: 32),
        itemCount: rows.length,
        itemBuilder: (context, index) {
          final row = rows[index];
          if (row is FoldRow) {
            return InkWell(
              onTap: () => setState(() => _open.add(row.start)),
              child: Container(
                height: 36,
                alignment: Alignment.center,
                color: c.info.withValues(alpha: 0.12),
                child: Text(
                  context.l10n.vUnchangedLines(row.count),
                  style: TextStyle(
                    fontSize: 12,
                    color: c.inkMuted,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            );
          }
          final line = (row as LineRow).line;
          if (line.kind == DiffKind.hunk) {
            return Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
              color: c.info.withValues(alpha: 0.14),
              child: Text(
                line.text,
                style: mono.copyWith(color: c.inkMuted, fontSize: 12),
              ),
            );
          }
          final add = line.kind == DiffKind.add;
          final del = line.kind == DiffKind.del;
          final tint = add
              ? c.diffAdd.withValues(alpha: 0.18)
              : del
              ? c.diffDel.withValues(alpha: 0.2)
              : null;
          final sign = add ? '+' : (del ? '−' : ' ');
          final accent = add
              ? c.diffAdd
              : del
              ? c.diffDel
              : c.wellInk.withValues(alpha: 0.5);
          return Container(
            color: tint,
            padding: const EdgeInsets.symmetric(vertical: 1),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SizedBox(
                  width: 38,
                  child: Text(
                    '${line.newNo ?? line.oldNo ?? ''}',
                    textAlign: TextAlign.right,
                    style: mono.copyWith(fontSize: 11, color: accent),
                  ),
                ),
                SizedBox(
                  width: 18,
                  child: Text(
                    sign,
                    textAlign: TextAlign.center,
                    style: mono.copyWith(
                      color: accent,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                Expanded(child: Text(line.text, style: mono)),
              ],
            ),
          );
        },
      ),
    );
  }
}
