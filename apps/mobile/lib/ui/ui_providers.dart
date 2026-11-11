import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../remote/providers.dart';
import 'machine_api.dart';
import 'model/pane_facts.dart';
import 'model/workspace_model.dart';
import 'theme/orrery_theme.dart';

final machineApiProvider = Provider.family<MachineApi, String>(
  (ref, id) => BundleMachineApi(ref.read(remoteHostProvider), id),
);

final workspaceModelProvider = Provider.family<WorkspaceModel, String>((
  ref,
  id,
) {
  final workspace = ref.watch(
    machineViewProvider(id).select((view) => view.workspace),
  );
  return WorkspaceModel.parse(workspace);
});

final paneFactsProvider = Provider.family<PaneFacts, (String, String?)>((
  ref,
  args,
) {
  final snapshot = ref.watch(
    machineViewProvider(args.$1).select((view) => view.snapshot),
  );
  return PaneFacts.of(snapshot, args.$2);
});

class ActiveMachine extends Notifier<String?> {
  @override
  String? build() {
    final machines = ref.read(machinesProvider);
    return machines.isEmpty ? null : machines.first.id;
  }

  void select(String? id) => state = id;
}

final activeMachineProvider = NotifierProvider<ActiveMachine, String?>(
  ActiveMachine.new,
);

final remoteThemeProvider = Provider<RemoteTheme>((ref) {
  final id = ref.watch(activeMachineProvider);
  if (id == null) return RemoteTheme.fallback;
  final hello = ref.watch(machineViewProvider(id).select((v) => v.hello));
  return RemoteTheme.fromHello(hello?['theme']);
});

final remoteLocaleProvider = Provider<Locale?>((ref) {
  final id = ref.watch(activeMachineProvider);
  if (id == null) return null;
  final hello = ref.watch(machineViewProvider(id).select((v) => v.hello));
  final value = hello?['locale'];
  if (value is! String || value.isEmpty) return null;
  final code = value.split(RegExp(r'[-_]')).first.toLowerCase();
  return code == 'vi' || code == 'en' ? Locale(code) : null;
});
