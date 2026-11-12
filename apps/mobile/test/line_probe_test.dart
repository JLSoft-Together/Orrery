import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/terminal/wp26_probe.dart';
import 'package:xterm/xterm.dart';

void main() {
  test('summary reports missing and duplicate numbered lines', () {
    final terminal = Terminal(maxLines: 100);
    for (final n in [1, 2, 3, 6, 7, 7, 9]) {
      terminal.write('$n\r\n');
    }
    final probe = LineProbe(terminal);
    final text = probe.summary();
    expect(text, contains('first=1 last=9'));
    expect(text, contains('missing=[4-5,8]'));
    expect(text, contains('dup=[7]'));
  });
}
