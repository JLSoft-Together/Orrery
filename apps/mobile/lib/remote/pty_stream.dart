import 'dart:async';
import 'dart:typed_data';

import 'connection.dart';
import 'protocol.dart';

const int ptyTailBytes = 64 * 1024;
const int maxAttach = 4;

class PtyAttachment {
  PtyAttachment._(
    this.stream,
    this.sessionId,
    this.paneKey,
    this.cols,
    this.rows,
  );

  final int stream;
  final String sessionId;
  final String paneKey;
  int cols;
  int rows;

  final StreamController<Uint8List> _output = StreamController.broadcast();
  final StreamController<JsonMap> _events = StreamController.broadcast();
  final List<Uint8List> _pendingInput = [];
  bool _pendingClaim = false;
  Timer? _throttle;
  bool _suspended = false;
  Uint8List _tail = Uint8List(0);

  Stream<Uint8List> get output => _output.stream;

  Stream<JsonMap> get events => _events.stream;

  Uint8List get tail => _tail;

  void _appendTail(Uint8List bytes) {
    if (bytes.length >= ptyTailBytes) {
      _tail = Uint8List.fromList(bytes.sublist(bytes.length - ptyTailBytes));
      return;
    }
    final total = _tail.length + bytes.length;
    final builder = BytesBuilder(copy: false);
    if (total > ptyTailBytes) {
      builder.add(_tail.sublist(total - ptyTailBytes));
    } else {
      builder.add(_tail);
    }
    builder.add(bytes);
    _tail = builder.takeBytes();
  }
}

class PtyStreams {
  PtyStreams(
    this.connection, {
    this.coalesce = const Duration(milliseconds: 30),
    this.limit = maxAttach,
  }) {
    _ptySub = connection.ptyOutput.listen(_onOutput);
    _eventSub = connection.events.listen(_onEvent);
    _readySub = connection.readySessions.listen((_) => _attachAll());
  }

  final MachineConnection connection;
  final Duration coalesce;
  final int limit;

  final Map<int, PtyAttachment> _entries = {};
  late final StreamSubscription<PtyChunk> _ptySub;
  late final StreamSubscription<JsonMap> _eventSub;
  late final StreamSubscription<RemoteSession> _readySub;
  int _next = 1;

  List<PtyAttachment> get attachments => _entries.values.toList();

  Map<String, Uint8List> get tails => {
    for (final entry in _entries.values)
      if (entry.tail.isNotEmpty) entry.paneKey: entry.tail,
  };

  Future<PtyAttachment> attach(
    String sessionId,
    int cols,
    int rows, {
    String? paneKey,
    Uint8List? initialTail,
  }) async {
    if (_entries.length >= limit) {
      throw const RemoteCommandException('limit');
    }
    final entry = PtyAttachment._(
      _next++,
      sessionId,
      paneKey ?? sessionId,
      cols,
      rows,
    );
    if (initialTail != null) entry._appendTail(initialTail);
    _entries[entry.stream] = entry;
    final session = connection.session;
    if (session != null && connection.isReady) {
      try {
        await _sendAttach(session, entry);
      } on RemoteCommandException {
        _entries.remove(entry.stream);
        await _close(entry);
        rethrow;
      }
    }
    return entry;
  }

  Future<void> _sendAttach(RemoteSession session, PtyAttachment entry) async {
    await session.request('pty.attach', {
      'stream': entry.stream,
      'sessionId': entry.sessionId,
      'cols': entry.cols,
      'rows': entry.rows,
    });
  }

  Future<void> _attachAll() async {
    final session = connection.session;
    if (session == null) return;
    for (final entry in _entries.values.toList()) {
      if (entry._suspended) continue;
      try {
        await _sendAttach(session, entry);
      } on RemoteCommandException catch (error) {
        entry._events.add({
          't': 'pty.error',
          'stream': entry.stream,
          'error': error.code,
        });
      }
    }
  }

