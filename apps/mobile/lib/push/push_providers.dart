import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../ui/commands.dart';
import '../ui/kit/kit.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'push_service.dart';

final pushServiceProvider = Provider<PushService?>((ref) => null);

class PushState {
  const PushState({
    required this.enabled,
    required this.configured,
    required this.usable,
    required this.foreign,
    required this.revoked,
    required this.error,
    this.muted = '',
    this.content = true,
  });

  const PushState.none()
    : enabled = false,
      configured = false,
      usable = false,
      foreign = 0,
      revoked = false,
      error = null,
      muted = '',
      content = true;

  final bool enabled;
  final bool configured;
  final bool usable;
  final int foreign;
  final bool revoked;
  final String? error;
  final String muted;
  final bool content;

  bool wants(String group) => !muted.split(',').contains(group);

  @override
  bool operator ==(Object other) =>
      other is PushState &&
      other.enabled == enabled &&
      other.configured == configured &&
      other.usable == usable &&
      other.foreign == foreign &&
      other.revoked == revoked &&
      other.error == error &&
      other.muted == muted &&
      other.content == content;

  @override
  int get hashCode => Object.hash(
    enabled,
    configured,
    usable,
    foreign,
    revoked,
    error,
    muted,
    content,
  );
}

class PushStateNotifier extends Notifier<PushState> {
  @override
  PushState build() {
    final service = ref.read(pushServiceProvider);
    if (service == null) return const PushState.none();
    final sub = service.changes.listen((_) => state = _read(service));
    ref.onDispose(sub.cancel);
    return _read(service);
  }

  PushState _read(PushService s) => PushState(
    enabled: s.enabled,
    configured: s.configured,
    usable: s.usable,
    foreign: s.foreignCount,
    revoked: s.revoked,
    error: s.error,
    muted: [
      for (final g in pushGroups)
        if (!s.wants(g)) g,
    ].join(','),
    content: s.content,
  );
}

final pushStateProvider = NotifierProvider<PushStateNotifier, PushState>(
  PushStateNotifier.new,
);

class PushTile extends ConsumerWidget {
  const PushTile({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final service = ref.watch(pushServiceProvider);
    if (service == null) return const SizedBox.shrink();
    final state = ref.watch(pushStateProvider);
    final l = context.l10n;
    if (!state.configured) {
      return _hint(
        const ValueKey('push-not-set-up'),
        l.pushTitle,
        l.pushNotSetUp,
      );
    }
    if (!state.usable) {
      return _hint(const ValueKey('push-foreign'), l.pushTitle, l.pushForeign);
    }
    final subtitle = state.revoked
        ? l.pushRevoked
        : state.error != null
        ? l.pushUnavailable
        : state.foreign > 0
        ? l.pushForeign
        : l.pushHint;
    return OrreryCard(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          _master(context, service, state, subtitle),
          QuietSize(
            child: state.enabled
                ? Column(
                    children: [
                      for (final (group, icon, label) in [
                        (
                          'waiting',
                          Icons.notifications_active_outlined,
                          l.pushGroupWaiting,
                        ),
                        (
                          'done',
                          Icons.check_circle_outline_rounded,
                          l.pushGroupDone,
                        ),
                        (
                          'error',
                          Icons.error_outline_rounded,
                          l.pushGroupError,
                        ),
                      ]) ...[
                        Divider(height: 1, color: context.orrery.line),
                        SwitchListTile(
                          key: ValueKey('push-group-$group'),
                          secondary: Icon(icon),
                          title: Text(label),
                          value: state.wants(group),
                          onChanged: (on) => service.setGroup(group, on),
                        ),
                      ],
                      Divider(height: 1, color: context.orrery.line),
                      SwitchListTile(
                        key: const ValueKey('push-content'),
                        secondary: const Icon(Icons.subject_rounded),
                        title: Text(l.pushContent),
                        subtitle: Text(l.pushContentHint),
                        value: state.content,
                        onChanged: service.setContent,
                      ),
                    ],
                  )
                : const SizedBox(width: double.infinity),
          ),
        ],
      ),
    );
  }

  Widget _master(
    BuildContext context,
    PushService service,
    PushState state,
    String subtitle,
  ) {
    final l = context.l10n;
    return SwitchListTile(
      key: const ValueKey('push-switch'),
      secondary: const Icon(Icons.notifications_none_rounded),
      title: Text(l.pushTitle),
      subtitle: Text(subtitle),
      value: state.enabled,
      onChanged: (value) async {
        final result = await service.setEnabled(value);
        if (!context.mounted) return;
        if (result == PushResult.denied) toast(context, l.pushDenied);
        if (result == PushResult.unavailable) {
          toast(context, l.pushUnavailable);
        }
      },
    );
  }

  Widget _hint(Key key, String title, String text) => OrreryCard(
    key: key,
    padding: EdgeInsets.zero,
    child: ListTile(
      leading: const Icon(Icons.notifications_off_outlined),
      title: Text(title),
      subtitle: Text(text),
    ),
  );
}
