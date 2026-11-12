import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../commands.dart';
import '../widgets/common.dart';
import '../widgets/name_dialog.dart';

const int maxSessionAlias = 80;

Future<void> renameSession(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  String terminalId,
  String? alias,
) async {
  final l = context.l10n;
  final name = await showNameDialog(
    context,
    title: l.renameSession,
    initial: alias ?? '',
    maxLength: maxSessionAlias,
    hint: l.renameSessionHint,
  );
  if (name == null || !context.mounted) return;
  await runCommand(context, ref, machineId, 'terminal.alias', {
    'terminalId': terminalId,
    'alias': name.isEmpty ? null : name,
  });
}