  Future<void> detach(PtyAttachment entry) async {
    if (_entries.remove(entry.stream) == null) return;
    final session = connection.session;
    if (session != null && connection.isReady && !entry._suspended) {
      try {
        await session.request('pty.detach', {'stream': entry.stream});
      } on RemoteCommandException {
        await _close(entry);
        return;
      }
    }
    await _close(entry);
  }

  Future<void> _close(PtyAttachment entry) async {
    entry._throttle?.cancel();
    await entry._output.close();
    await entry._events.close();
  }

  Future<void> suspendAll() async {
    final session = connection.session;
    for (final entry in _entries.values) {
      entry._suspended = true;
      entry._pendingInput.clear();
      entry._pendingClaim = false;
      entry._throttle?.cancel();
      entry._throttle = null;
      if (session != null && connection.isReady) {
        try {
          await session.request('pty.detach', {'stream': entry.stream});
        } on RemoteCommandException {
          continue;
        }
      }
    }
  }

  Future<void> resumeAll() async {
    for (final entry in _entries.values) {
      entry._suspended = false;
    }
    if (connection.isReady) await _attachAll();
  }

  void input(PtyAttachment entry, Uint8List bytes, {bool claim = false}) {
    if (bytes.isEmpty || entry._suspended) return;
    final session = connection.session;
    if (session == null || !connection.isReady) return;
    if (entry._throttle == null) {
      session.sendPtyInput(entry.stream, bytes, claim: claim);
      entry._throttle = Timer(coalesce, () => _flushInput(entry));
      return;
    }
    entry._pendingInput.add(bytes);
    entry._pendingClaim = entry._pendingClaim || claim;
  }

  void _flushInput(PtyAttachment entry) {
    entry._throttle = null;
    if (entry._pendingInput.isEmpty) return;
    final builder = BytesBuilder(copy: false);
    for (final chunk in entry._pendingInput) {
      builder.add(chunk);
    }
    entry._pendingInput.clear();
    final claim = entry._pendingClaim;
    entry._pendingClaim = false;
    final session = connection.session;
    if (session == null || !connection.isReady) return;
    session.sendPtyInput(entry.stream, builder.takeBytes(), claim: claim);
    entry._throttle = Timer(coalesce, () => _flushInput(entry));
  }

  Future<void> resize(PtyAttachment entry, int cols, int rows) async {
    entry.cols = cols;
    entry.rows = rows;
    final session = connection.session;
    if (session == null || !connection.isReady || entry._suspended) return;
    try {
      await session.request('pty.resize', {
        'stream': entry.stream,
        'cols': cols,
        'rows': rows,
      });
    } on RemoteCommandException {
      return;
    }
  }

  void _onOutput(PtyChunk chunk) {
    final entry = _entries[chunk.stream];
    if (entry == null || entry._output.isClosed) return;
    entry._appendTail(chunk.bytes);
    entry._output.add(chunk.bytes);
  }

  void _onEvent(JsonMap event) {
    final type = event['t'];
    if (type is! String || !type.startsWith('pty.')) return;
    final stream = event['stream'];
    if (stream is! int) return;
    final entry = _entries[stream];
    if (entry == null || entry._events.isClosed) return;
    if (type == 'pty.ready') entry._tail = Uint8List(0);
    entry._events.add(event);
    if (type == 'pty.overflow') unawaited(_reattach(entry));
  }

  Future<void> _reattach(PtyAttachment entry) async {
    final session = connection.session;
    if (session == null || !connection.isReady || entry._suspended) return;
    try {
      await session.request('pty.detach', {'stream': entry.stream});
      await _sendAttach(session, entry);
    } on RemoteCommandException {
      return;
    }
  }

  Future<void> dispose() async {
    await _ptySub.cancel();
    await _eventSub.cancel();
    await _readySub.cancel();
    for (final entry in _entries.values.toList()) {
      await _close(entry);
    }
    _entries.clear();
  }
}
