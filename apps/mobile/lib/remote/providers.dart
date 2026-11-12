import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'link_state.dart';
import 'machines.dart';
import 'remote_host.dart';
import 'store.dart';

final remoteHostProvider = Provider<RemoteHost>(
  (ref) => throw UnimplementedError('remoteHostProvider must be overridden'),
);

class MachinesNotifier extends Notifier<List<Machine>> {
  @override
  List<Machine> build() => ref.read(remoteHostProvider).machines.all;

  void refresh() => state = ref.read(remoteHostProvider).machines.all;

  Future<Machine> pair(String qrText) async {
    final machine = await ref.read(remoteHostProvider).pairWithQr(qrText);
    refresh();
    return machine;
  }

  Future<void> rename(String machineId, String? alias) async {
    await ref.read(remoteHostProvider).machines.rename(machineId, alias);
    refresh();
  }

  Future<void> forget(String machineId) async {
    await ref.read(remoteHostProvider).forget(machineId);
    refresh();
  }
}

final machinesProvider = NotifierProvider<MachinesNotifier, List<Machine>>(
  MachinesNotifier.new,
);

class LinkNotifier extends Notifier<LinkState> {
  LinkNotifier(this.machineId);

  final String machineId;

  @override
  LinkState build() {
    final connection = ref
        .read(remoteHostProvider)
        .bundle(machineId)
        .connection;
    final sub = connection.states.listen((next) => state = next);
    ref.onDispose(sub.cancel);
    return connection.state;
  }
}

final linkProvider = NotifierProvider.family<LinkNotifier, LinkState, String>(
  LinkNotifier.new,
);

class MachineViewNotifier extends Notifier<MachineView> {
  MachineViewNotifier(this.machineId);

  final String machineId;

  @override
  MachineView build() {
    final store = ref.read(remoteHostProvider).bundle(machineId).store;
    final sub = store.changes.listen((next) => state = next);
    ref.onDispose(sub.cancel);
    return store.view;
  }
}

final machineViewProvider =
    NotifierProvider.family<MachineViewNotifier, MachineView, String>(
      MachineViewNotifier.new,
    );
