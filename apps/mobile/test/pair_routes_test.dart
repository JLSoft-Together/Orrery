import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/discovery.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/pair_routes.dart';
import 'package:orrery_remote/remote/pairing.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/remote/wire.dart';

import 'support/fakes.dart';

const ticket = 'ticket-0001-AAAAAAAAAA';
const v2Vector =
    'eyJ2IjoyLCJtYWNoaW5lIjoibWFjaGluZS0wMDAxIiwibmFtZSI6IlN0dWRpbyIsImxhbiI6WyIxOTIuMTY4LjEuMjA6NDMxOCIsIjEwLjAuMC43OjQzMjAiXSwicmVsYXkiOiJodHRwczovL29ycmVyeS1yZWxheS5leGFtcGxlLndvcmtlcnMuZGV2IiwicHQiOiJ0aWNrZXQtMDAwMS1BQUFBQUFBQUFBIiwibWtleSI6IkI2Tjh2QlFnazhpM1Zkd2JFT2hzdENZM1N0RnFxRlB0QzlfQXNyaHRISHciLCJjb2RlIjoid2NMRHhNWEd4OGpKeXN2TXpjN1AwQSIsImV4cCI6MTkwMDAwMDAwMH0';
const v1Vector =
    'eyJ2IjoxLCJtYWNoaW5lIjoibWFjaGluZS0wMDAxIiwibmFtZSI6IlN0dWRpbyIsImhvc3QiOiIxOTIuMTY4LjEuMjAiLCJwb3J0Ijo0MzE4LCJta2V5IjoiQjZOOHZCUWdrOGkzVmR3YkVPaHN0Q1kzU3RGcXFGUHRDOV9Bc3JodEhIdyIsImNvZGUiOiJ3Y0xEeE1YR3g4akp5c3ZNemM3UDBBIiwiZXhwIjoxOTAwMDAwMDAwfQ';

class Rig {
  Rig(this.pairing, this.repo, this.sockets, this.routes);

  final Pairing pairing;
  final MachineRepository repo;
  final List<String> sockets;
  final List<LinkRoute> routes;

  Future<Machine> pair(String text) => pairing.pair(text, onRoute: routes.add);
}

Rig rig(Future<WireSocket> Function(Uri uri) open) {
  final kv = MemorySecureKv();
  final crypto = FakeCrypto();
  final repo = MachineRepository(secure: kv);
  final sockets = <String>[];
  final pairing = Pairing(
    crypto: crypto,
    connector: (uri, timeout) {
      sockets.add(uri.toString());
      return open(uri);
    },
    keys: KeyStore(kv, crypto),
    machines: repo,
    deviceName: 'Test phone',
    config: const ConnectionConfig(pairLanBudget: Duration(milliseconds: 60)),
    confirmTimeout: const Duration(milliseconds: 300),
  );
  return Rig(pairing, repo, sockets, []);
}

String v2({List<String> lan = const ['192.168.1.20:4320'], String? pt}) =>
    FakeCrypto.qrText(
      machine: 'machine-x',
      relay: 'https://relay.example',
      lan: lan,
      pt: pt,
    );

Future<WireSocket> refuse(Uri _) => Future.error(StateError('refused'));

Future<PairFailure> failure(Future<Object?> run) async {
  try {
    await run;
  } on PairException catch (error) {
    return error.failure;
  }
  throw StateError('expected PairException');
}

const okVerdict = {'t': 'pair.ok', 'name': 'Studio PC'};

