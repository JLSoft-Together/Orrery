import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/media/dictation.dart';
import 'package:orrery_remote/media/media_flow.dart';
import 'package:orrery_remote/media/media_prefs.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/chat/chat_composer.dart';
import 'package:orrery_remote/ui/chat/chat_media.dart';
import 'package:orrery_remote/ui/viewers/viewer_api.dart';

import 'support/media_fakes.dart';
import 'support/ui_fakes.dart';

void main() {
  group('dictation', () {
    Future<(DictationFail?, List<String>)> run(
      FakeEngine engine,
      MediaPrefs prefs, {
      bool answer = true,
      List<int>? asked,
    }) async {
      final texts = <String>[];
      final fail = await Dictation(engine, prefs).start(
        localeId: 'vi_VN',
        askOnline: () async {
          asked?.add(1);
          return answer;
        },
        onText: (w, d) => texts.add(w),
      );
      return (fail, texts);
    }

    test('a refused microphone reports denied, no permission means no listen', () async {
      final engine = FakeEngine()
        ..initOk = false
        ..permission = false;
      expect((await run(engine, memPrefs())).$1, DictationFail.denied);
      expect(engine.listens, 0);
    });

    test('no recognizer at all reports unavailable', () async {
      final engine = FakeEngine()..initOk = false;
      expect((await run(engine, memPrefs())).$1, DictationFail.unavailable);
    });

    test('an offline pack listens on device without asking', () async {
      final engine = FakeEngine()..offline = true;
      final asked = <int>[];
      final (fail, texts) = await run(engine, memPrefs(), asked: asked);
      expect(fail, isNull);
      expect(asked, isEmpty);
      expect(engine.lastOnDevice, isTrue);
      engine.result!('xin chao', false);
      expect(texts, ['xin chao']);
    });

    test('without a pack it asks once, remembers yes and never asks again', () async {
      final prefs = memPrefs();
      final asked = <int>[];
      final first = await run(FakeEngine(), prefs, asked: asked);
      expect(first.$1, isNull);
      final engine = FakeEngine();
      final second = await run(engine, prefs, asked: asked);
      expect(second.$1, isNull);
      expect(asked.length, 1);
      expect(engine.lastOnDevice, isFalse);
    });

    test('saying no keeps the microphone off and asks again next time', () async {
      final prefs = memPrefs();
      final asked = <int>[];
      final engine = FakeEngine();
      final out = await run(engine, prefs, answer: false, asked: asked);
      expect(out.$1, DictationFail.onlineRefused);
      expect(engine.listens, 0);
      await run(FakeEngine(), prefs, answer: false, asked: asked);
      expect(asked.length, 2);
    });

    test('an engine that throws reports failed and can start again', () async {
      final engine = FakeEngine()
        ..offline = true
        ..throwOnListen = true;
      final dictation = Dictation(engine, memPrefs());
      Future<DictationFail?> go() => dictation.start(
        localeId: 'en_US',
        askOnline: () async => true,
        onText: (_, _) {},
      );
      expect(await go(), DictationFail.failed);
      engine.throwOnListen = false;
      expect(await go(), isNull);
      expect(dictation.listening, isTrue);
      await dictation.stop();
      expect(dictation.listening, isFalse);
    });

    test('joins dictated words after the existing draft', () {
      expect(joinDictated('', 'hello'), 'hello');
      expect(joinDictated('fix', 'the bug'), 'fix the bug');
      expect(joinDictated('fix ', 'the bug'), 'fix the bug');
      expect(joinDictated('fix', '  '), 'fix');
      expect(speechLocaleFor('vi'), 'vi_VN');
      expect(speechLocaleFor('fr'), 'fr');
    });
  });

  group('media flow', () {
    MediaFlow flow(FakeCall call, PickImage pick, {String root = 'D:\\proj'}) =>
        MediaFlow(call: call.call, root: root, prefs: memPrefs(), pick: pick);

    test('a cancelled picker sends nothing', () async {
      final call = FakeCall();
      final f = flow(call, (_) async => null);
      expect(await f.sendImage(PickSource.gallery), isNull);
      expect(call.calls, isEmpty);
      expect(f.sending, isFalse);
    });

    test('a picked photo is prepared and uploaded, a second tap meanwhile is busy', () async {
      final call = FakeCall();
      final f = flow(call, (_) async {
        await Future<void>.delayed(const Duration(milliseconds: 20));
        return jpeg(800, 600, extra: 500);
      });
      final first = f.sendImage(PickSource.camera);
      await expectLater(
        f.sendImage(PickSource.camera),
        throwsA(isA<MediaFailure>().having((e) => e.error, 'e', MediaError.busy)),
      );
      final sent = await first;
      expect(sent!.path, endsWith('20260101-000000.jpg'));
      expect(call.of('fs.put').length, 1);
      expect(f.sending, isFalse);
    });

    test('an unsupported file never reaches the desktop', () async {
      final call = FakeCall();
      final f = flow(call, (_) async => Uint8List.fromList(List.filled(50, 3)));
      await expectLater(
        f.sendImage(PickSource.gallery),
        throwsA(isA<MediaFailure>().having((e) => e.error, 'e', MediaError.unsupported)),
      );
      expect(call.calls, isEmpty);
      expect(f.sending, isFalse);
    });

    test('desktop failures map to user errors and free the flow', () async {
      final call = FakeCall()
        ..failOn = 0
        ..failAt = const ViewerFailure('expired');
      final f = flow(call, (_) async => jpeg(10, 10));
      await expectLater(
        f.sendImage(PickSource.gallery),
        throwsA(isA<MediaFailure>().having((e) => e.error, 'e', MediaError.interrupted)),
      );
      expect(f.sending, isFalse);
    });

    test('the gitignore offer is shown once per root', () async {
      final f = flow(FakeCall(), (_) async => null);
      expect(await f.shouldOfferIgnore(), isTrue);
      await f.declineIgnore();
      expect(await f.shouldOfferIgnore(), isFalse);
    });

    test('the path is added to the message, quoted when it has spaces', () {
      expect(insertPath('', '/a/b.jpg'), '/a/b.jpg ');
      expect(insertPath('look', '/a/b.jpg'), 'look /a/b.jpg ');
      expect(insertPath('look ', 'C:\\my dir\\b.jpg'), 'look "C:\\my dir\\b.jpg" ');
    });
  });

  group('composer', () {
    Harness harness() => Harness(
      api: FakeMachineApi(),
      viewState: MachineView(hello: const {}),
      linkState: linkReady,
    );

    Future<ChatComposerState> pumpComposer(
      WidgetTester tester,
      ComposerMedia media,
      List<String> sent,
    ) async {
      final key = GlobalKey<ChatComposerState>();
      await harness().pump(
        tester,
        Scaffold(
          body: Column(
            children: [
              const Spacer(),
              ChatComposer(
                key: key,
                hint: 'Message',
                enabled: true,
                working: false,
                onSend: (text, authed) async {
                  sent.add(text);
                  return true;
                },
                onStop: () {},
                media: media,
              ),
            ],
          ),
        ),
      );
      return key.currentState!;
    }

    testWidgets('holding the mic fills the box and never sends', (tester) async {
      final engine = FakeEngine()..offline = true;
      final sent = <String>[];
      await pumpComposer(
        tester,
        ComposerMedia(dictation: Dictation(engine, memPrefs())),
        sent,
      );
      await tester.enterText(find.byKey(const ValueKey('chat-input')), 'fix');
      final gesture = await tester.startGesture(
        tester.getCenter(find.byKey(const ValueKey('chat-mic'))),
      );
      await tester.pump(const Duration(milliseconds: 700));
      await tester.pump();
      expect(engine.listens, 1);
      engine.result!('the login bug', false);
      await tester.pump();
      expect(
        tester.widget<TextField>(find.byKey(const ValueKey('chat-input'))).controller!.text,
        'fix the login bug',
      );
      await gesture.up();
      await tester.pump();
      expect(engine.stops, 1);
      expect(sent, isEmpty);
    });

    testWidgets('a short tap on the mic only shows the hint', (tester) async {
      final engine = FakeEngine()..offline = true;
      await pumpComposer(
        tester,
        ComposerMedia(dictation: Dictation(engine, memPrefs())),
        [],
      );
      await tester.tap(find.byKey(const ValueKey('chat-mic')));
      await tester.pump();
      expect(find.textContaining('Hold the mic button'), findsOneWidget);
      expect(engine.listens, 0);
    });

    testWidgets('a refused microphone shows a message', (tester) async {
      final engine = FakeEngine()
        ..initOk = false
        ..permission = false;
      await pumpComposer(
        tester,
        ComposerMedia(dictation: Dictation(engine, memPrefs())),
        [],
      );
      final gesture = await tester.startGesture(
        tester.getCenter(find.byKey(const ValueKey('chat-mic'))),
      );
      await tester.pump(const Duration(milliseconds: 700));
      await tester.pump();
      await gesture.up();
      await tester.pump();
      expect(find.textContaining('Microphone access is off'), findsOneWidget);
    });

    testWidgets('the photo adds its path, then the gitignore write waits for the tap', (tester) async {
      final call = FakeCall()
        ..replies['fs.read'] = const ViewerReply(data: {'kind': 'missing'})
        ..replies['fs.write'] = const ViewerReply(data: {'ok': true});
      final f = MediaFlow(
        call: call.call,
        root: 'D:\\proj',
        prefs: memPrefs(),
        pick: (_) async => jpeg(100, 100, extra: 10),
      );
      final sent = <String>[];
      final state = await pumpComposer(tester, ComposerMedia(photo: f), sent);
      await tester.tap(find.byKey(const ValueKey('chat-photo')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('photo-gallery')));
      await tester.pumpAndSettle();
      expect(state.draft, contains('20260101-000000.jpg'));
      expect(find.text('Add .orrery/inbox/ to .gitignore?'), findsOneWidget);
      expect(call.of('fs.write'), isEmpty);
      await tester.tap(find.byKey(const ValueKey('ignore-skip')));
      await tester.pumpAndSettle();
      expect(call.of('fs.write'), isEmpty);
      expect(call.of('fs.read'), isEmpty);
      expect(await f.shouldOfferIgnore(), isFalse);
      expect(sent, isEmpty);
    });

    testWidgets('tapping Add writes the gitignore line', (tester) async {
      final call = FakeCall()
        ..replies['fs.read'] = const ViewerReply(data: {'kind': 'missing'})
        ..replies['fs.write'] = const ViewerReply(data: {'ok': true});
      final f = MediaFlow(
        call: call.call,
        root: 'D:\\proj',
        prefs: memPrefs(),
        pick: (_) async => jpeg(100, 100, extra: 10),
      );
      await pumpComposer(tester, ComposerMedia(photo: f), []);
      await tester.tap(find.byKey(const ValueKey('chat-photo')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('photo-camera')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('ignore-add')));
      await tester.pumpAndSettle();
      expect(call.of('fs.write').single['text'], '.orrery/inbox/\n');
    });

    testWidgets('a second photo in the same project does not ask again', (tester) async {
      final call = FakeCall();
      final prefs = memPrefs();
      await prefs.markIgnoreAsked('D:\\proj');
      final f = MediaFlow(
        call: call.call,
        root: 'D:\\proj',
        prefs: prefs,
        pick: (_) async => jpeg(100, 100, extra: 10),
      );
      await pumpComposer(tester, ComposerMedia(photo: f), []);
      await tester.tap(find.byKey(const ValueKey('chat-photo')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('photo-gallery')));
      await tester.pumpAndSettle();
      expect(find.text('Add .orrery/inbox/ to .gitignore?'), findsNothing);
    });

    testWidgets('an upload error is shown and the draft stays empty', (tester) async {
      final call = FakeCall()
        ..failOn = 0
        ..failAt = const RemoteCommandException('denied');
      final f = MediaFlow(
        call: call.call,
        root: 'D:\\proj',
        prefs: memPrefs(),
        pick: (_) async => jpeg(100, 100, extra: 10),
      );
      final state = await pumpComposer(tester, ComposerMedia(photo: f), []);
      await tester.tap(find.byKey(const ValueKey('chat-photo')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('photo-gallery')));
      await tester.pumpAndSettle();
      expect(find.textContaining('does not accept photos'), findsOneWidget);
      expect(state.draft, isEmpty);
    });

    testWidgets('without media nothing extra is drawn', (tester) async {
      await pumpComposer(tester, const ComposerMedia(), []);
      expect(find.byKey(const ValueKey('chat-mic')), findsNothing);
      expect(find.byKey(const ValueKey('chat-photo')), findsNothing);
    });
  });
}
