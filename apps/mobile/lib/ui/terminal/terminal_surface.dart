import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:xterm/xterm.dart';

class TerminalSurface extends StatefulWidget {
  const TerminalSurface({
    super.key,
    required this.terminal,
    required this.controller,
    required this.typing,
    required this.direct,
    required this.deskCols,
    required this.deskRows,
    required this.onFit,
    this.overlay,
  });

  final Terminal terminal;
  final TerminalController controller;
  final bool typing;
  final bool direct;
  final int deskCols;
  final int deskRows;
  final void Function(int cols, int rows) onFit;
  final Widget? overlay;

  @override
  State<TerminalSurface> createState() => _TerminalSurfaceState();
}

class _TerminalSurfaceState extends State<TerminalSurface> {
  static const double _minFont = 8;
  static const double _maxFont = 22;

  final ScrollController _scroll = ScrollController();
  final FocusNode _focus = FocusNode();
  final Map<int, Offset> _pointers = {};
  double _fontSize = 12;
  double _pinchStartDistance = 0;
  double _pinchStartFont = 12;
  int _lastFitCols = 0;
  int _lastFitRows = 0;

  @override
  void initState() {
    super.initState();
    _syncDesk();
  }

  void _syncDesk() {
    final terminal = widget.terminal;
    if (widget.typing) return;
    if (terminal.viewWidth != widget.deskCols ||
        terminal.viewHeight != widget.deskRows) {
      terminal.resize(widget.deskCols, widget.deskRows);
    }
  }

  @override
  void didUpdateWidget(TerminalSurface old) {
    super.didUpdateWidget(old);
    _syncDesk();
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
    super.dispose();
  }

  Size _cell() {
    final painter = TextPainter(
      text: TextSpan(
        text: 'M' * 20,
        style: TerminalStyle(fontSize: _fontSize).toTextStyle(),
      ),
      textDirection: TextDirection.ltr,
    )..layout();
    return Size(painter.width / 20, painter.height);
  }

  void _pointerDown(PointerDownEvent event) {
    _pointers[event.pointer] = event.position;
    if (_pointers.length == 2) {
      _pinchStartDistance = _distance();
      _pinchStartFont = _fontSize;
    }
  }

  void _pointerMove(PointerMoveEvent event) {
    if (!_pointers.containsKey(event.pointer)) return;
    _pointers[event.pointer] = event.position;
    if (_pointers.length != 2 || _pinchStartDistance < 8) return;
    final next = (_pinchStartFont * _distance() / _pinchStartDistance).clamp(
      _minFont,
      _maxFont,
    );
    if ((next - _fontSize).abs() >= 0.5) setState(() => _fontSize = next);
  }

  void _pointerUp(PointerEvent event) {
    _pointers.remove(event.pointer);
  }

  double _distance() {
    final points = _pointers.values.toList();
    return (points[0] - points[1]).distance;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final cell = _cell();
        final cols = math.max(20, (constraints.maxWidth / cell.width).floor());
        final rows = math.max(5, (constraints.maxHeight / cell.height).floor());
        if (cols != _lastFitCols || rows != _lastFitRows) {
          _lastFitCols = cols;
          _lastFitRows = rows;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted) widget.onFit(cols, rows);
          });
        }
        final deskWidth = widget.deskCols * cell.width;
        final wide = !widget.typing && deskWidth > constraints.maxWidth;
        final style = TerminalStyle(fontSize: _fontSize);
        final view = TerminalView(
          widget.terminal,
          controller: widget.controller,
          scrollController: _scroll,
          focusNode: _focus,
          autoResize: widget.typing,
          hardwareKeyboardOnly: !widget.direct,
          autofocus: false,
          textStyle: style,
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
          deleteDetection: true,
          keyboardType: TextInputType.multiline,
        );
        final body = wide
            ? SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: SizedBox(
                  width: deskWidth + 8,
                  height: constraints.maxHeight,
                  child: view,
                ),
              )
            : view;
        return Listener(
          onPointerDown: _pointerDown,
          onPointerMove: _pointerMove,
          onPointerUp: _pointerUp,
          onPointerCancel: _pointerUp,
          child: ClipRect(
            child: Stack(
              children: [
                Positioned.fill(
                  child: ColoredBox(
                    color: const Color(0xFF14151A),
                    child: body,
                  ),
                ),
                if (widget.overlay != null)
                  Positioned.fill(child: widget.overlay!),
              ],
            ),
          ),
        );
      },
    );
  }
}
