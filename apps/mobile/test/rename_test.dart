import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/ui/monitor/monitor_page.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

void main() {
  group('machine alias', () {
    late Directory dir;
    late MachineRepository repo;

    MachineRepository reopen() =>
        MachineRepository(directory: dir, secure: MemorySecureKv());

    setUp(() async {
      dir = await Directory.systemTemp.createTemp('alias');
      repo = reopen();
      await repo.save(machine('m1', 'Dev PC'));
    });

    tearDown(() => dir.delete(recursive: true));

    test('rename persists, empty restores the paired name', () async {
      await repo.rename('m1', '  Office  ');
      expect(repo.get('m1')!.displayName, 'Office');
      expect(repo.get('m1')!.name, 'Dev PC');
      final again = reopen();
      await again.load();
      expect(again.get('m1')!.alias, 'Office');
      await again.rename('m1', '   ');
      expect(again.get('m1')!.displayName, 'Dev PC');
      final third = reopen();
      await third.load();
      expect(third.get('m1')!.alias, isNull);
    });

    test('limit 40 runes counted by emoji, control chars dropped', () {
      expect(cleanAlias('\u{1F4F1}' * 60)!.runes.length, 40);
      expect(cleanAlias('a\u0007b\u200b\u202ec'), 'abc');
      expect(cleanAlias('\n \t'), isNull);
      expect(cleanAlias(null), isNull);
    });

    test(
      'a later save of the same machine never loses or reverts it',
      () async {
        await repo.rename('m1', 'Office');
        await repo.save(repo.get('m1')!.copyWith(name: 'Renamed on PC'));
        expect(repo.get('m1')!.alias, 'Office');
        expect(repo.get('m1')!.name, 'Renamed on PC');
        await repo.save(machine('m1', 'Dev PC'));
        expect(repo.get('m1')!.alias, 'Office');
      },
    );

    test('unknown id is ignored and forgetting clears the alias', () async {
      await repo.rename('nope', 'x');
      expect(repo.get('nope'), isNull);
      await repo.rename('m1', 'Office');
      await repo.remove('m1');
      await repo.save(machine('m1', 'Dev PC'));
      expect(repo.get('m1')!.alias, isNull);
    });
  });

  testWidgets('home renames the machine from the menu and shows the original', (
    tester,
  ) async {
    final h = harness();
    await h.pump(tester, const HomePage());
    await tester.tap(find.byKey(const ValueKey('machine-menu-m1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('machine-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-field')),
      'Office',
    );
    await tester.tap(find.byKey(const ValueKey('rename-save')));
    await tester.pumpAndSettle();
    expect(find.text('Office'), findsOneWidget);
    expect(find.text('Named Dev PC on the computer'), findsOneWidget);
    await tester.longPress(find.text('Office'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const ValueKey('rename-field')), '');
    await tester.tap(find.byKey(const ValueKey('rename-save')));
    await tester.pumpAndSettle();
    expect(find.text('Dev PC'), findsOneWidget);
    expect(find.textContaining('Named'), findsNothing);
    expect(h.container.read(machinesProvider).single.alias, isNull);
  });

  testWidgets('pane menu renames the session with terminal.alias', (
    tester,
  ) async {
    final h = harness(
      snapshot: dashboard(
        terminals: [
          {
            'id': 't-a',
            'title': 'refactor',
            'alias': 'Mine',
            'status': 'working',
          },
        ],
      ),
    );
    await h.pump(
      tester,
      const TabPage(machineId: 'm1', tabKey: 't1', paneKey: 'a'),
    );
    expect(find.text('Mine'), findsWidgets);
    await tester.tap(find.byTooltip('Tab actions'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('action-rename-session')));
    await tester.pumpAndSettle();
    final field = tester.widget<TextField>(
      find.byKey(const ValueKey('rename-field')),
    );
    expect(field.controller!.text, 'Mine');
    expect(field.maxLength, 80);
    await tester.enterText(
      find.byKey(const ValueKey('rename-field')),
      '  Api  ',
    );
    await tester.tap(find.byKey(const ValueKey('rename-save')));
    await tester.pumpAndSettle();
    expect(h.api.commands.last.$1, 'terminal.alias');
    expect(h.api.commands.last.$2, {'terminalId': 't-a', 'alias': 'Api'});
    await tester.tap(find.byTooltip('Tab actions'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('action-rename-session')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const ValueKey('rename-field')), '   ');
    await tester.tap(find.byKey(const ValueKey('rename-save')));
    await tester.pumpAndSettle();
    expect(h.api.commands.last.$2, {'terminalId': 't-a', 'alias': null});
  });

  testWidgets('monitor list renames a session from its menu or long press', (
    tester,
  ) async {
    final h = harness(
      snapshot: dashboard(
        terminals: [
          {
            'id': 't-1',
            'title': 'Refactor',
            'alias': 'Old name',
            'status': 'working',
            'cwd': 'C:\\work\\alpha',
          },
        ],
      ),
    );
    await h.pump(tester, MonitorPage(machineId: 'm1', onOpen: (_) {}));
    await tester.pump();
    expect(find.text('Old name'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('mon-menu-t-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('mon-rename-session')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const ValueKey('rename-field')), 'New');
    await tester.tap(find.byKey(const ValueKey('rename-save')));
    await tester.pumpAndSettle();
    expect(h.api.commands.last.$2, {'terminalId': 't-1', 'alias': 'New'});
    await tester.longPress(find.byKey(const ValueKey('mon-t-1')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('rename-field')), findsOneWidget);
  });
}
