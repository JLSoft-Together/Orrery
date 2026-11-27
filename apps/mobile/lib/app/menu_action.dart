import 'dart:async';

import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../remote/chat_protocol.dart';
import '../remote/protocol.dart';
import '../ui/commands.dart';
import '../ui/model/workspace_model.dart';

const Duration menuActionUndo = Duration(seconds: 3);

typedef MenuSend = Future<void> Function(String sig, int index);

RemoteMenuItem? menuItemFor(PaneInfo? pane, String sig, int index) {
  final menu = pane?.menu;
  if (menu == null || menu.sig != sig) return null;
  for (final item in menu.items) {
    if (item.index == index &&
        !item.other &&
        !item.submit &&
        item.checked == null) {
      return item;
    }
  }
  return null;
}

bool isStaleMenuError(Object error) =>
    error is RemoteCommandException &&
    (error.code == 'stale' || error.code == 'busy');

Future<void> runMenuAction({
  required BuildContext context,
  required PaneInfo? pane,
  required String sig,
  required int index,
  required MenuSend send,
  Duration undo = menuActionUndo,
  bool Function()? proceed,
}) async {
  final l = AppLocalizations.of(context);
  final messenger = ScaffoldMessenger.maybeOf(context);
  final item = menuItemFor(pane, sig, index);
  if (item == null) {
    toast(context, l.pushMenuStale);
    return;
  }
  if (messenger == null) return;
  final controller = messenger.showSnackBar(
    SnackBar(
      content: Text(l.chatPicked(item.label)),
      duration: undo,
      persist: false,
      action: SnackBarAction(label: l.chatUndo, onPressed: () {}),
    ),
  );
  final reason = await controller.closed;
  if (reason == SnackBarClosedReason.action) return;
  if (proceed != null && !proceed()) return;
  try {
    await send(sig, index);
  } on Object catch (error) {
    messenger
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(
            isStaleMenuError(error) ? l.pushMenuStale : errorText(l, error),
          ),
        ),
      );
  }
}
