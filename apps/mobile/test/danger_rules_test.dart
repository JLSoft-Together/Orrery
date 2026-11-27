import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:orrery_remote/ui/run/crash_draft.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/app_lock_scope.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/security/danger_rules.dart';
import 'package:orrery_remote/security/device_auth.dart';
import 'package:orrery_remote/ui/chat/chat_controller.dart';
import 'package:orrery_remote/ui/chat/chat_screen.dart';

import 'chat_ui_test.dart' show chatPage, ws;
import 'support/lock_fakes.dart';
import 'support/ui_fakes.dart';

const defaults = [
  r'\bgit\s+push\b.*\s(?:-f|--force)(?:\s|$)',
  r'\breset\s+--hard\b',
  r'\brm\s+-[a-z]*(?:rf|fr)[a-z]*(?:\s|$)',
  r'\bgit\s+clean\s+-[a-z]*f',
  r'\bdrop\s+(?:table|database)\b',
  r'\bgit\s+branch\s+-d\b',
];

Harness dangerHarness(FakeMachineApi api) => Harness(
  api: api
    ..responder = (type, fields) {
      if (type == 'chat.watch') return chatPage();
      if (type == 'ws.send') return {'submitted': true};
      return {};
    },
  viewState: view(
    ws: ws(),
    snapshot: dashboard(
      terminals: [
        {'id': 't-a', 'lastActivityAt': '2026-11-20T09:00:00Z'},
      ],
    ),
    hello: {'chat': 1, 'dangerRules': defaults},
  ),
  linkState: linkReady,
);

Future<FakeAuth> pumpChat(
  WidgetTester tester,
  Harness h,
  AuthOutcome outcome,
) async {
  final auth = FakeAuth()..fallback = outcome;
  final lock = makeLock(auth);
  await h.pump(
    tester,
    AppLockScope(
      lock: lock,
      child: const ChatScreen(machineId: 'm1', paneKey: 'a'),
    ),
  );
  await tester.pump();
  return auth;
}

Future<void> typeAndSend(WidgetTester tester, String text) async {
  await tester.enterText(find.byKey(const ValueKey('chat-input')), text);
  await tester.pump();
  await tester.tap(find.byKey(const ValueKey('chat-send')));
  await tester.pumpAndSettle();
}

List<(String, JsonMap?)> sends(FakeMachineApi api) =>
    api.commands.where((c) => c.$1 == 'ws.send').toList();

