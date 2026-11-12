import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/crypto.dart';
import 'package:orrery_remote/remote/discovery.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/remote/wire.dart';

const int fakeClaimBit = 0x80;

Uint8List fakeFrame(
  int kind,
  int stream,
  List<int> payload, {
  bool claim = false,
}) {
  final out = BytesBuilder();
  final header = ByteData(5)
    ..setUint8(0, claim ? kind | fakeClaimBit : kind)
    ..setUint32(1, stream);
  out.add(header.buffer.asUint8List());
  out.add(payload);
  return out.toBytes();
}

class FakeLink implements SecureLink {
  @override
  bool closed = false;

  @override
  List<Uint8List> seal(
    int kind,
    int stream,
    Uint8List payload, {
    bool claim = false,
  }) => [fakeFrame(kind, stream, payload, claim: claim)];

  @override
  InboundFrame? open(Uint8List ciphertext) {
    final view = ByteData.sublistView(ciphertext);
    return InboundFrame(
      view.getUint8(0) & ~fakeClaimBit,
      view.getUint32(1),
      Uint8List.sublistView(ciphertext, 5),
    );
  }
}

class _FakePair implements PairHandshake {
  _FakePair(this.qr);

  final QrData qr;

  @override
  Uint8List get msg1 => utf8.encode('pair1');

  @override
  PairResult finish(Uint8List msg2) {
    if (utf8.decode(msg2) == 'pair2:mismatch') {
      throw const CryptoException(CryptoFailure.keyMismatch);
    }
    return PairResult(
      machineId: qr.machine,
      desktopKey: qr.mkey,
      msg3: utf8.encode('pair3'),
      link: FakeLink(),
    );
  }
}

class _FakeSession implements SessionHandshake {
  @override
  Uint8List get msg1 => utf8.encode('sess1');

  @override
  SecureLink finish(Uint8List msg2) {
    if (utf8.decode(msg2) != 'sess2') {
      throw const CryptoException(CryptoFailure.crypto);
    }
    return FakeLink();
  }
}

class FakeCrypto implements RemoteCrypto {
  int nowSeconds = 0;

  @override
  Uint8List generateKey() => Uint8List.fromList(List.generate(32, (i) => i));

  static String qrText({
    required String machine,
    String name = 'Studio',
    String? host,
    int? port,
    String? relay,
    int exp = 4000000000,
    List<int>? mkey,
  }) => jsonEncode({
    'machine': machine,
    'name': name,
    'host': host,
    'port': port,
    'relay': relay,
    'exp': exp,
    'mkey': base64Url.encode(mkey ?? List.filled(32, 7)),
  });

  @override
  QrData decodeQr(String text) {
    try {
      final map = jsonDecode(text) as Map<String, dynamic>;
      return QrData(
        machine: map['machine'] as String,
        name: map['name'] as String,
        host: map['host'] as String?,
        port: map['port'] as int?,
        relay: map['relay'] as String?,
        mkey: Uint8List.fromList(base64Url.decode(map['mkey'] as String)),
        exp: map['exp'] as int,
      );
    } catch (_) {
      throw const CryptoException(CryptoFailure.badQr);
    }
  }

  @override
  PairHandshake pairStart({
    required Uint8List privateKey,
    required String qrText,
    required String deviceId,
    required String deviceName,
    required int nowSeconds,
  }) {
    final qr = decodeQr(qrText);
    if (nowSeconds >= qr.exp) {
      throw const CryptoException(CryptoFailure.expired);
    }
    return _FakePair(qr);
  }

  @override
  SessionHandshake sessionStart({
    required Uint8List privateKey,
    required Uint8List desktopKey,
    required String deviceId,
  }) => _FakeSession();
}

class FakeServer implements WireSocket {
  FakeServer({
    this.answerHandshake = true,
    this.handshakeReply = 'sess2',
    this.relayToken,
    this.autoSnapshot = true,
    this.silent = false,
    this.closeOnHandshake,
    this.dropOnHandshake = false,
  });

  final bool answerHandshake;
  final String handshakeReply;
  final String? relayToken;
  final bool autoSnapshot;
  final int? closeOnHandshake;
  final bool dropOnHandshake;
  bool silent;
  bool hub = false;
  final Map<int, List<int>> ring = {};
  final Set<int> gone = {};

