import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:xterm/xterm.dart';

class LineProbe {
  LineProbe(this.terminal, {void Function(String)? sink})
    : _sink = sink ?? debugPrint;

  final Terminal terminal;
  final void Function(String) _sink;
  Timer? _timer;

  static final RegExp _number = RegExp(r'^\d+$');

  void event(String type) {
    if (!kDebugMode) return;
    _sink('WP26 event $type');
    if (type == 'pty.ready') schedule();
  }

  void schedule() {
    if (!kDebugMode) return;
    _timer?.cancel();
    _timer = Timer(const Duration(seconds: 2), () => _sink(summary()));
  }

  void dispose() => _timer?.cancel();

  String summary() {
    final seen = <int, int>{};
    final lines = terminal.buffer.lines;
    for (var i = 0; i < lines.length; i++) {
      final text = lines[i].getText().trim();
      if (_number.hasMatch(text)) {
        final n = int.parse(text);
        seen[n] = (seen[n] ?? 0) + 1;
      }
    }
    if (seen.isEmpty) return 'WP26 lines=${lines.length} numbers=0';
    final keys = seen.keys.toList()..sort();
    final dup = [
      for (final e in seen.entries)
        if (e.value > 1) e.key,
    ]..sort();
    final missing = <String>[];
    var start = -1;
    for (var n = keys.first; n <= keys.last + 1; n++) {
      final gap = n <= keys.last && !seen.containsKey(n);
      if (gap && start < 0) start = n;
      if (!gap && start >= 0) {
        missing.add(start == n - 1 ? '$start' : '$start-${n - 1}');
        start = -1;
      }
    }
    return 'WP26 lines=${lines.length} numbers=${keys.length} '
        'first=${keys.first} last=${keys.last} missing=[${missing.join(',')}] '
        'dup=[${dup.join(',')}]';
  }
}