void main() {
  group('DangerRules', () {
    final rules = DangerRules.parse(defaults);

    test('matches each default and ignores safe commands', () {
      for (final text in [
        'git push -f',
        'git push origin main --force',
        'GIT PUSH --FORCE',
        'git reset --hard HEAD~1',
        'rm -rf /tmp/x',
        'rm -fr build',
        'git clean -fd',
        'DROP TABLE users;',
        'git branch -D old',
        'ls\nrm -rf x',
      ]) {
        expect(rules.match(text), isNotNull, reason: text);
      }
      for (final text in [
        'git push',
        'git push --force-with-lease',
        'git reset --soft HEAD',
        'rm -r dir',
        'echo hello',
        '',
      ]) {
        expect(rules.match(text), isNull, reason: text);
      }
    });

    test('bad, long, duplicate and surplus rules are dropped', () {
      final parsed = DangerRules.parse([
        '(unclosed',
        '',
        'a' * (maxDangerRuleLength + 1),
        'ok',
        'ok',
        42,
        null,
      ]);
      expect(parsed.patterns, ['ok']);
      expect(
        DangerRules.parse([for (var i = 0; i < 50; i++) 'r$i']).patterns.length,
        maxDangerRules,
      );
      expect(DangerRules.parse('nope').isEmpty, isTrue);
      expect(DangerRules.fromHello(null).isEmpty, isTrue);
      expect(DangerRules.fromHello({'dangerRules': defaults}).isEmpty, isFalse);
    });

    test('guardDanger asks only on a match and maps the outcome', () async {
      var calls = 0;
      Future<AuthOutcome> Function() answer(AuthOutcome o) => () async {
        calls++;
        return o;
      };
      expect(
        await guardDanger(
          rules: rules,
          text: 'ls',
          authenticate: answer(AuthOutcome.success),
        ),
        DangerVerdict.safe,
      );
      expect(calls, 0);
      Future<DangerVerdict> run(AuthOutcome o) => guardDanger(
        rules: rules,
        text: 'rm -rf x',
        authenticate: answer(o),
      );
      expect(await run(AuthOutcome.success), DangerVerdict.authed);
      expect(await run(AuthOutcome.notAvailable), DangerVerdict.authed);
      expect(await run(AuthOutcome.cancelled), DangerVerdict.cancelled);
      expect(await run(AuthOutcome.failed), DangerVerdict.cancelled);
      expect(calls, 4);
      expect(
        await guardDanger(
          rules: rules,
          text: 'rm -rf x',
          authenticate: () => throw StateError('x'),
        ),
        DangerVerdict.cancelled,
      );
    });
  });

  group('ChatController needsAuth', () {
    late FakeMachineApi api;

    setUp(() {
      chatLogCache.clear();
      api = FakeMachineApi();
    });

    test('asks once, then resends with authed', () async {
      var asked = 0;
      api.responder = (type, fields) {
        if (type == 'ws.send' && fields?['authed'] != true) {
          throw const RemoteCommandException('needsAuth');
        }
        return {};
      };
      final chat = ChatController(
        api: api,
        paneKey: 'a',
        reauthorize: (text) async {
          asked++;
          return true;
        },
      );
      expect(await chat.send('rm -rf x'), isTrue);
      expect(asked, 1);
      final calls = sends(api);
      expect(calls.length, 2);
      expect(calls[0].$2!.containsKey('authed'), isFalse);
      expect(calls[1].$2!['authed'], isTrue);
      chat.dispose();
    });

    test('a cancelled auth keeps the message unsent without looping', () async {
      api.responder = (type, fields) =>
          throw const RemoteCommandException('needsAuth');
      var asked = 0;
      final chat = ChatController(
        api: api,
        paneKey: 'a',
        reauthorize: (text) async {
          asked++;
          return false;
        },
      );
      expect(await chat.send('rm -rf x'), isFalse);
      expect(asked, 1);
      expect(sends(api).length, 1);
      expect(chat.outbox.single.state, OutboxState.unsent);
      expect(chat.outbox.single.error, 'needsAuth');
      chat.dispose();
    });

    test('needsAuth after authed does not loop', () async {
      api.responder = (type, fields) =>
          throw const RemoteCommandException('needsAuth');
      var asked = 0;
      final chat = ChatController(
        api: api,
        paneKey: 'a',
        reauthorize: (text) async {
          asked++;
          return true;
        },
      );
      expect(await chat.send('rm -rf x', authed: true), isFalse);
      expect(asked, 1);
      expect(sends(api).length, 2);
      expect(chat.outbox.single.error, 'needsAuth');
      chat.dispose();
    });

    test('without a reauthorize hook it fails like any error', () async {
      api.responder = (type, fields) =>
          throw const RemoteCommandException('needsAuth');
      final chat = ChatController(api: api, paneKey: 'a');
      expect(await chat.send('rm -rf x'), isFalse);
      expect(sends(api).length, 1);
      chat.dispose();
    });
  });

  group('ChatComposer', () {
    setUp(chatLogCache.clear);

    testWidgets('a dangerous line asks the device and sends authed', (
      tester,
    ) async {
      final api = FakeMachineApi();
      final auth = await pumpChat(
        tester,
        dangerHarness(api),
        AuthOutcome.success,
      );
      await typeAndSend(tester, 'git push --force');
      expect(auth.calls, 1);
      expect(sends(api).single.$2, {
        'paneKey': 'a',
        'text': 'git push --force',
        'authed': true,
      });
    });

    testWidgets('a cancelled auth sends nothing and keeps the text', (
      tester,
    ) async {
      final api = FakeMachineApi();
      final auth = await pumpChat(
        tester,
        dangerHarness(api),
        AuthOutcome.cancelled,
      );
      await typeAndSend(tester, 'rm -rf build');
      expect(auth.calls, 1);
      expect(sends(api), isEmpty);
      final field = tester.widget<TextField>(
        find.byKey(const ValueKey('chat-input')),
      );
      expect(field.controller!.text, 'rm -rf build');
    });

    testWidgets('a second tap while auth is open sends once and keeps later typing', (
      tester,
    ) async {
      final api = FakeMachineApi();
      final auth = await pumpChat(
        tester,
        dangerHarness(api),
        AuthOutcome.success,
      );
      final gate = Completer<AuthOutcome>();
      auth.hold = gate;
      final input = find.byKey(const ValueKey('chat-input'));
      await tester.enterText(input, 'rm -rf build');
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('chat-send')));
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('chat-send')));
      await tester.pump();
      await tester.enterText(input, 'rm -rf build and more');
      gate.complete(AuthOutcome.success);
      await tester.pumpAndSettle();
      expect(sends(api).length, 1);
      final field = tester.widget<TextField>(input);
      expect(field.controller!.text, 'rm -rf build and more');
    });

    testWidgets('a crash draft for the pane fills the composer once, unsent', (
      tester,
    ) async {
      final api = FakeMachineApi();
      final h = dangerHarness(api);
      final lock = makeLock(FakeAuth());
      await h.pump(
        tester,
        AppLockScope(
          lock: lock,
          child: Consumer(
            builder: (context, ref, _) => TextButton(
              key: const ValueKey('go'),
              onPressed: () {
                ref.read(crashDraftProvider.notifier).put('a', 'CRASH log');
                Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) =>
                        const ChatScreen(machineId: 'm1', paneKey: 'a'),
                  ),
                );
              },
              child: const Text('go'),
            ),
          ),
        ),
      );
      await tester.tap(find.byKey(const ValueKey('go')));
      await tester.pumpAndSettle();
      final field = tester.widget<TextField>(
        find.byKey(const ValueKey('chat-input')),
      );
      expect(field.controller!.text, 'CRASH log');
      expect(h.container.read(crashDraftProvider), isEmpty);
      expect(sends(api), isEmpty);
    });

    testWidgets('a safe line never asks and carries no authed', (tester) async {
      final api = FakeMachineApi();
      final auth = await pumpChat(
        tester,
        dangerHarness(api),
        AuthOutcome.success,
      );
      await typeAndSend(tester, 'git push --force-with-lease');
      expect(auth.calls, 0);
      expect(sends(api).single.$2, {
        'paneKey': 'a',
        'text': 'git push --force-with-lease',
      });
    });
  });
}
