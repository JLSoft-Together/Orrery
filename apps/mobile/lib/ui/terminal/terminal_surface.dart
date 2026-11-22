import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:xterm/xterm.dart';

import 'terminal_fonts.dart';

const EdgeInsets terminalPadding = EdgeInsets.symmetric(
  horizontal: 4,
  vertical: 2,
);

const double terminalMinFont = 1;
const double terminalMaxFont = 18;
const double terminalMaxZoom = 8;

Size terminalCell(double fontSize) {
  final painter = TextPainter(
    text: TextSpan(
      text: 'mmmmmmmmmm',
      style: orreryTerminalStyle(fontSize).toTextStyle(),
    ),
    textScaler: TextScaler.noScaling,
    textDirection: TextDirection.ltr,
  )..layout();
  final size = Size(painter.width / 10, painter.height);
  painter.dispose();
  return size;
}

double terminalFitFont(int cols, double width) {
  final room = width - terminalPadding.horizontal;
  if (cols < 1 || room <= 0) return terminalMinFont;
  const base = 12.0;
  var font = (base * room / (cols * terminalCell(base).width)).clamp(
    terminalMinFont,
    terminalMaxFont,
  );
  for (var i = 0; i < 4 && font > terminalMinFont; i++) {
    if (cols * terminalCell(font).width <= room) break;
    font = math.max(terminalMinFont, font * 0.98);
  }
  return font;
}

class TerminalSurface extends StatefulWidget {
  const TerminalSurface({
    super.key,
    required this.terminal,
    required this.controller,
    required this.direct,
    required this.deskCols,
    required this.deskRows,
    this.overlay,
  });

  final Terminal terminal;
  final TerminalController controller;
  final bool direct;
  final int deskCols;
  final int deskRows;
  final Widget? overlay;

  @override
  State<TerminalSurface> createState() => _TerminalSurfaceState();
}

class _TerminalSurfaceState extends State<TerminalSurface> {
  final ScrollController _scroll = ScrollController();
  final FocusNode _focus = FocusNode();
  final TransformationController _zoom = TransformationController();
  int _fontCols = 0;
  double _fontWidth = 0;
  double _font = 12;
  bool _zoomed = false;
  final Map<int, (Offset, Duration)> _downs = {};
  (Offset, Duration)? _lastTap;

  @override
  void initState() {
    super.initState();
    _zoom.addListener(_onZoom);
    _syncDesk();
  }

  void _onZoom() {
    final zoomed = _zoom.value.getMaxScaleOnAxis() > 1.01;
    if (zoomed != _zoomed) setState(() => _zoomed = zoomed);
  }

  void _syncDesk() {
    final terminal = widget.terminal;
    if (terminal.viewWidth != widget.deskCols ||
        terminal.viewHeight != widget.deskRows) {
      terminal.resize(widget.deskCols, widget.deskRows);
    }
  }

  @override
  void didUpdateWidget(TerminalSurface old) {
    super.didUpdateWidget(old);
    if (widget.deskCols != old.deskCols || widget.deskRows != old.deskRows) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _syncDesk();
      });
    }
    if (widget.direct && !old.direct) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _focus.requestFocus();
      });
    }
    if (!widget.direct && old.direct) _focus.unfocus();
  }

  @override
  void dispose() {
    _scroll.dispose();
    _focus.dispose();
    _zoom.dispose();
    super.dispose();
  }

  double _fontFor(double width) {
    if (_fontCols != widget.deskCols || _fontWidth != width) {
      _fontCols = widget.deskCols;
      _fontWidth = width;
      _font = terminalFitFont(widget.deskCols, width);
    }
    return _font;
  }

  void _resetZoom() => _zoom.value = Matrix4.identity();

  void _pointerDown(PointerDownEvent event) {
    _downs[event.pointer] = (event.position, event.timeStamp);
    if (_downs.length > 1) _lastTap = null;
  }

  void _pointerUp(PointerUpEvent event) {
    final down = _downs.remove(event.pointer);
    if (down == null || _downs.isNotEmpty) return;
    final (at, since) = down;
    final tap =
        (event.position - at).distance < 18 &&
        event.timeStamp - since < const Duration(milliseconds: 250);
    if (!tap) {
      _lastTap = null;
      return;
    }
    final last = _lastTap;
    if (last != null &&
        event.timeStamp - last.$2 < const Duration(milliseconds: 300) &&
        (event.position - last.$1).distance < 40) {
      _lastTap = null;
      _resetZoom();
      return;
    }
    _lastTap = (event.position, event.timeStamp);
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final font = _fontFor(constraints.maxWidth);
        final view = TerminalView(
          widget.terminal,
          controller: widget.controller,
          scrollController: _scroll,
          focusNode: _focus,
          autoResize: false,
          hardwareKeyboardOnly: !widget.direct,
          autofocus: false,
          textStyle: orreryTerminalStyle(font),
          textScaler: TextScaler.noScaling,
          padding: terminalPadding,
          deleteDetection: true,
          keyboardType: TextInputType.multiline,
        );
        return ClipRect(
          child: Stack(
            children: [
              Positioned.fill(
                child: ColoredBox(
                  color: const Color(0xFF14151A),
                  child: Listener(
                    onPointerDown: _pointerDown,
                    onPointerUp: _pointerUp,
                    onPointerCancel: (e) => _downs.remove(e.pointer),
                    child: InteractiveViewer(
                      key: const ValueKey('term-zoom'),
                      transformationController: _zoom,
                      minScale: 1,
                      maxScale: terminalMaxZoom,
                      panEnabled: _zoomed,
                      child: SizedBox(
                        width: constraints.maxWidth,
                        height: constraints.maxHeight,
                        child: view,
                      ),
                    ),
                  ),
                ),
              ),
              if (widget.overlay != null)
                Positioned.fill(child: widget.overlay!),
            ],
          ),
        );
      },
    );
  }
}
