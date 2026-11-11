import 'dart:async';

import 'package:clock/clock.dart';

import 'connect_attempt.dart';
import 'connection_config.dart';
export 'connection_config.dart';
import 'crypto.dart';
import 'lan_dialer.dart';
import 'link_rules.dart';
import 'keys.dart';
import 'link_state.dart';
import 'machines.dart';
import 'protocol.dart';
import 'session_handshake.dart';
import 'wire.dart';

class MachineConnection {
  MachineConnection({required this.machineId, required this.deps});

  final String machineId;
  final ConnectionDeps deps;

  final StreamController<LinkState> _states = StreamController.broadcast();
  final StreamController<JsonMap> _events = StreamController.broadcast();
  final StreamController<PtyChunk> _pty = StreamController.broadcast();
  final StreamController<RemoteSession> _sessions =
      StreamController.broadcast();

  LinkState _state = const LinkState();
  RemoteSession? _session;
  StreamSubscription<JsonMap>? _eventSub;
  StreamSubscription<PtyChunk>? _ptySub;
  StreamSubscription<String>? _networkSub;
  Timer? _keepaliveTimer;
  Timer? _backgroundTimer;
  Timer? _backoffTimer;
  Completer<void>? _backoffWait;
  int _generation = 0;
  int _failures = 0;
  bool _skipDelay = false;
  bool _suspended = false;
  String? _lastNetwork;
  LinkReason _hint = LinkReason.none;

  LinkState get state => _state;

  Stream<LinkState> get states => _states.stream;

  Stream<JsonMap> get events => _events.stream;

  Stream<PtyChunk> get ptyOutput => _pty.stream;

  Stream<RemoteSession> get readySessions => _sessions.stream;

  RemoteSession? get session => _session;

  bool get isReady => _state.status == LinkStatus.ready && _session != null;

  void _emit(LinkState next) {
    if (next == _state) return;
    _state = next;
    if (!_states.isClosed) _states.add(next);
  }

  Future<void> connect() async {
    _backgroundTimer?.cancel();
    _suspended = false;
    final generation = ++_generation;
    _wake();
    await _dropSession();
    _failures = 0;
    _skipDelay = false;
    _hint = LinkReason.none;
    _watchNetwork();
    _emit(const LinkState(status: LinkStatus.discovering));
    unawaited(_run(generation));
  }

  Future<void> disconnect() async {
    _backgroundTimer?.cancel();
    _generation++;
    _wake();
    await _dropSession();
    unawaited(_networkSub?.cancel());
    _networkSub = null;
    _emit(const LinkState(status: LinkStatus.idle));
  }

  Future<void> dispose() async {
    await disconnect();
    await _states.close();
    await _events.close();
    await _pty.close();
    await _sessions.close();
  }

  void goBackground() {
    if (_suspended) return;
    _backgroundTimer?.cancel();
    _backgroundTimer = Timer(deps.config.backgroundGrace, () {
      _suspended = true;
      _generation++;
      _wake();
      unawaited(_dropSession());
      _emit(
        const LinkState(status: LinkStatus.idle, reason: LinkReason.background),
      );
    });
  }

  Future<void> goForeground() async {
    _backgroundTimer?.cancel();
    if (_suspended) {
      await connect();
      return;
    }
    if (_state.isTerminal || _state.status == LinkStatus.idle) return;
    final session = _session;
    if (session == null || session.isClosed) {
      _skipDelay = true;
      _failures = 0;
      _wake();
      return;
    }
    await refresh();
  }

  Future<void> refresh() async {
    final session = _session;
    if (session == null) return;
    try {
      await session.request('resync');
      await session.request('ws.get');
    } on RemoteCommandException catch (error) {
      if (error.code == 'timeout' || error.code == 'offline') {
        _skipDelay = true;
        await _dropSession();
      }
    }
  }

  void _watchNetwork() {
    _networkSub ??= deps.network.changes.listen(_onNetwork);
  }

