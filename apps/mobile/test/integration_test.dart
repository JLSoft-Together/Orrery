@Tags(['integration'])
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/crypto_rust.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/pairing.dart';
import 'package:orrery_remote/remote/pty_stream.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/remote/wire.dart';

import 'support/gateway.dart';

void main() {
  late Gateway gateway;

  setUpAll(buildGateway);

  setUp(() async {
    gateway = await Gateway.start();
  });

  tearDown(() => gateway.stop());

  test(
    'real pairing, IK session, snapshot, PTY echo and reconnect over WS',
    () async {
      final kv = MemorySecureKv();
      const crypto = RustRemoteCrypto();
      final keys = KeyStore(kv, crypto);
      final repo = MachineRepository(secure: kv);
      final deps = realDeps(repo, keys);

      final pairing = Pairing(
        crypto: crypto,
        connector: IoWireSocket.connect,
        keys: keys,
        machines: repo,
        deviceName: 'Integration phone',
      );
      final machine = await pairing.pair(gateway.qr);
      expect(machine.id, gateway.machine);
      expect(machine.name, 'Fake Desktop');
      expect(machine.lastPort, gateway.port);
      expect(machine.relayUrl, 'https://relay.example.dev');

      final conn = MachineConnection(machineId: machine.id, deps: deps);
      final store = MachineStore(
        machineId: machine.id,
        commander: (t, [f]) => conn.session!.request(t, f),
      );
      conn.events.listen(store.handleEvent);
      final pty = PtyStreams(conn);
      await conn.connect();
      final ready = await waitFor(conn, (s) => s.status == LinkStatus.ready);
      expect(ready.route, LinkRoute.lan);
      await Future<void>.delayed(const Duration(milliseconds: 300));
      expect(store.view.hello!['name'], 'Fake Desktop');
      expect((store.view.snapshot!['terminals'] as List).single['id'], 't1');
      expect(store.view.tabs, hasLength(1));

      final attachment = await pty.attach('s1', 80, 24, paneKey: 'pane1');
      final chunks = <String>[];
      attachment.output.listen((b) => chunks.add(utf8.decode(b)));
      await Future<void>.delayed(const Duration(milliseconds: 200));
      expect(chunks, contains('replay\r\n'));
      pty.input(attachment, Uint8List.fromList(utf8.encode('ls')));
      await Future<void>.delayed(const Duration(milliseconds: 200));
      expect(chunks, contains('echo:ls'));

      final resync = await conn.session!.request('resync');
      expect(resync['ok'], isTrue);
      await expectLater(
        () => conn.session!.request('does.not.exist'),
        throwsA(
          isA<Object>().having(
            (e) => e.toString(),
            'error',
            contains('notFound'),
          ),
        ),
      );

      await conn.session!.close();
      await waitFor(conn, (s) => s.status == LinkStatus.reconnecting);
      await waitFor(conn, (s) => s.status == LinkStatus.ready);
      await Future<void>.delayed(const Duration(milliseconds: 300));
      expect(
        chunks.where((c) => c == 'replay\r\n').length,
        greaterThanOrEqualTo(2),
      );
      await pty.dispose();
      await conn.dispose();
    },
  );

  test(
    'a phone the gateway never paired is closed with 4401 and ends revoked',
    () async {
      final kv = MemorySecureKv();
      const crypto = RustRemoteCrypto();
      final keys = KeyStore(kv, crypto);
      final repo = MachineRepository(secure: kv);
      final info = const RustRemoteCrypto().decodeQr(gateway.qr);
      await repo.save(
        Machine(
          id: info.machine,
          name: info.name,
          mkey: info.mkey,
          pairedAt: 1,
          lastHost: '127.0.0.1',
          lastPort: gateway.port,
        ),
      );
      final conn = MachineConnection(
        machineId: info.machine,
        deps: realDeps(repo, keys),
      );
      await conn.connect();
      final state = await waitFor(conn, (s) => s.status == LinkStatus.revoked);
      expect(state.route, LinkRoute.lan);
      expect(repo.get(info.machine)!.revoked, isTrue);
      await conn.dispose();
    },
  );

  test('the gateway QR is v2 and an old v1 QR of the same code still pairs over LAN', () async {
    final wire = jsonDecode(
      utf8.decode(base64Url.decode(base64Url.normalize(gateway.qr))),
    ) as Map<String, dynamic>;
    expect(wire['v'], 2);
    expect(wire['lan'], ['127.0.0.1:${gateway.port}']);
    const crypto = RustRemoteCrypto();
    final info = crypto.decodeQr(gateway.qr);
    expect(info.lan.single.port, gateway.port);
    expect((info.host, info.port), ('127.0.0.1', gateway.port));
    final lan = (wire.remove('lan') as List).single as String;
    wire['v'] = 1;
    wire['host'] = lan.split(':').first;
    wire['port'] = int.parse(lan.split(':').last);
    final v1 = base64Url
        .encode(utf8.encode(jsonEncode(wire)))
        .replaceAll('=', '');
    final old = crypto.decodeQr(v1);
    expect(old.lan.single.port, gateway.port);
    expect(old.ticket, isNull);
    final kv = MemorySecureKv();
    final repo = MachineRepository(secure: kv);
    final machine = await Pairing(
      crypto: crypto,
      connector: IoWireSocket.connect,
      keys: KeyStore(kv, crypto),
      machines: repo,
      deviceName: 'Old QR phone',
    ).pair(v1);
    expect(machine.id, gateway.machine);
    expect(machine.lastPort, gateway.port);
  });

  test('QR hết hạn bị từ chối, mã QR dùng lần 2 bị đóng 4400', () async {
    final kv = MemorySecureKv();
    const crypto = RustRemoteCrypto();
    final repo = MachineRepository(secure: kv);
    final pairing = Pairing(
      crypto: crypto,
      connector: IoWireSocket.connect,
      keys: KeyStore(kv, crypto),
      machines: repo,
      deviceName: 'Phone',
    );
    await pairing.pair(gateway.qr);
    final second = Pairing(
      crypto: crypto,
      connector: IoWireSocket.connect,
      keys: KeyStore(MemorySecureKv(), crypto),
      machines: MachineRepository(secure: MemorySecureKv()),
      deviceName: 'Phone 2',
    );
    try {
      await second.pair(gateway.qr);
      fail('second use of the pairing code must fail');
    } on PairException catch (error) {
      expect(error.failure, PairFailure.network);
    }
    gateway.stop();
    final expired = await Gateway.start(expIn: -5);
    addTearDown(expired.stop);
    try {
      await pairing.pair(expired.qr);
      fail('expired QR must fail');
    } on PairException catch (error) {
      expect(error.failure, PairFailure.expired);
    }
  });
}
