import 'dart:async';

import 'package:clock/clock.dart';

import 'connection_config.dart';
import 'lan_dialer.dart';
import 'link_state.dart';
import 'machines.dart';
import 'protocol.dart';

Future<RelayOpen> openRoute(
  ConnectionDeps deps,
  Machine machine,
  bool Function() isCurrent,
) async {
  final lan = await openLan(deps, machine);
  if (!isCurrent()) {
    await lan?.socket.close();
    return const RelayOpen.failed(LinkState());
  }
  if (lan != null) return RelayOpen.opened(lan);
  final relay = await openRelay(
    deps,
    machine,
    clock.now().millisecondsSinceEpoch ~/ 1000,
  );
  if (relay.failure != null) return relay;
  if (!isCurrent()) {
    await relay.opened!.socket.close();
    return const RelayOpen.failed(LinkState());
  }
  return relay;
}

Future<JsonMap> sendHello(RemoteSession session, ConnectionConfig config) {
  return session.request('hello', {
    'proto': config.proto,
    'app': config.appName,
    'platform': config.platform,
    'caps': <String>[],
  });
}

Timer startKeepalive(
  RemoteSession session,
  ConnectionConfig config, {
  required void Function(int millis) onRtt,
  required void Function() onDead,
}) {
  if (!session.viaRelay) {
    session.onRtt = (rtt) {
      if (!session.isClosed) onRtt(rtt.inMilliseconds);
    };
    session.sendKeepalive();
  }
  return Timer.periodic(config.keepalive, (_) {
    if (session.isClosed) return;
    final idle = clock.now().difference(session.lastActivity);
    if (idle > config.deadAfter) {
      onDead();
      unawaited(session.close());
      return;
    }
    session.sendKeepalive();
  });
}

Future<int?> closeCodeAfterOffline(RemoteSession session) {
  return session.closed.timeout(
    const Duration(milliseconds: 500),
    onTimeout: () => null,
  );
}