  void _onNetwork(String signature) {
    final previous = _lastNetwork;
    _lastNetwork = signature;
    if (previous == null || previous == signature) return;
    if (_suspended || _state.isTerminal || _state.status == LinkStatus.idle) {
      return;
    }
    if (signature == 'none') {
      _hint = LinkReason.networkLost;
      if (_session != null) unawaited(_dropSession());
      _emit(
        _state.copyWith(
          status: LinkStatus.reconnecting,
          reason: LinkReason.networkLost,
        ),
      );
      return;
    }
    _skipDelay = true;
    _failures = 0;
    if (_session != null) {
      unawaited(_dropSession());
    } else {
      _wake();
    }
  }

  void _wake() {
    _backoffTimer?.cancel();
    final wait = _backoffWait;
    _backoffWait = null;
    if (wait != null && !wait.isCompleted) wait.complete();
  }

  Future<void> _delay(Duration duration) {
    final completer = Completer<void>();
    _backoffWait = completer;
    _backoffTimer = Timer(duration, () {
      if (!completer.isCompleted) completer.complete();
    });
    return completer.future;
  }

  Future<void> _dropSession() async {
    _keepaliveTimer?.cancel();
    _keepaliveTimer = null;
    final session = _session;
    _session = null;
    final eventSub = _eventSub;
    final ptySub = _ptySub;
    _eventSub = null;
    _ptySub = null;
    unawaited(eventSub?.cancel());
    unawaited(ptySub?.cancel());
    if (session != null && !session.isClosed) {
      await session.close();
    }
  }

  Future<void> _run(int generation) async {
    while (generation == _generation) {
      final attempt = await _attempt(generation);
      if (generation != _generation) {
        await attempt.session?.close();
        return;
      }
      final session = attempt.session;
      if (session != null) {
        _failures = 0;
        final code = await session.closed;
        if (generation != _generation) return;
        _session = null;
        unawaited(_eventSub?.cancel());
        unawaited(_ptySub?.cancel());
        _eventSub = null;
        _ptySub = null;
        _keepaliveTimer?.cancel();
        final next = _classify(code, session.viaRelay);
        if (next.isTerminal) {
          _emit(next);
          return;
        }
        _failures = 1;
        _emit(next.copyWith(attempt: _failures));
      } else {
        final next = attempt.state!;
        if (next.isTerminal) {
          _emit(next);
          return;
        }
        _failures += 1;
        _emit(next.copyWith(attempt: _failures));
      }
      var delay = deps.config.delayFor(_failures);
      if (_state.reason == LinkReason.rateLimited) {
        delay = deps.config.backoff.last;
      }
      if (_skipDelay) {
        _skipDelay = false;
        delay = Duration.zero;
      }
      if (delay > Duration.zero) {
        _emit(_state.copyWith(retryIn: delay));
        await _delay(delay);
        if (generation != _generation) return;
      }
      _emit(_state.copyWith(status: LinkStatus.discovering, clearRetry: true));
    }
  }

  LinkState _classify(int? code, bool viaRelay) {
    final hint = _hint;
    _hint = LinkReason.none;
    return classifyClose(
      code: code,
      viaRelay: viaRelay,
      hint: hint,
      proto: deps.config.proto,
    );
  }

  Future<ConnectAttempt> _attempt(int generation) async {
    final machine = deps.machines.get(machineId);
    if (machine == null) {
      return const ConnectAttempt.failed(
        LinkState(status: LinkStatus.offline, reason: LinkReason.noRoute),
      );
    }
    if (machine.revoked) {
      return const ConnectAttempt.failed(LinkState(status: LinkStatus.revoked));
    }
    final identity = await deps.keys.load();
    if (generation != _generation) {
      return const ConnectAttempt.failed(LinkState());
    }
    _emit(_state.copyWith(status: LinkStatus.discovering, clearRoute: true));
    final lan = await openLan(deps, machine);
    if (generation != _generation) {
      await lan?.socket.close();
      return const ConnectAttempt.failed(LinkState());
    }
    var opened = lan;
    if (opened == null) {
      final relay = await openRelay(
        deps,
        machine,
        clock.now().millisecondsSinceEpoch ~/ 1000,
      );
      final failure = relay.failure;
      if (failure != null) return ConnectAttempt.failed(failure);
      opened = relay.opened;
      if (generation != _generation) {
        await opened!.socket.close();
        return const ConnectAttempt.failed(LinkState());
      }
    }
    return _handshake(generation, machine, identity, opened!);
  }

