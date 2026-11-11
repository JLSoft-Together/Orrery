import 'dart:async';
import 'dart:typed_data';

import 'connection_config.dart';
import 'crypto.dart';
import 'keys.dart';
import 'lan_dialer.dart';
import 'link_state.dart';
import 'machines.dart';
import 'protocol.dart';
import 'wire.dart';

class HandshakeOutcome {
  const HandshakeOutcome.ok(this.link, this.inbox) : failure = null;

  const HandshakeOutcome.failed(this.failure) : link = null, inbox = null;

  final SecureLink? link;
  final Inbox? inbox;
  final LinkState? failure;
}

Future<HandshakeOutcome> runSessionHandshake({
  required ConnectionDeps deps,
  required Machine machine,
  required PhoneIdentity identity,
  required Opened opened,
  required LinkState Function(int? code, bool viaRelay) classify,
}) async {
  final socket = opened.socket;
  final inbox = Inbox(socket);
  LinkState reconnecting() => LinkState(
    status: LinkStatus.reconnecting,
    route: opened.route,
    reason: LinkReason.handshakeFailed,
  );
  try {
    final handshake = deps.crypto.sessionStart(
      privateKey: identity.privateKey,
      desktopKey: machine.mkey,
      deviceId: identity.deviceId,
    );
    socket.sendBinary(handshake.msg1);
    final Uint8List msg2 = await inbox.nextBinary(deps.config.handshakeTimeout);
    return HandshakeOutcome.ok(handshake.finish(msg2), inbox);
  } on SocketClosedException catch (error) {
    final code = error.code;
    final next = classify(code, opened.route == LinkRoute.relay);
    if (code == CloseCode.revoked && opened.route == LinkRoute.lan) {
      await deps.machines.save(machine.copyWith(revoked: true));
    }
    return HandshakeOutcome.failed(next);
  } on TimeoutException {
    await socket.close(CloseCode.handshake);
    return HandshakeOutcome.failed(reconnecting());
  } on CryptoException catch (error) {
    await socket.close(CloseCode.handshake);
    if (error.failure == CryptoFailure.protoTooOld) {
      return HandshakeOutcome.failed(
        LinkState(
          status: LinkStatus.protoTooOld,
          route: opened.route,
          minProto: error.value,
        ),
      );
    }
    return HandshakeOutcome.failed(reconnecting());
  }
}
