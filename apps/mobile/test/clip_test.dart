import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/clip_sync.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/clip/clip_ui.dart';

import 'support/ui_fakes.dart';

Harness build({List<String> caps = const ['clip']}) => Harness(
  api: FakeMachineApi(),
  viewState: MachineView(hello: {'caps': caps}),
  linkState: linkReady,
);

Widget tile(String? text) => Scaffold(
  body: ClipTile(machineId: 'm1', read: () async => text),
);

Future<void> settle(WidgetTester tester) async {
  for (var i = 0; i < 4; i++) {
    await tester.pump(const Duration(milliseconds: 150));
  }
}

void main() {
  group('clipEventText', () {
    test('accepts plain clip events only', () {
      expect(clipEventText({'t': 'clip', 'text': 'hello'}), 'hello');
      expect(clipEventText({'t': 'clip', 'text': ''}), isNull);
      expect(clipEventText({'t': 'clip', 'text': 5}), isNull);
      expect(clipEventText({'t': 'clip'}), isNull);
      expect(clipEventText({'t': 'ws.patch', 'text': 'x'}), isNull);
      expect(
        clipEventText({'t': 'clip', 'text': 'a' * (maxClipBytes + 1)}),
        isNull,
      );
      expect(
        clipEventText({'t': 'clip', 'text': 'a' * maxClipBytes}),
        isNotNull,
      );
    });
  });

  group('pushClip', () {
    final calls = <(String, JsonMap?)>[];
    Future<JsonMap> ok(String type, [JsonMap? fields]) async {
      calls.add((type, fields));
      return {'re': 'x', 'ok': true};
    }

    setUp(calls.clear);

    test('sends the text and reports sent', () async {
      expect(await pushClip(ok, 'copied'), ClipPushResult.sent);
      expect(calls.single.$1, 'clip.push');
      expect(calls.single.$2, {'text': 'copied'});
    });

    test(
      'never calls the computer for an empty or oversized clipboard',
      () async {
        expect(await pushClip(ok, null), ClipPushResult.empty);
        expect(await pushClip(ok, ''), ClipPushResult.empty);
        expect(await pushClip(ok, 'é' * maxClipBytes), ClipPushResult.tooLarge);
        expect(calls, isEmpty);
      },
    );

    test('maps the computer errors', () async {
      Future<ClipPushResult> fail(String code) => pushClip(
        (type, [fields]) => Future.error(RemoteCommandException(code)),
        'x',
      );
      expect(await fail('secret'), ClipPushResult.secret);
      expect(await fail('limit'), ClipPushResult.tooLarge);
      expect(await fail('denied'), ClipPushResult.denied);
      expect(await fail('offline'), ClipPushResult.offline);
      expect(await fail('timeout'), ClipPushResult.offline);
      expect(await fail('internal'), ClipPushResult.failed);
      expect(
        await pushClip((type, [fields]) => Future.error(StateError('x')), 'x'),
        ClipPushResult.failed,
      );
    });
  });

  group('ClipTile', () {
    testWidgets('is hidden when the computer has no clip cap', (tester) async {
      final h = build(caps: ['devinfo']);
      await h.pump(tester, tile('abc'));
      expect(find.byKey(const ValueKey('clip-send')), findsNothing);
    });

    testWidgets('sends the phone clipboard and shows a snackbar', (
      tester,
    ) async {
      final h = build();
      await h.pump(tester, tile('from phone'));
      await tester.tap(find.byKey(const ValueKey('clip-send')));
      await settle(tester);
      expect(h.api.commands.single.$1, 'clip.push');
      expect(h.api.commands.single.$2, {'text': 'from phone'});
      expect(find.text('Sent to the computer clipboard'), findsOneWidget);
    });

    testWidgets('tells the owner when the clipboard has no text', (
      tester,
    ) async {
      final h = build();
      await h.pump(tester, tile(null));
      await tester.tap(find.byKey(const ValueKey('clip-send')));
      await settle(tester);
      expect(h.api.commands, isEmpty);
      expect(find.text('The phone clipboard has no text'), findsOneWidget);
    });

    testWidgets('shows the secret block from the computer', (tester) async {
      final h = build();
      h.api.commandError = const RemoteCommandException('secret');
      await h.pump(tester, tile('token=abcdef123456'));
      await tester.tap(find.byKey(const ValueKey('clip-send')));
      await settle(tester);
      expect(find.text('Looks like a secret, blocked'), findsOneWidget);
    });

    testWidgets('shows offline when the session dropped mid send', (
      tester,
    ) async {
      final h = build();
      h.api.commandError = const RemoteCommandException('offline');
      await h.pump(tester, tile('abc'));
      await tester.tap(find.byKey(const ValueKey('clip-send')));
      await settle(tester);
      expect(find.text('Computer offline.'), findsOneWidget);
    });
  });

  group('ClipListener', () {
    testWidgets(
      'copies a clip event into the phone clipboard and shows a snackbar',
      (tester) async {
        final h = build();
        final written = <String>[];
        await h.pump(
          tester,
          Scaffold(
            body: ClipListener(
              machineId: 'm1',
              write: (text) async => written.add(text),
              child: const Text('body'),
            ),
          ),
        );
        h.api.eventBus.add({'t': 'ws.patch'});
        h.api.eventBus.add({'t': 'clip', 'text': ''});
        h.api.eventBus.add({'t': 'clip', 'text': 'from desktop'});
        await settle(tester);
        expect(written, ['from desktop']);
        expect(
          find.text('Copied the computer clipboard to this phone'),
          findsOneWidget,
        );
      },
    );

    testWidgets('stops listening when removed', (tester) async {
      final h = build();
      final written = <String>[];
      await h.pump(
        tester,
        ClipListener(
          machineId: 'm1',
          write: (text) async => written.add(text),
          child: const SizedBox(),
        ),
      );
      await tester.pumpWidget(const SizedBox());
      h.api.eventBus.add({'t': 'clip', 'text': 'late'});
      await settle(tester);
      expect(written, isEmpty);
    });
  });
}
