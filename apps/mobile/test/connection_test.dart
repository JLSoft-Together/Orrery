import 'dart:async';

import 'package:clock/clock.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/discovery.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/protocol.dart';

import 'support/fakes.dart';

T settle<T>(FakeAsync async, Future<T> future) {
  T? value;
  Object? error;
  var done = false;
  future.then(
    (v) {
      value = v;
      done = true;
    },
    onError: (Object e) {
      error = e;
      done = true;
    },
  );
  async.flushMicrotasks();
  if (!done) throw StateError('future still pending');
  if (error != null) throw error!;
  return value as T;
}

class Rig {
  Rig(this.async, this.deps, this.net);

  final FakeAsync async;
  final ConnectionDeps deps;
  final FakeNet net;
  final List<FakeServer> servers = [];

  MachineConnection connection([String id = 'm1']) =>
      MachineConnection(machineId: id, deps: deps);

  void tick([Duration by = const Duration(milliseconds: 20)]) =>
      async.elapse(by);
}

Rig rig(
  FakeAsync async, {
  List<Machine>? machines,
  LanDiscovery discovery = const NoDiscovery(),
  NetworkWatcher network = const NoNetworkWatcher(),
  FakeServer Function()? serverFactory,
}) {
  final net = FakeNet()..now = () => clock.now();
  final deps = settle(
    async,
    testDeps(
      net: net,
      machines: machines ?? [testMachine()],
      discovery: discovery,
      network: network,
    ),
  );
  final result = Rig(async, deps, net);
  if (serverFactory != null) {
    FakeServer make(Uri _) {
      final server = serverFactory();
      result.servers.add(server);
      return server;
    }

    net.lan = make;
    net.relay = make;
  }
  return result;
}

FakeServer plain() => FakeServer();

