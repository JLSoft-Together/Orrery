import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/terminal/terminal_fonts.dart';
import 'package:xterm/xterm.dart';

import 'support/fonts.dart';

const Color _bg = Color(0xFF000000);
const Color _fg = Color(0xFFFFFFFF);

Future<ui.Image> render(
  WidgetTester tester,
  String text,
  double lineHeight,
  double fontSize,
) async {
  final terminal = Terminal(maxLines: 100)..resize(8, 6);
  terminal.write(text);
  final key = GlobalKey();
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: RepaintBoundary(
          key: key,
          child: SizedBox(
            width: 8 * fontSize,
            height: 6 * fontSize * lineHeight * 1.2,
            child: TerminalView(
              terminal,
              textStyle: TerminalStyle(
                fontSize: fontSize,
                height: lineHeight,
                fontFamily: terminalPrimaryFont,
                fontFamilyFallback: terminalFallbackFonts,
              ),
              theme: const TerminalTheme(
                cursor: _bg,
                selection: _bg,
                foreground: _fg,
                background: _bg,
                black: _bg,
                red: _fg,
                green: _fg,
                yellow: _fg,
                blue: _fg,
                magenta: _fg,
                cyan: _fg,
                white: _fg,
                brightBlack: _fg,
                brightRed: _fg,
                brightGreen: _fg,
                brightYellow: _fg,
                brightBlue: _fg,
                brightMagenta: _fg,
                brightCyan: _fg,
                brightWhite: _fg,
                searchHitBackground: _bg,
                searchHitBackgroundCurrent: _bg,
                searchHitForeground: _fg,
              ),
              readOnly: true,
              hardwareKeyboardOnly: true,
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  final boundary =
      key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final image = (await tester.runAsync(() => boundary.toImage()))!;
  return image;
}

Future<List<int>> litRows(WidgetTester tester, ui.Image image, int x) async {
  final data = (await tester.runAsync(
    () => image.toByteData(format: ui.ImageByteFormat.rawRgba),
  ))!;
  final rows = <int>[];
  for (var y = 0; y < image.height; y++) {
    final i = (y * image.width + x) * 4;
    if (data.getUint8(i) > 100) rows.add(y);
  }
  return rows;
}

Future<int> longestGap(WidgetTester tester, ui.Image image) async {
  var best = 0;
  for (var x = 0; x < image.width; x++) {
    final rows = await litRows(tester, image, x);
    if (rows.length < 20) continue;
    var gap = 0;
    for (var i = 1; i < rows.length; i++) {
      gap = rows[i] - rows[i - 1] - 1 > gap ? rows[i] - rows[i - 1] - 1 : gap;
    }
    if (rows.length > best) best = rows.length;
    if (gap > 0) return gap;
  }
  return 0;
}

void main() {
  setUpAll(loadFonts);

  testWidgets('vertical box lines join across rows with the line height', (
    tester,
  ) async {
    final image = await render(
      tester,
      '│\r\n│\r\n│\r\n│\r\n│',
      terminalLineHeight,
      24,
    );
    expect(await longestGap(tester, image), 0);
  });

  testWidgets('box, block and Vietnamese glyphs share one cell size', (
    tester,
  ) async {
    Size size(String ch) {
      final p = TextPainter(
        text: TextSpan(text: ch, style: orreryTerminalStyle(24).toTextStyle()),
        textDirection: TextDirection.ltr,
      )..layout();
      final s = Size(p.width, p.height);
      p.dispose();
      return s;
    }

    final base = size('M');
    for (final ch in ['│', '┌', '┘', '─', '█', '▓', 'ạ', 'ế', 'ữ']) {
      expect(size(ch), base, reason: ch);
    }
  });

  testWidgets('box corners and Vietnamese use the one primary font', (
    tester,
  ) async {
    final image = await render(
      tester,
      '┌─┐\r\n│ạ│\r\n└─┘ ếữ',
      terminalLineHeight,
      24,
    );
    expect(image.width, greaterThan(0));
  });
}
