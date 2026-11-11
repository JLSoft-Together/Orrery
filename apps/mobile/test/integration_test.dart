@Tags(['integration'])
library;

import 'dart:convert';
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
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
import 'package:orrery_remote/src/rust/frb_generated.dart';

String libraryPath() {
  final name = Platform.isWindows
      ? 'rust_lib_orrery_remote.dll'
      : Platform.isMacOS
      ? 'librust_lib_orrery_remote.dylib'
      : 'librust_lib_orrery_remote.so';
  return 'rust/target/debug/$name';
}

String gatewayPath() {
  final exe = Platform.isWindows ? '.exe' : '';
  return 'rust/target/debug/examples/fake_gateway$exe';
}

class Gateway {
  Gateway(this.process, this.qr, this.port, this.machine);

  final Process process;
  final String qr;
  final int port;
  final String machine;

  static Future<Gateway> start({int expIn = 300}) async {
    final process = await Process.start(File(gatewayPath()).absolute.path, [
      '0',
      '$expIn',
    ]);
    final line = await process.stdout
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .first
        .timeout(const Duration(seconds: 20));
    final info = jsonDecode(line) as Map<String, dynamic>;
    return Gateway(
      process,
      info['qr'] as String,
      info['port'] as int,
      info['machine'] as String,
    );
  }

  void stop() => process.kill();
}

ConnectionDeps realDeps(MachineRepository repo, KeyStore keys) =>
    ConnectionDeps(
      crypto: const RustRemoteCrypto(),
      connector: IoWireSocket.connect,
      keys: keys,
      machines: repo,
      config: const ConnectionConfig(
        handshakeTimeout: Duration(seconds: 5),
        backoff: [Duration(milliseconds: 100), Duration(milliseconds: 200)],
      ),
    );

Future<LinkState> waitFor(
  MachineConnection conn,
  bool Function(LinkState) test, {
  Duration timeout = const Duration(seconds: 10),
}) async {
  if (test(conn.state)) return conn.state;
  return conn.states.firstWhere(test).timeout(timeout);
}

void main() {
  late Gateway gateway;

  setUpAll(() async {
    final build = await Process.run('cargo', [
      'build',
      '-j',
      '3',
      '--examples',
    ], workingDirectory: 'rust');
    expect(build.exitCode, 0, reason: '${build.stderr}');
    await RustLib.init(externalLibrary: ExternalLibrary.open(libraryPath()));
  });

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
