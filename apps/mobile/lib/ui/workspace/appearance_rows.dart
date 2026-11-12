import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../kit/kit.dart';
import '../monitor/view_prefs.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

String languageLabel(AppLocalizations l, String v) => switch (v) {
  'vi' => 'Tiếng Việt',
  'en' => 'English',
  _ => l.settingsFollowsMachine,
};

String themeLabel(AppLocalizations l, String v) => switch (v) {
  'system' => l.settingsThemeSystem,
  'light' => l.settingsThemeLight,
  'dark' => l.settingsThemeDark,
  _ => l.settingsFollowsMachine,
};

class AppearanceRows extends ConsumerWidget {
  const AppearanceRows({super.key});

  Future<void> _pick(
    BuildContext context, {
    required String title,
    required List<String> options,
    required String current,
    required String Function(String) label,
    required ValueChanged<String> onPick,
  }) async {
    final picked = await showOrrerySheet<String>(
      context,
      builder: (context) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 16, 20, 8),
              child: Text(
                title,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            RadioGroup<String>(
              groupValue: current,
              onChanged: (v) => Navigator.of(context).pop(v),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  for (final o in options)
                    RadioListTile<String>(
                      key: ValueKey('pick-$o'),
                      value: o,
                      title: Text(label(o)),
                    ),
                ],
              ),
            ),
            const SizedBox(height: 8),
          ],
        ),
      ),
    );
    if (picked != null && picked != current) onPick(picked);
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final c = context.orrery;
    final prefs = ref.watch(viewPrefsProvider);
    final notifier = ref.read(viewPrefsProvider.notifier);
    return OrreryCard(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          _Row(
            key: const ValueKey('settings-language'),
            icon: Icons.language_rounded,
            label: l.settingsLanguage,
            value: languageLabel(l, prefs.language),
            onTap: () => _pick(
              context,
              title: l.settingsLanguage,
              options: ViewPrefs.languages,
              current: prefs.language,
              label: (v) => languageLabel(l, v),
              onPick: notifier.setLanguage,
            ),
          ),
          Divider(height: 1, color: c.line),
          _Row(
            key: const ValueKey('settings-theme'),
            icon: Icons.dark_mode_outlined,
            label: l.settingsTheme,
            value: themeLabel(l, prefs.theme),
            onTap: () => _pick(
              context,
              title: l.settingsTheme,
              options: ViewPrefs.themes,
              current: prefs.theme,
              label: (v) => themeLabel(l, v),
              onPick: notifier.setTheme,
            ),
          ),
        ],
      ),
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
    super.key,
    required this.icon,
    required this.label,
    required this.value,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final String value;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        child: Row(
          children: [
            Icon(icon, size: 20, color: c.accentInk),
            const SizedBox(width: 12),
            Expanded(
              child: Text(label, style: Theme.of(context).textTheme.titleSmall),
            ),
            const SizedBox(width: 12),
            Flexible(
              child: AnimatedSwitcher(
                duration: motion(context, 180),
                child: Text(
                  value,
                  key: ValueKey(value),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(fontSize: 14, color: c.inkMuted),
                ),
              ),
            ),
            Icon(Icons.chevron_right_rounded, size: 20, color: c.inkFaint),
          ],
        ),
      ),
    );
  }
}
