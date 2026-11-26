import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../../remote/clip_sync.dart';
import '../../remote/providers.dart';
import '../commands.dart';
import '../kit/kit.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';

typedef ClipReader = Future<String?> Function();
typedef ClipWriter = Future<void> Function(String text);

Future<String?> readSystemClip() async =>
    (await Clipboard.getData(Clipboard.kTextPlain))?.text;

Future<void> writeSystemClip(String text) =>
    Clipboard.setData(ClipboardData(text: text));

String clipResultText(AppLocalizations l, ClipPushResult result) =>
    switch (result) {
      ClipPushResult.sent => l.clipSent,
      ClipPushResult.empty => l.clipEmpty,
      ClipPushResult.tooLarge => l.clipTooLarge,
      ClipPushResult.secret => l.clipSecret,
      ClipPushResult.denied => l.clipDenied,
      ClipPushResult.offline => l.errOffline,
      ClipPushResult.failed => l.clipFailed,
    };

class ClipTile extends ConsumerWidget {
  const ClipTile({
    super.key,
    required this.machineId,
    this.read = readSystemClip,
  });

  final String machineId;
  final ClipReader read;

  Future<void> _send(BuildContext context, WidgetRef ref) async {
    final api = ref.read(machineApiProvider(machineId));
    final l = context.l10n;
    final text = await read();
    final result = await pushClip(api.command, text);
    if (!context.mounted) return;
    toast(context, clipResultText(l, result));
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final available = ref.watch(
      machineViewProvider(machineId).select((v) => v.hasCap(clipCap)),
    );
    if (!available) return const SizedBox.shrink();
    final c = context.orrery;
    return OrreryCard(
      key: const ValueKey('clip-send'),
      onTap: () => unawaited(_send(context, ref)),
      child: Row(
        children: [
          Icon(Icons.content_paste_go_rounded, size: 20, color: c.accentInk),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l.clipSendTitle,
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                Text(
                  l.clipSendHint,
                  style: TextStyle(fontSize: 12, color: c.inkMuted),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class ClipListener extends ConsumerStatefulWidget {
  const ClipListener({
    super.key,
    required this.machineId,
    required this.child,
    this.write = writeSystemClip,
  });

  final String machineId;
  final Widget child;
  final ClipWriter write;

  @override
  ConsumerState<ClipListener> createState() => _ClipListenerState();
}

class _ClipListenerState extends ConsumerState<ClipListener> {
  StreamSubscription<dynamic>? _sub;

  @override
  void initState() {
    super.initState();
    _sub = ref.read(machineApiProvider(widget.machineId)).events.listen((e) {
      final text = clipEventText(e);
      if (text != null) unawaited(_receive(text));
    });
  }

  Future<void> _receive(String text) async {
    try {
      await widget.write(text);
    } on Object {
      return;
    }
    if (!mounted) return;
    toast(context, context.l10n.clipReceived);
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
