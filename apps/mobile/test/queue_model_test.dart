import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/ui/model/workspace_model.dart';
import 'package:orrery_remote/ui/queue/queue_model.dart';

Map<String, dynamic> paneOf(
  String key, {
  String rail = 'asked',
  String? since,
  String? menuSig,
  String? question,
  bool snoozed = false,
}) => {
  'key': key,
  'kind': 'term',
  'title': key,
  'rail': rail,
  'agent': 'claude',
  'terminalId': 't-$key',
  'sessionId': 's-$key',
  'attention': 'waiting',
  'question': ?question,
  if (snoozed) 'snoozed': true,
  if (since != null) 'inbox': {'reason': 'question', 'since': since},
  if (menuSig != null)
    'menu': {
      'sig': menuSig,
      'kind': 'ask',
      'items': [
        {'index': 1, 'label': 'Yes'},
        {'index': 2, 'label': 'No'},
      ],
    },
};

WorkspaceModel modelOf(List<Map<String, dynamic>> panes) =>
    WorkspaceModel.parse({
      'tabs': [
        {
          'key': 't1',
          'project': 'C:\\work\\api',
          'tree': {'pane': panes.first['key']},
          'panes': panes,
        },
      ],
    });

void main() {
  test('orders the first batch by since, oldest wait first', () {
    final flow = QueueFlow();
    flow.sync(
      modelOf([
        paneOf('late', since: '2026-11-27T10:30:00Z'),
        paneOf('old', since: '2026-11-27T09:00:00Z'),
        paneOf('none'),
      ]),
      live: true,
    );
    expect([for (final s in flow.open) s.key], ['old', 'late', 'none']);
    expect(flow.total, 3);
    expect(flow.position, 1);
  });

  test('a pane that arrives later goes to the end of the run', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', since: '2026-11-27T10:00:00Z')]), live: true);
    flow.sync(
      modelOf([
        paneOf('a', since: '2026-11-27T10:00:00Z'),
        paneOf('b', since: '2026-11-27T08:00:00Z'),
      ]),
      live: true,
    );
    expect([for (final s in flow.open) s.key], ['a', 'b']);
    expect(flow.total, 2);
  });

  test('non waiting, snoozed and non terminal panes are not queued', () {
    final flow = QueueFlow();
    flow.sync(
      modelOf([
        paneOf('idle', rail: 'idle'),
        paneOf('snoozed', snoozed: true),
        paneOf('ok'),
      ]),
      live: true,
    );
    expect([for (final s in flow.open) s.key], ['ok']);
    expect(
      queueCount(
        modelOf([paneOf('a'), paneOf('b', snoozed: true), paneOf('c', rail: 'working')]),
      ),
      1,
    );
  });

  test('offline sync changes nothing', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a')]), live: true);
    final changed = flow.sync(modelOf([paneOf('b', rail: 'idle')]), live: false);
    expect(changed, isFalse);
    expect(flow.slot('a')!.phase, QueuePhase.ready);
  });

  test('a ready card becomes gone when the pane stops waiting', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    flow.sync(modelOf([paneOf('a', rail: 'working')]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.gone);
  });

  test('a ready card becomes gone when the pane is closed', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a'), paneOf('b')]), live: true);
    flow.sync(modelOf([paneOf('b')]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.gone);
    expect(flow.slot('b')!.phase, QueuePhase.ready);
  });

  test('a changed menu makes the card stale, reload follows the new one', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    flow.sync(modelOf([paneOf('a', menuSig: 's2')]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.stale);
    flow.refresh('a');
    expect(flow.slot('a')!.phase, QueuePhase.ready);
    expect(flow.slot('a')!.signature, 'm:s2');
  });

  test('a changed text question while typing makes the card stale', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', question: 'Name?')]), live: true);
    flow.sync(modelOf([paneOf('a', question: 'Which file?')]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.stale);
  });

  test('pending undo is never treated as gone', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    flow.begin('a', QueuePhase.pendingUndo);
    flow.sync(modelOf([paneOf('a', rail: 'working')]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.pendingUndo);
    flow.sync(modelOf([paneOf('b')]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.pendingUndo);
  });

  test('undo after the pane stopped waiting ends as gone', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    flow.begin('a', QueuePhase.pendingUndo);
    flow.sync(modelOf([paneOf('a', rail: 'working')]), live: true);
    flow.resume('a');
    expect(flow.slot('a')!.phase, QueuePhase.gone);
  });

  test('undo with a changed menu ends as stale', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    flow.begin('a', QueuePhase.pendingUndo);
    flow.sync(modelOf([paneOf('a', menuSig: 's2')]), live: true);
    flow.resume('a');
    expect(flow.slot('a')!.phase, QueuePhase.stale);
  });

  test('a snoozed card ignores the desktop and a late snoozed flag after undo', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    flow.begin('a', QueuePhase.snoozed);
    flow.sync(modelOf([paneOf('a', menuSig: 's1', snoozed: true)]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.snoozed);
    flow.slot('a')!.phase = QueuePhase.sending;
    flow.resume('a', afterUnsnooze: true);
    flow.sync(modelOf([paneOf('a', menuSig: 's1', snoozed: true)]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.ready);
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    flow.sync(modelOf([paneOf('a', menuSig: 's1', snoozed: true)]), live: true);
    expect(flow.slot('a')!.phase, QueuePhase.gone);
  });

  test('skip hides a card for the run and review brings it back', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a'), paneOf('b')]), live: true);
    flow.skip('a');
    expect(flow.current!.key, 'b');
    expect(flow.skippedCount, 1);
    expect(flow.position, 2);
    flow.remove('b');
    expect(flow.current, isNull);
    flow.reviewSkipped();
    expect(flow.current!.key, 'a');
    expect(flow.skippedCount, 0);
  });

  test('a sent card is replaced when the same pane asks something new', () {
    final flow = QueueFlow();
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    final first = flow.slot('a')!;
    flow.begin('a', QueuePhase.sent);
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    expect(flow.slot('a'), same(first));
    flow.remove('a');
    flow.sync(modelOf([paneOf('a', menuSig: 's1')]), live: true);
    expect(flow.slot('a'), same(first));
    flow.sync(modelOf([paneOf('a', menuSig: 's2')]), live: true);
    expect(flow.slot('a'), isNot(same(first)));
    expect(flow.current!.key, 'a');
    expect(flow.total, 2);
  });

  test('pane parse keeps question and snoozed, caps the question', () {
    final long = 'x' * 500;
    final parsed = PaneInfo.parse({
      'key': 'a',
      'question': long,
      'snoozed': true,
    })!;
    expect(parsed.question!.length, questionMax);
    expect(parsed.snoozed, isTrue);
    final plain = PaneInfo.parse({'key': 'b'})!;
    expect(plain.question, isNull);
    expect(plain.snoozed, isFalse);
  });
}
