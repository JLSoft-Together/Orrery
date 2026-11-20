import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/terminal/anchored_terminal.dart';
import 'package:orrery_remote/ui/terminal/pty_snapshot.dart';
import 'package:xterm/xterm.dart';

List<String> _text(Terminal t) {
  final out = <String>[];
  for (var i = 0; i < t.buffer.lines.length; i++) {
    final s = t.buffer.lines[i].getText().trimRight();
    if (s.isNotEmpty) out.add(s);
  }
  return out;
}

String _snapshot(int from, int to, int cursorRow) {
  final lines = [for (var n = from; n <= to; n++) '\x1b[mline-$n']
      .join('\x1b[m\r\n');
  return '$lines\x1b[m\x1b[$cursorRow;8H';
}

void main() {
  test('parse needs cols, rows and string data', () {
    expect(PtySnapshot.parse({'cols': 80, 'rows': 19, 'data': 'x'})?.rows, 19);
    expect(PtySnapshot.parse({'cols': 80, 'rows': 19}), isNull);
    expect(PtySnapshot.parse({'cols': 0, 'rows': 19, 'data': ''}), isNull);
    expect(PtySnapshot.parse({'cols': '80', 'rows': 19, 'data': ''}), isNull);
  });

  test('a snapshot resets, resizes and replaces what was there', () {
    final t = AnchoredTerminal(maxLines: 5000)..resize(80, 41);
    t.write(List.generate(300, (i) => 'stale-$i').join('\r\n'));
    t.write('\x1b[?2004h\x1b[?1h\x1b[?25l');
    t.applySnapshot(PtySnapshot(80, 19, _snapshot(1, 41, 19)));
    expect([t.viewWidth, t.viewHeight], [80, 19]);
    expect(t.bracketedPasteMode, isFalse);
    expect(t.cursorKeysMode, isFalse);
    expect(t.cursorVisibleMode, isTrue);
    expect([t.buffer.cursorY, t.buffer.cursorX], [18, 7]);
    t.write('\r\nline-42\r\n');
    expect(_text(t), List.generate(42, (i) => 'line-${i + 1}'));
  });

  test('a snapshot leaves the alternate screen of the old state', () {
    final t = AnchoredTerminal(maxLines: 5000)..resize(40, 10);
    t.write('main\r\n\x1b[?1049hfull screen app');
    expect(t.isUsingAltBuffer, isTrue);
    t.applySnapshot(PtySnapshot(40, 10, _snapshot(1, 3, 4)));
    expect(t.isUsingAltBuffer, isFalse);
    expect(_text(t), ['line-1', 'line-2', 'line-3']);
  });
}
