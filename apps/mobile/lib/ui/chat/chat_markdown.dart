sealed class MdBlock {
  const MdBlock();
}

class MdPara extends MdBlock {
  const MdPara(this.text);

  final String text;
}

class MdHeading extends MdBlock {
  const MdHeading(this.level, this.text);

  final int level;
  final String text;
}

class MdList extends MdBlock {
  const MdList(this.ordered, this.items, {this.start = 1});

  final bool ordered;
  final List<String> items;
  final int start;
}

class MdQuote extends MdBlock {
  const MdQuote(this.text);

  final String text;
}

class MdCode extends MdBlock {
  const MdCode(this.code, this.lang);

  final String code;
  final String? lang;
}

class MdRule extends MdBlock {
  const MdRule();
}

final RegExp _fence = RegExp(r'^\s{0,3}(```|~~~)\s*([\w+#.-]*)');
final RegExp _heading = RegExp(r'^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$');
final RegExp _bullet = RegExp(r'^\s{0,3}[-*+]\s+(.*)$');
final RegExp _ordered = RegExp(r'^\s{0,3}(\d{1,9})[.)]\s+(.*)$');
final RegExp _quote = RegExp(r'^\s{0,3}>\s?(.*)$');
final RegExp _rule = RegExp(r'^\s{0,3}([-*_])(\s*\1){2,}\s*$');

List<MdBlock> parseMarkdown(String source) {
  final lines = source.replaceAll('\r\n', '\n').split('\n');
  final out = <MdBlock>[];
  final para = <String>[];
  void flush() {
    if (para.isEmpty) return;
    out.add(MdPara(para.join(' ')));
    para.clear();
  }

  var i = 0;
  while (i < lines.length) {
    final line = lines[i];
    final fence = _fence.firstMatch(line);
    if (fence != null) {
      flush();
      final mark = fence.group(1)!;
      final lang = fence.group(2);
      final body = <String>[];
      i++;
      while (i < lines.length && !lines[i].trimLeft().startsWith(mark)) {
        body.add(lines[i]);
        i++;
      }
      i++;
      out.add(
        MdCode(body.join('\n'), lang == null || lang.isEmpty ? null : lang),
      );
      continue;
    }
    if (line.trim().isEmpty) {
      flush();
      i++;
      continue;
    }
    if (_rule.hasMatch(line)) {
      flush();
      out.add(const MdRule());
      i++;
      continue;
    }
    final heading = _heading.firstMatch(line);
    if (heading != null) {
      flush();
      out.add(MdHeading(heading.group(1)!.length, heading.group(2)!));
      i++;
      continue;
    }
    if (_quote.hasMatch(line)) {
      flush();
      final body = <String>[];
      while (i < lines.length) {
        final m = _quote.firstMatch(lines[i]);
        if (m == null) break;
        body.add(m.group(1)!);
        i++;
      }
      out.add(MdQuote(body.join(' ')));
      continue;
    }
    final bullet = _bullet.firstMatch(line);
    final ordered = _ordered.firstMatch(line);
    if (bullet != null || ordered != null) {
      flush();
      final isOrdered = bullet == null;
      final start = isOrdered ? int.tryParse(ordered!.group(1)!) ?? 1 : 1;
      final items = <String>[];
      while (i < lines.length) {
        final l = lines[i];
        final b = _bullet.firstMatch(l);
        final o = _ordered.firstMatch(l);
        final m = isOrdered ? o : b;
        if (m != null) {
          items.add(m.group(isOrdered ? 2 : 1)!);
          i++;
          continue;
        }
        if (l.trim().isNotEmpty &&
            l.startsWith(RegExp(r'\s{2,}')) &&
            items.isNotEmpty) {
          items[items.length - 1] = '${items.last} ${l.trim()}';
          i++;
          continue;
        }
        break;
      }
      out.add(MdList(isOrdered, items, start: start));
      continue;
    }
    para.add(line.trim());
    i++;
  }
  flush();
  return out;
}

enum MdStyle { plain, bold, italic, code, link }

class MdRun {
  const MdRun(this.text, this.styles);

  final String text;
  final Set<MdStyle> styles;
}

final RegExp _link = RegExp(r'^\[([^\]]+)\]\(([^)\s]+)\)');

List<MdRun> parseInline(String text) {
  final runs = <MdRun>[];
  final buf = StringBuffer();
  var bold = false;
  var italic = false;
  Set<MdStyle> current() => {
    if (bold) MdStyle.bold,
    if (italic) MdStyle.italic,
  };
  void flush() {
    if (buf.isEmpty) return;
    runs.add(MdRun(buf.toString(), current()));
    buf.clear();
  }

  var i = 0;
  while (i < text.length) {
    final c = text[i];
    if (c == '\\' && i + 1 < text.length) {
      buf.write(text[i + 1]);
      i += 2;
      continue;
    }
    if (c == '`') {
      final end = text.indexOf('`', i + 1);
      if (end > i) {
        flush();
        runs.add(
          MdRun(text.substring(i + 1, end), {...current(), MdStyle.code}),
        );
        i = end + 1;
        continue;
      }
    }
    if (c == '[') {
      final m = _link.firstMatch(text.substring(i));
      if (m != null) {
        flush();
        runs.add(MdRun(m.group(1)!, {...current(), MdStyle.link}));
        i += m.group(0)!.length;
        continue;
      }
    }
    if ((c == '*' || c == '_') && i + 1 < text.length && text[i + 1] == c) {
      final close = text.indexOf('$c$c', i + 2);
      if (bold || close > i + 2) {
        flush();
        bold = !bold;
        i += 2;
        continue;
      }
    }
    if (c == '*' || (c == '_' && _wordEdge(text, i))) {
      final close = text.indexOf(c, i + 1);
      if (italic || (close > i + 1 && text[i + 1] != ' ')) {
        flush();
        italic = !italic;
        i++;
        continue;
      }
    }
    buf.write(c);
    i++;
  }
  flush();
  return runs;
}

bool _wordEdge(String text, int i) {
  final before = i == 0 ? ' ' : text[i - 1];
  final after = i + 1 >= text.length ? ' ' : text[i + 1];
  final word = RegExp(r'[A-Za-z0-9]');
  return !word.hasMatch(before) || !word.hasMatch(after);
}
