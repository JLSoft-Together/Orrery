import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/media/image_prep.dart';
import 'package:orrery_remote/media/image_upload.dart';
import 'package:orrery_remote/media/inbox_ignore.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/ui/viewers/viewer_api.dart';

import 'support/media_fakes.dart';

void main() {
  group('image prep', () {
    test('reads the kind and size of png and jpeg headers', () {
      expect(imageKind(png(10, 20)), ImageKind.png);
      expect(imageKind(jpeg(10, 20)), ImageKind.jpeg);
      expect(imageKind(Uint8List.fromList([1, 2, 3, 4])), isNull);
      expect(imageSize(png(3000, 40))!.w, 3000);
      expect(imageSize(jpeg(4032, 3024))!.h, 3024);
      expect(imageSize(jpeg(4032, 3024))!.w, 4032);
    });

    test('a small image is passed through untouched', () async {
      final raw = jpeg(1000, 800, extra: 100);
      final out = await prepareImage(raw);
      expect(out.bytes, same(raw));
      expect(out.kind, ImageKind.jpeg);
    });

    test('an image with a side over 2048 px goes through the shrinker', () async {
      var asked = false;
      final out = await prepareImage(
        jpeg(4032, 3024),
        shrink: (b, w, h, [side = 2048]) async {
          asked = true;
          expect((w, h), (4032, 3024));
          return png(2048, 1536);
        },
      );
      expect(asked, isTrue);
      expect(out.kind, ImageKind.png);
    });

    test('a shrunk png still over the byte limit is retried on a smaller side', () async {
      final sides = <int>[];
      final out = await prepareImage(
        jpeg(4032, 3024),
        shrink: (b, w, h, [side = 2048]) async {
          sides.add(side);
          return side > 1536
              ? Uint8List(maxImageBytes + 10)
              : png(side, 1000);
        },
      );
      expect(sides, [2048, 1536]);
      expect(out.kind, ImageKind.png);
      await expectLater(
        prepareImage(
          jpeg(4032, 3024),
          shrink: (b, w, h, [side = 2048]) async => Uint8List(maxImageBytes + 10),
        ),
        throwsA(
          isA<PrepFailure>().having((e) => e.error, 'error', PrepError.tooLarge),
        ),
      );
    });

    test('unknown bytes and oversized results are refused', () async {
      await expectLater(
        prepareImage(Uint8List.fromList(List.filled(40, 9))),
        throwsA(
          isA<PrepFailure>().having((e) => e.error, 'error', PrepError.unsupported),
        ),
      );
      await expectLater(
        prepareImage(jpeg(100, 100, extra: maxImageBytes)),
        throwsA(
          isA<PrepFailure>().having((e) => e.error, 'error', PrepError.tooLarge),
        ),
      );
      await expectLater(
        prepareImage(png(5000, 5000), shrink: (b, w, h, [side = 2048]) async => throw StateError('x')),
        throwsA(
          isA<PrepFailure>().having((e) => e.error, 'error', PrepError.failed),
        ),
      );
    });

    testWidgets('the real shrinker brings a wide png to 2048 px', (tester) async {
      final out = await tester.runAsync(() async {
        final image = await createTestImage(width: 3000, height: 60);
        final data = await image.toByteData(format: ui.ImageByteFormat.png);
        final raw = data!.buffer.asUint8List();
        return prepareImage(raw);
      });
      expect(out!.kind, ImageKind.png);
      expect(imageSize(out.bytes)!.w, 2048);
    });
  });

  group('chunks and upload', () {
    test('chunks stay within 384 KB raw and 512 KB base64', () {
      final data = Uint8List(chunkBytes * 2 + 5);
      final parts = splitChunks(data);
      expect(parts.length, 3);
      expect(parts.map((p) => p.length), [chunkBytes, chunkBytes, 5]);
      expect(base64Encode(parts.first).length, lessThanOrEqualTo(512 * 1024));
      expect(splitChunks(Uint8List(0)), isEmpty);
      expect(splitChunks(Uint8List(chunkBytes)).length, 1);
    });

    test('putImage sends ordered chunks with one id and flags only the last', () async {
      final call = FakeCall();
      final data = Uint8List.fromList(List.generate(chunkBytes + 10, (i) => i % 251));
      final result = await putImage(call.call, 'D:\\proj', data, up: 'abc');
      expect(result.name, '20260101-000000.jpg');
      final puts = call.of('fs.put');
      expect(puts.map((p) => p['seq']), [0, 1]);
      expect(puts.map((p) => p['last']), [false, true]);
      expect(puts.every((p) => p['up'] == 'abc' && p['root'] == 'D:\\proj'), isTrue);
      final joined = [
        ...base64Decode(puts[0]['b64'] as String),
        ...base64Decode(puts[1]['b64'] as String),
      ];
      expect(joined, data);
    });

    test('two uploads get different ids', () async {
      final call = FakeCall();
      await Future.wait([
        putImage(call.call, 'r', jpeg(10, 10)),
        putImage(call.call, 'r', jpeg(10, 10)),
      ]);
      final ids = call.of('fs.put').map((p) => p['up']).toSet();
      expect(ids.length, 2);
      expect(ids.every((i) => RegExp(r'^[a-z0-9]+$').hasMatch(i as String)), isTrue);
    });

    test('a failure in the middle stops sending and maps the code', () async {
      final call = FakeCall()
        ..failOn = 1
        ..failAt = const RemoteCommandException('timeout');
      await expectLater(
        putImage(call.call, 'r', Uint8List(chunkBytes * 3)),
        throwsA(isA<PutFailure>().having((e) => e.error, 'error', PutError.offline)),
      );
      expect(call.of('fs.put').length, 2);
    });

    test('an old desktop (unknown op) is reported as denied', () async {
      final call = FakeCall()
        ..failOn = 0
        ..failAt = const RemoteCommandException('denied');
      await expectLater(
        putImage(call.call, 'r', jpeg(10, 10)),
        throwsA(isA<PutFailure>().having((e) => e.error, 'error', PutError.denied)),
      );
    });

    test('desktop error codes map to put errors', () async {
      for (final (code, want) in [
        ('order', PutError.order),
        ('expired', PutError.expired),
        ('size', PutError.tooLarge),
        ('image', PutError.image),
        ('busy', PutError.busy),
        ('weird', PutError.failed),
      ]) {
        final call = FakeCall()
          ..failOn = 0
          ..failAt = ViewerFailure(code);
        await expectLater(
          putImage(call.call, 'r', jpeg(10, 10)),
          throwsA(isA<PutFailure>().having((e) => e.error, code, want)),
        );
      }
    });

    test('empty and over 8 MB inputs never reach the desktop', () async {
      final call = FakeCall();
      await expectLater(putImage(call.call, 'r', Uint8List(0)), throwsA(isA<PutFailure>()));
      await expectLater(
        putImage(call.call, 'r', Uint8List(maxImageBytes + 1)),
        throwsA(isA<PutFailure>().having((e) => e.error, 'e', PutError.tooLarge)),
      );
      expect(call.calls, isEmpty);
    });
  });

  group('inbox ignore', () {
    test('detects an existing rule and appends otherwise', () {
      expect(ignoresInbox('build/\n.orrery/inbox/\n'), isTrue);
      expect(ignoresInbox('/.orrery\n'), isTrue);
      expect(ignoresInbox('build/\n'), isFalse);
      expect(withInboxIgnore(''), '.orrery/inbox/\n');
      expect(withInboxIgnore('a'), 'a\n.orrery/inbox/\n');
      expect(withInboxIgnore('a\r\n', eol: '\r\n'), 'a\r\n.orrery/inbox/\r\n');
      expect(joinRoot('D:\\proj\\', '.gitignore'), 'D:\\proj\\.gitignore');
      expect(joinRoot('/home/p', '.gitignore'), '/home/p/.gitignore');
    });

    test('a missing .gitignore is created with a single write', () async {
      final call = FakeCall()
        ..replies['fs.read'] = const ViewerReply(data: {'kind': 'missing'});
      call.replies['fs.write'] = const ViewerReply(data: {'ok': true});
      expect(await addInboxIgnore(call.call, 'D:\\proj'), IgnoreResult.added);
      final write = call.of('fs.write').single;
      expect(write['path'], 'D:\\proj\\.gitignore');
      expect(write['text'], '.orrery/inbox/\n');
      expect(write.containsKey('base'), isFalse);
    });

    test('an existing file keeps its text, eol and base mtime', () async {
      final call = FakeCall()
        ..replies['fs.read'] = const ViewerReply(
          data: {'kind': 'text', 'text': 'node_modules\r\n', 'eol': 'crlf', 'mtimeMs': 5},
        );
      call.replies['fs.write'] = const ViewerReply(pending: true);
      expect(await addInboxIgnore(call.call, '/p'), IgnoreResult.waiting);
      final write = call.of('fs.write').single;
      expect(write['text'], 'node_modules\r\n.orrery/inbox/\r\n');
      expect(write['base'], 5);
    });

    test('a file that already ignores the inbox is not written', () async {
      final call = FakeCall()
        ..replies['fs.read'] = const ViewerReply(
          data: {'kind': 'text', 'text': '.orrery/\n', 'mtimeMs': 1},
        );
      expect(await addInboxIgnore(call.call, '/p'), IgnoreResult.already);
      expect(call.of('fs.write'), isEmpty);
    });

    test('an unreadable .gitignore is never overwritten', () async {
      final call = FakeCall()
        ..replies['fs.read'] = const ViewerReply(data: {'kind': 'binary'});
      await expectLater(addInboxIgnore(call.call, '/p'), throwsA(isA<ViewerFailure>()));
      expect(call.of('fs.write'), isEmpty);
    });
  });
}
