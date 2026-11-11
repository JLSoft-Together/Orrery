import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/cache.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/store.dart';

class Harness {
  Harness({MachineCache? cache, Map<String, Uint8List> Function()? tails}) {
    _store = MachineStore(
      machineId: 'm1',
      commander: (type, [fields]) {
        sent.add({'t': type, ...?fields});
        return Future.value(<String, dynamic>{'ok': true});
      },
      cache: cache,
      tailsSource: tails,
      now: () => DateTime(2026, 11, 12, 9, 30),
    );
  }

  late final MachineStore _store;
  final List<Map<String, dynamic>> sent = [];

  MachineStore get s => _store;

  void snapshot({int seq = 5, List<Map<String, dynamic>>? terminals}) {
    _store.handleEvent({
      't': 'snapshot',
      'seq': seq,
      'data': {
        'terminals':
            terminals ??
            [
              {'id': 't1', 'title': 'one', 'status': 'idle'},
            ],
        'agents': [],
        'activity': [],
      },
    });
  }

  void events(int seq, List<Map<String, dynamic>> events, {bool? direct}) {
    _store.handleEvent({
      't': 'events',
      'seq': seq,
      'events': events,
      ...?(direct == null ? null : {'direct': direct}),
    });
  }

  List terminals() => _store.view.snapshot!['terminals'] as List;
}

