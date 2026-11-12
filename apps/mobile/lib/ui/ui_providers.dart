import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../remote/providers.dart';
import 'machine_api.dart';
import 'model/pane_facts.dart';
import 'model/workspace_model.dart';
import 'monitor/view_prefs.dart';
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
  ref.watch(
    machineViewProvider(id)
        .select((view) => sessionAliasKey(sessionAliases(view.snapshot))),
  );
  final snapshot = ref.read(machineViewProvider(id)).snapshot;
  return WorkspaceModel.parse(workspace).withAliases(sessionAliases(snapshot));
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
  final pick = ref.watch(viewPrefsProvider.select((p) => p.theme));
  final hello = id == null
      ? null
      : ref.watch(machineViewProvider(id).select((v) => v.hello));
  final remote = RemoteTheme.fromHello(hello?['theme']);
  final mode = switch (pick) {
    'system' => ThemeMode.system,
    'light' => ThemeMode.light,
    'dark' => ThemeMode.dark,
    _ => remote.mode,
  };
  return RemoteTheme(mode, remote.palette);
});

final remoteLocaleProvider = Provider<Locale?>((ref) {
  final pick = ref.watch(viewPrefsProvider.select((p) => p.language));
  if (pick != 'auto') return Locale(pick);
  final id = ref.watch(activeMachineProvider);
  if (id == null) return null;
  final hello = ref.watch(machineViewProvider(id).select((v) => v.hello));
  final value = hello?['locale'];
  if (value is! String || value.isEmpty) return null;
  final code = value.split(RegExp(r'[-_]')).first.toLowerCase();
  return code == 'vi' || code == 'en' ? Locale(code) : null;
});
