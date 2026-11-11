import 'dart:async';
import 'dart:typed_data';

import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

abstract class WireSocket {
  Stream<Object?> get incoming;

  void sendBinary(Uint8List data);

  void sendText(String text);

  Future<void> close([int? code]);

  int? get closeCode;
}

typedef WireConnector = Future<WireSocket> Function(Uri uri, Duration timeout);

class SocketClosedException implements Exception {
  const SocketClosedException([this.code]);

  final int? code;

  @override
  String toString() => 'SocketClosedException($code)';
}

class Inbox {
  Inbox(this._socket) : _iterator = StreamIterator(_socket.incoming);

  final WireSocket _socket;
  final StreamIterator<Object?> _iterator;
  bool _done = false;

  Future<Object?> next([Duration? timeout]) async {
    if (_done) throw SocketClosedException(_socket.closeCode);
    bool hasNext;
    try {
      final pending = _iterator.moveNext();
      hasNext = await (timeout == null ? pending : pending.timeout(timeout));
    } on TimeoutException {
      rethrow;
    } catch (_) {
      _done = true;
      throw SocketClosedException(_socket.closeCode);
    }
    if (!hasNext) {
      _done = true;
      throw SocketClosedException(_socket.closeCode);
    }
    return _iterator.current;
  }

  Future<Uint8List> nextBinary([Duration? timeout]) async {
    final deadline = timeout == null ? null : DateTime.now().add(timeout);
    while (true) {
      final remaining = deadline?.difference(DateTime.now());
      if (remaining != null && remaining <= Duration.zero) {
        throw TimeoutException('handshake');
      }
      final message = await next(remaining);
      if (message is Uint8List) return message;
      if (message is List<int>) return Uint8List.fromList(message);
    }
  }
}

class IoWireSocket implements WireSocket {
  IoWireSocket._(this._channel) {
    _channel.stream.listen(
      (data) {
        if (data is String) {
          _controller.add(data);
        } else if (data is List<int>) {
          _controller.add(data is Uint8List ? data : Uint8List.fromList(data));
        }
      },
      onError: _controller.addError,
      onDone: _controller.close,
      cancelOnError: false,
    );
  }

  final WebSocketChannel _channel;
  final StreamController<Object?> _controller = StreamController<Object?>();

  static Future<WireSocket> connect(Uri uri, Duration timeout) async {
    final channel = IOWebSocketChannel.connect(uri, connectTimeout: timeout);
    try {
      await channel.ready.timeout(timeout);
    } catch (_) {
      unawaited(channel.sink.close().catchError((Object _) {}));
      rethrow;
    }
    return IoWireSocket._(channel);
  }

  @override
  Stream<Object?> get incoming => _controller.stream;

  @override
  void sendBinary(Uint8List data) => _channel.sink.add(data);

  @override
  void sendText(String text) => _channel.sink.add(text);

  @override
  Future<void> close([int? code]) async {
    try {
      await _channel.sink.close(code).timeout(const Duration(seconds: 2));
    } catch (_) {}
  }

  @override
  int? get closeCode => _channel.closeCode;
}
