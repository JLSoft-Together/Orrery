import 'dart:typed_data';

import '../src/rust/api/remote.dart' as rust;
import 'crypto.dart';

CryptoException _map(rust.RemoteError error) {
  final failure = switch (error.kind) {
    rust.RemoteErrorKind.crypto => CryptoFailure.crypto,
    rust.RemoteErrorKind.keyMismatch => CryptoFailure.keyMismatch,
    rust.RemoteErrorKind.unauthorized => CryptoFailure.unauthorized,
    rust.RemoteErrorKind.protoTooOld => CryptoFailure.protoTooOld,
    rust.RemoteErrorKind.protoTooNew => CryptoFailure.protoTooNew,
    rust.RemoteErrorKind.badMessage => CryptoFailure.badMessage,
    rust.RemoteErrorKind.badPayload => CryptoFailure.badPayload,
    rust.RemoteErrorKind.badKey => CryptoFailure.badKey,
    rust.RemoteErrorKind.badQr => CryptoFailure.badQr,
    rust.RemoteErrorKind.expired => CryptoFailure.expired,
    rust.RemoteErrorKind.frame => CryptoFailure.frame,
    rust.RemoteErrorKind.tooLarge => CryptoFailure.tooLarge,
    rust.RemoteErrorKind.closed => CryptoFailure.closed,
    rust.RemoteErrorKind.rng => CryptoFailure.rng,
    rust.RemoteErrorKind.busy => CryptoFailure.busy,
  };
  return CryptoException(failure, error.value);
}

T _guard<T>(T Function() body) {
  try {
    return body();
  } on rust.RemoteError catch (error) {
    throw _map(error);
  }
}

class _RustLink implements SecureLink {
  _RustLink(this._channel);

  final rust.SecureChannel _channel;

  @override
  List<Uint8List> seal(int kind, int stream, Uint8List payload) =>
      _guard(() => _channel.seal(kind: kind, stream: stream, payload: payload));

  @override
  InboundFrame? open(Uint8List ciphertext) {
    final message = _guard(() => _channel.open(ciphertext: ciphertext));
    if (message == null) return null;
    return InboundFrame(message.kind, message.stream, message.payload);
  }

  @override
  bool get closed => _channel.isClosed();
}

class _RustPair implements PairHandshake {
  _RustPair(this._start);

  final rust.PairingStart _start;

  @override
  Uint8List get msg1 => _start.msg1;

  @override
  PairResult finish(Uint8List msg2) {
    final done = _guard(() => _start.handshake.finish(msg2: msg2));
    return PairResult(
      machineId: done.machineId,
      desktopKey: done.desktopKey,
      msg3: done.msg3,
      link: _RustLink(done.channel),
    );
  }
}

class _RustSession implements SessionHandshake {
  _RustSession(this._start);

  final rust.SessionStart _start;

  @override
  Uint8List get msg1 => _start.msg1;

  @override
  SecureLink finish(Uint8List msg2) =>
      _RustLink(_guard(() => _start.handshake.finish(msg2: msg2)));
}

class RustRemoteCrypto implements RemoteCrypto {
  const RustRemoteCrypto();

  @override
  Uint8List generateKey() => _guard(rust.generateStaticKey);

  @override
  QrData decodeQr(String text) {
    final qr = _guard(() => rust.decodeQr(text: text));
    return QrData(
      machine: qr.machine,
      name: qr.name,
      host: qr.host,
      port: qr.port,
      relay: qr.relay,
      mkey: qr.mkey,
      exp: qr.exp.toInt(),
    );
  }

  @override
  PairHandshake pairStart({
    required Uint8List privateKey,
    required String qrText,
    required String deviceId,
    required String deviceName,
    required int nowSeconds,
  }) => _RustPair(
    _guard(
      () => rust.pairStart(
        private: privateKey,
        qrText: qrText,
        deviceId: deviceId,
        deviceName: deviceName,
        now: BigInt.from(nowSeconds),
      ),
    ),
  );

  @override
  SessionHandshake sessionStart({
    required Uint8List privateKey,
    required Uint8List desktopKey,
    required String deviceId,
  }) => _RustSession(
    _guard(
      () => rust.sessionStart(
        private: privateKey,
        desktopKey: desktopKey,
        deviceId: deviceId,
      ),
    ),
  );
}
