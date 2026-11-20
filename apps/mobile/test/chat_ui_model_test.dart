import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations_en.dart';
import 'package:orrery_remote/l10n/app_localizations_vi.dart';
import 'package:orrery_remote/remote/chat_last.dart';
import 'package:orrery_remote/remote/chat_protocol.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/chat/chat_controller.dart';
import 'package:orrery_remote/ui/chat/chat_markdown.dart';
import 'package:orrery_remote/ui/chat/chat_rows.dart';
import 'package:orrery_remote/ui/chat/chat_text.dart';
import 'package:orrery_remote/ui/model/workspace_model.dart';

import 'support/ui_fakes.dart';

Map<String, dynamic> page(
  List<Map<String, dynamic>> items, {
  String gen = 'g1',
  int? older,
}) => {
  'paneKey': 'a',
  'agent': 'claude',
  'gen': gen,
  'items': items,
  'older': older,
};

Map<String, dynamic> me(String id, int seq, String text) => {
  'id': id,
  'k': 'me',
  'seq': seq,
  'rev': 0,
  'text': text,
};

Map<String, dynamic> ai(String id, int seq, String text, {int rev = 0}) => {
  'id': id,
  'k': 'ai',
  'seq': seq,
  'rev': rev,
  'text': text,
};

Future<void> settle() => Future<void>.delayed(Duration.zero);

