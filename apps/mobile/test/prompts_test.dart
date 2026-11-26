import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/prompts/prompt_model.dart';
import 'package:orrery_remote/prompts/prompts_api.dart';
import 'package:orrery_remote/prompts/prompts_controller.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/ui/chat/chat_controller.dart';
import 'package:orrery_remote/ui/chat/chat_screen.dart';
import 'package:orrery_remote/ui/viewers/viewer_api.dart';

import 'support/ui_fakes.dart';

Map<String, dynamic> item(
  String id,
  String title,
  String body, {
  String? text,
  List<String>? missing,
  bool local = false,
}) => {
  'id': id,
  'title': title,
  'body': body,
  'scope': 'all',
  'updatedAt': 1,
  'text': ?text,
  'missing': ?missing,
  if (local) 'local': true,
};

JsonMap listReply(List<Map<String, dynamic>> items, {int hidden = 0}) => {
  'data': {'ok': true, 'items': items, 'hidden': hidden},
};

const JsonMap okReply = {
  'data': {'ok': true, 'id': 'n1'},
};

TextEditingValue at(String text, int offset, [int? end]) => TextEditingValue(
  text: text,
  selection: TextSelection(baseOffset: offset, extentOffset: end ?? offset),
);

Harness chatHarness({
  bool cap = true,
  List<Map<String, dynamic>>? items,
  int hidden = 0,
  FakeMachineApi? api,
}) {
  final fake = api ?? FakeMachineApi();
  fake.responder ??= (type, fields) {
    switch (type) {
      case 'chat.watch':
        return {
          'paneKey': 'a',
          'agent': 'claude',
          'gen': 'g1',
          'older': null,
          'items': [],
        };
      case 'prompts.list':
        return listReply(
          items ??
              [
                item('p1', 'Review', 'Review {project}', text: 'Review api'),
                item('p2', 'Tests', 'Run the tests'),
              ],
          hidden: hidden,
        );
      case 'prompts.save' || 'prompts.delete':
        return okReply;
    }
    return {};
  };
  return Harness(
    api: fake,
    viewState: view(
      ws: workspace([
        tab('t1', [pane('a', title: 'fix login')], project: 'C:\\work\\api'),
      ]),
      snapshot: dashboard(
        terminals: [
          {'id': 't-a', 'lastActivityAt': '2026-11-20T09:00:00Z'},
        ],
      ),
      hello: {
        'chat': 1,
        'caps': [if (cap) 'prompts'],
      },
    ),
    linkState: linkReady,
  );
}

TextEditingController inputOf(WidgetTester tester) => tester
    .widget<TextField>(find.byKey(const ValueKey('chat-input')))
    .controller!;

Future<void> tapChip(WidgetTester tester, String key) async {
  final chip = find.byKey(ValueKey(key));
  await tester.ensureVisible(chip);
  await tester.pump();
  await tester.tap(chip);
  await tester.pump();
}

Future<void> pressChip(WidgetTester tester, String key) async {
  final chip = find.byKey(ValueKey(key));
  await tester.ensureVisible(chip);
  await tester.pump();
  await tester.longPress(chip);
  await tester.pumpAndSettle();
}

Future<Harness> open(WidgetTester tester, Harness h) async {
  await h.pump(tester, const ChatScreen(machineId: 'm1', paneKey: 'a'));
  await tester.pump();
  await tester.pump();
  return h;
}

