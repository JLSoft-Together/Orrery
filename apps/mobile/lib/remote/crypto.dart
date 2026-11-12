import 'dart:typed_data';

import 'discovery.dart';

class FrameKind {
  static const int json = 0;
  static const int ptyOut = 1;
  static const int ptyIn = 2;
  static const int ping = 3;
}

enum CryptoFailure {
  crypto,
  keyMismatch,
  unauthorized,
  protoTooOld,
  protoTooNew,
  badMessage,
  badPayload,
  badKey,
  badQr,
  expired,
  frame,
  tooLarge,
  closed,
  rng,
  busy,
}

class CryptoException implements Exception {
  const CryptoException(this.failure, [this.value = 0]);

  final CryptoFailure failure;
  final int value;

  @override
  String toString() => 'CryptoException($failure, $value)';
}

class QrData {
  const QrData({
    required this.machine,
    required this.name,
    required this.mkey,
    required this.exp,
    this.host,
    this.port,
    this.relay,
    this.lan = const [],
    this.ticket,
  });

  final String machine;
  final String name;
  final String? host;
  final int? port;
  final String? relay;
  final List<LanEndpoint> lan;
  final String? ticket;
  final Uint8List mkey;
  final int exp;
}

class InboundFrame {
  const InboundFrame(this.kind, this.stream, this.payload);

  final int kind;
  final int stream;
  final Uint8List payload;
}

abstract class SecureLink {
  List<Uint8List> seal(
    int kind,
    int stream,
    Uint8List payload, {
    bool claim = false,
  });

  InboundFrame? open(Uint8List ciphertext);

  bool get closed;
}

class PairResult {
  const PairResult({
    required this.machineId,
    required this.desktopKey,
    required this.msg3,
    required this.link,
  });

  final String machineId;
  final Uint8List desktopKey;
  final Uint8List msg3;
  final SecureLink link;
}

abstract class PairHandshake {
  Uint8List get msg1;

  PairResult finish(Uint8List msg2);
}

abstract class SessionHandshake {
  Uint8List get msg1;

  SecureLink finish(Uint8List msg2);
}

abstract class RemoteCrypto {
  Uint8List generateKey();

  QrData decodeQr(String text);

  PairHandshake pairStart({
    required Uint8List privateKey,
    required String qrText,
    required String deviceId,
    required String deviceName,
    required int nowSeconds,
  });

  SessionHandshake sessionStart({
    required Uint8List privateKey,
    required Uint8List desktopKey,
    required String deviceId,
  });
}
