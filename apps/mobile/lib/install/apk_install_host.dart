import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import '../push/push_apk.dart';
import '../security/app_lock.dart';
import '../push/push_service.dart';
import 'apk_flow.dart';
import 'apk_install_sheet.dart';
import 'apk_io.dart';

const String apkDirName = 'apk_offer';

Future<Directory> apkDirectory() async =>
    Directory('${(await getTemporaryDirectory()).path}/$apkDirName');

Future<void> showApkInstall(
  BuildContext context,
  ApkTap tap, {
  ApkSource? source,
  ApkInstaller? installer,
  Directory? dir,
}) async {
  final target = dir ?? await apkDirectory();
  if (!context.mounted) return;
  final real = installer ?? const ChannelApkInstaller();
  final flow = ApkInstallFlow(
    offer: tap.offer,
    source: source ?? HttpApkSource(),
    installer: real,
    dir: target,
  );
  await showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    showDragHandle: true,
    builder: (_) => ApkInstallSheet(
      flow: flow,
      autoStart: tap.auto,
      openSettings: real.openSettings,
    ),
  );
  flow.dispose();
}

class ApkInstallHost extends StatefulWidget {
  const ApkInstallHost({
    super.key,
    required this.push,
    required this.navigatorKey,
    required this.child,
    this.lock,
  });

  final AppLock? lock;
  final PushService? push;
  final GlobalKey<NavigatorState> navigatorKey;
  final Widget child;

  @override
  State<ApkInstallHost> createState() => _ApkInstallHostState();
}

class _ApkInstallHostState extends State<ApkInstallHost> {
  StreamSubscription<ApkTap>? _sub;
  bool _open = false;
  ApkTap? _held;

  @override
  void initState() {
    super.initState();
    widget.lock?.addListener(_onLock);
    final push = widget.push;
    if (push == null) return;
    _sub = push.apkTaps.listen(_show);
    final first = push.takeInitialApk();
    if (first != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _show(first));
    }
    unawaited(_sweep());
  }

  @override
  void dispose() {
    widget.lock?.removeListener(_onLock);
    _sub?.cancel();
    super.dispose();
  }

  Future<void> _sweep() async {
    try {
      await cleanApkDir(
        await apkDirectory(),
        olderThan: const Duration(hours: 1),
      );
    } on Object {
      return;
    }
  }

  bool get _shut {
    final lock = widget.lock;
    return lock != null && lock.enabled && (lock.locked || !lock.loaded);
  }

  void _onLock() {
    final held = _held;
    if (held == null || _shut) return;
    _held = null;
    unawaited(_show(held));
  }

  Future<void> _show(ApkTap tap) async {
    if (_shut) {
      _held = tap;
      return;
    }
    final context = widget.navigatorKey.currentState?.overlay?.context;
    if (context == null || !context.mounted || _open) return;
    _open = true;
    try {
      await showApkInstall(context, tap);
    } finally {
      _open = false;
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