void main() {
  test(
    'LAN first: reachable lastHost gives ready over LAN and sends hello',
    () {
      fakeAsync((async) {
        final r = rig(async, serverFactory: plain);
        final conn = r.connection();
        final seen = <String>[];
        conn.events.listen((e) => seen.add(e['t'] as String));
        unawaited(conn.connect());
        r.tick();
        expect(conn.state.status, LinkStatus.ready);
        expect(conn.state.route, LinkRoute.lan);
        final hello = r.servers.single.commands.first;
        expect(hello['t'], 'hello');
        expect(hello['proto'], 1);
        expect(hello['platform'], 'android');
        expect(seen, containsAll(['hello', 'snapshot']));
        expect(
          r.net.lanCalls.single.uri.toString(),
          'ws://10.0.0.5:4320/remote',
        );
        unawaited(conn.dispose());
      });
    },
  );

  test('an event that carries re is still an event, not a stray reply', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      final seen = <Map<String, dynamic>>[];
      conn.events.listen(seen.add);
      unawaited(conn.connect());
      r.tick();
      r.servers.single.pushJson({
        't': 'confirm.ask',
        'cid': 'cid1',
        're': 'c9',
        'title': 'Close?',
      });
      r.tick();
      expect(seen.where((e) => e['t'] == 'confirm.ask'), hasLength(1));
      unawaited(conn.dispose());
    });
  });

  test('LAN unreachable: relay is used after the 1.5 s LAN budget', () {
    fakeAsync((async) {
      final r = rig(async, discovery: FakeDiscovery(), serverFactory: plain);
      r.net.lan = (uri) => throw StateError('down');
      r.net.relay = (uri) {
        final server = FakeServer();
        r.servers.add(server);
        return server;
      };
      final started = clock.now();
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(milliseconds: 1400));
      expect(r.net.relayCalls, isEmpty);
      expect(conn.state.status, LinkStatus.discovering);
      r.tick(const Duration(milliseconds: 200));
      expect(r.net.relayCalls, hasLength(1));
      expect(
        r.net.relayCalls.single.at.difference(started),
        const Duration(milliseconds: 1500),
      );
      expect(
        r.net.relayCalls.single.uri.toString(),
        'wss://relay.example/p/m1?t=tok-m1',
      );
      expect(conn.state.status, LinkStatus.ready);
      expect(conn.state.route, LinkRoute.relay);
      unawaited(conn.dispose());
    });
  });

  test('mDNS endpoint is used when no LAN address is remembered', () {
    fakeAsync((async) {
      final r = rig(
        async,
        machines: [testMachine(lastHost: null, lastPort: null)],
        discovery: FakeDiscovery(
          endpoint: const LanEndpoint('192.168.1.9', 4333),
          delay: const Duration(milliseconds: 200),
        ),
        serverFactory: plain,
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(milliseconds: 300));
      expect(conn.state.route, LinkRoute.lan);
      expect(
        r.net.lanCalls.single.uri.toString(),
        'ws://192.168.1.9:4333/remote',
      );
      final saved = r.deps.machines.get('m1')!;
      expect(saved.lastHost, '192.168.1.9');
      expect(saved.lastPort, 4333);
      unawaited(conn.dispose());
    });
  });

  test('backoff doubles 1 2 4 8 16 then stays at 30 seconds', () {
    fakeAsync((async) {
      final r = rig(async, machines: [testMachine(lastHost: null)]);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(minutes: 5));
      final gaps = <int>[];
      final calls = r.net.relayCalls;
      for (var i = 1; i < calls.length; i++) {
        gaps.add(calls[i].at.difference(calls[i - 1].at).inSeconds);
      }
      expect(gaps.take(8).toList(), [1, 2, 4, 8, 16, 30, 30, 30]);
      expect(conn.state.status, LinkStatus.reconnecting);
      unawaited(conn.dispose());
    });
  });

  test('delayFor maps failures to the documented sequence', () {
    const config = ConnectionConfig();
    expect(
      [for (var i = 1; i <= 8; i++) config.delayFor(i).inSeconds],
      [1, 2, 4, 8, 16, 30, 30, 30],
    );
  });

  test('a successful connection resets the backoff', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      final readyAt = <DateTime>[];
      final droppedAt = <DateTime>[];
      conn.states.listen((s) {
        if (s.status == LinkStatus.ready) readyAt.add(clock.now());
        if (s.status == LinkStatus.reconnecting && s.retryIn == null) {
          droppedAt.add(clock.now());
        }
      });
      unawaited(conn.connect());
      r.tick();
      r.servers.last.serverClose(null);
      r.tick(const Duration(seconds: 2));
      r.servers.last.serverClose(null);
      r.tick(const Duration(seconds: 2));
      expect(readyAt, hasLength(3));
      expect(droppedAt, hasLength(2));
      expect(readyAt[1].difference(droppedAt[0]).inSeconds, 1);
      expect(readyAt[2].difference(droppedAt[1]).inSeconds, 1);
      unawaited(conn.dispose());
    });
  });

  test('close 4401 on LAN handshake marks the machine revoked and stops', () {
    fakeAsync((async) {
      final r = rig(
        async,
        serverFactory: () => FakeServer(closeOnHandshake: 4401),
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(minutes: 2));
      expect(conn.state.status, LinkStatus.revoked);
      expect(r.deps.machines.get('m1')!.revoked, isTrue);
      expect(r.net.calls, hasLength(1));
      unawaited(conn.dispose());
    });
  });

  test('close 4426 gives protoTooOld and no retry', () {
    fakeAsync((async) {
      final r = rig(
        async,
        serverFactory: () => FakeServer(closeOnHandshake: 4426),
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(minutes: 2));
      expect(conn.state.status, LinkStatus.protoTooOld);
      expect(r.net.calls, hasLength(1));
      unawaited(conn.dispose());
    });
  });

  test('relay close 4503 means desktop absent and keeps retrying', () {
    fakeAsync((async) {
      final r = rig(
        async,
        machines: [testMachine(lastHost: null)],
        serverFactory: () => FakeServer(closeOnHandshake: 4503),
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(milliseconds: 100));
      expect(conn.state.status, LinkStatus.reconnecting);
      expect(conn.state.reason, LinkReason.desktopAbsent);
      r.tick(const Duration(seconds: 4));
      expect(r.net.relayCalls.length, greaterThanOrEqualTo(3));
      unawaited(conn.dispose());
    });
  });

  test('relay close 4429 backs off at the maximum delay', () {
    fakeAsync((async) {
      final r = rig(
        async,
        machines: [testMachine(lastHost: null)],
        serverFactory: () => FakeServer(closeOnHandshake: 4429),
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(seconds: 29));
      expect(r.net.relayCalls, hasLength(1));
      expect(conn.state.reason, LinkReason.rateLimited);
      r.tick(const Duration(seconds: 2));
      expect(r.net.relayCalls, hasLength(2));
      unawaited(conn.dispose());
    });
  });

  test('expired relay token never touches the relay', () {
    fakeAsync((async) {
      final r = rig(
        async,
        machines: [
          testMachine(lastHost: null, token: const RelayToken('old', 1)),
        ],
        serverFactory: plain,
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(seconds: 20));
      expect(r.net.relayCalls, isEmpty);
      expect(conn.state.status, LinkStatus.offline);
      expect(conn.state.reason, LinkReason.tokenExpired);
      unawaited(conn.dispose());
    });
  });

  test('handshake that never answers times out after 5 s and retries', () {
    fakeAsync((async) {
      final r = rig(
        async,
        serverFactory: () => FakeServer(answerHandshake: false),
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(seconds: 4));
      expect(conn.state.status, LinkStatus.handshaking);
      r.tick(const Duration(seconds: 2));
      expect(r.servers.first.closedByClient, isTrue);
      expect(conn.state.status, isNot(LinkStatus.ready));
      expect(conn.state.reason, LinkReason.handshakeFailed);
      r.tick(const Duration(seconds: 3));
      expect(r.net.lanCalls.length, greaterThanOrEqualTo(2));
      unawaited(conn.dispose());
    });
  });

  test('connection dropping mid handshake reconnects without revoking', () {
    fakeAsync((async) {
      var first = true;
      final r = rig(
        async,
        serverFactory: () {
          final drop = first;
          first = false;
          return FakeServer(dropOnHandshake: drop);
        },
      );
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick(const Duration(seconds: 3));
      expect(conn.state.status, LinkStatus.ready);
      expect(r.deps.machines.get('m1')!.revoked, isFalse);
      unawaited(conn.dispose());
    });
  });

  test('device.revoked event stops the connection for good', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      r.servers.single.pushJson({'t': 'device.revoked'});
      r.tick(const Duration(minutes: 2));
      expect(conn.state.status, LinkStatus.revoked);
      expect(r.deps.machines.get('m1')!.revoked, isTrue);
      expect(r.net.calls, hasLength(1));
      unawaited(conn.dispose());
    });
  });

  test('network change reconnects at once, losing the network waits', () {
    fakeAsync((async) {
      final network = FakeNetwork();
      final r = rig(async, network: network, serverFactory: plain);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      network.controller.add('wifi');
      r.tick();
      expect(r.net.calls, hasLength(1));
      network.controller.add('mobile');
      r.tick(const Duration(milliseconds: 10));
      expect(r.net.calls, hasLength(2));
      expect(conn.state.status, LinkStatus.ready);
      network.controller.add('none');
      r.tick(const Duration(milliseconds: 10));
      expect(conn.state.status, LinkStatus.reconnecting);
      expect(conn.state.reason, LinkReason.networkLost);
      network.controller.add('wifi');
      r.tick(const Duration(milliseconds: 10));
      expect(conn.state.status, LinkStatus.ready);
      expect(r.net.calls.length, greaterThanOrEqualTo(3));
      unawaited(conn.dispose());
    });
  });

  test('background closes the socket after 30 s, foreground reconnects', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      conn.goBackground();
      r.tick(const Duration(seconds: 29));
      expect(conn.state.status, LinkStatus.ready);
      r.tick(const Duration(seconds: 2));
      expect(conn.state.status, LinkStatus.idle);
      expect(conn.state.reason, LinkReason.background);
      expect(r.servers.first.closedByClient, isTrue);
      unawaited(conn.goForeground());
      r.tick();
      expect(conn.state.status, LinkStatus.ready);
      expect(r.servers, hasLength(2));
      expect(r.servers.last.helloCount, 1);
      unawaited(conn.dispose());
    });
  });

  test('foreground within the grace period resyncs without reconnecting', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      conn.goBackground();
      r.tick(const Duration(seconds: 10));
      unawaited(conn.goForeground());
      r.tick();
      r.tick(const Duration(minutes: 1));
      expect(r.servers, hasLength(1));
      final types = r.servers.single.commands.map((c) => c['t']).toList();
      expect(types, ['hello', 'resync', 'ws.get']);
      expect(conn.state.status, LinkStatus.ready);
      unawaited(conn.dispose());
    });
  });

  test('requests time out after 10 s with code timeout', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      Object? error;
      conn.session!
          .request('silent')
          .then(
            (_) {},
            onError: (Object e) {
              error = e;
            },
          );
      r.tick(const Duration(seconds: 9));
      expect(error, isNull);
      r.tick(const Duration(seconds: 2));
      expect(error, isA<RemoteCommandException>());
      expect((error! as RemoteCommandException).code, 'timeout');
      unawaited(conn.dispose());
    });
  });

  test('error replies carry the contract error code', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      RemoteCommandException? error;
      conn.session!
          .request('fail')
          .then(
            (_) {},
            onError: (Object e) {
              error = e as RemoteCommandException;
            },
          );
      r.tick();
      expect(error?.code, 'denied');
      expect(error?.detail, 'no');
      unawaited(conn.dispose());
    });
  });

  test('keepalive: text ping on relay, Noise ping frame on LAN', () {
    fakeAsync((async) {
      final lan = rig(async, serverFactory: plain);
      final lanConn = lan.connection();
      unawaited(lanConn.connect());
      lan.tick();
      lan.tick(const Duration(seconds: 31));
      expect(lan.servers.single.pingFrames, 2);
      expect(lan.servers.single.texts, isEmpty);
      unawaited(lanConn.dispose());

      final relay = rig(
        async,
        machines: [testMachine(lastHost: null)],
        serverFactory: plain,
      );
      final relayConn = relay.connection();
      unawaited(relayConn.connect());
      relay.tick();
      relay.tick(const Duration(seconds: 31));
      expect(relay.servers.single.texts, ['ping']);
      expect(relay.servers.single.pingFrames, 0);
      unawaited(relayConn.dispose());
    });
  });

  test('a silent peer is declared dead after 75 s and reconnected', () {
    fakeAsync((async) {
      final r = rig(async, serverFactory: plain);
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      r.servers.single.silent = true;
      r.tick(const Duration(seconds: 100));
      expect(r.servers.length, greaterThanOrEqualTo(2));
      unawaited(conn.dispose());
    });
  });

  test('hello reply can refresh the stored relay token', () {
    fakeAsync((async) {
      final fresh = makeTokenText(4100000000);
      final r = rig(async, serverFactory: () => FakeServer(relayToken: fresh));
      final conn = r.connection();
      unawaited(conn.connect());
      r.tick();
      final saved = r.deps.machines.get('m1')!;
      expect(saved.relayToken!.exp, 4100000000);
      unawaited(conn.dispose());
    });
  });

  test('two machines connect independently at the same time', () {
    fakeAsync((async) {
      final r = rig(
        async,
        machines: [
          testMachine(id: 'm1', lastHost: '10.0.0.5'),
          testMachine(id: 'm2', lastHost: '10.0.0.6'),
        ],
        serverFactory: plain,
      );
      final a = r.connection('m1');
      final b = r.connection('m2');
      unawaited(a.connect());
      unawaited(b.connect());
      r.tick();
      expect(a.state.status, LinkStatus.ready);
      expect(b.state.status, LinkStatus.ready);
      expect(r.servers, hasLength(2));
      r.servers.first.serverClose(null);
      r.tick(const Duration(milliseconds: 10));
      expect(a.state.status, LinkStatus.reconnecting);
      expect(b.state.status, LinkStatus.ready);
      unawaited(a.dispose());
      unawaited(b.dispose());
    });
  });
}
