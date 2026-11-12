import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:clock/clock.dart';

import 'connection.dart';
import 'crypto.dart';
import 'discovery.dart';
import 'keys.dart';
import 'link_state.dart';
import 'machines.dart';
import 'pair_routes.dart';
import 'protocol.dart';
import 'wire.dart';

enum PairFailure {
  badQr,
  expired,
  noRoute,
  network,
  keyMismatch,
  timeout,
  denied,
  protocol,
}

class PairException implements Exception {
  const PairException(this.failure);

  final PairFailure failure;

  @override
  String toString() => 'PairException($failure)';
}

class _Early implements Exception {
  const _Early(this.failure);

  final PairFailure failure;
}

typedef PairRouteListener = void Function(LinkRoute route);

class Pairing {
  Pairing({
    required this.crypto,
    required this.connector,
    required this.keys,
    required this.machines,
    required this.deviceName,
    this.discovery = const NoDiscovery(),
    this.config = const ConnectionConfig(),
    this.confirmTimeout = const Duration(seconds: 90),
  });

  final RemoteCrypto crypto;
  final WireConnector connector;
  final KeyStore keys;
  final MachineRepository machines;
  final String deviceName;
  final LanDiscovery discovery;
  final ConnectionConfig config;
  final Duration confirmTimeout;

  Future<Machine> pair(String qrText, {PairRouteListener? onRoute}) async {
    final text = qrText.trim();
    final QrData qr;
    try {
      qr = crypto.decodeQr(text);
    } on CryptoException {
      throw const PairException(PairFailure.badQr);
    }
    final nowSeconds = clock.now().millisecondsSinceEpoch ~/ 1000;
    if (nowSeconds >= qr.exp) throw const PairException(PairFailure.expired);
    final identity = await keys.load();
    final first = _start(text, identity);
    final candidates = _candidates(qr);
    final relay = qr.relay;
    final ticket = qr.ticket;
    final canRelay = relay != null && relay.isNotEmpty && ticket != null;
    onRoute?.call(LinkRoute.lan);
    final opened = await openPairLan(
      connector: connector,
      candidates: candidates,
      discovery: discovery,
      machineId: qr.machine,
      budget: config.pairLanBudget,
    );
    if (opened != null) {
      try {
        return await _attempt(opened.socket, first, qr, opened.endpoint);
      } on _Early catch (early) {
        if (!canRelay) throw PairException(early.failure);
      }
    } else if (!canRelay) {
      throw PairException(
        candidates.isEmpty ? PairFailure.noRoute : PairFailure.network,
      );
    }
    onRoute?.call(LinkRoute.relay);
    final WireSocket socket;
    try {
      socket = await connector(
        Uri.parse(relayPairUrl(relay, qr.machine, ticket)),
        config.relayConnectTimeout,
      );
    } catch (_) {
      throw const PairException(PairFailure.network);
    }
    final again = opened == null ? first : _start(text, identity);
    try {
      return await _attempt(socket, again, qr, null);
    } on _Early catch (early) {
      throw PairException(early.failure);
    }
  }

  List<LanEndpoint> _candidates(QrData qr) {
    if (qr.lan.isNotEmpty) return qr.lan;
    final host = qr.host;
    if (host == null || host.isEmpty) return const [];
    return [LanEndpoint(host, qr.port ?? config.lanPort)];
  }

  PairHandshake _start(String text, PhoneIdentity identity) {
    try {
      return crypto.pairStart(
        privateKey: identity.privateKey,
        qrText: text,
        deviceId: identity.deviceId,
        deviceName: deviceName,
        nowSeconds: clock.now().millisecondsSinceEpoch ~/ 1000,
      );
    } on CryptoException catch (error) {
      throw PairException(
        error.failure == CryptoFailure.expired
            ? PairFailure.expired
            : PairFailure.badQr,
      );
    }
  }

  Future<Machine> _attempt(
    WireSocket socket,
    PairHandshake handshake,
    QrData qr,
    LanEndpoint? endpoint,
  ) async {
    try {
      final inbox = Inbox(socket);
      final result = await _exchange(socket, inbox, handshake, qr);
      final reply = await _verdict(inbox, result.link);
      if (reply['t'] == 'pair.denied') {
        throw const PairException(PairFailure.denied);
      }
      final machine = _machineFrom(reply, qr, endpoint);
      await machines.save(machine);
      return machine;
    } finally {
      await socket.close();
    }
  }

  Future<PairResult> _exchange(
    WireSocket socket,
    Inbox inbox,
    PairHandshake handshake,
    QrData qr,
  ) async {
    final PairResult result;
    try {
      socket.sendBinary(handshake.msg1);
      final Uint8List msg2 = await inbox.nextBinary(config.handshakeTimeout);
      result = handshake.finish(msg2);
    } on TimeoutException {
      throw const _Early(PairFailure.timeout);
    } on SocketClosedException {
      throw const _Early(PairFailure.network);
    } on CryptoException catch (error) {
      if (error.failure == CryptoFailure.keyMismatch) {
        throw const _Early(PairFailure.keyMismatch);
      }
      throw const _Early(PairFailure.protocol);
    }
    if (!_equal(result.desktopKey, qr.mkey) || result.machineId != qr.machine) {
      throw const _Early(PairFailure.keyMismatch);
    }
    socket.sendBinary(result.msg3);
    return result;
  }

  Future<JsonMap> _verdict(Inbox inbox, SecureLink link) async {
    try {
      return await _awaitVerdict(inbox, link);
    } on TimeoutException {
      throw const PairException(PairFailure.timeout);
    } on SocketClosedException {
      throw const PairException(PairFailure.network);
    } on CryptoException {
      throw const PairException(PairFailure.protocol);
    }
  }

  Future<JsonMap> _awaitVerdict(Inbox inbox, SecureLink link) async {
    final deadline = clock.now().add(confirmTimeout);
    while (true) {
      final left = deadline.difference(clock.now());
      if (left <= Duration.zero) throw TimeoutException('pair confirm');
      final cipher = await inbox.nextBinary(left);
      final frame = link.open(cipher);
      if (frame == null || frame.kind != FrameKind.json) continue;
      final decoded = jsonDecode(utf8.decode(frame.payload));
      if (decoded is! Map) continue;
      final type = decoded['t'];
      if (type == 'pair.ok' || type == 'pair.denied') {
        return decoded.cast<String, dynamic>();
      }
    }
  }

  Machine _machineFrom(JsonMap reply, QrData qr, LanEndpoint? endpoint) {
    final name = reply['name'];
    final relay = reply['relay'];
    final tokenField = reply['relayToken'];
    String? tokenText;
    if (tokenField is String) {
      tokenText = tokenField;
    } else if (tokenField is Map && tokenField['token'] is String) {
      tokenText = tokenField['token'] as String;
    }
    final lan = endpoint ?? _candidates(qr).firstOrNull;
    return Machine(
      id: qr.machine,
      name: name is String && name.isNotEmpty ? name : qr.name,
      mkey: qr.mkey,
      pairedAt: clock.now().millisecondsSinceEpoch ~/ 1000,
      lastHost: lan?.host,
      lastPort: lan?.port,
      relayUrl: relay is String && relay.isNotEmpty ? relay : qr.relay,
      relayToken: tokenText == null ? null : RelayToken.parse(tokenText),
    );
  }

  bool _equal(Uint8List a, Uint8List b) {
    if (a.length != b.length) return false;
    var diff = 0;
    for (var i = 0; i < a.length; i++) {
      diff |= a[i] ^ b[i];
    }
    return diff == 0;
  }
}
