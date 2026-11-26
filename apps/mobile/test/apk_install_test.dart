import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:cryptography/dart.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/install/apk_flow.dart';
import 'package:orrery_remote/install/apk_io.dart';
import 'package:orrery_remote/install/apk_offer.dart';
import 'package:orrery_remote/push/local_notifier.dart';
import 'package:orrery_remote/push/push_apk.dart';
import 'package:orrery_remote/push/push_notice.dart';

import 'support/seal_maker.dart';

final List<int> bytes = List.generate(5000, (i) => i % 251);

String shaOf(List<int> data) => const DartSha256()
    .hashSync(data)
    .bytes
    .map((b) => b.toRadixString(16).padLeft(2, '0'))
    .join();

Map<String, Object?> offerJson({
  List<int>? data,
  int? exp,
  List<String>? urls,
  String? sha,
}) => {
  'url': 'http://192.168.1.5:4000/apk/tk',
  'urls':
      urls ?? ['http://192.168.1.5:4000/apk/tk', 'http://10.0.0.7:4000/apk/tk'],
  'size': (data ?? bytes).length,
  'sha256': sha ?? shaOf(data ?? bytes),
  'app': 'dev.test.app',
  'variant': 'debug',
  'exp': exp ?? DateTime.now().millisecondsSinceEpoch ~/ 1000 + 600,
};

class FakeSource implements ApkSource {
  FakeSource(this.handler);

  final Future<ApkResponse> Function(Uri url) handler;
  final List<Uri> opened = [];
  int closed = 0;

  @override
  Future<ApkResponse> open(Uri url) {
    opened.add(url);
    return handler(url);
  }

  @override
  void close() => closed++;
}

class FakeInstaller implements ApkInstaller {
  bool allowed = true;
  int? free;
  InstallStart result = InstallStart.started;
  final List<String> installed = [];
  int settings = 0;

  @override
  Future<bool> canInstall() async => allowed;

  @override
  Future<void> openSettings() async => settings++;

  @override
  Future<int?> freeBytes(String path) async => free;

  @override
  Future<InstallStart> install(String path) async {
    installed.add(path);
    return result;
  }
}

ApkResponse okBody(List<int> data, {int? length}) => ApkResponse(
  status: 200,
  length: length ?? data.length,
  body: Stream.fromIterable([
    for (var i = 0; i < data.length; i += 1000)
      data.sublist(i, (i + 1000).clamp(0, data.length)),
  ]),
);

