import 'dart:math' as math;

import 'package:xterm/xterm.dart';

class AnchoredTerminal extends Terminal {
  AnchoredTerminal({super.maxLines});

  @override
  void resize(
    int newWidth,
    int newHeight, [
    int? pixelWidth,
    int? pixelHeight,
  ]) {
    final main = mainBuffer;
    final oldWidth = viewWidth;
    final oldHeight = viewHeight;
    final scrollback = math.max(0, main.lines.length - oldHeight);
    super.resize(newWidth, newHeight, pixelWidth, pixelHeight);
    final grow = viewHeight - oldHeight;
    if (grow <= 0 || viewWidth != oldWidth) return;
    final pulled = math.min(grow, scrollback);
    if (pulled == 0) return;
    for (var i = 0; i < pulled; i++) {
      main.lines.push(BufferLine(viewWidth));
    }
    main.setCursorY(main.cursorY - pulled);
  }
}

extension TerminalFit on Terminal {
  void fitTo(int cols, int rows) {
    if (viewWidth != cols || viewHeight != rows) resize(cols, rows);
  }
}

class TerminalTextSink implements Sink<String> {
  TerminalTextSink(this.onData);

  final void Function(String) onData;

  @override
  void add(String data) => onData(data);

  @override
  void close() {}
}
