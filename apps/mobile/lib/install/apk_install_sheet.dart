import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import 'apk_flow.dart';
import 'apk_offer.dart';

String apkErrorText(AppLocalizations l, ApkError error) => switch (error) {
  ApkError.expired => l.apkErrExpired,
  ApkError.lan => l.apkErrLan,
  ApkError.interrupted => l.apkErrInterrupted,
  ApkError.space => l.apkErrSpace,
  ApkError.size => l.apkErrSize,
  ApkError.sha => l.apkErrSha,
  ApkError.permission => l.apkNeedPermission,
  ApkError.installFailed => l.apkErrInstall,
  ApkError.invalid => l.apkErrInvalid,
};

String apkSizeText(int bytes) => '${(bytes / 1048576).toStringAsFixed(1)} MB';

class ApkInstallSheet extends StatefulWidget {
  const ApkInstallSheet({
    super.key,
    required this.flow,
    required this.autoStart,
    required this.openSettings,
  });

  final ApkInstallFlow flow;
  final bool autoStart;
  final Future<void> Function() openSettings;

  @override
  State<ApkInstallSheet> createState() => _ApkInstallSheetState();
}

class _ApkInstallSheetState extends State<ApkInstallSheet>
    with WidgetsBindingObserver {
  bool _inSettings = false;

  ApkInstallFlow get _flow => widget.flow;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _flow.addListener(_changed);
    if (widget.autoStart) _flow.start();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _flow.removeListener(_changed);
    _flow.cancel();
    super.dispose();
  }

  void _changed() {
    if (mounted) setState(() {});
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed || !_inSettings) return;
    _inSettings = false;
    if (_flow.state.phase == ApkPhase.needPermission) _flow.start();
  }

  Future<void> _settings() async {
    _inSettings = true;
    await widget.openSettings();
  }

  String _status(AppLocalizations l, ApkInstallState s) {
    switch (s.phase) {
      case ApkPhase.idle:
        return l.apkReady;
      case ApkPhase.downloading:
        final f = s.fraction;
        return f == null
            ? l.apkDownloading
            : l.apkDownloadingPercent((f * 100).floor());
      case ApkPhase.verifying:
        return l.apkVerifying;
      case ApkPhase.installing:
        return l.apkInstalling;
      case ApkPhase.started:
        return l.apkStarted;
      case ApkPhase.needPermission:
        return l.apkNeedPermission;
      case ApkPhase.failed:
        return apkErrorText(l, s.error ?? ApkError.interrupted);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final offer = _flow.offer;
    final s = _flow.state;
    final failed = s.phase == ApkPhase.failed;
    final name = [
      if (offer.app.isNotEmpty) offer.app,
      if (offer.variant.isNotEmpty) offer.variant,
    ].join(', ');
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 4, 20, 20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              l.apkTitle,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 4),
            Text(
              [if (name.isNotEmpty) name, apkSizeText(offer.size)].join(', '),
              style: TextStyle(color: scheme.onSurfaceVariant),
            ),
            const SizedBox(height: 16),
            if (s.busy)
              Padding(
                padding: const EdgeInsets.only(bottom: 12),
                child: LinearProgressIndicator(
                  key: const ValueKey('apk-progress'),
                  value: s.phase == ApkPhase.downloading ? s.fraction : null,
                ),
              ),
            Text(
              _status(l, s),
              key: const ValueKey('apk-status'),
              style: TextStyle(color: failed ? scheme.error : null),
            ),
            const SizedBox(height: 16),
            if (s.phase == ApkPhase.idle || failed)
              FilledButton(
                key: const ValueKey('apk-start'),
                onPressed: _flow.start,
                child: Text(failed ? l.apkRetry : l.apkStart),
              ),
            if (s.phase == ApkPhase.needPermission) ...[
              FilledButton(
                key: const ValueKey('apk-settings'),
                onPressed: _settings,
                child: Text(l.apkOpenSettings),
              ),
              TextButton(
                key: const ValueKey('apk-retry'),
                onPressed: _flow.start,
                child: Text(l.apkRetry),
              ),
            ],
            if (s.phase == ApkPhase.downloading)
              TextButton(
                key: const ValueKey('apk-cancel'),
                onPressed: _flow.cancel,
                child: Text(l.apkCancel),
              ),
            if (s.phase == ApkPhase.started)
              TextButton(
                key: const ValueKey('apk-close'),
                onPressed: () => Navigator.of(context).maybePop(),
                child: Text(l.apkClose),
              ),
          ],
        ),
      ),
    );
  }
}