void main() {
  late Directory dir;

  setUp(() async {
    dir = await Directory.systemTemp.createTemp('apk_test');
  });

  tearDown(() async {
    if (await dir.exists()) await dir.delete(recursive: true);
  });

  ApkOffer offerOf([Map<String, Object?>? json]) =>
      ApkOffer.parse(json ?? offerJson(), machine: 'M1')!;

  Future<List<File>> files() async =>
      dir.existsSync() ? dir.listSync().whereType<File>().toList() : <File>[];

  group('offer parse', () {
    test('accepts a LAN offer and keeps at most three private urls', () {
      final offer = ApkOffer.parse({
        ...offerJson(),
        'url': 'http://192.168.1.5:4000/a',
        'urls': [
          'http://192.168.1.5:4000/a',
          'http://8.8.8.8:4000/a',
          'https://192.168.1.6:4000/a',
          'http://10.0.0.7:4000/a',
          'http://172.16.0.2:4000/a',
          'http://172.31.0.2:4000/a',
        ],
      })!;
      expect(offer.urls.map((u) => u.host), [
        '192.168.1.5',
        '10.0.0.7',
        '172.16.0.2',
      ]);
      expect(offer.app, 'dev.test.app');
    });

    test('rejects bad size, sha, exp and offers without a LAN url', () {
      expect(ApkOffer.parse(offerJson(sha: 'XYZ')), isNull);
      expect(ApkOffer.parse({...offerJson(), 'size': 0}), isNull);
      expect(ApkOffer.parse({...offerJson(), 'size': apkMaxBytes + 1}), isNull);
      expect(ApkOffer.parse({...offerJson(), 'exp': 0}), isNull);
      expect(
        ApkOffer.parse({
          ...offerJson(urls: ['http://1.2.3.4:80/x']),
          'url': 'http://example.com/x',
        }),
        isNull,
      );
      expect(ApkOffer.parse('nope'), isNull);
      expect(ApkOffer.parse(null), isNull);
    });

    test('round trips through the notification payload string', () {
      final offer = offerOf();
      final again = ApkOffer.parse(offer.encode(), machine: 'M1')!;
      expect(again.sha256, offer.sha256);
      expect(again.urls, offer.urls);
      expect(again.exp, offer.exp);
    });
  });

  group('install flow', () {
    ApkInstallFlow flowOf(
      ApkOffer offer,
      FakeSource source,
      FakeInstaller installer, {
      DateTime Function()? now,
    }) => ApkInstallFlow(
      offer: offer,
      source: source,
      installer: installer,
      dir: dir,
      idle: const Duration(milliseconds: 300),
      now: now,
    );

    test('downloads, passes sha and hands the file to the installer', () async {
      final source = FakeSource((_) async => okBody(bytes));
      final installer = FakeInstaller();
      final flow = flowOf(offerOf(), source, installer);
      await flow.start();
      expect(flow.state.phase, ApkPhase.started);
      expect(installer.installed, hasLength(1));
      expect(await File(installer.installed.single).readAsBytes(), bytes);
      expect(source.opened.first.host, '192.168.1.5');
      expect(source.closed, 1);
    });

    test(
      'a wrong sha reports an error, deletes the file, never installs',
      () async {
        final offer = offerOf(offerJson(sha: shaOf([1, 2, 3])));
        final installer = FakeInstaller();
        final flow = flowOf(
          offer,
          FakeSource((_) async => okBody(bytes)),
          installer,
        );
        await flow.start();
        expect(flow.state.phase, ApkPhase.failed);
        expect(flow.state.error, ApkError.sha);
        expect(installer.installed, isEmpty);
        expect(await files(), isEmpty);
      },
    );

    test(
      'unknown sources not allowed asks for permission before any download',
      () async {
        final source = FakeSource((_) async => okBody(bytes));
        final installer = FakeInstaller()..allowed = false;
        final flow = flowOf(offerOf(), source, installer);
        await flow.start();
        expect(flow.state.phase, ApkPhase.needPermission);
        expect(source.opened, isEmpty);
        installer.allowed = true;
        await flow.start();
        expect(flow.state.phase, ApkPhase.started);
      },
    );

    test(
      'permission revoked at install time returns to needPermission',
      () async {
        final installer = FakeInstaller()..result = InstallStart.notAllowed;
        final flow = flowOf(
          offerOf(),
          FakeSource((_) async => okBody(bytes)),
          installer,
        );
        await flow.start();
        expect(flow.state.phase, ApkPhase.needPermission);
      },
    );

    test('an expired ticket fails without touching the network', () async {
      final past = DateTime.now().add(const Duration(hours: 1));
      final source = FakeSource((_) async => okBody(bytes));
      final flow = flowOf(offerOf(), source, FakeInstaller(), now: () => past);
      await flow.start();
      expect(flow.state.error, ApkError.expired);
      expect(source.opened, isEmpty);
    });

    test('404 from the desktop means the ticket expired', () async {
      final source = FakeSource(
        (_) async => const ApkResponse(status: 404, body: Stream.empty()),
      );
      final installer = FakeInstaller();
      final flow = flowOf(offerOf(), source, installer);
      await flow.start();
      expect(flow.state.error, ApkError.expired);
      expect(source.opened, hasLength(1));
      expect(installer.installed, isEmpty);
    });

    test(
      'off the home Wi-Fi every url fails and the message says so',
      () async {
        final source = FakeSource(
          (_) async => throw const SocketException('no route'),
        );
        final flow = flowOf(offerOf(), source, FakeInstaller());
        await flow.start();
        expect(flow.state.error, ApkError.lan);
        expect(source.opened, hasLength(2));
      },
    );

    test('falls back to the second url', () async {
      final source = FakeSource((url) async {
        if (url.host == '192.168.1.5') throw TimeoutException('slow');
        return okBody(bytes);
      });
      final flow = flowOf(offerOf(), source, FakeInstaller());
      await flow.start();
      expect(flow.state.phase, ApkPhase.started);
      expect(source.opened.map((u) => u.host), ['192.168.1.5', '10.0.0.7']);
    });

    test(
      'a connection cut mid download leaves no file and is retryable',
      () async {
        var first = true;
        final source = FakeSource((_) async {
          if (first) {
            first = false;
            return ApkResponse(
              status: 200,
              length: bytes.length,
              body: () async* {
                yield bytes.sublist(0, 2000);
                throw const HttpException(
                  'Connection closed while receiving data',
                );
              }(),
            );
          }
          return okBody(bytes);
        });
        final installer = FakeInstaller();
        final flow = flowOf(offerOf(), source, installer);
        await flow.start();
        expect(flow.state.error, ApkError.interrupted);
        expect(await files(), isEmpty);
        expect(installer.installed, isEmpty);
        await flow.start();
        expect(flow.state.phase, ApkPhase.started);
      },
    );

    test('a body that ends early counts as interrupted', () async {
      final flow = flowOf(
        offerOf(),
        FakeSource(
          (_) async => okBody(bytes.sublist(0, 3000), length: bytes.length),
        ),
        FakeInstaller(),
      );
      await flow.start();
      expect(flow.state.error, ApkError.interrupted);
      expect(await files(), isEmpty);
    });

    test('a stalled stream hits the idle deadline', () async {
      final stall = StreamController<List<int>>();
      addTearDown(stall.close);
      final flow = flowOf(
        offerOf(),
        FakeSource(
          (_) async => ApkResponse(
            status: 200,
            length: bytes.length,
            body: stall.stream,
          ),
        ),
        FakeInstaller(),
      );
      await flow.start();
      expect(flow.state.error, ApkError.interrupted);
    });

    test('too little free space stops before downloading', () async {
      final source = FakeSource((_) async => okBody(bytes));
      final installer = FakeInstaller()..free = 1000;
      final flow = flowOf(offerOf(), source, installer);
      await flow.start();
      expect(flow.state.error, ApkError.space);
      expect(source.opened, isEmpty);
    });

    test('a length that disagrees with the offer is rejected', () async {
      final flow = flowOf(
        offerOf(),
        FakeSource((_) async => okBody(bytes, length: bytes.length + 5)),
        FakeInstaller(),
      );
      await flow.start();
      expect(flow.state.error, ApkError.size);
    });

    test('more bytes than the offer stop the download', () async {
      final flow = flowOf(
        offerOf(),
        FakeSource((_) async => okBody([...bytes, 1, 2, 3], length: null)),
        FakeInstaller(),
      );
      await flow.start();
      expect(flow.state.error, ApkError.size);
      expect(await files(), isEmpty);
    });

    test('the installer failing reports installFailed', () async {
      final installer = FakeInstaller()..result = InstallStart.failed;
      final flow = flowOf(
        offerOf(),
        FakeSource((_) async => okBody(bytes)),
        installer,
      );
      await flow.start();
      expect(flow.state.error, ApkError.installFailed);
      expect(await files(), isEmpty);
    });

    test(
      'cancel mid download returns to idle and removes the partial file',
      () async {
        final stall = StreamController<List<int>>();
        addTearDown(stall.close);
        final flow = flowOf(
          offerOf(),
          FakeSource(
            (_) async => ApkResponse(
              status: 200,
              length: bytes.length,
              body: stall.stream,
            ),
          ),
          FakeInstaller(),
        );
        final run = flow.start();
        await Future<void>.delayed(const Duration(milliseconds: 50));
        stall.add(bytes.sublist(0, 1000));
        await Future<void>.delayed(const Duration(milliseconds: 20));
        expect(flow.state.phase, ApkPhase.downloading);
        await flow.cancel();
        await stall.close();
        await run;
        expect(flow.state.phase, ApkPhase.idle);
        expect(await files(), isEmpty);
      },
    );

    test('leftover files from a killed download are swept', () async {
      await dir.create(recursive: true);
      final stale = File('${dir.path}/offer-1.apk')..writeAsBytesSync([1]);
      final fresh = File('${dir.path}/offer-2.apk')..writeAsBytesSync([2]);
      stale.setLastModifiedSync(
        DateTime.now().subtract(const Duration(hours: 3)),
      );
      await cleanApkDir(dir, olderThan: const Duration(hours: 1));
      expect(stale.existsSync(), isFalse);
      expect(fresh.existsSync(), isTrue);
      await cleanApkDir(dir);
      expect(fresh.existsSync(), isFalse);
    });
  });

  group('push apk', () {
    test('a sealed apk push yields a notice with the install action and a payload that survives', () async {
      final rig = await SealRig.create();
      final data = await rig.push('apk', {
        't': 'APK ready',
        'b': 'dev.test.app debug',
        'apk': offerJson(),
      });
      final notice = await pushNotice(
        data,
        key: () async => rig.devicePrivate,
        machineKey: rig.machineKey,
        strings: pushStrings('en'),
      );
      expect(notice.kind, 'apk');
      expect(notice.apk, isNotNull);
      expect(notice.apk!.machine, rig.machine);
      final actions = noticeActions(notice, 'Open', 'Install');
      expect(actions.single.id, installActionId);
      expect(actions.single.title, 'Install');
      final payload = jsonDecode(notice.payload!) as Map<String, dynamic>;
      expect(payload['k'], 'apk');
      final tap = await apkTapOf(
        {...payload.cast<String, String>(), 'a': installActionId},
        key: () async => null,
        machineKey: (_) async => null,
        strings: pushStrings('en'),
      );
      expect(tap!.auto, isTrue);
      expect(tap.offer.sha256, notice.apk!.sha256);
    });

    test('a system shown apk push opens the seal on tap', () async {
      final rig = await SealRig.create();
      final data = await rig.push('apk', {
        't': 'APK ready',
        'b': 'x',
        'apk': offerJson(),
      });
      final tap = await apkTapOf(
        data,
        key: () async => rig.devicePrivate,
        machineKey: rig.machineKey,
        strings: pushStrings('en'),
      );
      expect(tap, isNotNull);
      expect(tap!.auto, isFalse);
    });

    test('an apk push without a usable offer yields no tap', () async {
      final tap = await apkTapOf(
        {'m': 'M1', 'k': 'apk'},
        key: () async => null,
        machineKey: (_) async => null,
        strings: pushStrings('en'),
      );
      expect(tap, isNull);
      final other = await apkTapOf(
        {'m': 'M1', 'k': 'done'},
        key: () async => null,
        machineKey: (_) async => null,
        strings: pushStrings('en'),
      );
      expect(other, isNull);
    });

    test('apk has fixed text in both languages', () {
      expect(
        fixedPushText(pushStrings('en'), 'apk').$1,
        'APK ready to install',
      );
      expect(fixedPushText(pushStrings('vi'), 'apk').$1, 'APK sẵn sàng để cài');
      expect(pushKind('apk'), 'apk');
    });
  });
}
