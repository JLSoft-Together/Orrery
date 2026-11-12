import 'package:flutter/material.dart';

import '../ui/commands.dart';
import '../ui/widgets/common.dart';
import 'app_lock.dart';
import 'device_auth.dart';

class LockSettingsPage extends StatelessWidget {
  const LockSettingsPage({super.key, required this.lock});

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
      final ok = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(l.lockTurnOffTitle),
          content: Text('${l.lockNoCredentials}\n\n${l.lockTurnOffWarn}'),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: Text(l.cancel),
            ),
            FilledButton(
              key: const ValueKey('lock-turn-off-confirm'),
              onPressed: () => Navigator.pop(context, true),
              child: Text(l.lockTurnOff),
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

  String _label(BuildContext context, LockTimeout value) {
    final l = context.l10n;
    return switch (value) {
      LockTimeout.everyOpen => l.lockEveryOpen,
      LockTimeout.minutes15 => l.lockMinutes15,
      LockTimeout.hour1 => l.lockHour1,
      LockTimeout.hours4 => l.lockHours4,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(title: Text(l.lockSettingsTitle)),
      body: ListenableBuilder(
        listenable: lock,
        builder: (context, _) => ListView(
          children: [
            SwitchListTile(
              key: const ValueKey('lock-switch'),
              title: Text(l.lockSwitch),
              subtitle: Text(l.lockSwitchHint),
              value: lock.enabled,
              onChanged: lock.busy
                  ? null
                  : (value) => _change(context, enabled: value),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
              child: Text(
                l.lockAfter,
                style: Theme.of(context).textTheme.titleSmall,
              ),
            ),
            RadioGroup<LockTimeout>(
              groupValue: lock.timeout,
              onChanged: (value) {
                if (value == null || lock.busy || !lock.enabled) return;
                _change(context, timeout: value);
              },
              child: Column(
                children: [
                  for (final value in LockTimeout.values)
                    RadioListTile<LockTimeout>(
                      key: ValueKey('lock-timeout-${value.name}'),
                      title: Text(_label(context, value)),
                      value: value,
                      enabled: lock.enabled && !lock.busy,
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
