import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../l10n/app_localizations.dart';
import '../remote/protocol.dart';
import 'ui_providers.dart';
import 'widgets/common.dart';

String errorText(AppLocalizations l, Object error) {
  final code = error is RemoteCommandException ? error.code : 'internal';
  return switch (code) {
    'denied' => l.errDenied,
    'busy' => l.errBusy,
    'limit' => l.errLimit,
    'notFound' => l.errNotFound,
    'invalid' => l.errInvalid,
    'timeout' => l.errTimeout,
    'offline' => l.errOffline,
    'rate' => l.errRate,
    'proto' => l.errProto,
    _ => l.errInternal,
  };
}

void toast(BuildContext context, String message) {
  final messenger = ScaffoldMessenger.maybeOf(context);
  messenger
    ?..hideCurrentSnackBar()
    ..showSnackBar(
      SnackBar(content: Text(message), duration: const Duration(seconds: 4)),
    );
}

Future<JsonMap?> runCommand(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  String type, [
  JsonMap? fields,
  String? successText,
]) async {
  final api = ref.read(machineApiProvider(machineId));
  final l = context.l10n;
  final messenger = ScaffoldMessenger.maybeOf(context);
  try {
    final reply = await api.command(type, fields);
    if (successText != null && messenger != null) {
      messenger
        ..hideCurrentSnackBar()
        ..showSnackBar(SnackBar(content: Text(successText)));
    }
    return reply;
  } on Object catch (error) {
    messenger
      ?..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(errorText(l, error))));
    return null;
  }
}