void main() {
  group('inbox rows', () {
    WorkspaceModel model() => WorkspaceModel.parse(
      workspace([
        tab('t1', [
          pane('a', title: 'fix login', rail: 'idle'),
          pane('b', title: 'dark mode', rail: 'asked'),
          pane('c', title: 'dev', agent: null, kind: 'term'),
          pane('f', title: 'file', kind: 'file', agent: null),
        ], project: 'C:\\work\\api'),
      ]),
    );

    test(
      'waiting first, then by last activity, non terminal panes left out',
      () {
        final snapshot = dashboard(
          terminals: [
            {'id': 't-a', 'lastActivityAt': '2026-11-20T09:00:00Z'},
            {'id': 't-c', 'lastActivityAt': '2026-11-20T10:00:00Z'},
            {'id': 't-b', 'lastActivityAt': '2026-11-20T08:00:00Z'},
          ],
        );
        final rows = buildChatRows(model(), snapshot, {
          'a': ChatLast(paneKey: 'a', text: 'Fixed the redirect uri.'),
        });
        final groups = groupChatRows(rows);
        expect(groups.waiting.map((r) => r.pane.key), ['b']);
        expect(groups.rest.map((r) => r.pane.key), ['c', 'a']);
        expect(rows.first.project, 'api');
        expect(groupChatRows(rows, 'redirect').rest.map((r) => r.pane.key), [
          'a',
        ]);
      },
    );

    test('context % comes from the main agent of the terminal', () {
      final snapshot = dashboard(
        agents: [
          {'terminalId': 't-a', 'role': 'sub', 'contextPct': 10},
          {'terminalId': 't-a', 'role': 'main', 'contextPct': 64.4},
        ],
      );
      expect(contextPctOf(snapshot, 't-a'), 64);
      expect(contextPctOf(snapshot, 't-x'), isNull);
    });
  });

  group('text', () {
    test('tool summary follows the locale and capitalises', () {
      final sum = {ChatToolCat.edit: 3, ChatToolCat.run: 2};
      expect(
        toolSummary(AppLocalizationsVi(), sum),
        'Đã sửa 3 lần, chạy 2 lệnh',
      );
      expect(
        toolSummary(AppLocalizationsEn(), {ChatToolCat.read: 1}),
        'Read 1 file',
      );
      expect(catOfTool('apply_patch'), ChatToolCat.edit);
      expect(catOfTool('mcp__orrery__open_session'), ChatToolCat.other);
    });

    test('markdown blocks and inline runs', () {
      final blocks = parseMarkdown(
        '# Title\n\nLine one\nline two\n\n- a\n- b\n\n```ts\nconst x = 1;\n```\n1. one\n2. two',
      );
      expect(blocks.map((b) => b.runtimeType.toString()), [
        'MdHeading',
        'MdPara',
        'MdList',
        'MdCode',
        'MdList',
      ]);
      expect((blocks[1] as MdPara).text, 'Line one line two');
      expect((blocks[3] as MdCode).lang, 'ts');
      expect((blocks[4] as MdList).ordered, isTrue);
      final runs = parseInline('use `rg` and **bold** [link](http://x)');
      expect(runs.map((r) => r.text), [
        'use ',
        'rg',
        ' and ',
        'bold',
        ' ',
        'link',
      ]);
      expect(runs[1].styles, contains(MdStyle.code));
      expect(runs[3].styles, contains(MdStyle.bold));
      expect(runs[5].styles, contains(MdStyle.link));
    });
  });

  group('store chat.last', () {
    test('full replaces, partial upserts, null text removes', () {
      final store = MachineStore(
        machineId: 'm1',
        commander: (type, [fields]) async => <String, dynamic>{},
      );
      store.handleEvent({
        't': 'chat.last',
        'full': true,
        'items': [
          {'paneKey': 'a', 'text': 'one', 'at': 1763600000000},
          {'paneKey': 'b', 'text': 'two'},
        ],
      });
      expect(store.view.chatLast.keys, ['a', 'b']);
      store.handleEvent({
        't': 'chat.last',
        'items': [
          {'paneKey': 'a', 'text': null},
          {'paneKey': 'c', 'text': 'three'},
        ],
      });
      expect(store.view.chatLast.keys.toSet(), {'b', 'c'});
      store.handleEvent({'t': 'hello', 'chat': 1});
      expect(store.view.chatEnabled, isTrue);
    });
  });

  group('ChatController', () {
    late FakeMachineApi api;
    late Map<String, dynamic> watchReply;

    setUp(() {
      chatLogCache.clear();
      api = FakeMachineApi();
      watchReply = page([me('u1', 10, 'hi'), ai('a1', 20, 'hello')], older: 5);
      api.responder = (type, fields) {
        if (type == 'chat.watch') return watchReply;
        if (type == 'chat.page') return page([me('u0', 1, 'old')], older: null);
        if (type == 'ws.send') return {'submitted': true};
        return {};
      };
    });

    test('watch loads the newest page, events upsert by rev', () async {
      final chat = ChatController(api: api, paneKey: 'a')..start();
      await settle();
      expect(chat.loading, isFalse);
      expect(chat.items.map((i) => i.id), ['u1', 'a1']);
      api.eventBus.add({
        't': 'chat.items',
        'paneKey': 'a',
        'gen': 'g1',
        'items': [ai('a1', 20, 'hello there', rev: 2), ai('a2', 30, 'more')],
      });
      await settle();
      expect((chat.items[1] as ChatAi).text, 'hello there');
      expect(chat.items.length, 3);
      await chat.loadOlder();
      expect(chat.items.first.id, 'u0');
      expect(chat.log.hasOlder, isFalse);
      chat.dispose();
      await settle();
      expect(api.commands.last.$1, 'chat.unwatch');
    });

    test('reset and a new hello watch again', () async {
      final chat = ChatController(api: api, paneKey: 'a')..start();
      await settle();
      api.eventBus.add({'t': 'chat.reset', 'paneKey': 'a', 'gen': 'g2'});
      await settle();
      api.eventBus.add({'t': 'hello'});
      await settle();
      expect(api.commands.where((c) => c.$1 == 'chat.watch').length, 3);
      chat.dispose();
    });

    test('denied means an old desktop', () async {
      api.responder = (type, fields) =>
          throw const RemoteCommandException('denied');
      final chat = ChatController(api: api, paneKey: 'a')..start();
      await settle();
      expect(chat.unsupported, isTrue);
      chat.dispose();
    });

    test(
      'a sent message shows at once and leaves when the echo arrives',
      () async {
        final chat = ChatController(api: api, paneKey: 'a')..start();
        await settle();
        final done = chat.send('  run tests  ');
        expect(chat.outbox.single.text, 'run tests');
        expect(chat.outbox.single.state, OutboxState.sending);
        expect(await done, isTrue);
        expect(chat.outbox.single.state, OutboxState.sent);
        expect(api.commands.last.$2, {'paneKey': 'a', 'text': 'run tests'});
        api.eventBus.add({
          't': 'chat.items',
          'paneKey': 'a',
          'gen': 'g1',
          'items': [me('u2', 40, 'run tests')],
        });
        await settle();
        expect(chat.outbox, isEmpty);
        chat.dispose();
      },
    );

    test('a failed send is kept as unsent and can be sent again', () async {
      final chat = ChatController(api: api, paneKey: 'a')..start();
      await settle();
      api.commandError = const RemoteCommandException('offline');
      expect(await chat.send('hi'), isFalse);
      expect(chat.outbox.single.state, OutboxState.unsent);
      api.commandError = null;
      expect(await chat.resend(chat.outbox.single), isTrue);
      chat.dispose();
    });

    test('pause unwatches and ignores events, resume watches again', () async {
      final chat = ChatController(api: api, paneKey: 'a')..start();
      await settle();
      chat.pause();
      await settle();
      expect(api.commands.last.$1, 'chat.unwatch');
      api.eventBus.add({
        't': 'chat.items',
        'paneKey': 'a',
        'gen': 'g1',
        'items': [ai('a9', 90, 'x')],
      });
      await settle();
      expect(chat.items.length, 2);
      chat.resume();
      await settle();
      expect(api.commands.last.$1, 'chat.watch');
      chat.dispose();
    });

    test(
      'a reopened chat starts from the cached page, not a spinner',
      () async {
        final first = ChatController(api: api, paneKey: 'a', cacheKey: 'm1|a')
          ..start();
        await settle();
        first.dispose();
        final again = ChatController(api: api, paneKey: 'a', cacheKey: 'm1|a');
        expect(again.loading, isFalse);
        expect(again.items.length, 2);
        again.dispose();
      },
    );

    test('a menu answer sends ws.menu with sig and index', () async {
      final chat = ChatController(api: api, paneKey: 'a');
      final menu = RemoteMenu.parse({
        'sig': 's1',
        'items': [
          {'index': 1, 'label': 'Yes'},
        ],
      })!;
      await chat.answer(menu, menu.items.single);
      expect(api.commands.last.$1, 'ws.menu');
      expect(api.commands.last.$2, {'paneKey': 'a', 'sig': 's1', 'index': 1});
      unawaited(chat.stop(5));
      await settle();
      expect(api.commands.last.$2, {
        'paneKey': 'a',
        'keys': 'stop',
        'since': 5,
      });
      chat.dispose();
    });
  });
}
