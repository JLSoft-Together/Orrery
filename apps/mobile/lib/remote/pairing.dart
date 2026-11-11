import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:clock/clock.dart';

import 'connection.dart';
import 'crypto.dart';
import 'discovery.dart';
import 'keys.dart';
import 'machines.dart';
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

  Future<Machine> pair(String qrText) async {
    final QrData qr;
    try {
      qr = crypto.decodeQr(qrText.trim());
    } on CryptoException {
      throw const PairException(PairFailure.badQr);
    }
    final nowSeconds = clock.now().millisecondsSinceEpoch ~/ 1000;
    if (nowSeconds >= qr.exp) throw const PairException(PairFailure.expired);
    final identity = await keys.load();
    final PairHandshake handshake;
    try {
      handshake = crypto.pairStart(
        privateKey: identity.privateKey,
        qrText: qrText.trim(),
        deviceId: identity.deviceId,
        deviceName: deviceName,
        nowSeconds: nowSeconds,
      );
    } on CryptoException catch (error) {
      throw PairException(
        error.failure == CryptoFailure.expired
            ? PairFailure.expired
            : PairFailure.badQr,
      );
    }
    final endpoint = await _endpoint(qr);
    final socket = await _open(endpoint);
    try {
      return await _run(socket, handshake, qr, endpoint, identity);
    } finally {
      await socket.close();
    }
  }

  Future<LanEndpoint> _endpoint(QrData qr) async {
    final host = qr.host;
    if (host != null && host.isNotEmpty) {
      return LanEndpoint(host, qr.port ?? config.lanPort);
    }
    final found = await discovery.find(qr.machine, config.lanBudget * 2);
    if (found == null) throw const PairException(PairFailure.noRoute);
    return found;
  }

  Future<WireSocket> _open(LanEndpoint endpoint) async {
    final uri = Uri(
      scheme: 'ws',
      host: endpoint.host,
      port: endpoint.port,
      path: '/remote',
    );
    try {
      return await connector(uri, config.handshakeTimeout);
    } catch (_) {
      throw const PairException(PairFailure.network);
    }
  }

  Future<Machine> _run(
    WireSocket socket,
    PairHandshake handshake,
    QrData qr,
    LanEndpoint endpoint,
    PhoneIdentity identity,
  ) async {
    final inbox = Inbox(socket);
    final PairResult result;
    try {
      socket.sendBinary(handshake.msg1);
      final Uint8List msg2 = await inbox.nextBinary(config.handshakeTimeout);
      result = handshake.finish(msg2);
    } on TimeoutException {
      throw const PairException(PairFailure.timeout);
    } on SocketClosedException {
      throw const PairException(PairFailure.network);
    } on CryptoException catch (error) {
      if (error.failure == CryptoFailure.keyMismatch) {
        throw const PairException(PairFailure.keyMismatch);
      }
      throw const PairException(PairFailure.protocol);
    }
    if (!_equal(result.desktopKey, qr.mkey) || result.machineId != qr.machine) {
      throw const PairException(PairFailure.keyMismatch);
    }
    socket.sendBinary(result.msg3);
    final JsonMap reply;
    try {
      reply = await _awaitVerdict(inbox, result.link);
    } on TimeoutException {
      throw const PairException(PairFailure.timeout);
    } on SocketClosedException {
      throw const PairException(PairFailure.network);
    } on CryptoException {
      throw const PairException(PairFailure.protocol);
    }
    if (reply['t'] == 'pair.denied') {
      throw const PairException(PairFailure.denied);
    }
    final machine = _machineFrom(reply, qr, endpoint);
    await machines.save(machine);
    return machine;
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

  Machine _machineFrom(JsonMap reply, QrData qr, LanEndpoint endpoint) {
    final name = reply['name'];
    final relay = reply['relay'];
    final tokenField = reply['relayToken'];
    String? tokenText;
    if (tokenField is String) {
      tokenText = tokenField;
    } else if (tokenField is Map && tokenField['token'] is String) {
      tokenText = tokenField['token'] as String;
    }
    return Machine(
      id: qr.machine,
      name: name is String && name.isNotEmpty ? name : qr.name,
      mkey: qr.mkey,
      pairedAt: clock.now().millisecondsSinceEpoch ~/ 1000,
      lastHost: endpoint.host,
      lastPort: endpoint.port,
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
