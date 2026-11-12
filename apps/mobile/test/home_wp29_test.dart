import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/home_groups.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/providers.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

Machine paired(String id, String name, int at, {bool revoked = false}) =>
    machine(id, name).copyWith(revoked: revoked).withPairedAt(at);

extension on Machine {
  Machine withPairedAt(int at) => Machine(
    id: id,
    name: name,
    mkey: mkey,
    pairedAt: at,
    revoked: revoked,
    alias: alias,
  );
}

void main() {
  test(
    'grouping keeps one machine per name and prefers ready, then newest',
    () {
      final a = paired('a', 'PC', 100, revoked: true);
      final b = paired('b', 'PC', 50);
      final c = paired('c', 'PC', 200);
      final d = paired('d', 'Mac', 10);
      final byAge = groupMachines([a, b, c, d], {});
      expect(byAge.primary.map((m) => m.id), ['c', 'd']);
      expect(byAge.duplicates.map((m) => m.id), ['a', 'b']);
      final byLink = groupMachines([a, b, c, d], {'b'});
      expect(byLink.primary.map((m) => m.id), ['b', 'd']);
    },
  );

  testWidgets('home groups same-name copies and cleans them after a confirm', (
    tester,
  ) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(ws: sample()),
      linkState: linkReady,
      machines: [
        paired('m1', 'DESKTOP-DAUPH', 300),
        paired('m2', 'DESKTOP-DAUPH', 200, revoked: true),
        paired('m3', 'DESKTOP-DAUPH', 100),
      ],
    );
    await h.pump(tester, const HomePage());
    await tester.pumpAndSettle();
    expect(find.text('CONNECTED · 1'), findsOneWidget);
    expect(find.text('SAME NAME · 2'), findsOneWidget);
    expect(find.text('Revoked'), findsOneWidget);
    expect(find.textContaining(' ms'), findsNothing);
    await tester.ensureVisible(find.byKey(const ValueKey('clean-old')));
    await tester.tap(find.byKey(const ValueKey('clean-old')));
    await tester.pumpAndSettle();
    expect(find.text('Remove old copies?'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('orrery-dialog-cancel')));
    await tester.pumpAndSettle();
    final machines =
        h.container.read(machinesProvider.notifier) as FakeMachines;
    expect(machines.forgotten, isEmpty);
    await tester.tap(find.byKey(const ValueKey('clean-old')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('orrery-dialog-confirm')));
    await tester.pumpAndSettle();
    expect(machines.forgotten, ['m2', 'm3']);
    expect(find.text('SAME NAME · 2'), findsNothing);
    expect(find.text('Old copies removed.'), findsOneWidget);
    expect(find.text('DESKTOP-DAUPH'), findsOneWidget);
  });
}
