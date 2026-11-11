import 'package:xterm/xterm.dart';

const List<String> terminalFallbackFonts = [
  'OrreryTerminalSymbols2',
  'OrreryTerminalSymbols',
  'Menlo',
  'Monaco',
  'Consolas',
  'Liberation Mono',
  'Courier New',
  'Noto Sans Mono CJK SC',
  'Noto Sans Mono CJK TC',
  'Noto Sans Mono CJK KR',
  'Noto Sans Mono CJK JP',
  'Noto Sans Mono CJK HK',
  'Noto Color Emoji',
  'Noto Sans Symbols',
  'monospace',
  'sans-serif',
];

TerminalStyle orreryTerminalStyle(double fontSize) => TerminalStyle(
  fontSize: fontSize,
  fontFamilyFallback: terminalFallbackFonts,
);
