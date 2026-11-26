import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/away_store.dart';
import 'package:orrery_remote/remote/away_summary.dart';
import 'package:orrery_remote/remote/away_tracker.dart';

final t0 = DateTime.utc(2026, 11, 27, 10);
String iso(DateTime t) => t.toIso8601String();
DateTime plus(int minutes, [int seconds = 0]) =>
    t0.add(Duration(minutes: minutes, seconds: seconds));

int _seq = 0;

Map<String, dynamic> ev(
  String kind,
  DateTime at, {
  String? terminal = 't1',
  Map<String, dynamic>? data,
}) => {
  'id': '${_seq++}',
  'at': iso(at),
  'kind': kind,
  'terminalId': ?terminal,
  'data': ?data,
};

Map<String, dynamic> term(String id, String status, {DateTime? since}) => {
  'id': id,
  'status': status,
  'statusSince': since == null ? null : iso(since),
};

Map<String, dynamic> snap({
  List<Map<String, dynamic>> terminals = const [],
  List<Map<String, dynamic>> activity = const [],
}) => {'terminals': terminals, 'activity': activity};

void main() {
  group('summarizeActivity', () {
    test('counts only events after leaving (parity with web test)', () {
      final items = [
        ev('terminal.started', plus(-1)),
        ev('terminal.ended', plus(1)),
        ev('agent.failed', plus(2)),
        ev('terminal.compacted', plus(3)),
      ];
      final tally = summarizeActivity(items, AwayMark(leftAt: t0));
      expect(tally.started, 0);
      expect(tally.ended, 1);
      expect(tally.failedAgents, 1);
      expect(tally.compactions, 1);
      expect(tally.completedAgents, 0);
      expect(tally.jobsDone, 0);
    });

    test('status working to idle is done, to error is failed', () {
      final items = [
        ev(
          'terminal.status',
          plus(1),
          terminal: 'a',
          data: {'from': 'working', 'to': 'idle'},
        ),
        ev('terminal.status', plus(2), terminal: 'b', data: {'to': 'error'}),
        ev('terminal.status', plus(3), terminal: 'c', data: {'to': 'waiting'}),
        ev('turn.completed', plus(4), terminal: 'd'),
        ev('turn.failed', plus(5), terminal: 'e'),
        ev('agent.completed', plus(6), terminal: 'f'),
        ev('job.done', plus(7), terminal: null),
      ];
      final tally = summarizeActivity(items, AwayMark(leftAt: t0));
      expect(tally.doneIds, {'a', 'd'});
      expect(tally.failedIds, {'b', 'e'});
      expect(tally.completedAgents, 1);
      expect(tally.agentDoneIds, {'f'});
      expect(tally.jobsDone, 1);
    });

    test('mark ref (desktop clock) beats the phone clock', () {
      final mark = AwayMark(leftAt: plus(100), ref: plus(0));
      final items = [ev('terminal.ended', plus(5))];
      expect(summarizeActivity(items, mark).ended, 1);
      expect(
        summarizeActivity([ev('terminal.ended', plus(0))], mark).ended,
        0,
      );
    });

    test('ignores malformed items', () {
      final tally = summarizeActivity([
        'x',
        {'kind': 'terminal.ended'},
        {'kind': 'terminal.ended', 'at': 'bad'},
      ], AwayMark(leftAt: t0));
      expect(tally.ended, 0);
    });
  });

  group('awayDue', () {
    final mark = AwayMark(leftAt: t0);
    test('under 30 minutes is not due', () {
      expect(awayDue(mark, plus(29, 59)), isFalse);
    });
    test('30 minutes is due', () {
      expect(awayDue(mark, plus(30)), isTrue);
    });
    test('clock moved back is not due', () {
      expect(awayDue(mark, plus(-5)), isFalse);
    });
  });

  group('computeAway', () {
    final mark = AwayMark(
      leftAt: t0,
      ref: t0,
      statuses: {'a': 'working', 'b': 'working', 'c': 'idle', 'd': 'idle'},
    );
    final now = plus(80);

    test('exact numbers from a sample activity chain and state', () {
      final s = snap(
        terminals: [
          term('a', 'idle', since: plus(20)),
          term('b', 'error', since: plus(30)),
          term('c', 'waiting', since: plus(50)),
          term('d', 'waiting', since: plus(40)),
          term('n', 'idle', since: plus(10)),
        ],
        activity: [
          ev('terminal.started', plus(-10), terminal: 'old'),
          ev('terminal.started', plus(10), terminal: 'n'),
          ev('terminal.ended', plus(11), terminal: 'gone'),
          ev('terminal.compacted', plus(12), terminal: 'a'),
          ev('agent.completed', plus(13), terminal: 'a'),
          ev('agent.failed', plus(14), terminal: 'b'),
          ev('job.done', plus(15), terminal: null),
        ],
      );
      final r = computeAway(snapshot: s, mark: mark, now: now)!;
      expect(r.done, ['a']);
      expect(r.failed, ['b']);
      expect(r.waiting, ['d', 'c']);
      expect(r.started, 1);
      expect(r.ended, 1);
      expect(r.compactions, 1);
      expect(r.completedAgents, 1);
      expect(r.failedAgents, 1);
      expect(r.jobsDone, 1);
      expect(r.compactIds, ['a']);
      expect(r.partial, isFalse);
      expect(r.away, const Duration(minutes: 80));
    });

    test('idle to idle with work in between counts via statusSince', () {
      final s = snap(terminals: [term('d', 'idle', since: plus(30))]);
      final r = computeAway(snapshot: s, mark: mark, now: now)!;
      expect(r.done, ['d']);
    });

    test('idle since before leaving is not done', () {
      final s = snap(terminals: [term('d', 'idle', since: plus(-30))]);
      expect(computeAway(snapshot: s, mark: mark, now: now), isNull);
    });

    test('terminal that was already error is not failed again', () {
      final m = AwayMark(leftAt: t0, ref: t0, statuses: {'a': 'error'});
      final s = snap(terminals: [term('a', 'error')]);
      expect(computeAway(snapshot: s, mark: m, now: now), isNull);
    });

    test('removed terminal is dropped from done and failed', () {
      final s = snap(
        terminals: [term('z', 'idle')],
        activity: [
          ev(
            'terminal.status',
            plus(5),
            terminal: 'gone',
            data: {'from': 'working', 'to': 'idle'},
          ),
        ],
      );
      expect(computeAway(snapshot: s, mark: mark, now: now), isNull);
    });

    test('waiting pane is not also counted as done', () {
      final s = snap(
        terminals: [term('a', 'waiting', since: plus(40))],
        activity: [
          ev(
            'terminal.status',
            plus(5),
            terminal: 'a',
            data: {'from': 'working', 'to': 'idle'},
          ),
        ],
      );
      final r = computeAway(snapshot: s, mark: mark, now: now)!;
      expect(r.done, isEmpty);
      expect(r.waiting, ['a']);
    });

    test('buffer cut: oldest item newer than cutoff marks partial', () {
      final s = snap(
        terminals: [term('a', 'idle', since: plus(20))],
        activity: [ev('terminal.ended', plus(30), terminal: 'x')],
      );
      final r = computeAway(snapshot: s, mark: mark, now: now)!;
      expect(r.partial, isTrue);
      expect(r.done, ['a']);
    });

    test('buffer reaching back before leaving is not partial', () {
      final s = snap(
        activity: [
          ev('terminal.started', plus(-5), terminal: 'x'),
          ev('terminal.compacted', plus(30), terminal: 'x'),
        ],
      );
      expect(computeAway(snapshot: s, mark: mark, now: now)!.partial, isFalse);
    });

    test('empty buffer is not partial', () {
      final s = snap(terminals: [term('c', 'waiting', since: plus(50))]);
      expect(computeAway(snapshot: s, mark: mark, now: now)!.partial, isFalse);
    });

    test('quiet or missing snapshot gives null', () {
      expect(computeAway(snapshot: snap(), mark: mark, now: now), isNull);
      expect(computeAway(snapshot: null, mark: mark, now: now), isNull);
    });

    test('ids per group feed the chip filter', () {
      final s = snap(
        terminals: [term('c', 'waiting', since: plus(50))],
        activity: [ev('terminal.compacted', plus(5), terminal: 'q')],
      );
      final r = computeAway(snapshot: s, mark: mark, now: now)!;
      expect(r.idsOf(AwayGroup.waiting), ['c']);
      expect(r.idsOf(AwayGroup.compact), ['q']);
      expect(r.countOf(AwayGroup.done), 0);
    });
  });

  group('markFromSnapshot', () {
    test('ref is the newest desktop time, statuses by id', () {
      final m = markFromSnapshot(
        snap(
          terminals: [term('a', 'working', since: plus(3))],
          activity: [ev('terminal.ended', plus(7))],
        ),
        plus(9),
      );
      expect(m.ref, plus(7));
      expect(m.leftAt, plus(9));
      expect(m.statuses, {'a': 'working'});
    });

    test('empty snapshot falls back to leftAt', () {
      final m = markFromSnapshot(null, t0);
      expect(m.ref, isNull);
      expect(m.cutoff, t0);
    });

    test('json round trip and garbage', () {
      final m = AwayMark(leftAt: t0, ref: plus(1), statuses: {'a': 'idle'});
      final back = AwayMark.fromJson(m.toJson())!;
      expect(back.leftAt.toUtc(), t0);
      expect(back.ref!.toUtc(), plus(1));
      expect(back.statuses, {'a': 'idle'});
      expect(AwayMark.fromJson('x'), isNull);
      expect(AwayMark.fromJson({'leftAt': 'bad'}), isNull);
    });
  });

  group('AwayTracker', () {
    late DateTime clock;
    late Map<String, AwayFeed> feeds;
    late MemoryAwayStore store;

    AwayTracker make() => AwayTracker(
      store: store,
      source: () => feeds,
      now: () => clock,
    );

    final leaving = snap(
      terminals: [term('a', 'working')],
      activity: [ev('terminal.started', plus(-5), terminal: 'a')],
    );
    final returning = snap(
      terminals: [term('a', 'idle', since: plus(20))],
      activity: [
        ev('terminal.started', plus(-5), terminal: 'a'),
        ev('terminal.compacted', plus(10), terminal: 'a'),
      ],
    );

    setUp(() {
      clock = t0;
      store = MemoryAwayStore();
      feeds = {'m1': AwayFeed(snapshot: leaving, fresh: true)};
    });

    test('back within 30 minutes shows nothing and clears the mark', () async {
      final t = make();
      await t.leave();
      expect(store.marks.keys, ['m1']);
      clock = plus(29);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: true)};
      expect(await t.back(), isFalse);
      expect(t.summaries, isEmpty);
      expect(store.marks, isEmpty);
    });

    test('back after 30 minutes with a fresh snapshot shows the card', () async {
      final t = make();
      await t.leave();
      clock = plus(80);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: true)};
      expect(await t.back(), isTrue);
      final s = t.summaries['m1']!;
      expect(s.done, ['a']);
      expect(s.compactions, 1);
      expect(s.since, t0);
      expect(store.marks, isEmpty);
    });

    test('app killed in background: new tracker reads the mark', () async {
      await make().leave();
      clock = plus(45);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: true)};
      final reborn = make();
      expect(await reborn.back(), isTrue);
      expect(reborn.summaries['m1']!.done, ['a']);
    });

    test('stale or cached snapshot waits for a fresh one', () async {
      await make().leave();
      clock = plus(45);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: false)};
      final t = make();
      expect(await t.back(), isFalse);
      expect(t.pendingIds, {'m1'});
      expect(store.marks.keys, ['m1']);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: true)};
      expect(await t.evaluate(), isTrue);
      expect(t.summaries['m1'], isNotNull);
      expect(t.pendingIds, isEmpty);
      expect(store.marks, isEmpty);
    });

    test('machine that never comes online keeps its mark', () async {
      await make().leave();
      clock = plus(45);
      feeds = {};
      final t = make();
      await t.back();
      expect(t.summaries, isEmpty);
      expect(store.marks.keys, ['m1']);
    });

    test('clock moved back drops the mark', () async {
      await make().leave();
      clock = plus(-60);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: true)};
      final t = make();
      expect(await t.back(), isFalse);
      expect(t.summaries, isEmpty);
      expect(store.marks, isEmpty);
    });

    test('several machines each get their own card', () async {
      feeds = {
        'm1': AwayFeed(snapshot: leaving, fresh: true),
        'm2': AwayFeed(
          snapshot: snap(terminals: [term('w', 'working')]),
          fresh: true,
        ),
      };
      final t = make();
      await t.leave();
      expect(store.marks.keys.toSet(), {'m1', 'm2'});
      clock = plus(60);
      feeds = {
        'm1': AwayFeed(snapshot: returning, fresh: true),
        'm2': AwayFeed(
          snapshot: snap(terminals: [term('w', 'waiting', since: plus(40))]),
          fresh: true,
        ),
      };
      await t.back();
      expect(t.summaries['m1']!.done, ['a']);
      expect(t.summaries['m2']!.waiting, ['w']);
    });

    test('second pause without resume keeps the first mark', () async {
      final t = make();
      await t.leave();
      clock = plus(10);
      await t.leave();
      expect(store.marks['m1']!.leftAt, t0);
    });

    test('dismiss hides the card until the next absence', () async {
      final t = make();
      await t.leave();
      clock = plus(80);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: true)};
      await t.back();
      expect(t.dismiss('m1'), isTrue);
      expect(t.summaries, isEmpty);
      expect(t.dismiss('m1'), isFalse);
    });

    test('a new absence clears an unseen card', () async {
      final t = make();
      await t.leave();
      clock = plus(80);
      feeds = {'m1': AwayFeed(snapshot: returning, fresh: true)};
      await t.back();
      expect(t.summaries, isNotEmpty);
      await t.leave();
      expect(t.summaries, isEmpty);
    });
  });
}
