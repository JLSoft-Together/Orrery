import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/app_lock_scope.dart';
import '../../push/push_providers.dart';
import '../../remote/providers.dart';
import '../../security/lock_settings_page.dart';
import '../kit/kit.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'appearance_rows.dart';

class SettingsBody extends ConsumerWidget {
  const SettingsBody({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final lock = AppLockScope.maybeOf(context);
    final machine = ref
        .watch(machinesProvider)
        .where((m) => m.id == machineId)
        .firstOrNull;
    final hasPush = ref.watch(pushServiceProvider) != null;
    return Scaffold(
      appBar: AppBar(
        toolbarHeight: 64,
        title: Text(
          l.settingsTitle,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
      ),
      body: ListView(
        key: const ValueKey('settings-list'),
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
        children: [
          if (lock != null) ...[
            SectionLabel(l.settingsSecurity),
            LockControls(lock: lock),
          ],
          if (hasPush) ...[
            SectionLabel(l.pushTitle),
            const PushTile(),
            Padding(
              padding: const EdgeInsets.fromLTRB(4, 8, 4, 0),
              child: Text(
                l.settingsPushHint,
                style: TextStyle(fontSize: 12, color: context.orrery.inkFaint),
              ),
            ),
          ],
          SectionLabel(l.settingsAppearance),
          const AppearanceRows(),
          SectionLabel(l.settingsAbout),
          OrreryCard(
            padding: EdgeInsets.zero,
            child: Column(
              children: [
                _InfoRow(
                  icon: Icons.public_rounded,
                  label: l.settingsAppName,
                  value: '',
                  hint: l.settingsAppHint,
                ),
                if (machine != null) ...[
                  Divider(height: 1, color: context.orrery.line),
                  _InfoRow(
                    icon: Icons.computer_rounded,
                    label: l.settingsMachine,
                    value: machine.displayName,
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.icon,
    required this.label,
    required this.value,
    this.hint,
  });

  final IconData icon;
  final String label;
  final String value;
  final String? hint;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
      child: Row(
        children: [
          Icon(icon, size: 20, color: c.accentInk),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(label, style: Theme.of(context).textTheme.titleSmall),
                if (hint != null)
                  Text(
                    hint!,
                    style: TextStyle(fontSize: 12, color: c.inkMuted),
                  ),
              ],
            ),
          ),
          if (value.isNotEmpty) ...[
            const SizedBox(width: 12),
            Flexible(
              child: Text(
                value,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 14, color: c.inkMuted),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