  final StreamController<Object?> _in = StreamController<Object?>();
  final List<Map<String, dynamic>> commands = [];
  final List<Uint8List> ptyInput = [];
  final List<bool> ptyClaims = [];
  final List<String> texts = [];
  int pingFrames = 0;
  bool _handshook = false;
  bool closedByClient = false;
  int? _code;
  int seq = 1;

  @override
  Stream<Object?> get incoming => _in.stream;

  @override
  int? get closeCode => _code;

  bool get isClosed => _in.isClosed;

  @override
  void sendText(String text) {
    texts.add(text);
    if (text == 'ping' && !silent) _in.add('pong');
  }

  @override
  void sendBinary(Uint8List data) {
    if (!_handshook) {
      _handshook = true;
      if (closeOnHandshake != null || dropOnHandshake) {
        serverClose(closeOnHandshake);
        return;
      }
      if (answerHandshake) _in.add(utf8.encode(handshakeReply));
      return;
    }
    final frame = FakeLink().open(data)!;
    switch (frame.kind) {
      case FrameKind.json:
        final command =
            jsonDecode(utf8.decode(frame.payload)) as Map<String, dynamic>;
        commands.add(command);
        if (!silent) _answer(command);
      case FrameKind.ptyIn:
        ptyInput.add(frame.payload);
        ptyClaims.add(data[0] & fakeClaimBit != 0);
        pushPty(
          frame.stream,
          utf8.encode('echo:${utf8.decode(frame.payload)}'),
        );
      case FrameKind.ping:
        pingFrames++;
        if (!silent) _in.add(fakeFrame(FrameKind.ping, 0, const []));
    }
  }

  void _answer(Map<String, dynamic> command) {
    final id = command['id'];
    switch (command['t']) {
      case 'hello':
        pushJson({
          're': id,
          'ok': true,
          'name': 'Studio',
          'version': 'x',
          'minProto': 1,
          ...?(relayToken == null ? null : {'relayToken': relayToken}),
        });
        if (autoSnapshot) {
          pushJson({
            't': 'snapshot',
            'seq': seq,
            'data': {
              'terminals': [
                {'id': 't1', 'title': 'one'},
              ],
              'agents': [],
              'activity': [],
            },
          });
        }
      case 'resync':
        pushJson({'re': id, 'ok': true});
        pushJson({
          't': 'snapshot',
          'seq': seq,
          'data': {'terminals': [], 'agents': [], 'activity': []},
        });
      case 'fail':
        pushJson({'re': id, 'ok': false, 'error': 'denied', 'detail': 'no'});
      case 'silent':
        break;
      case 'pty.attach' when hub:
        final stream = command['stream'] as int;
        gone.remove(stream);
        pushJson({'re': id, 'ok': true, 'stream': stream});
        pushJson({'t': 'pty.ready', 'stream': stream, 'cols': 80, 'rows': 24});
        final kept = ring[stream];
        if (kept != null && kept.isNotEmpty) {
          _in.add(fakeFrame(FrameKind.ptyOut, stream, List<int>.of(kept)));
        }
      case 'pty.detach' when hub && gone.contains(command['stream']):
        pushJson({'re': id, 'ok': false, 'error': 'notFound'});
      default:
        pushJson({'re': id, 'ok': true});
    }
  }

  void pushJson(Map<String, dynamic> value) {
    if (_in.isClosed) return;
    _in.add(fakeFrame(FrameKind.json, 0, utf8.encode(jsonEncode(value))));
  }

  void hubOverflow(int stream) {
    gone.add(stream);
    pushJson({'t': 'pty.overflow', 'stream': stream});
  }

  void pushPty(int stream, List<int> bytes) {
    if (_in.isClosed) return;
    if (hub) ring.putIfAbsent(stream, () => []).addAll(bytes);
    _in.add(fakeFrame(FrameKind.ptyOut, stream, bytes));
  }

  void pushText(String text) {
    if (!_in.isClosed) _in.add(text);
  }

  void serverClose(int? code) {
    _code = code;
    if (!_in.isClosed) unawaited(_in.close());
  }

  @override
  Future<void> close([int? code]) async {
    closedByClient = true;
    _code ??= code;
    if (!_in.isClosed) unawaited(_in.close());
  }

  int get helloCount => commands.where((c) => c['t'] == 'hello').length;
}

class FakeDiscovery implements LanDiscovery {
  FakeDiscovery({this.endpoint, this.delay = Duration.zero});

