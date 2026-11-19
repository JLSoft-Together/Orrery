import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/terminal/anchored_terminal.dart';
import 'package:xterm/xterm.dart';

List<String> _text(Terminal t) {
  final out = <String>[];
  for (var i = 0; i < t.mainBuffer.lines.length; i++) {
    final s = t.mainBuffer.lines[i].getText().trimRight();
    if (s.isNotEmpty) out.add(s);
  }
  return out;
}

void _fill(Terminal t, int n) {
  t.write(List.generate(n, (i) => 'L${i + 1}').join('\r\n'));
}

void _conptyRepaint(Terminal t, int first, int last, int rows) {
  final b = StringBuffer('\x1b[H');
  for (var i = first; i <= last; i++) {
    b.write('L$i\x1b[K');
    if (i < last) b.write('\r\n');
  }
  b.write('\x1b[J\x1b[${last - first + 1};${'L$last'.length + 1}H');
  t.write(b.toString());
}

void main() {
  test('growing keeps lines in place so a repaint loses nothing', () {
    final t = AnchoredTerminal(maxLines: 5000)..resize(30, 19);
    _fill(t, 100);
    t.resize(30, 41);
    _conptyRepaint(t, 82, 100, 41);
    final lines = _text(t);
    expect(lines, List.generate(100, (i) => 'L${i + 1}'));
    expect(t.buffer.cursorY, 18);
  });

  test('the plain terminal drops the lines it pulled from scrollback', () {
    final t = Terminal(maxLines: 5000)..resize(30, 19);
    _fill(t, 100);
    t.resize(30, 41);
    _conptyRepaint(t, 82, 100, 41);
    expect(_text(t).length, lessThan(100));
  });

  test('shrinking and width changes behave like the plain terminal', () {
    final a = AnchoredTerminal(maxLines: 5000)..resize(30, 41);
    final p = Terminal(maxLines: 5000)..resize(30, 41);
    _fill(a, 100);
    _fill(p, 100);
    a.resize(30, 19);
    p.resize(30, 19);
    expect(_text(a), _text(p));
    expect(a.buffer.cursorY, p.buffer.cursorY);
    a.resize(40, 30);
    p.resize(40, 30);
    expect(_text(a), _text(p));
  });

  test('growing with a short buffer adds no extra lines', () {
    final t = AnchoredTerminal(maxLines: 5000)..resize(30, 19);
    _fill(t, 5);
    t.resize(30, 41);
    expect(t.mainBuffer.lines.length, 41);
    expect(t.buffer.cursorY, 4);
  });

  test('fitTo resizes only when the size differs', () {
    final t = AnchoredTerminal(maxLines: 100)..resize(30, 19);
    var calls = 0;
    t.onResize = (_, _, _, _) => calls++;
    t.fitTo(30, 19);
    expect(calls, 0);
    t.fitTo(30, 20);
    expect(calls, 1);
  });
}
