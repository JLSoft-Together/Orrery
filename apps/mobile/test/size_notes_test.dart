import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/size_notes.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/monitor/monitor_page.dart';

import 'support/ui_fakes.dart';
import 'support/wp31_scenario.dart';

MachineStore storeWith(List<String> sent, {Object? resyncError}) {
  return MachineStore(
    machineId: 'm1',
    commander: (type, [fields]) {
      sent.add(type);
      if (resyncError != null) return Future.error(resyncError);
      return Future.value(<String, dynamic>{'ok': true});
    },
    now: () => DateTime(2026, 11, 19, 9),
  );
}

Map<String, dynamic> snapshotEvent(Object? trimmed) => {
  't': 'snapshot',
  'seq': 3,
  'data': {'terminals': [], 'agents': [], 'activity': []},
  'trimmed': ?trimmed,
};

void main() {
  test('snapshot reads the trimmed note and ignores empty or junk', () {
    final store = storeWith([]);
    store.handleEvent(
      snapshotEvent({'activity': 12, 'agents': 3, 'text': true}),
    );
    expect(
      store.view.trimmed,
      const SnapshotTrim(activity: 12, agents: 3, text: true),
    );
    store.handleEvent(snapshotEvent({'activity': 0, 'agents': -1}));
    expect(store.view.trimmed, isNull);
    store.handleEvent(snapshotEvent('junk'));
    expect(store.view.trimmed, isNull);
    store.handleEvent(snapshotEvent(null));
    expect(store.view.trimmed, isNull);
  });

  test('limit event sets the notice and the next snapshot clears it', () {
    final store = storeWith([]);
    store.handleEvent({
      't': 'limit',
      'what': 'snapshot',
      'bytes': 2000000,
      'max': 1048576,
    });
    expect(
      store.view.limit,
      const SizeLimit(what: 'snapshot', bytes: 2000000, max: 1048576),
    );
    expect(store.view.snapshot, isNull);
    store.handleEvent({'t': 'limit', 'bytes': 5});
    expect(store.view.limit?.what, 'snapshot');
    store.handleEvent({'t': 'limit', 'what': 'x'});
    expect(store.view.limit?.max, SizeLimit.defaultMax);
    store.handleEvent(snapshotEvent(null));
    expect(store.view.limit, isNull);
  });

  test('resync reply error limit becomes the notice', () async {
    final sent = <String>[];
    final store = storeWith(
      sent,
      resyncError: const RemoteCommandException('limit'),
    );
    await store.resync();
    expect(sent, ['resync']);
    expect(store.view.limit?.what, 'snapshot');
    final other = storeWith(
      [],
      resyncError: const RemoteCommandException('timeout'),
    );
    await other.resync();
    expect(other.view.limit, isNull);
  });

  testWidgets('limit banner replaces the empty text and retry resyncs', (
    tester,
  ) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(limit: const SizeLimit(what: 'snapshot', bytes: 2500000)),
      linkState: linkReady,
    );
    await h.pump(tester, MonitorPage(machineId: 'm1', onOpen: (_) {}));
    await tester.pump();
    expect(find.byKey(const ValueKey('monitor-limit')), findsOneWidget);
    expect(find.text('Data too large to send'), findsOneWidget);
    expect(find.text('No sessions on the computer right now.'), findsNothing);
    await tester.tap(find.byKey(const ValueKey('monitor-limit-retry')));
    await tester.pump();
    expect(h.api.resyncs, 1);
  });

  testWidgets('trimmed shows a quiet note next to the sessions', (
    tester,
  ) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(
        ws: scenarioWorkspace(),
        snapshot: scenarioSnapshot(),
        trimmed: const SnapshotTrim(agents: 2),
      ),
      linkState: linkReady,
    );
    await h.pump(tester, MonitorPage(machineId: 'm1', onOpen: (_) {}));
    await tester.pump();
    expect(find.byKey(const ValueKey('monitor-trimmed')), findsOneWidget);
    expect(find.byKey(const ValueKey('monitor-limit')), findsNothing);
    expect(find.text('Review PR #482'), findsOneWidget);
  });
}
