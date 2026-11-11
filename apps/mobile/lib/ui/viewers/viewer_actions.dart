import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../../remote/providers.dart';
import '../commands.dart';
import '../widgets/common.dart';
import 'async_panel.dart';
import 'viewer_api.dart';

void reloadAfterConfirm(
  WidgetRef ref,
  String machineId,
  bool Function() alive,
  VoidCallback reload,
) {
  ProviderSubscription<int>? sub;
  void close() {
    sub?.close();
  }

  sub = ref.listenManual<int>(
    machineViewProvider(machineId).select((v) => v.confirms.length),
    (previous, next) {
      if (next != 0) return;
      close();
      Timer(const Duration(milliseconds: 800), () {
        if (alive()) reload();
      });
    },
  );
}

Future<ViewerReply?> viewerAction(
  BuildContext context,
  WidgetRef ref,
  String machineId,
  String type,
  JsonMap fields, {
  Duration? timeout,
  String? success,
  VoidCallback? after,
}) async {
  final l = context.l10n;
  final api = ref.read(viewerApiProvider(machineId));
  try {
    final reply = await api.call(type, fields, timeout);
    if (!context.mounted) return reply;
    if (reply.cancelled) {
      toast(context, l.vCancelled);
    } else if (reply.pending) {
      toast(context, l.vWaitConfirm);
      if (after != null) {
        reloadAfterConfirm(ref, machineId, () => context.mounted, after);
      }
    } else {
      if (success != null) toast(context, success);
      after?.call();
    }
    return reply;
  } on Object catch (error) {
    if (context.mounted) toast(context, viewerErrorText(l, error));
    return null;
  }
}
