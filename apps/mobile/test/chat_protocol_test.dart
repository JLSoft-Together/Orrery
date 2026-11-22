import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/chat_protocol.dart';

int tsConst(String source, String name) {
  final m = RegExp('export const $name = ([0-9* ]+)\r?\n').firstMatch(source)!;
  return m
      .group(1)!
      .split('*')
      .map((s) => int.parse(s.trim()))
      .reduce((a, b) => a * b);
}

List<String> tsList(String source, String name) {
  final m = RegExp('export const $name = \\[([^\\]]*)\\]').firstMatch(source)!;
  return RegExp("'([a-z]+)'")
      .allMatches(m.group(1)!)
      .map((x) => x.group(1)!)
      .toList();
}

Map<String, dynamic> me(
  String id,
  int seq, {
  int rev = 0,
  String text = 'hi',
}) => {'id': id, 'k': 'me', 'seq': seq, 'rev': rev, 'text': text};

Map<String, dynamic> ai(
  String id,
  int seq, {
  int rev = 0,
  String text = 'ok',
}) => {'id': id, 'k': 'ai', 'seq': seq, 'rev': rev, 'text': text};

void main() {
  test('limits and enums match packages/shared/src/remote.ts', () {
    final ts = File('../../packages/shared/src/remote.ts').readAsStringSync();
    expect(tsConst(ts, 'REMOTE_CHAT_PROTO'), ChatLimits.proto);
    expect(tsConst(ts, 'REMOTE_CHAT_PAGE'), ChatLimits.page);
    expect(tsConst(ts, 'REMOTE_CHAT_PAGE_MAX'), ChatLimits.pageMax);
    expect(tsConst(ts, 'REMOTE_CHAT_WATCH_MAX'), ChatLimits.watchMax);
    expect(tsConst(ts, 'REMOTE_CHAT_ME_MAX'), ChatLimits.meMax);
    expect(tsConst(ts, 'REMOTE_CHAT_AI_MAX'), ChatLimits.aiMax);
    expect(tsConst(ts, 'REMOTE_CHAT_TOOLS_MAX'), ChatLimits.toolsMax);
    expect(tsConst(ts, 'REMOTE_CHAT_FILES_MAX'), ChatLimits.filesMax);
    expect(tsConst(ts, 'REMOTE_CHAT_LAST_MAX'), ChatLimits.lastMax);
    expect(tsConst(ts, 'REMOTE_CHAT_ID_MAX'), ChatLimits.idMax);
    expect(tsConst(ts, 'REMOTE_MENU_ITEMS_MAX'), ChatLimits.menuItemsMax);
    expect(
      tsList(ts, 'REMOTE_CHAT_TOOL_CATS'),
      ChatToolCat.values.map((c) => c.name).toList(),
    );
    expect(
      tsList(ts, 'REMOTE_CHAT_SYS_KINDS'),
      ChatSysKind.values.map((c) => c.name).toList(),
    );
  });

  test('items parse tolerantly and unknown kinds are skipped', () {
    final page = ChatPage.parse({
      'paneKey': 'p1',
      'agent': 'codex',
      'gen': 'g1',
      'older': 1200,
      'extra': {'ignored': true},
      'items': [
        me('m1', 1300, text: 'Sửa lỗi'),
        {
          'id': 't1',
          'k': 'tools',
          'seq': 1400,
          'rev': 2,
          'sum': {'edit': 2, 'run': 1, 'future': 3, 'read': 0},
          'items': [
            {'tool': 'Edit', 'cat': 'edit', 'target': 'auth/google.ts'},
            {'tool': 'Bash', 'cat': 'run', 'target': 'npm test', 'err': true},
            {'tool': 'Zap', 'cat': 'teleport'},
            {'cat': 'edit'},
          ],
          'more': 4,
        },
        {
          'id': 'f1',
          'k': 'files',
          'seq': 1500,
          'rev': 0,
          'root': 'C:\\work\\api',
          'files': [
            {'path': 'auth/google.ts', 'add': 2, 'del': 1},
            {'path': 'test/auth.test.ts', 'add': 14},
            {'path': '~/notes.md', 'out': true},
          ],
        },
        {'id': 's1', 'k': 'sys', 'seq': 1600, 'rev': 0, 'kind': 'stop'},
        {'id': 's2', 'k': 'sys', 'seq': 1700, 'rev': 0, 'kind': 'dance'},
        {'id': 'x1', 'k': 'thinking', 'seq': 1800, 'rev': 0, 'text': 'secret'},
        {'id': '', 'k': 'me', 'seq': 1900, 'text': 'no id'},
        {'id': 'n1', 'k': 'ai', 'text': 'no seq'},
        ai('a1', 2000, text: '**done**'),
      ],
    })!;
    expect(page.agent, ChatAgent.codex);
    expect(page.older, 1200);
    expect(page.items.map((i) => i.id), ['m1', 't1', 'f1', 's1', 'a1']);
    final tools = page.items[1] as ChatTools;
    expect(tools.sum, {
      ChatToolCat.edit: 2,
      ChatToolCat.run: 1,
      ChatToolCat.other: 3,
    });
    expect(tools.items.map((t) => t.cat), [
      ChatToolCat.edit,
      ChatToolCat.run,
      ChatToolCat.other,
    ]);
    expect(tools.items[1].err, isTrue);
    expect(tools.more, 4);
    final files = page.items[2] as ChatFiles;
    expect([files.added, files.removed], [16, 1]);
    expect(files.files.last.outside, isTrue);
    expect((page.items[3] as ChatSys).kind, ChatSysKind.stop);
  });

  test('log dedupes by id, keeps the highest rev, sorts by seq', () {
    final log = ChatLog('p1')
      ..addPage(
        ChatPage.parse({
          'paneKey': 'p1',
          'agent': 'claude',
          'gen': 'g1',
          'older': 500,
          'items': [me('m2', 900), ai('a2', 950, rev: 1, text: 'part')],
        })!,
      );
    expect(
      log.apply(
        ChatEvent.parse({
          't': 'chat.items',
          'paneKey': 'p1',
          'gen': 'g1',
          'items': [
            ai('a2', 950, rev: 3, text: 'full'),
            ai('a2', 950, rev: 2, text: 'old'),
            me('m3', 1000),
          ],
        })!,
      ),
      isTrue,
    );
    expect(log.items.map((i) => i.id), ['m2', 'a2', 'm3']);
    expect((log.items[1] as ChatAi).text, 'full');

    log.addPage(
      ChatPage.parse({
        'paneKey': 'p1',
        'agent': 'claude',
        'gen': 'g1',
        'older': null,
        'items': [me('m1', 100), ai('a1', 200), me('m2', 900)],
      })!,
    );
    expect(log.items.map((i) => i.id), ['m1', 'a1', 'm2', 'a2', 'm3']);
    expect(log.hasOlder, isFalse);

    log.addPage(
      ChatPage.parse({
        'paneKey': 'p1',
        'agent': 'claude',
        'gen': 'g1',
        'older': 800,
        'items': [me('m3', 1000)],
      })!,
    );
    expect(log.hasOlder, isFalse);
  });

  test('view moves each files card to the end of its turn', () {
    final files = {
      'id': 'f1',
      'k': 'files',
      'seq': 101,
      'rev': 0,
      'root': '/w',
      'files': [
        {'path': 'a.ts', 'add': 1},
      ],
    };
    final log = ChatLog('p1')
      ..addPage(
        ChatPage.parse({
          'paneKey': 'p1',
          'agent': 'claude',
          'gen': 'g1',
          'older': null,
          'items': [
            me('m1', 100),
            files,
            ai('a1', 200),
            me('m2', 300),
            ai('a2', 400),
          ],
        })!,
      );
    expect(log.items.map((i) => i.id), ['m1', 'f1', 'a1', 'm2', 'a2']);
    expect(log.view.map((i) => i.id), ['m1', 'a1', 'f1', 'm2', 'a2']);
    log.apply(
      ChatEvent.parse({
        't': 'chat.items',
        'paneKey': 'p1',
        'gen': 'g1',
        'items': [
          {...files, 'id': 'f2', 'seq': 301},
        ],
      })!,
    );
    expect(log.view.map((i) => i.id), ['m1', 'a1', 'f1', 'm2', 'a2', 'f2']);
  });

  test(
    'a new gen or a reset drops the old log; a stale gen event marks it stale',
    () {
      final log = ChatLog('p1')
        ..addPage(
          ChatPage.parse({
            'paneKey': 'p1',
            'agent': 'claude',
            'gen': 'g1',
            'older': null,
            'items': [me('m1', 1)],
          })!,
        );
      expect(
        log.apply(
          ChatEvent.parse({
            't': 'chat.items',
            'paneKey': 'p1',
            'gen': 'g0',
            'items': [me('z', 2)],
          })!,
        ),
        isFalse,
      );
      expect(log.stale, isTrue);
      expect(log.items.map((i) => i.id), ['m1']);
      expect(
        log.apply(
          ChatEvent.parse({
            't': 'chat.items',
            'paneKey': 'p2',
            'gen': 'g1',
            'items': [me('z', 2)],
          })!,
        ),
        isFalse,
      );

      expect(
        log.apply(
          ChatEvent.parse({'t': 'chat.reset', 'paneKey': 'p1', 'gen': 'g2'})!,
        ),
        isTrue,
      );
      expect(log.items, isEmpty);
      expect(log.gen, 'g2');
      expect(log.stale, isTrue);

      log.addPage(
        ChatPage.parse({
          'paneKey': 'p1',
          'agent': 'claude',
          'gen': 'g3',
          'older': 10,
          'items': [me('n1', 20)],
        })!,
      );
      expect(log.items.map((i) => i.id), ['n1']);
      expect(log.stale, isFalse);
    },
  );

  test('chat.last and unknown events', () {
    final e = ChatEvent.parse({
      't': 'chat.last',
      'full': true,
      'items': [
        {'paneKey': 'p1', 'text': 'Đã sửa xong', 'at': 1763600000000},
        {'paneKey': 'p2', 'text': null},
        {'text': 'no pane'},
      ],
    }) as ChatLastEvent;
    expect(e.full, isTrue);
    expect(e.items.map((i) => i.paneKey), ['p1', 'p2']);
    expect(e.items[0].at, isNotNull);
    expect(e.items[1].text, isNull);
    expect(ChatEvent.parse({'t': 'chat.future'}), isNull);
    expect(ChatEvent.parse('nope'), isNull);
  });

  test(
    'menu parses items, skips bad rows, rejects menus without sig or items',
    () {
      final menu = RemoteMenu.parse({
        'sig': '1a2b3c4d',
        'kind': 'ask-multi',
        'agent': 'claude',
        'question': 'Chọn cách làm',
        'numbered': true,
        'confident': true,
        'since': 1763600000000,
        'items': [
          {
            'index': 1,
            'label': 'Một danh sách',
            'selected': true,
            'checked': false,
          },
          {'index': 2, 'label': 'Hai tab', 'checked': true},
          {'index': 3, 'label': 'Other', 'other': true},
          {'index': 4, 'label': 'Submit', 'submit': true},
          {'index': 0, 'label': 'bad'},
          {'index': 5},
        ],
      })!;
      expect(menu.kind, RemoteMenuKind.askMulti);
      expect(menu.items.map((i) => i.index), [1, 2, 3, 4]);
      expect(menu.items.first.selected, isTrue);
      expect(menu.items[1].checked, isTrue);
      expect(menu.answerable.map((i) => i.index), [1, 2, 4]);
      expect(
        RemoteMenu.parse({
          'kind': 'ask',
          'items': [
            {'index': 1, 'label': 'a'},
          ],
        }),
        isNull,
      );
      expect(
        RemoteMenu.parse({'sig': 'x', 'kind': 'ask', 'items': []}),
        isNull,
      );
      expect(
        RemoteMenu.parse({
          'sig': 'x',
          'kind': 'new-kind',
          'items': [
            {'index': 1, 'label': 'a'},
          ],
        })!.kind,
        RemoteMenuKind.unknown,
      );
    },
  );
}