void main() {
  test('snapshot then contiguous events are applied in order', () {
    final h = Harness()..snapshot();
    expect(h.s.view.seq, 5);
    expect(h.s.view.stale, isFalse);
    h.events(6, [
      {
        'type': 'terminal.updated',
        'payload': {'id': 't1', 'title': 'renamed'},
      },
      {
        'type': 'terminal.created',
        'payload': {'id': 't2', 'title': 'two'},
      },
    ]);
    expect(h.s.view.seq, 6);
    expect(h.terminals().map((t) => (t as Map)['title']), ['renamed', 'two']);
    expect((h.terminals().first as Map)['status'], 'idle');
    h.events(7, [
      {
        'type': 'terminal.removed',
        'payload': {'id': 't1'},
      },
      {
        'type': 'activity',
        'payload': {'id': 'a1'},
      },
    ]);
    expect(h.terminals(), hasLength(1));
    expect(h.s.view.snapshot!['activity'], hasLength(1));
    expect(h.sent, isEmpty);
  });

  test('a seq gap requests exactly one resync and skips the events', () {
    final h = Harness()..snapshot();
    h.events(8, [
      {
        'type': 'terminal.updated',
        'payload': {'id': 't1', 'title': 'lost'},
      },
    ]);
    h.events(9, [
      {
        'type': 'terminal.updated',
        'payload': {'id': 't1', 'title': 'lost2'},
      },
    ]);
    expect(h.sent, [
      {'t': 'resync'},
    ]);
    expect((h.terminals().first as Map)['title'], 'one');
    expect(h.s.view.seq, 5);
    h.snapshot(seq: 9);
    expect(h.s.view.seq, 9);
    h.events(12, []);
    expect(h.sent, hasLength(2));
  });

  test('old or duplicate seq is ignored, direct messages skip seq', () {
    final h = Harness()..snapshot();
    h.events(5, [
      {
        'type': 'terminal.updated',
        'payload': {'id': 't1', 'title': 'dup'},
      },
    ]);
    expect((h.terminals().first as Map)['title'], 'one');
    h.events(99, [
      {
        'type': 'terminal.updated',
        'payload': {'id': 't1', 'title': 'direct'},
      },
    ], direct: true);
    expect((h.terminals().first as Map)['title'], 'direct');
    expect(h.s.view.seq, 5);
    expect(h.sent, isEmpty);
  });

  test('events before any snapshot trigger a resync', () {
    final h = Harness();
    h.events(1, [
      {
        'type': 'terminal.removed',
        'payload': {'id': 'x'},
      },
    ]);
    expect(h.sent, [
      {'t': 'resync'},
    ]);
  });

  test('activity is capped at 500 entries', () {
    final h = Harness()..snapshot();
    h.events(6, [
      for (var i = 0; i < 520; i++)
        {
          'type': 'activity',
          'payload': {'id': 'a$i'},
        },
    ]);
    final list = h.s.view.snapshot!['activity'] as List;
    expect(list, hasLength(500));
    expect((list.first as Map)['id'], 'a20');
  });

  Map<String, dynamic> wsSnapshot(int rev) => {
    't': 'ws.snapshot',
    'rev': rev,
    'closedCount': 0,
    'tabs': [
      {
        'key': 'a',
        'name': 'one',
        'panes': [
          {'key': 'p1', 'title': 'claude'},
        ],
      },
    ],
  };

  test('ws.patch applies JSON Patch on the next rev', () {
    final h = Harness();
    h.s.handleEvent(wsSnapshot(1));
    h.s.handleEvent({
      't': 'ws.patch',
      'rev': 2,
      'ops': [
        {'op': 'replace', 'path': '/tabs/0/name', 'value': 'renamed'},
        {
          'op': 'add',
          'path': '/tabs/0/panes/-',
          'value': {'key': 'p2'},
        },
      ],
    });
    expect(h.s.view.wsRev, 2);
    expect((h.s.view.tabs.first as Map)['name'], 'renamed');
    expect(((h.s.view.tabs.first as Map)['panes'] as List), hasLength(2));
    expect(h.s.view.workspace!['rev'], 2);
    expect(h.sent, isEmpty);
  });

  test('ws.patch with a rev gap or bad op asks ws.get once', () {
    final h = Harness();
    h.s.handleEvent(wsSnapshot(1));
    h.s.handleEvent({'t': 'ws.patch', 'rev': 5, 'ops': []});
    h.s.handleEvent({
      't': 'ws.patch',
      'rev': 6,
      'ops': [
        {'op': 'remove', 'path': '/nope'},
      ],
    });
    expect(h.sent, [
      {'t': 'ws.get'},
    ]);
    expect(h.s.view.wsRev, 1);
    h.s.handleEvent(wsSnapshot(6));
    h.s.handleEvent({
      't': 'ws.patch',
      'rev': 7,
      'ops': [
        {'op': 'remove', 'path': '/nope'},
      ],
    });
    expect(h.sent, hasLength(2));
  });

  test('ws.patch with base applies only on the current rev', () {
    final h = Harness();
    h.s.handleEvent(wsSnapshot(1));
    h.s.handleEvent({
      't': 'ws.patch',
      'base': 1,
      'rev': 3,
      'ops': [
        {'op': 'replace', 'path': '/tabs/0/name', 'value': 'jumped'},
      ],
    });
    expect(h.s.view.wsRev, 3);
    expect((h.s.view.tabs.first as Map)['name'], 'jumped');
    h.s.handleEvent({
      't': 'ws.patch',
      'base': 2,
      'rev': 4,
      'ops': [
        {'op': 'replace', 'path': '/tabs/0/name', 'value': 'lost'},
      ],
    });
    expect(h.s.view.wsRev, 3);
    expect((h.s.view.tabs.first as Map)['name'], 'jumped');
    expect(h.sent, [
      {'t': 'ws.get'},
    ]);
  });

  test('stale ws.patch is ignored', () {
    final h = Harness();
    h.s.handleEvent(wsSnapshot(3));
    h.s.handleEvent({
      't': 'ws.patch',
      'rev': 3,
      'ops': [
        {'op': 'replace', 'path': '/tabs/0/name', 'value': 'x'},
      ],
    });
    expect((h.s.view.tabs.first as Map)['name'], 'one');
    expect(h.sent, isEmpty);
  });

  test('confirm.ask lives 60 s and answer sends confirm.answer', () {
    fakeAsync((async) {
      final h = Harness();
      h.s.handleEvent({
        't': 'confirm.ask',
        'cid': 'c1',
        'title': 'Close pane?',
        'ok': 'Close',
        'cancel': 'Keep',
      });
      h.s.handleEvent({
        't': 'confirm.ask',
        'cid': 'c2',
        'title': 'Other',
        'ok': 'Yes',
        'cancel': 'No',
      });
      expect(h.s.view.confirms.map((c) => c.cid), ['c1', 'c2']);
      unawaited(h.s.answerConfirm('c1', true));
      async.flushMicrotasks();
      expect(h.s.view.confirms.map((c) => c.cid), ['c2']);
      expect(h.sent.single, {'t': 'confirm.answer', 'cid': 'c1', 'ok': true});
      async.elapse(const Duration(seconds: 59));
      expect(h.s.view.confirms, hasLength(1));
      async.elapse(const Duration(seconds: 2));
      expect(h.s.view.confirms, isEmpty);
    });
  });

  test('leaving the ready state marks data stale, snapshot clears it', () {
    final h = Harness()..snapshot();
    expect(h.s.view.stale, isFalse);
    h.s.onLink(const LinkState(status: LinkStatus.reconnecting));
    expect(h.s.view.stale, isTrue);
    expect(h.s.view.snapshot, isNotNull);
    h.snapshot(seq: 1);
    expect(h.s.view.stale, isFalse);
  });

  test(
    'cache: written snapshot, workspace and pane tails are read back',
    () async {
      final dir = await Directory.systemTemp.createTemp('orrery_cache');
      addTearDown(() => dir.delete(recursive: true));
      final cache = MachineCache(dir);
      final tail = Uint8List.fromList(List.generate(300, (i) => i % 256));
      final writer = Harness(cache: cache, tails: () => {'pane1': tail});
      writer.snapshot(seq: 42);
      writer.s.handleEvent(wsSnapshot(7));
      await writer.s.flush();

      final reader = Harness(cache: cache);
      await reader.s.restoreFromCache();
      final view = reader.s.view;
      expect(view.fromCache, isTrue);
      expect(view.stale, isTrue);
      expect(view.seq, 42);
      expect(view.wsRev, 7);
      expect(view.updatedAt, DateTime(2026, 11, 12, 9, 30));
      expect((view.snapshot!['terminals'] as List).single['title'], 'one');
      expect(reader.s.cachedTails['pane1'], tail);

      reader.events(43, []);
      expect(reader.sent, [
        {'t': 'resync'},
      ]);
      reader.snapshot(seq: 50);
      expect(reader.s.view.fromCache, isFalse);
      expect(reader.s.view.stale, isFalse);
    },
  );

  test(
    'overlapping saves of one machine are queued and the last one wins',
    () async {
      final dir = await Directory.systemTemp.createTemp('orrery_cache');
      addTearDown(() => dir.delete(recursive: true));
      final cache = MachineCache(dir);
      CacheEntry entry(int seq) => CacheEntry(
        savedAt: DateTime(2026, 11, 12),
        seq: seq,
        wsRev: 0,
        tails: const {},
      );
      await Future.wait([
        for (var i = 1; i <= 12; i++) cache.save('m1', entry(i)),
      ]);
      expect((await cache.load('m1'))!.seq, 12);
    },
  );

  test('a corrupt cache file is ignored', () async {
    final dir = await Directory.systemTemp.createTemp('orrery_cache');
    addTearDown(() => dir.delete(recursive: true));
    await File('${dir.path}${Platform.pathSeparator}m1.json')
        .writeAsString('{oops');
    final h = Harness(cache: MachineCache(dir));
    await h.s.restoreFromCache();
    expect(h.s.view.snapshot, isNull);
  });

  test('hello reply is kept as machine info without reply fields', () {
    final h = Harness();
    h.s.handleEvent({
      't': 'hello',
      're': 'c1',
      'ok': true,
      'name': 'Studio',
      'version': '2.3.0',
    });
    expect(h.s.view.hello, {
      't': 'hello',
      'name': 'Studio',
      'version': '2.3.0',
    });
  });

  test('RemoteCommandException keeps code and detail', () {
    const error = RemoteCommandException('busy', 'x');
    expect(error.code, 'busy');
    expect(error.toString(), contains('busy'));
  });
}
