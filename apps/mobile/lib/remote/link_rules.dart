import 'lan_dialer.dart';
import 'link_state.dart';
import 'machines.dart';
import 'protocol.dart';

LinkState classifyClose({
  required int? code,
  required bool viaRelay,
  required LinkReason hint,
  required int proto,
}) {
  final route = viaRelay ? LinkRoute.relay : LinkRoute.lan;
  switch (code) {
    case CloseCode.revoked:
      return LinkState(status: LinkStatus.revoked, route: route);
    case CloseCode.protoTooOld:
      return LinkState(
        status: LinkStatus.protoTooOld,
        route: route,
        minProto: proto,
      );
    case CloseCode.gatewayDown:
      return LinkState(
        status: LinkStatus.reconnecting,
        route: route,
        reason: hint == LinkReason.none ? LinkReason.desktopAbsent : hint,
      );
    case CloseCode.rate:
      return LinkState(
        status: LinkStatus.reconnecting,
        route: route,
        reason: LinkReason.rateLimited,
      );
    default:
      return LinkState(
        status: LinkStatus.reconnecting,
        route: route,
        reason: hint == LinkReason.none ? LinkReason.handshakeFailed : hint,
      );
  }
}

Future<void> rememberRoute(
  MachineRepository machines,
  Machine machine,
  Opened opened,
  JsonMap reply,
) async {
  var updated = machine;
  final endpoint = opened.endpoint;
  if (endpoint != null &&
      (machine.lastHost != endpoint.host ||
          machine.lastPort != endpoint.port)) {
    updated = updated.copyWith(
      lastHost: endpoint.host,
      lastPort: endpoint.port,
    );
  }
  final token = reply['relayToken'];
  if (token is String) {
    final parsed = RelayToken.parse(token);
    if (parsed != null) updated = updated.copyWith(relayToken: parsed);
  }
  final name = reply['name'];
  if (name is String && name.isNotEmpty && name != machine.name) {
    updated = updated.copyWith(name: name);
  }
  if (!identical(updated, machine)) await machines.save(updated);
}

LinkState helloFailure(String code, LinkRoute route, int proto) {
  if (code == 'proto') {
    return LinkState(
      status: LinkStatus.protoTooOld,
      route: route,
      minProto: proto,
    );
  }
  return LinkState(
    status: LinkStatus.reconnecting,
    route: route,
    reason: LinkReason.handshakeFailed,
  );
}

Future<void> storeRelayToken(
  MachineRepository machines,
  String machineId,
  String token,
) async {
  final parsed = RelayToken.parse(token);
  final machine = machines.get(machineId);
  if (parsed == null || machine == null) return;
  await machines.save(machine.copyWith(relayToken: parsed));
}
