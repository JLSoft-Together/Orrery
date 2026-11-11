import 'dart:async';

import 'connection_config.dart';
import 'discovery.dart';
import 'link_state.dart';
import 'machines.dart';
import 'wire.dart';

class Opened {
  const Opened(this.socket, this.route, this.endpoint);

  final WireSocket socket;
  final LinkRoute route;
  final LanEndpoint? endpoint;
}

Future<Opened?> openLan(ConnectionDeps deps, Machine machine) {
  final budget = deps.config.lanBudget;
  final stopwatch = Stopwatch()..start();
  final result = Completer<Opened?>();
  var outstanding = 0;

  void settle(Opened? opened) {
    outstanding -= 1;
    if (opened != null) {
      if (result.isCompleted) {
        unawaited(opened.socket.close());
      } else {
        result.complete(opened);
      }
    } else if (outstanding == 0 && !result.isCompleted) {
      result.complete(null);
    }
  }

  Future<Opened?> tryEndpoint(LanEndpoint endpoint, Duration timeout) async {
    try {
      final uri = Uri(
        scheme: 'ws',
        host: endpoint.host,
        port: endpoint.port,
        path: '/remote',
      );
      final socket = await deps.connector(uri, timeout);
      return Opened(socket, LinkRoute.lan, endpoint);
    } catch (_) {
      return null;
    }
  }

  LanEndpoint? known;
  final host = machine.lastHost;
  if (host != null) {
    known = LanEndpoint(host, machine.lastPort ?? deps.config.lanPort);
    outstanding += 1;
    unawaited(tryEndpoint(known, budget).then(settle));
  }
  outstanding += 1;
  unawaited(
    deps.discovery
        .find(machine.id, budget)
        .then((found) async {
          if (found == null || found == known) return null;
          final left = budget - stopwatch.elapsed;
          const floor = Duration(milliseconds: 500);
          return tryEndpoint(found, left < floor ? floor : left);
        })
        .catchError((Object _) => null)
        .then(settle),
  );
  return result.future;
}
