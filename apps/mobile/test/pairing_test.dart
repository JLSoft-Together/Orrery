import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/crypto.dart';
import 'package:orrery_remote/remote/discovery.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/pairing.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/remote/wire.dart';

import 'support/fakes.dart';

class Setup {
  Setup(this.pairing, this.repo, this.kv, this.sockets);

  final Pairing pairing;
  final MachineRepository repo;
  final MemorySecureKv kv;
  final List<Uri> sockets;
}

Setup setup(
  WireSocket Function(Uri uri)? open, {
  LanDiscovery discovery = const NoDiscovery(),
}) {
  final kv = MemorySecureKv();
  final crypto = FakeCrypto();
  final repo = MachineRepository(secure: kv);
  final sockets = <Uri>[];
  final pairing = Pairing(
    crypto: crypto,
    connector: (uri, timeout) async {
      sockets.add(uri);
      if (open == null) throw StateError('refused');
      return open(uri);
    },
    keys: KeyStore(kv, crypto),
    machines: repo,
    deviceName: 'Test phone',
    discovery: discovery,
    config: const ConnectionConfig(lanBudget: Duration(milliseconds: 20)),
    confirmTimeout: const Duration(milliseconds: 300),
  );
  return Setup(pairing, repo, kv, sockets);
}

String qr({
  String? host = '192.168.1.20',
  int exp = 4000000000,
  List<int>? mkey,
  String? relay = 'https://relay.example',
}) => FakeCrypto.qrText(
  machine: 'machine-x',
  host: host,
  port: host == null ? null : 4320,
  relay: relay,
  exp: exp,
  mkey: mkey,
);

Future<PairFailure> failure(Future<Object?> run) async {
  try {
    await run;
  } on PairException catch (error) {
    return error.failure;
  }
  throw StateError('expected PairException');
}

void main() {
  test(
    'successful pairing stores the machine, route and relay token',
    () async {
      final token = makeTokenText(4200000000);
      final s = setup(
        (_) => FakePairServer(
          verdict: {
            't': 'pair.ok',
            'name': 'Studio PC',
            'relay': 'https://relay.example/',
            'relayToken': token,
          },
        ),
      );
      final machine = await s.pairing.pair(qr());
      expect(machine.id, 'machine-x');
      expect(machine.name, 'Studio PC');
      expect(machine.lastHost, '192.168.1.20');
      expect(machine.lastPort, 4320);
      expect(machine.relayUrl, 'https://relay.example/');
      expect(machine.relayToken!.exp, 4200000000);
      expect(s.sockets.single.toString(), 'ws://192.168.1.20:4320/remote');
      expect(s.repo.get('machine-x'), isNotNull);
      expect(await s.kv.read('remote.relay_token.machine-x'), token);
      expect(await s.kv.read(KeyStore.keyName), isNotNull);
    },
  );

  test('token in map form and missing token are both accepted', () async {
    final s = setup(
      (_) => FakePairServer(
        verdict: {
          't': 'pair.ok',
          'relayToken': {'token': makeTokenText(4300000000)},
        },
      ),
    );
    final machine = await s.pairing.pair(qr());
    expect(machine.name, 'Studio');
    expect(machine.relayToken!.exp, 4300000000);
    final bare = setup((_) => FakePairServer(verdict: {'t': 'pair.ok'}));
    final second = await bare.pairing.pair(qr());
    expect(second.relayToken, isNull);
    expect(second.relayUrl, 'https://relay.example');
  });

  test('desktop key different from the QR stops before msg3', () async {
    late FakePairServer server;
    final s = setup((_) => server = FakePairServer(msg2: 'pair2:mismatch'));
    expect(await failure(s.pairing.pair(qr())), PairFailure.keyMismatch);
    expect(server.received, ['pair1']);
    expect(server.closedByClient, isTrue);
    expect(s.repo.all, isEmpty);
  });

  test('expired or malformed QR never opens a socket', () async {
    final s = setup((_) => FakePairServer());
    expect(await failure(s.pairing.pair(qr(exp: 5))), PairFailure.expired);
    expect(await failure(s.pairing.pair('garbage')), PairFailure.badQr);
    expect(s.sockets, isEmpty);
  });

  test('desktop refusing gives denied and stores nothing', () async {
    final s = setup((_) => FakePairServer(verdict: {'t': 'pair.denied'}));
    expect(await failure(s.pairing.pair(qr())), PairFailure.denied);
    expect(s.repo.all, isEmpty);
  });

  test('connection lost during the handshake is a network failure', () async {
    final s = setup((_) => FakePairServer(dropAfterMsg3: true));
    expect(await failure(s.pairing.pair(qr())), PairFailure.network);
    final refused = setup(null);
    expect(await failure(refused.pairing.pair(qr())), PairFailure.network);
    expect(refused.repo.all, isEmpty);
  });

  test('no answer from the desktop confirm dialog times out', () async {
    final s = setup((_) => FakePairServer(verdict: null));
    expect(await failure(s.pairing.pair(qr())), PairFailure.timeout);
  });

  test(
    'QR without host falls back to mDNS, none found means noRoute',
    () async {
      final found = setup(
        (_) => FakePairServer(),
        discovery: FakeDiscovery(endpoint: const LanEndpoint('10.1.1.1', 4400)),
      );
      await found.pairing.pair(qr(host: null));
      expect(found.sockets.single.toString(), 'ws://10.1.1.1:4400/remote');
      final none = setup((_) => FakePairServer(), discovery: FakeDiscovery());
      expect(
        await failure(none.pairing.pair(qr(host: null))),
        PairFailure.noRoute,
      );
    },
  );

  test('device key and id are created once and reused', () async {
    final s = setup((_) => FakePairServer());
    await s.pairing.pair(qr());
    final first = await s.pairing.keys.load();
    final again = KeyStore(s.kv, FakeCrypto());
    final loaded = await again.load();
    expect(loaded.deviceId, first.deviceId);
    expect(loaded.privateKey, first.privateKey);
    expect(loaded.deviceId.length, greaterThan(10));
  });

  test('relay token parse reads the expiry from the token tail', () {
    final parsed = RelayToken.parse(makeTokenText(1234567890))!;
    expect(parsed.exp, 1234567890);
    expect(parsed.expired(1234567889), isFalse);
    expect(parsed.expired(1234567890), isTrue);
    expect(RelayToken.parse('!!!'), isNull);
    expect(RelayToken.parse('abcd'), isNull);
  });

  test('relayPhoneUrl converts https to wss and encodes the query', () {
    expect(
      relayPhoneUrl('https://r.dev/', 'a b', 'x+y/='),
      'wss://r.dev/p/a%20b?t=x%2By%2F%3D',
    );
  });

  test('CryptoException equality on failures is by failure kind', () {
    expect(const CryptoException(CryptoFailure.protoTooOld, 2).value, 2);
  });
}
