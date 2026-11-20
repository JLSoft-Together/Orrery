import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/chat_last.dart';
import '../../remote/providers.dart';
import '../commands.dart';
import '../model/workspace_model.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import '../workspace/workspace_page.dart' show openPane;
import 'chat_screen.dart';

bool chatFor(WidgetRef ref, String machineId, PaneInfo pane) =>
    pane.isAgent && ref.read(machineViewProvider(machineId)).chatEnabled;

void openSession(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  TabInfo tab,
  PaneInfo pane,
) {
  if (chatFor(ref, machineId, pane)) {
    Navigator.of(context).push(
      machineRoute<void>(
        machineId,
        (_) => ChatScreen(machineId: machineId, paneKey: pane.key),
      ),
    );
    return;
  }
  openPane(context, machineId, tab, pane.key);
}

Future<bool> openNewSession(
  BuildContext context,
  WidgetRef ref,
  String machineId, {
  required String preset,
  required String cwd,
  Duration wait = const Duration(seconds: 8),
}) async {
  final reply = await runCommand(context, ref, machineId, 'ws.open', {
    'cwd': cwd,
    'preset': preset,
  });
  final key = reply?['paneKey'];
  if (key is! String || !context.mounted) return false;
  final found = Completer<(TabInfo, PaneInfo)?>();
  void check(WorkspaceModel model) {
    final tab = model.tabOfPane(key);
    final pane = tab?.pane(key);
    if (tab != null && pane != null && !found.isCompleted) {
      found.complete((tab, pane));
    }
  }

  final sub = ref.listenManual(
    workspaceModelProvider(machineId),
    (_, next) => check(next),
    fireImmediately: true,
  );
  final hit = await found.future.timeout(wait, onTimeout: () => null);
  sub.close();
  if (hit != null && context.mounted) {
    openSession(context, ref, machineId, hit.$1, hit.$2);
    return true;
  }
  if (context.mounted) toast(context, context.l10n.paneOpened);
  return false;
}
