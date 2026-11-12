import 'dart:async';
import 'dart:convert';

import 'discovery.dart';
import 'wire.dart';

class QrRoutes {
  const QrRoutes({this.lan = const [], this.ticket});

  final List<LanEndpoint> lan;
  final String? ticket;
}

final RegExp _ticket = RegExp(r'^[A-Za-z0-9_-]{22}$');
final RegExp _port = RegExp(r'^[0-9]{1,5}$');

LanEndpoint? _endpoint(Object? item) {
  if (item is! String) return null;
  final at = item.lastIndexOf(':');
  if (at <= 0) return null;
  final host = item.substring(0, at);
  final portText = item.substring(at + 1);
  if (host.contains(':') || !_port.hasMatch(portText)) return null;
  final port = int.parse(portText);
  if (port < 1 || port > 65535) return null;
  return LanEndpoint(host, port);
}

QrRoutes readQrRoutes(String text) {
  try {
    final raw = utf8.decode(base64Url.decode(base64Url.normalize(text.trim())));
    final json = jsonDecode(raw);
    if (json is! Map || json['v'] != 2) return const QrRoutes();
    final list = json['lan'];
    final lan = list is List
        ? list.map(_endpoint).whereType<LanEndpoint>().toList()
        : const <LanEndpoint>[];
    final pt = json['pt'];
    return QrRoutes(
      lan: lan,
      ticket: pt is String && _ticket.hasMatch(pt) ? pt : null,
    );
  } catch (_) {
    return const QrRoutes();
  }
}

class OpenedLan {
  const OpenedLan(this.socket, this.endpoint);

  final WireSocket socket;
  final LanEndpoint endpoint;
}

Uri lanUri(LanEndpoint endpoint) => Uri(
  scheme: 'ws',
  host: endpoint.host,
  port: endpoint.port,
  path: '/remote',
);

Future<OpenedLan?> openPairLan({
  required WireConnector connector,
  required List<LanEndpoint> candidates,
  required LanDiscovery discovery,
  required String machineId,
  required Duration budget,
}) {
  final result = Completer<OpenedLan?>();
  final tried = <LanEndpoint>{};
  var outstanding = 1;

  void settle(OpenedLan? opened) {
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

  Future<OpenedLan?> attempt(LanEndpoint endpoint) async {
    try {
      return OpenedLan(await connector(lanUri(endpoint), budget), endpoint);
    } catch (_) {
      return null;
    }
  }

  for (final endpoint in candidates) {
    if (!tried.add(endpoint)) continue;
    outstanding += 1;
    unawaited(attempt(endpoint).then(settle));
  }
  unawaited(
    discovery
        .find(machineId, budget)
        .then((found) async {
          if (found == null || !tried.add(found)) return null;
          return attempt(found);
        })
        .catchError((Object _) => null)
        .then(settle),
  );
  final timer = Timer(budget, () {
    if (!result.isCompleted) result.complete(null);
  });
  return result.future.whenComplete(timer.cancel);
}
