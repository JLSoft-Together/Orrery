import 'dart:convert';
import 'dart:typed_data';

import 'package:cryptography/dart.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/push/push_notice.dart';
import 'package:orrery_remote/push/push_seal.dart';
import 'package:orrery_remote/push/push_service.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/secure_kv.dart';

import 'support/push_vector.dart';

void main() {
  final vi = pushStrings('vi');

  test('the Rust vector opens byte for byte in Dart', () async {
    final pair = await const DartX25519().newKeyPairFromSeed(devicePrivate);
    expect(
      toHex((await pair.extractPublicKey()).bytes),
      vector['devicePublic'],
    );
    final okm = await pushSealOkm(
      privateKey: devicePrivate,
      ephemeral: hex(vector['ephemeralPublic'] as String),
      desktop: desktopPublic,
      machine: machine,
    );
    expect(toHex(okm!), vector['okm']);
    final open = await openSeal(
      privateKey: devicePrivate,
      desktop: desktopPublic,
      machine: machine,
      kind: kind,
      sealed: sealed,
    );
    final plain = jsonDecode(vector['plain'] as String) as Map;
    expect(open!.title, plain['t']);
    expect(open.body, plain['b']);
    expect(open.pane, plain['p']);
  });

  test(
    'a changed byte, machine, kind, phone or desktop key does not open',
    () async {
      Future<Object?> tryOpen({
        List<int>? key,
        List<int>? desktop,
        String? m,
        String? k,
        String? e,
      }) => openSeal(
        privateKey: key ?? devicePrivate,
        desktop: desktop ?? desktopPublic,
        machine: m ?? machine,
        kind: k ?? kind,
        sealed: e ?? sealed,
      );
      final length = base64Url.decode(base64Url.normalize(sealed)).length;
      for (final index in [0, 31, 32, length - 17, length - 1]) {
        expect(await tryOpen(e: flip(sealed, index)), isNull, reason: '$index');
      }
      expect(await tryOpen(m: '${machine}x'), isNull);
      expect(await tryOpen(k: 'done'), isNull);
      expect(await tryOpen(key: Uint8List(32)..[0] = 9), isNull);
      expect(await tryOpen(key: Uint8List(31)), isNull);
      expect(
        await tryOpen(desktop: hex(vector['ephemeralPublic'] as String)),
        isNull,
      );
      expect(await tryOpen(desktop: Uint8List(32)), isNull);
      expect(await tryOpen(desktop: Uint8List(31)), isNull);
      expect(await tryOpen(e: sealed.substring(0, 40)), isNull);
      expect(await tryOpen(e: 'not base64 !!'), isNull);
      expect(await tryOpen(e: 'A' * 5000), isNull);
      expect(await tryOpen(e: base64Url.encode(Uint8List(64))), isNull);
    },
  );

  test(
    'a sealed push becomes the full notice with a pane tag and link',
    () async {
      final notice = await pushNotice(
        sealedData(),
        key: () async => devicePrivate,
        machineKey: pairedVector,
        strings: vi,
      );
      expect(notice.sealed, isTrue);
      expect(notice.title, 'Claude · Sửa lỗi · orrery');
      expect(notice.body, 'Có chạy migration không?');
      expect(notice.tag, '$machine:p1');
      expect(
        pushLink(pushTapData(notice.payload)!).toString(),
        'orrery-remote://m/$machine/p/p1',
      );
    },
  );

  test(
    'anything that does not open falls back to the fixed text of its kind',
    () async {
      final cases = <(Map<String, String>, Future<Uint8List?> Function())>[
        (sealedData(), () async => null),
        (sealedData(), () async => throw const FormatException('kv')),
        (sealedData(), () async => Uint8List(32)..[0] = 7),
        (sealedData(v: '1'), () async => devicePrivate),
        ({...sealedData(), 'm': 'unpairedMachine'}, () async => devicePrivate),
        (sealedData(k: 'permission'), () async => devicePrivate),
        (sealedData(e: flip(sealed, 40)), () async => devicePrivate),
        ({'m': machine, 'p': 'p9', 'k': 'question'}, () async => devicePrivate),
      ];
      for (final (data, key) in cases) {
        final notice = await pushNotice(
          data,
          key: key,
          machineKey: pairedVector,
          strings: vi,
        );
        final k = pushKind(data['k']);
        expect(notice.sealed, isFalse);
        expect((notice.title, notice.body), fixedPushText(vi, k));
        expect(notice.kind, k);
      }
      final probe = await pushNotice(
        {'m': machine, 'p': 'p9', 'k': 'done'},
        key: () async => null,
        machineKey: pairedVector,
        strings: pushStrings('en'),
      );
      expect(probe.title, 'Agent finished');
      expect(probe.tag, '$machine:p9');
      final odd = fixedPushNotice({
        'm': 'bad id!',
        'k': 'weird',
        'p': '../x',
      }, vi);
      expect((odd.machine, odd.pane, odd.kind), (null, null, 'attention'));
      expect(odd.payload, isNull);
      expect(odd.tag, 'orrery:attention');
    },
  );

  test('every kind has fixed text in both languages', () {
    for (final lang in ['vi', 'en']) {
      final l = pushStrings(lang);
      final seen = <String>{};
      for (final k in pushKinds) {
        final (title, body) = fixedPushText(l, k);
        expect(title, isNotEmpty);
        expect(body, isNotEmpty);
        seen.add(body);
      }
      expect(seen.length, pushKinds.length);
    }
    expect(pushStrings('fr').pushFixedDoneTitle, 'Agent finished');
  });

  test(
    'the static key is read from secure storage like KeyStore writes it',
    () async {
      final kv = MemorySecureKv();
      expect(await readStaticKey(kv), isNull);
      await kv.write(KeyStore.keyName, base64Url.encode(devicePrivate));
      expect(await readStaticKey(kv), devicePrivate);
      await kv.write(KeyStore.keyName, 'broken!');
      expect(await readStaticKey(kv), isNull);
      await kv.write(KeyStore.keyName, base64Url.encode(Uint8List(16)));
      expect(await readStaticKey(kv), isNull);
    },
  );

  test('show or hide: app open with that session open hides it', () {
    for (final (background, shown, resumed, open, want) in [
      (true, false, false, false, true),
      (true, true, false, false, false),
      (true, false, true, true, true),
      (false, true, true, true, false),
      (false, false, true, true, false),
      (false, false, false, true, true),
      (false, false, true, false, true),
      (false, true, true, false, true),
    ]) {
      expect(
        shouldShowPush(
          background: background,
          shown: shown,
          resumed: resumed,
          sessionOpen: open,
        ),
        want,
        reason: '$background $shown $resumed $open',
      );
    }
  });

  test('only a paired, not revoked machine gives a desktop key', () async {
    final repo = MachineRepository(secure: MemorySecureKv());
    expect(await pairedMachineKey(repo, machine), isNull);
    final m = Machine(
      id: machine,
      name: 'PC',
      mkey: desktopPublic,
      pairedAt: 1,
    );
    await repo.save(m);
    expect(await pairedMachineKey(repo, machine), desktopPublic);
    expect(await pairedMachineKey(repo, 'other'), isNull);
    await repo.save(m.copyWith(revoked: true));
    expect(await pairedMachineKey(repo, machine), isNull);
  });

  test('a push without a pane links to its machine', () async {
    final notice = await pushNotice(
      {...sealedData(), 'e': flip(sealed, 40)},
      key: () async => devicePrivate,
      machineKey: pairedVector,
      strings: vi,
    );
    expect(notice.pane, isNull);
    expect(
      pushLink(pushTapData(notice.payload)!).toString(),
      'orrery-remote://m/$machine',
    );
    expect(pushLink({'m': machine, 'p': '../x'}), isNull);
  });
}
