import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'crypto.dart';
import 'wire.dart';

class CloseCode {
  static const int handshake = 4400;
  static const int revoked = 4401;
  static const int desktopReplaced = 4409;
  static const int protoTooOld = 4426;
  static const int rate = 4429;
  static const int gatewayDown = 4503;
}

class RemoteCommandException implements Exception {
  const RemoteCommandException(this.code, [this.detail]);

  final String code;
  final String? detail;

  @override
  String toString() => 'RemoteCommandException($code, $detail)';
}

class PtyChunk {
  const PtyChunk(this.stream, this.bytes);

  final int stream;
  final Uint8List bytes;
}

typedef JsonMap = Map<String, dynamic>;

class RemoteSession {
  RemoteSession({
    required this._socket,
    required this._inbox,
    required this._link,
    required this.viaRelay,
    this.requestTimeout = const Duration(seconds: 10),
    DateTime Function()? clock,
  }) : _clock = clock ?? DateTime.now {
    lastActivity = _clock();
  }

  static const int maxJsonBytes = 1 << 20;

  final bool viaRelay;
  final Duration requestTimeout;
  final WireSocket _socket;
  final Inbox _inbox;
  final SecureLink _link;
  final DateTime Function() _clock;
  final Map<String, Completer<JsonMap>> _pending = {};
  final StreamController<JsonMap> _events = StreamController.broadcast();
  final StreamController<PtyChunk> _pty = StreamController.broadcast();
  final Completer<int?> _closed = Completer<int?>();
  int _counter = 0;
  bool _pingOutstanding = false;
  bool _running = false;
  late DateTime lastActivity;

  Stream<JsonMap> get events => _events.stream;

  Stream<PtyChunk> get ptyOutput => _pty.stream;

  Future<int?> get closed => _closed.future;

  bool get isClosed => _closed.isCompleted;

  void start() {
    if (_running) return;
    _running = true;
    unawaited(_loop());
  }

  Future<void> _loop() async {
    int? code;
    try {
      while (true) {
        final message = await _inbox.next();
        lastActivity = _clock();
        if (message is String) continue;
        if (message is Uint8List) {
          _handleCipher(message);
        }
      }
    } on SocketClosedException catch (error) {
      code = error.code;
    } on CryptoException {
      code = CloseCode.handshake;
      unawaited(_socket.close(CloseCode.handshake));
    } catch (_) {
      code = CloseCode.handshake;
      unawaited(_socket.close(CloseCode.handshake));
    }
    _finish(code);
  }

  void _finish(int? code) {
    if (_closed.isCompleted) return;
    for (final pending in _pending.values) {
      if (!pending.isCompleted) {
        pending.completeError(const RemoteCommandException('offline'));
      }
    }
    _pending.clear();
    _closed.complete(code);
    unawaited(_events.close());
    unawaited(_pty.close());
  }

  void _handleCipher(Uint8List cipher) {
    final frame = _link.open(cipher);
    if (frame == null) return;
    switch (frame.kind) {
      case FrameKind.json:
        _handleJson(frame.payload);
      case FrameKind.ptyOut:
        if (!_pty.isClosed) _pty.add(PtyChunk(frame.stream, frame.payload));
      case FrameKind.ping:
        if (_pingOutstanding) {
          _pingOutstanding = false;
        } else {
          _sendFrame(FrameKind.ping, 0, Uint8List(0));
        }
    }
  }

  void _handleJson(Uint8List payload) {
    final Object? decoded;
    try {
      decoded = jsonDecode(utf8.decode(payload));
    } on FormatException {
      return;
    }
    if (decoded is! Map) return;
    final map = decoded.cast<String, dynamic>();
    final re = map['re'];
    if (re is String && map['t'] is! String) {
      final pending = _pending.remove(re);
      if (pending == null || pending.isCompleted) return;
      if (map['ok'] == true) {
        pending.complete(map);
      } else {
        final detail = map['detail'];
        pending.completeError(
          RemoteCommandException(
            (map['error'] as String?) ?? 'internal',
            detail is String ? detail : null,
          ),
        );
      }
      return;
    }
    if (!_events.isClosed) _events.add(map);
  }

  void _sendFrame(
    int kind,
    int stream,
    Uint8List payload, {
    bool claim = false,
  }) {
    if (_closed.isCompleted) return;
    for (final cipher in _link.seal(kind, stream, payload, claim: claim)) {
      _socket.sendBinary(cipher);
    }
  }

  Future<JsonMap> request(String type, [JsonMap? fields, Duration? timeout]) {
    if (_closed.isCompleted) {
      return Future.error(const RemoteCommandException('offline'));
    }
    final id = 'c${(++_counter).toRadixString(36)}';
    final body = utf8.encode(jsonEncode({'id': id, 't': type, ...?fields}));
    if (body.length > maxJsonBytes) {
      return Future.error(const RemoteCommandException('invalid', 'size'));
    }
    final completer = Completer<JsonMap>();
    _pending[id] = completer;
    try {
      _sendFrame(FrameKind.json, 0, Uint8List.fromList(body));
    } on CryptoException {
      _pending.remove(id);
      return Future.error(const RemoteCommandException('internal'));
    }
    return completer.future.timeout(
      timeout ?? requestTimeout,
      onTimeout: () {
        _pending.remove(id);
        throw const RemoteCommandException('timeout');
      },
    );
  }

  void send(JsonMap message) {
    final body = utf8.encode(jsonEncode(message));
    if (body.length > maxJsonBytes) return;
    _sendFrame(FrameKind.json, 0, Uint8List.fromList(body));
  }

  void sendPtyInput(int stream, Uint8List bytes, {bool claim = false}) {
    if (bytes.isEmpty) return;
    _sendFrame(FrameKind.ptyIn, stream, bytes, claim: claim);
  }

  void sendKeepalive() {
    if (_closed.isCompleted) return;
    if (viaRelay) {
      _socket.sendText('ping');
    } else {
      _pingOutstanding = true;
      _sendFrame(FrameKind.ping, 0, Uint8List(0));
    }
  }

  Future<void> close([int? code]) async {
    await _socket.close(code);
    _finish(_socket.closeCode ?? code);
  }
}