void main() {
  test('reads every LAN route and the ticket from a v2 QR, none from v1', () {
    final routes = readQrRoutes(v2Vector);
    expect(routes.lan, const [
      LanEndpoint('192.168.1.20', 4318),
      LanEndpoint('10.0.0.7', 4320),
    ]);
    expect(routes.ticket, ticket);
    final old = readQrRoutes(v1Vector);
    expect(old.lan, isEmpty);
    expect(old.ticket, isNull);
    expect(readQrRoutes('garbage!').lan, isEmpty);
  });

  test('the first LAN route that answers wins and is remembered', () async {
    final r = rig(
      (uri) => uri.host == '10.0.0.9'
          ? Future.value(FakePairServer(verdict: okVerdict))
          : refuse(uri),
    );
    final machine = await r.pair(
      v2(lan: ['192.168.1.20:4320', '10.0.0.9:4400'], pt: ticket),
    );
    expect(r.sockets, [
      'ws://192.168.1.20:4320/remote',
      'ws://10.0.0.9:4400/remote',
    ]);
    expect((machine.lastHost, machine.lastPort), ('10.0.0.9', 4400));
    expect(r.routes, [LinkRoute.lan]);
  });

  test('no LAN answer: pairs through the relay with the ticket', () async {
    final r = rig(
      (uri) => uri.scheme == 'wss'
          ? Future.value(FakePairServer(verdict: okVerdict))
          : refuse(uri),
    );
    final machine = await r.pair(v2(pt: ticket));
    expect(r.sockets.last, 'wss://relay.example/p/machine-x?pt=$ticket');
    expect(r.routes, [LinkRoute.lan, LinkRoute.relay]);
    expect((machine.lastHost, machine.lastPort), ('192.168.1.20', 4320));
    expect(machine.relayUrl, 'https://relay.example');
    expect(r.repo.get('machine-x'), isNotNull);
  });

  test(
    'a LAN socket that never opens in time falls back to the relay',
    () async {
      final hang = Completer<WireSocket>();
      final r = rig(
        (uri) => uri.scheme == 'wss'
            ? Future.value(FakePairServer(verdict: okVerdict))
            : hang.future,
      );
      final watch = Stopwatch()..start();
      await r.pair(v2(pt: ticket));
      expect(watch.elapsedMilliseconds, lessThan(2000));
      expect(r.routes, [LinkRoute.lan, LinkRoute.relay]);
    },
  );

  test('LAN failure before msg3 falls back, after msg3 it does not', () async {
    final mismatch = rig(
      (uri) => Future.value(
        uri.scheme == 'wss'
            ? FakePairServer(verdict: okVerdict)
            : FakePairServer(msg2: 'pair2:mismatch'),
      ),
    );
    await mismatch.pair(v2(pt: ticket));
    expect(mismatch.routes, [LinkRoute.lan, LinkRoute.relay]);
    final late = rig(
      (uri) => Future.value(
        uri.scheme == 'wss'
            ? FakePairServer(verdict: okVerdict)
            : FakePairServer(dropAfterMsg3: true),
      ),
    );
    expect(await failure(late.pair(v2(pt: ticket))), PairFailure.network);
    expect(late.routes, [LinkRoute.lan]);
    expect(late.sockets.where((s) => s.startsWith('wss')), isEmpty);
  });

  test(
    'without a ticket there is no relay try; a refused relay is network',
    () async {
      final noTicket = rig(refuse);
      expect(await failure(noTicket.pair(v2())), PairFailure.network);
      expect(noTicket.sockets, ['ws://192.168.1.20:4320/remote']);
      final noLan = rig(refuse);
      expect(await failure(noLan.pair(v2(lan: []))), PairFailure.noRoute);
      final relayDown = rig(refuse);
      expect(
        await failure(relayDown.pair(v2(pt: ticket))),
        PairFailure.network,
      );
      expect(relayDown.routes, [LinkRoute.lan, LinkRoute.relay]);
    },
  );

  test('a denied relay pairing stores nothing', () async {
    final r = rig(
      (uri) => uri.scheme == 'wss'
          ? Future.value(FakePairServer(verdict: {'t': 'pair.denied'}))
          : refuse(uri),
    );
    expect(await failure(r.pair(v2(pt: ticket))), PairFailure.denied);
    expect(r.repo.all, isEmpty);
  });

  test('relayPairUrl puts the ticket in pt, never in t', () {
    expect(
      relayPairUrl('https://r.dev/', 'm 1', 'a-b_c'),
      'wss://r.dev/p/m%201?pt=a-b_c',
    );
  });
}