void main() {
  setUp(chatLogCache.clear);

  group('insertAtCursor', () {
    test('puts the text at the cursor and moves the cursor after it', () {
      final v = insertAtCursor(at('abc def', 3), 'X')!;
      expect(v.text, 'abc X def');
      expect(v.selection, const TextSelection.collapsed(offset: 5));
    });

    test('adds a space only when the text before the cursor needs one', () {
      expect(insertAtCursor(at('abc', 3), 'X')!.text, 'abc X');
      expect(insertAtCursor(at('abc ', 4), 'X')!.text, 'abc X');
      expect(insertAtCursor(at('abc\n', 4), 'X')!.text, 'abc\nX');
      expect(insertAtCursor(at('', 0), 'X')!.text, 'X');
      expect(insertAtCursor(at('abc', 0), 'X')!.text, 'Xabc');
    });

    test('replaces a selection and appends when there is no cursor', () {
      expect(
        insertAtCursor(at('hello world', 6, 11), 'there')!.text,
        'hello there',
      );
      expect(
        insertAtCursor(const TextEditingValue(text: 'abc'), 'X')!.text,
        'abc X',
      );
    });

    test('keeps multi line text as is and refuses to pass the box limit', () {
      expect(insertAtCursor(at('', 0), 'a\nb\n')!.text, 'a\nb\n');
      expect(insertAtCursor(at('x' * 1990, 1990), 'y' * 20), isNull);
      expect(insertAtCursor(at('x' * 1990, 1990), 'y' * 9)!.text.length, 2000);
      expect(insertAtCursor(at('abc', 1), '')!.text, 'abc');
    });
  });

  group('models', () {
    test('a list ignores broken rows and reads counters', () {
      final list = PromptList.parse({
        'items': [
          item('a', 'A', 'b', text: 'filled', missing: ['branch'], local: true),
          {'id': 1},
          'x',
          {'id': 'z', 'title': 'Z', 'body': 'zz'},
        ],
        'hidden': 2,
        'more': 1,
      });
      expect(list.items.map((p) => p.id), ['a', 'z']);
      expect((list.hidden, list.more), (2, 1));
      expect(list.items[0].insertText, 'filled');
      expect(list.items[0].missing, ['branch']);
      expect(list.items[0].local, true);
      expect(list.items[1].insertText, 'zz');
      expect(list.items[1].scope, 'all');
      expect(PromptList.parse({}).items, isEmpty);
    });

    test('a draft is checked against the desktop limits', () {
      expect(draftError(const PromptDraft(title: 'a', body: 'b')), isNull);
      expect(
        draftError(const PromptDraft(title: ' ', body: 'b')),
        'invalid-title',
      );
      expect(
        draftError(PromptDraft(title: 'a' * 81, body: 'b')),
        'invalid-title',
      );
      expect(
        draftError(const PromptDraft(title: 'a', body: '  ')),
        'invalid-body',
      );
      expect(
        draftError(PromptDraft(title: 'a', body: 'b' * 4001)),
        'invalid-body',
      );
      expect(
        const PromptDraft(
          id: 'i',
          title: ' T ',
          body: 'B',
          scope: 'C:\\w',
        ).toFields(),
        {'promptId': 'i', 'title': 'T', 'body': 'B', 'scope': 'C:\\w'},
      );
      expect(
        const PromptDraft(title: 'T', body: 'B').toFields().containsKey('id'),
        false,
      );
    });
  });

  group('controller', () {
    test(
      'loads for its pane, reloads on the changed event and after writes',
      () async {
        final api = FakeMachineApi();
        var rows = [item('a', 'A', 'b')];
        api.responder = (type, fields) =>
            type == 'prompts.list' ? listReply(rows, hidden: 3) : okReply;
        final c = PromptsController(
          api: PromptsApi(ViewerApi(api)),
          paneKey: 'p9',
        );
        await Future<void>.delayed(Duration.zero);
        expect(c.status, PromptsStatus.ready);
        expect(c.list.items.single.id, 'a');
        expect(c.list.hidden, 3);
        expect(api.commands.first.$1, 'prompts.list');
        expect(api.commands.first.$2, {'paneKey': 'p9'});

        rows = [item('a', 'A', 'b'), item('b', 'B', 'c')];
        api.eventBus.add({'t': 'prompts', 'count': 2});
        await Future<void>.delayed(Duration.zero);
        expect(c.list.items.length, 2);

        await c.save(const PromptDraft(id: 'a', title: 'A2', body: 'x'));
        expect(
          api.commands.any((e) => e.$1 == 'prompts.save' && e.$2!['promptId'] == 'a'),
          true,
        );
        await c.remove('b');
        expect(
          api.commands.any(
            (e) => e.$1 == 'prompts.delete' && e.$2!['promptId'] == 'b',
          ),
          true,
        );
        c.dispose();
        api.eventBus.add({'t': 'prompts', 'count': 3});
        await Future<void>.delayed(Duration.zero);
      },
    );

    test('a failed load shows as failed and a retry recovers', () async {
      final api = FakeMachineApi()
        ..commandError = const RemoteCommandException('offline');
      final c = PromptsController(
        api: PromptsApi(ViewerApi(api)),
        paneKey: 'a',
      );
      await Future<void>.delayed(Duration.zero);
      expect(c.status, PromptsStatus.failed);
      api
        ..commandError = null
        ..responder = (t, f) => listReply([item('a', 'A', 'b')]);
      await c.load();
      expect(c.status, PromptsStatus.ready);
      c.dispose();
    });

    test('desk errors reach the caller so the sheet can show them', () async {
      final api = FakeMachineApi();
      api.responder = (type, fields) => type == 'prompts.list'
          ? listReply([])
          : {
              'data': {'ok': false, 'error': 'secret'},
            };
      final c = PromptsController(
        api: PromptsApi(ViewerApi(api)),
        paneKey: 'a',
      );
      await Future<void>.delayed(Duration.zero);
      await expectLater(
        c.save(const PromptDraft(title: 'a', body: 'b')),
        throwsA(isA<ViewerFailure>().having((e) => e.code, 'code', 'secret')),
      );
      c.dispose();
    });
  });

  group('composer strip', () {
    testWidgets('chips appear for a desktop with the prompts cap', (
      tester,
    ) async {
      final h = await open(tester, chatHarness(hidden: 2));
      expect(find.byKey(const ValueKey('prompt-strip')), findsOneWidget);
      expect(find.byKey(const ValueKey('prompt-chip-p1')), findsOneWidget);
      expect(find.byKey(const ValueKey('prompt-chip-p2')), findsOneWidget);
      expect(find.byKey(const ValueKey('prompt-add')), findsOneWidget);
      await tester.scrollUntilVisible(
        find.text('2 on the computer only'),
        100,
        scrollable: find.descendant(
          of: find.byKey(const ValueKey('prompt-strip')),
          matching: find.byType(Scrollable),
        ),
      );
      expect(find.text('2 on the computer only'), findsOneWidget);
      expect(
        h.api.commands.any(
          (e) => e.$1 == 'prompts.list' && e.$2!['paneKey'] == 'a',
        ),
        true,
      );
    });

    testWidgets(
      'an old desktop without the cap shows no strip and asks nothing',
      (tester) async {
        final h = await open(tester, chatHarness(cap: false));
        expect(find.byKey(const ValueKey('prompt-strip')), findsNothing);
        expect(h.api.commands.any((e) => e.$1.startsWith('prompts.')), false);
      },
    );

    testWidgets('a tap puts the filled text at the cursor and never sends', (
      tester,
    ) async {
      final h = await open(tester, chatHarness());
      final input = inputOf(tester);
      input.value = at('please ', 7);
      await tapChip(tester, 'prompt-chip-p1');
      await tester.pump();
      expect(input.text, 'please Review api');
      expect(input.selection, const TextSelection.collapsed(offset: 17));
      input.value = at('AB', 1);
      await tapChip(tester, 'prompt-chip-p2');
      await tester.pump();
      expect(input.text, 'A Run the testsB');
      expect(h.api.commands.any((e) => e.$1 == 'ws.send'), false);
    });

    testWidgets(
      'a prompt with unknown variables is inserted as written and says so',
      (tester) async {
        final h = await open(
          tester,
          chatHarness(
            items: [
              item('m', 'Branch', 'on {branch}', missing: ['branch']),
            ],
          ),
        );
        await tapChip(tester, 'prompt-chip-m');
        await tester.pump();
        expect(inputOf(tester).text, 'on {branch}');
        expect(find.text('Left as written: branch'), findsOneWidget);
        expect(h.api.commands.any((e) => e.$1 == 'ws.send'), false);
      },
    );

    testWidgets(
      'a prompt that does not fit the box is refused and the draft is kept',
      (tester) async {
        await open(tester, chatHarness(items: [item('big', 'Big', 'y' * 300)]));
        final input = inputOf(tester);
        input.value = at('x' * 1900, 1900);
        await tapChip(tester, 'prompt-chip-big');
        await tester.pump();
        expect(input.text.length, 1900);
        expect(
          find.text('Not enough room in the message box for this prompt'),
          findsOneWidget,
        );
      },
    );

    testWidgets('long press then Edit saves the raw body with the same id', (
      tester,
    ) async {
      final h = await open(tester, chatHarness());
      await pressChip(tester, 'prompt-chip-p1');
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('prompt-action-edit')));
      await tester.pumpAndSettle();
      final body = tester.widget<TextField>(
        find.byKey(const ValueKey('prompt-body')),
      );
      expect(body.controller!.text, 'Review {project}');
      await tester.enterText(
        find.byKey(const ValueKey('prompt-title')),
        'Review 2',
      );
      await tester.tap(find.byKey(const ValueKey('prompt-save')));
      await tester.pumpAndSettle();
      final save = h.api.commands.lastWhere((e) => e.$1 == 'prompts.save').$2!;
      expect(save, {
        'promptId': 'p1',
        'title': 'Review 2',
        'body': 'Review {project}',
        'scope': 'all',
      });
      expect(find.byKey(const ValueKey('prompt-save')), findsNothing);
    });

    testWidgets('long press then Delete asks first and then deletes', (
      tester,
    ) async {
      final h = await open(tester, chatHarness());
      await pressChip(tester, 'prompt-chip-p2');
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('prompt-action-delete')));
      await tester.pumpAndSettle();
      expect(h.api.commands.any((e) => e.$1 == 'prompts.delete'), false);
      await tester.tap(find.byKey(const ValueKey('prompt-delete-yes')));
      await tester.pumpAndSettle();
      expect(h.api.commands.lastWhere((e) => e.$1 == 'prompts.delete').$2, {
        'promptId': 'p2',
      });
    });

    testWidgets(
      'the plus button adds a prompt for all projects or this project',
      (tester) async {
        final h = await open(tester, chatHarness());
        await tester.tap(find.byKey(const ValueKey('prompt-add')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('prompt-save')));
        await tester.pumpAndSettle();
        expect(find.byKey(const ValueKey('prompt-error')), findsOneWidget);
        expect(h.api.commands.any((e) => e.$1 == 'prompts.save'), false);
        await tester.enterText(
          find.byKey(const ValueKey('prompt-title')),
          'Fix',
        );
        await tester.enterText(
          find.byKey(const ValueKey('prompt-body')),
          'Fix it\nnow',
        );
        await tester.tap(find.byKey(const ValueKey('prompt-scope-here')));
        await tester.pump();
        await tester.tap(find.byKey(const ValueKey('prompt-save')));
        await tester.pumpAndSettle();
        final save = h.api.commands
            .lastWhere((e) => e.$1 == 'prompts.save')
            .$2!;
        expect(save, {
          'title': 'Fix',
          'body': 'Fix it\nnow',
          'scope': 'C:\\work\\api',
        });
      },
    );

    testWidgets('a desk error stays in the sheet with a readable message', (
      tester,
    ) async {
      final api = FakeMachineApi();
      api.responder = (type, fields) {
        if (type == 'chat.watch') {
          return {
            'paneKey': 'a',
            'agent': 'claude',
            'gen': 'g',
            'older': null,
            'items': [],
          };
        }
        if (type == 'prompts.list') return listReply([]);
        return {
          'data': {'ok': false, 'error': 'secret'},
        };
      };
      await open(tester, chatHarness(api: api));
      await tester.tap(find.byKey(const ValueKey('prompt-add')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byKey(const ValueKey('prompt-title')), 'Env');
      await tester.enterText(
        find.byKey(const ValueKey('prompt-body')),
        'API_KEY=abc',
      );
      await tester.tap(find.byKey(const ValueKey('prompt-save')));
      await tester.pumpAndSettle();
      expect(
        find.text(
          'This looks like it holds a secret. Keep it on the computer.',
        ),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('prompt-save')), findsOneWidget);
    });

    testWidgets(
      'the strip scrolls sideways and the page does not overflow on a narrow phone',
      (tester) async {
        final rows = [
          for (var i = 0; i < 12; i++) item('i$i', 'Prompt number $i', 'b'),
        ];
        final h = chatHarness(items: rows);
        await h.pump(
          tester,
          const ChatScreen(machineId: 'm1', paneKey: 'a'),
          size: const Size(320, 640),
        );
        await tester.pump();
        await tester.pump();
        expect(tester.takeException(), isNull);
        final strip = tester.getRect(
          find.byKey(const ValueKey('prompt-strip')),
        );
        expect(strip.right, lessThanOrEqualTo(320));
        await tester.drag(
          find.byKey(const ValueKey('prompt-strip')),
          const Offset(-400, 0),
        );
        await tester.pump();
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets('the vietnamese strings are present', (tester) async {
      final h = chatHarness();
      await h.pump(
        tester,
        const ChatScreen(machineId: 'm1', paneKey: 'a'),
        locale: const Locale('vi'),
      );
      await tester.pump();
      await tester.pump();
      expect(find.text('Prompt mới'), findsOneWidget);
    });
  });
}
