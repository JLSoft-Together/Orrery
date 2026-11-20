import 'package:xterm/xterm.dart';

import 'anchored_terminal.dart';

const String snapshotReset =
    '\x1b[?1049l\x1b[r\x1b[0m\x1b[?1l\x1b>\x1b[?2004l\x1b[?1000l'
    '\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?6l\x1b[?7h\x1b[?25h';

class PtySnapshot {
  const PtySnapshot(this.cols, this.rows, this.data);

  final int cols;
  final int rows;
  final String data;

  static PtySnapshot? parse(Map<String, Object?> event) {
    final cols = event['cols'];
    final rows = event['rows'];
    final data = event['data'];
    if (cols is! num || rows is! num || data is! String) return null;
    if (cols < 1 || rows < 1) return null;
    return PtySnapshot(cols.toInt(), rows.toInt(), data);
  }
}

extension TerminalSnapshot on Terminal {
  void applySnapshot(PtySnapshot snap) {
    write(snapshotReset);
    altBuffer.clear();
    mainBuffer.clear();
    write('\x1b[H');
    fitTo(snap.cols, snap.rows);
    write(snap.data);
  }
}
