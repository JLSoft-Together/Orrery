import 'package:flutter/material.dart';

import '../ui/commands.dart';
import '../ui/kit/kit.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'app_lock.dart';
import 'device_auth.dart';

class LockControls extends StatelessWidget {
  const LockControls({super.key, required this.lock});

  final AppLock lock;

  Future<void> _change(
    BuildContext context, {
    bool? enabled,
    LockTimeout? timeout,
  }) async {
    final l = context.l10n;
    final outcome = await lock.confirm();
    if (!context.mounted) return;
    if (outcome == AuthOutcome.notAvailable && enabled == false) {
      final ok = await showOrreryDialog<bool>(
        context,
        builder: (context) => OrreryDialog(
          title: l.lockTurnOffTitle,
          body: '${l.lockNoCredentials}\n\n${l.lockTurnOffWarn}',
          actions: [
            OrreryButton(
              label: l.cancel,
              kind: ButtonKind.text,
              expand: false,
              onPressed: () => Navigator.pop(context, false),
            ),
            OrreryButton(
              key: const ValueKey('lock-turn-off-confirm'),
              label: l.lockTurnOff,
              kind: ButtonKind.danger,
              expand: false,
              onPressed: () => Navigator.pop(context, true),
            ),
          ],
        ),
      );
      if (ok != true) return;
      final done = await lock.turnOffWithoutAuth();
      if (!context.mounted) return;
      if (!done) toast(context, l.lockFailed);
      return;
    }
    if (!context.mounted) return;
    switch (outcome) {
      case AuthOutcome.success:
        final saved = await lock.apply(enabled: enabled, timeout: timeout);
        if (!saved && context.mounted) toast(context, l.lockSaveFailed);
      case AuthOutcome.notAvailable:
        toast(context, l.lockNoCredentials);
      case AuthOutcome.cancelled:
        toast(context, l.lockCancelled);
      case AuthOutcome.failed:
        toast(context, l.lockFailed);
    }
  }

  String _short(BuildContext context, LockTimeout value) {
    final l = context.l10n;
    return switch (value) {
      LockTimeout.everyOpen => l.lockShortEvery,
      LockTimeout.minutes15 => l.lockShortMinutes15,
      LockTimeout.hour1 => l.lockShortHour1,
      LockTimeout.hours4 => l.lockShortHours4,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return ListenableBuilder(
      listenable: lock,
      builder: (context, _) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          OrreryCard(
            padding: EdgeInsets.zero,
            child: Column(
              children: [
                SwitchListTile(
                  key: const ValueKey('lock-switch'),
                  secondary: const Icon(Icons.lock_outline_rounded),
                  title: Text(l.lockSwitch),
                  subtitle: Text(l.lockSwitchHint),
                  value: lock.enabled,
                  onChanged: lock.busy
                      ? null
                      : (value) => _change(context, enabled: value),
                ),
                Divider(height: 1, color: context.orrery.line),
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 12, 16, 14),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(
                            Icons.schedule_rounded,
                            size: 20,
                            color: context.orrery.accentInk,
                          ),
                          const SizedBox(width: 12),
                          Text(
                            l.lockAfter,
                            style: Theme.of(context).textTheme.titleSmall,
                          ),
                        ],
                      ),
                      const SizedBox(height: 10),
                      AnimatedOpacity(
                        duration: motion(context),
                        opacity: lock.enabled ? 1 : 0.45,
                        child: IgnorePointer(
                          ignoring: !lock.enabled || lock.busy,
                          child: OrrerySegmented(
                            keyPrefix: 'lock-timeout',
                            labels: [
                              for (final v in LockTimeout.values)
                                _short(context, v),
                            ],
                            index: LockTimeout.values.indexOf(lock.timeout),
                            onChanged: (i) => _change(
                              context,
                              timeout: LockTimeout.values[i],
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class LockSettingsPage extends StatelessWidget {
  const LockSettingsPage({super.key, required this.lock});

  final AppLock lock;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(title: Text(l.lockSettingsTitle)),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
        children: [LockControls(lock: lock)],
      ),
    );
  }
}
