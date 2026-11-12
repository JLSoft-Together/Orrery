import 'package:flutter/material.dart';

import '../ui/commands.dart';
import '../ui/kit/kit.dart';
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
          padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
          children: [
            OrreryCard(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: SwitchListTile(
                key: const ValueKey('lock-switch'),
                title: Text(l.lockSwitch),
                subtitle: Text(l.lockSwitchHint),
                value: lock.enabled,
                onChanged: lock.busy
                    ? null
                    : (value) => _change(context, enabled: value),
              ),
            ),
            SectionLabel(l.lockAfter),
            OrreryCard(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: RadioGroup<LockTimeout>(
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
            ),
          ],
        ),
      ),
    );
  }
}