  LanEndpoint? endpoint;
  Duration delay;
  int calls = 0;

  @override
  Future<LanEndpoint?> find(String machineId, Duration timeout) async {
    calls++;
    final wait = endpoint == null ? timeout : delay;
    await Future<void>.delayed(wait);
    return endpoint;
  }
}

class FakeNetwork implements NetworkWatcher {
  final StreamController<String> controller = StreamController<String>();

  @override
  Stream<String> get changes => controller.stream;
}

class ConnectCall {
  ConnectCall(this.uri, this.at);

  final Uri uri;
  final DateTime at;
  bool get isLan => uri.scheme == 'ws';
  bool get isRelay => uri.scheme == 'wss';
}

typedef Responder = FutureOr<WireSocket> Function(Uri uri);

class FakeNet {
  final List<ConnectCall> calls = [];
  Responder lan = (uri) => throw StateError('lan down');
  Responder relay = (uri) => throw StateError('relay down');
  DateTime Function() now = DateTime.now;

  Future<WireSocket> connector(Uri uri, Duration timeout) async {
    calls.add(ConnectCall(uri, now()));
    return uri.scheme == 'ws' ? lan(uri) : relay(uri);
  }

  List<ConnectCall> get relayCalls => calls.where((c) => c.isRelay).toList();

  List<ConnectCall> get lanCalls => calls.where((c) => c.isLan).toList();
}

Machine testMachine({
  String id = 'm1',
  String? lastHost = '10.0.0.5',
  int? lastPort = 4320,
  String? relayUrl = 'https://relay.example',
  RelayToken? token,
  bool revoked = false,
}) => Machine(
  id: id,
  name: 'Studio',
  mkey: Uint8List.fromList(List.filled(32, 7)),
  pairedAt: 1,
  lastHost: lastHost,
  lastPort: lastPort,
  relayUrl: relayUrl,
  relayToken: token ?? RelayToken('tok-$id', 4000000000),
  revoked: revoked,
);

Future<ConnectionDeps> testDeps({
  required FakeNet net,
  required List<Machine> machines,
  LanDiscovery discovery = const NoDiscovery(),
  NetworkWatcher network = const NoNetworkWatcher(),
  ConnectionConfig config = const ConnectionConfig(),
}) async {
  final crypto = FakeCrypto();
  final repo = MachineRepository(secure: MemorySecureKv());
  for (final machine in machines) {
    await repo.save(machine);
  }
  return ConnectionDeps(
    crypto: crypto,
    connector: net.connector,
    keys: KeyStore(MemorySecureKv(), crypto),
    machines: repo,
    discovery: discovery,
    network: network,
    config: config,
  );
}

class FakePairServer implements WireSocket {
  FakePairServer({
    this.msg2 = 'pair2',
    this.verdict = const {'t': 'pair.ok', 'name': 'Studio PC'},
    this.dropAfterMsg3 = false,
  });

  final String msg2;
  final Map<String, dynamic>? verdict;
  final bool dropAfterMsg3;
  final StreamController<Object?> _in = StreamController<Object?>();
  final List<String> received = [];
  bool closedByClient = false;
  int _step = 0;

  @override
  Stream<Object?> get incoming => _in.stream;

  @override
  int? get closeCode => null;

  @override
  void sendText(String text) {}

  @override
  void sendBinary(Uint8List data) {
    received.add(utf8.decode(data));
    _step++;
    if (_step == 1) {
      _in.add(utf8.encode(msg2));
    } else if (_step == 2) {
      if (dropAfterMsg3) {
        unawaited(_in.close());
        return;
      }
      final body = verdict;
      if (body != null) {
        _in.add(fakeFrame(FrameKind.json, 0, utf8.encode(jsonEncode(body))));
      }
    }
  }

  @override
  Future<void> close([int? code]) async {
    closedByClient = true;
    if (!_in.isClosed) unawaited(_in.close());
  }
}

Uint8List makeToken(int exp, {String deviceId = 'dev'}) {
  final out = BytesBuilder();
  out.add(utf8.encode(deviceId));
  final view = ByteData(8)..setUint64(0, exp);
  out.add(view.buffer.asUint8List());
  out.add(Uint8List(32));
  return out.toBytes();
}

String makeTokenText(int exp) =>
    base64Url.encode(makeToken(exp)).replaceAll('=', '');
