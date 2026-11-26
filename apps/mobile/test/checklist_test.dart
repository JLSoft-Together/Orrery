import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/checklist/checklist_model.dart';
import 'package:orrery_remote/ui/checklist/checklist_page.dart';
import 'package:orrery_remote/ui/viewers/viewer_menu.dart';

import 'support/ui_fakes.dart';

const String root = 'C:\\work\\alpha';

JsonMap ok(Object? data) => {'re': 'x', 'ok': true, 'data': data};

Map<String, dynamic> list({
  List<Map<String, dynamic>>? items,
  String script = 'npm run qa',
  String state = 'idle',
  String checkHead = 'aaa1111',
  String head = 'aaa1111',
}) => {
  'ok': true,
  'items': items ?? [],
  'check': {'script': script, 'state': state, 'at': 1, 'head': checkHead},
  'head': head,
};

Map<String, dynamic> item(
  String id,
  String text,
  bool done, [
  String by = 'owner',
]) => {'id': id, 'text': text, 'done': done, 'by': by, 'at': 1};

Harness build(
  JsonMap Function(String type, JsonMap? fields) responder, {
  List<String> caps = const ['checklist'],
}) {
  final h = Harness(
    api: FakeMachineApi(),
    viewState: MachineView(hello: {'caps': caps}),
    linkState: linkReady,
  );
  h.api.responder = responder;
  return h;
}

Future<void> settle(WidgetTester tester) async {
  for (var i = 0; i < 6; i++) {
    await tester.pump(const Duration(milliseconds: 150));
  }
}

void main() {
  group('ChecklistData', () {
    test('keeps a result only for the commit it ran on', () {
      ChecklistData parse(String state, String head) => ChecklistData.parse(
        list(state: state, checkHead: 'aaa1111', head: head),
      );
      expect(parse('pass', 'aaa1111').view, CheckView.pass);
      expect(parse('fail', 'aaa1111').view, CheckView.fail);
      expect(parse('pass', 'bbb2222').view, CheckView.stale);
      expect(parse('running', 'bbb2222').view, CheckView.running);
      expect(parse('idle', 'aaa1111').view, CheckView.idle);
      expect(ChecklistData.parse(list(script: '')).view, CheckView.none);
    });

    test('is ready only with nothing open and a passing check', () {
      final done = [item('t1', 'a', true)];
      final open = [item('t1', 'a', true), item('t2', 'b', false)];
      expect(ChecklistData.parse(list(items: done, state: 'pass')).ready, true);
      expect(
        ChecklistData.parse(list(items: open, state: 'pass')).ready,
        false,
      );
      expect(
        ChecklistData.parse(list(items: done, state: 'fail')).ready,
        false,
      );
      expect(ChecklistData.parse(list(items: open)).open, 1);
    });

    test('drops malformed rows and tolerates a missing check', () {
      final data = ChecklistData.parse({
        'items': [
          {'id': 't1', 'text': 'ok', 'done': true, 'by': 'agent'},
          {'id': 5, 'text': 'bad'},
          'junk',
        ],
      });
      expect(data.items.length, 1);
      expect(data.items.single.byAgent, true);
      expect(data.view, CheckView.none);
    });
  });

  testWidgets('shows items and the check state, ticking sends a toggle', (
    tester,
  ) async {
    var toggled = false;
    final h = build((type, f) {
      if (type == 'checklist.toggle') toggled = true;
      if (type == 'checklist.get' && !toggled) {
        return ok(
          list(
            items: [
              item('t1', 'Write tests', false),
              item('t2', 'Ship', true, 'agent'),
            ],
            state: 'pass',
          ),
        );
      }
      return ok(
        list(
          items: [item('t1', 'Write tests', true), item('t2', 'Ship', true)],
          state: 'pass',
        ),
      );
    });
    await h.pump(tester, const ChecklistPage(machineId: 'm1', root: root));
    await settle(tester);
    expect(find.text('Write tests'), findsOneWidget);
    expect(find.text('1 open'), findsOneWidget);
    expect(find.text('Passed'), findsOneWidget);
    expect(find.text('npm run qa'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('cl-item-t1')));
    await settle(tester);
    final toggle = h.api.commands.firstWhere((c) => c.$1 == 'checklist.toggle');
    expect(toggle.$2, {'dir': root, 'id': 't1', 'done': true});
    expect(find.text('Ready to merge'), findsOneWidget);
  });

  testWidgets(
    'run asks for the check through the desktop and tells when it started',
    (tester) async {
      final h = build((type, f) {
        if (type == 'checklist.run') return ok({'ok': true, 'started': true});
        return ok(list());
      });
      await h.pump(tester, const ChecklistPage(machineId: 'm1', root: root));
      await settle(tester);
      await tester.tap(find.byKey(const ValueKey('cl-run')));
      await settle(tester);
      expect(h.api.commands.where((c) => c.$1 == 'checklist.run').single.$2, {
        'dir': root,
      });
      expect(find.text('Check started on the computer'), findsOneWidget);
    },
  );

  testWidgets(
    'a run waiting for the phone confirm only asks, and a running check shows as running',
    (tester) async {
      final h = build((type, f) {
        if (type == 'checklist.run') {
          return {'re': 'x', 'ok': true, 'confirm': 'c1'};
        }
        return ok(list(state: 'running'));
      });
      await h.pump(tester, const ChecklistPage(machineId: 'm1', root: root));
      await settle(tester);
      expect(find.text('Running'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('cl-run')));
      await settle(tester);
      expect(find.text('Confirm on this phone to continue.'), findsOneWidget);
      expect(find.text('Check started on the computer'), findsNothing);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('a desktop that is already running a check reports it', (
    tester,
  ) async {
    final h = build((type, f) {
      if (type == 'checklist.run') {
        return ok({'ok': false, 'error': 'running'});
      }
      return ok(list());
    });
    await h.pump(tester, const ChecklistPage(machineId: 'm1', root: root));
    await settle(tester);
    await tester.tap(find.byKey(const ValueKey('cl-run')));
    await settle(tester);
    expect(
      find.text('A check is already running on the computer.'),
      findsOneWidget,
    );
  });

  testWidgets('run is off without a script', (tester) async {
    final h = build((type, f) => ok(list(script: '')));
    await h.pump(tester, const ChecklistPage(machineId: 'm1', root: root));
    await settle(tester);
    expect(find.text('No check script set'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('cl-run')));
    await settle(tester);
    expect(h.api.commands.where((c) => c.$1 == 'checklist.run'), isEmpty);
  });

  for (final has in [true, false]) {
    testWidgets(
      'the viewer menu ${has ? 'offers' : 'hides'} the checklist by the desktop cap',
      (tester) async {
        final h = build(
          (type, f) => ok(null),
          caps: has ? ['checklist'] : ['devinfo'],
        );
        await h.pump(
          tester,
          Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () => showViewerMenu(context, 'm1', root),
                child: const Text('menu'),
              ),
            ),
          ),
        );
        await tester.tap(find.text('menu'));
        await settle(tester);
        expect(
          find.byKey(const ValueKey('viewer-checklist')),
          has ? findsOneWidget : findsNothing,
        );
        expect(find.byKey(const ValueKey('viewer-run')), findsOneWidget);
      },
    );
  }
}
