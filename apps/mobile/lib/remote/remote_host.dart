import 'dart:io';

import 'cache.dart';
import 'connection.dart';
import 'machines.dart';
import 'pairing.dart';
import 'pty_stream.dart';
import 'store.dart';

class MachineBundle {
  MachineBundle(this.connection, this.store, this.pty);

  final MachineConnection connection;
  final MachineStore store;
  final PtyStreams pty;
}

class RemoteHost {
  RemoteHost({
    required this.deps,
    required this.cacheDirectory,
    required this.deviceName,
  }) : cache = MachineCache(cacheDirectory);

  final ConnectionDeps deps;
  final Directory cacheDirectory;
  final String deviceName;
  final MachineCache cache;
  final Map<String, MachineBundle> _bundles = {};
  bool _foreground = true;

  MachineRepository get machines => deps.machines;

  Future<List<Machine>> init() => machines.load();

  MachineBundle bundle(String machineId) {
    final existing = _bundles[machineId];
    if (existing != null) return existing;
    final connection = MachineConnection(machineId: machineId, deps: deps);
    final pty = PtyStreams(connection);
    final store = MachineStore(
      machineId: machineId,
      commander: (type, [fields]) {
        final session = connection.session;
        if (session == null) {
          return Future.error(StateError('offline'));
        }
        return session.request(type, fields);
      },
      cache: cache,
      tailsSource: () => pty.tails,
    );
    connection.events.listen(store.handleEvent);
    connection.states.listen(store.onLink);
    final created = MachineBundle(connection, store, pty);
    _bundles[machineId] = created;
    return created;
  }

  Future<void> start(String machineId) async {
    final entry = bundle(machineId);
    await entry.store.restoreFromCache();
    if (_foreground) await entry.connection.connect();
  }

  Future<void> startAll() async {
    for (final machine in machines.all) {
      if (!machine.revoked) await start(machine.id);
    }
  }

  Future<Machine> pairWithQr(String qrText) async {
    final pairing = Pairing(
      crypto: deps.crypto,
      connector: deps.connector,
      keys: deps.keys,
      machines: machines,
      deviceName: deviceName,
      discovery: deps.discovery,
      config: deps.config,
    );
    final machine = await pairing.pair(qrText);
    await start(machine.id);
    return machine;
  }

  Future<void> forget(String machineId) async {
    final entry = _bundles.remove(machineId);
    if (entry != null) {
      await entry.pty.dispose();
      await entry.connection.dispose();
      await entry.store.dispose();
    }
    await machines.remove(machineId);
    await cache.delete(machineId);
  }

  Future<void> setForeground(bool foreground) async {
    if (_foreground == foreground) return;
    _foreground = foreground;
    for (final entry in _bundles.values) {
      if (foreground) {
        await entry.connection.goForeground();
        await entry.pty.resumeAll();
      } else {
        await entry.pty.suspendAll();
        entry.connection.goBackground();
      }
    }
  }

  Future<void> dispose() async {
    for (final entry in _bundles.values) {
      await entry.pty.dispose();
      await entry.connection.dispose();
      await entry.store.dispose();
    }
    _bundles.clear();
  }
}
