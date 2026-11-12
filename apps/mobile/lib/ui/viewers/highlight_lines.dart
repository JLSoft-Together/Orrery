import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:highlight/highlight.dart' show Node, highlight;
import 'package:highlight/languages/all.dart' show allLanguages;

typedef Token = (String, String?);
typedef TokenLine = List<Token>;

const Map<String, String> _alias = {
  'ts': 'typescript',
  'tsx': 'typescript',
  'js': 'javascript',
  'jsx': 'javascript',
  'mjs': 'javascript',
  'cjs': 'javascript',
  'kt': 'kotlin',
  'kts': 'kotlin',
  'rs': 'rust',
  'py': 'python',
  'yml': 'yaml',
  'md': 'markdown',
  'vue': 'xml',
  'html': 'xml',
  'svg': 'xml',
  'plist': 'xml',
  'sh': 'bash',
  'zsh': 'bash',
  'toml': 'ini',
  'gradle': 'groovy',
  'h': 'cpp',
  'hpp': 'cpp',
  'cc': 'cpp',
  'cs': 'cs',
  'rb': 'ruby',
  'yaml': 'yaml',
  'dockerfile': 'dockerfile',
  'makefile': 'makefile',
};

const int highlightMaxChars = 160000;
const int isolateAbove = 24000;

String? languageOf(String? lang) {
  if (lang == null) return null;
  final id = _alias[lang.toLowerCase()] ?? lang.toLowerCase();
  return allLanguages.containsKey(id) ? id : null;
}

void _flatten(List<Node>? nodes, String? inherited, List<Token> out) {
  if (nodes == null) return;
  for (final node in nodes) {
    final cls = node.className ?? inherited;
    final value = node.value;
    if (value != null) out.add((value, cls));
    _flatten(node.children, cls, out);
  }
}

List<TokenLine> _split(List<Token> tokens) {
  final lines = <TokenLine>[<Token>[]];
  for (final (text, cls) in tokens) {
    final parts = text.split('\n');
    for (var i = 0; i < parts.length; i++) {
      if (i > 0) lines.add(<Token>[]);
      if (parts[i].isNotEmpty) lines.last.add((parts[i], cls));
    }
  }
  return lines;
}

List<TokenLine> plainLines(String code) => [
  for (final line in code.split('\n'))
    if (line.isEmpty) <Token>[] else <Token>[(line, null)],
];

List<TokenLine> highlightSync((String, String?) args) {
  final (code, lang) = args;
  final id = languageOf(lang);
  if (id == null) return plainLines(code);
  try {
    final result = highlight.parse(code, language: id);
    final tokens = <Token>[];
    _flatten(result.nodes, null, tokens);
    return _split(tokens);
  } on Object {
    return plainLines(code);
  }
}

Future<List<TokenLine>> highlightText(String code, String? lang) {
  if (code.length > highlightMaxChars || languageOf(lang) == null) {
    return Future.value(plainLines(code));
  }
  if (code.length < isolateAbove) {
    return Future.value(highlightSync((code, lang)));
  }
  return compute(highlightSync, (code, lang));
}

class CodeColors {
  const CodeColors(this.dark);

  final bool dark;

  Color? of(String? cls) {
    if (cls == null) return null;
    final key = cls.split(' ').first;
    if (key.startsWith('comment') || key == 'quote') {
      return dark ? const Color(0xFF8E8E93) : const Color(0xFF6C6C70);
    }
    if (key.startsWith('keyword') || key == 'selector-tag' || key == 'doctag') {
      return dark ? const Color(0xFFFF7AB2) : const Color(0xFFAD3DA4);
    }
    if (key.startsWith('string') ||
        key == 'regexp' ||
        key == 'symbol' ||
        key == 'bullet') {
      return dark ? const Color(0xFFFF8170) : const Color(0xFFC41A16);
    }
    if (key == 'number' || key == 'literal') {
      return dark ? const Color(0xFFD9C97C) : const Color(0xFF1C00CF);
    }
    if (key == 'built_in' ||
        key == 'type' ||
        key == 'class' ||
        key == 'title' ||
        key == 'name' ||
        key == 'section') {
      return dark ? const Color(0xFF4EB0CC) : const Color(0xFF0B4F79);
    }
    if (key == 'attr' ||
        key == 'attribute' ||
        key == 'variable' ||
        key == 'params' ||
        key == 'property') {
      return dark ? const Color(0xFFB2A4FF) : const Color(0xFF5C2699);
    }
    if (key == 'meta' || key == 'tag' || key == 'addition') {
      return dark ? const Color(0xFFFFA14F) : const Color(0xFF643820);
    }
    return null;
  }
}
