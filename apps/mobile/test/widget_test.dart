import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/main.dart';
import 'package:xterm/xterm.dart';

void main() {
  testWidgets('spike page shows rust status, loads fixture and sends UTF-8 line', (tester) async {
    final terminal = Terminal(maxLines: 10000);
    await tester.binding.setSurfaceSize(const Size(400, 800));
    await tester.pumpWidget(OrreryRemoteApp(rustStatus: 'proto_version = 1', terminal: terminal));
    await tester.pump();

    expect(find.text('proto_version = 1'), findsOneWidget);
    expect(find.text('chưa đo'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('load-fixture')));
    await tester.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 500)));
    await tester.pump();
    await tester.pump();
    expect(find.textContaining('nạp 256KiB'), findsOneWidget);

    await tester.enterText(find.byKey(const ValueKey('input')), 'Xin chào ⬛');
    await tester.tap(find.byKey(const ValueKey('send')));
    await tester.pump();
    expect(terminal.buffer.getText().contains('Xin chào ⬛'), isTrue);
    expect(find.textContaining('58 69 6e 20 63 68 c3 a0 6f 20 e2 ac 9b'), findsOneWidget);
  });
}
