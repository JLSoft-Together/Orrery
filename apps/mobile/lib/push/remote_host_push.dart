import 'dart:async';

import '../remote/connection.dart';
import '../remote/protocol.dart';
import '../remote/remote_host.dart';
import 'push_config.dart';
import 'push_service.dart';

class _ConnectionSession implements PushSession {
  _ConnectionSession(this.machineId, this.session);

  @override
  final String machineId;
  final RemoteSession session;

  @override
  Future<void> send(String type, Map<String, Object?> fields) =>
      session.request(type, fields);
}

class RemoteHostPush implements PushHost {
  RemoteHostPush(this.host) {
    for (final connection in host.connections) {
      _watch(connection);
    }
    host.connectionCreated.listen(_watch);
  }

  final RemoteHost host;
  final StreamController<PushSession> _ready = StreamController.broadcast();
  final StreamController<PushConfigUpdate> _configs =
      StreamController.broadcast();

  void _watch(MachineConnection connection) {
    connection.readySessions.listen((session) {
      if (!_ready.isClosed) {
        _ready.add(_ConnectionSession(connection.machineId, session));
      }
    });
    connection.events.listen((event) {
      if (event['t'] != 'push.config' || _configs.isClosed) return;
      _configs.add(
        PushConfigUpdate(connection.machineId, PushConfig.tryParse(event)),
      );
    });
  }

  @override
  Stream<PushSession> get readySessions => _ready.stream;

  @override
  Stream<PushConfigUpdate> get configs => _configs.stream;

  @override
  List<PushSession> get readyNow => [
    for (final connection in host.connections)
      if (connection.isReady && connection.session != null)
        _ConnectionSession(connection.machineId, connection.session!),
  ];
}