  Future<ConnectAttempt> _handshake(
    int generation,
    Machine machine,
    PhoneIdentity identity,
    Opened opened,
  ) async {
    _emit(_state.copyWith(status: LinkStatus.handshaking, route: opened.route));
    final outcome = await runSessionHandshake(
      deps: deps,
      machine: machine,
      identity: identity,
      opened: opened,
      classify: _classify,
    );
    final failure = outcome.failure;
    if (failure != null) return ConnectAttempt.failed(failure);
    if (generation != _generation) {
      await opened.socket.close();
      return const ConnectAttempt.failed(LinkState());
    }
    return _bringUp(generation, machine, opened, outcome.inbox!, outcome.link!);
  }

  Future<ConnectAttempt> _bringUp(
    int generation,
    Machine machine,
    Opened opened,
    Inbox inbox,
    SecureLink link,
  ) async {
    final session = RemoteSession(
      socket: opened.socket,
      inbox: inbox,
      link: link,
      viaRelay: opened.route == LinkRoute.relay,
      clock: () => clock.now(),
    );
    _session = session;
    _eventSub = session.events.listen(_onSessionEvent);
    _ptySub = session.ptyOutput.listen(_pty.add);
    session.start();
    final JsonMap reply;
    try {
      reply = await session.request('hello', {
        'proto': deps.config.proto,
        'app': deps.config.appName,
        'platform': deps.config.platform,
        'caps': <String>[],
      });
    } on RemoteCommandException catch (error) {
      if (error.code == 'offline') {
        final code = await session.closed.timeout(
          const Duration(milliseconds: 500),
          onTimeout: () => null,
        );
        await _dropSession();
        return ConnectAttempt.failed(
          _classify(code, opened.route == LinkRoute.relay),
        );
      }
      await _dropSession();
      return ConnectAttempt.failed(
        helloFailure(error.code, opened.route, deps.config.proto),
      );
    }
    if (generation != _generation) {
      await session.close();
      return const ConnectAttempt.failed(LinkState());
    }
    await rememberRoute(deps.machines, machine, opened, reply);
    _events.add({...reply, 't': 'hello'});
    _emit(LinkState(status: LinkStatus.ready, route: opened.route));
    _startKeepalive(session);
    if (!_sessions.isClosed) _sessions.add(session);
    return ConnectAttempt.ready(session);
  }

  void _startKeepalive(RemoteSession session) {
    _keepaliveTimer?.cancel();
    _keepaliveTimer = Timer.periodic(deps.config.keepalive, (_) {
      if (session.isClosed) return;
      final idle = clock.now().difference(session.lastActivity);
      if (idle > deps.config.deadAfter) {
        _hint = LinkReason.networkLost;
        unawaited(session.close());
        return;
      }
      session.sendKeepalive();
    });
  }

  void _onSessionEvent(JsonMap event) {
    switch (event['t']) {
      case 'device.revoked':
        unawaited(_onRevoked());
      case 'remote.off':
        _hint = LinkReason.remoteOff;
      case 'machine.restarting':
        _hint = LinkReason.restarting;
      case 'relay.token':
        final token = event['token'];
        if (token is String) {
          unawaited(storeRelayToken(deps.machines, machineId, token));
        }
    }
    if (!_events.isClosed) _events.add(event);
  }

  Future<void> _onRevoked() async {
    final machine = deps.machines.get(machineId);
    if (machine != null) {
      await deps.machines.save(machine.copyWith(revoked: true));
    }
    _generation++;
    _wake();
    await _dropSession();
    _emit(const LinkState(status: LinkStatus.revoked));
  }
}
