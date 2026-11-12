import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../ui/commands.dart';
import '../ui/widgets/common.dart';
import 'push_service.dart';

final pushServiceProvider = Provider<PushService?>((ref) => null);

class PushState {
  const PushState({required this.enabled, required this.configured});

  final bool enabled;
  final bool configured;

  @override
  bool operator ==(Object other) =>
      other is PushState &&
      other.enabled == enabled &&
      other.configured == configured;

  @override
  int get hashCode => Object.hash(enabled, configured);
}

class PushStateNotifier extends Notifier<PushState> {
  @override
  PushState build() {
    final service = ref.read(pushServiceProvider);
    if (service == null) {
      return const PushState(enabled: false, configured: false);
    }
    final sub = service.changes.listen((_) => state = _read(service));
    ref.onDispose(sub.cancel);
    return _read(service);
  }

  PushState _read(PushService s) =>
      PushState(enabled: s.enabled, configured: s.configured);
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
      return Card(
        key: const ValueKey('push-not-set-up'),
        child: ListTile(
          leading: const Icon(Icons.notifications_off_outlined),
          title: Text(l.pushTitle),
          subtitle: Text(l.pushNotSetUp),
        ),
      );
    }
    return Card(
      child: SwitchListTile(
        key: const ValueKey('push-switch'),
        secondary: const Icon(Icons.notifications_active_outlined),
        title: Text(l.pushTitle),
        subtitle: Text(l.pushHint),
        value: state.enabled,
        onChanged: (value) async {
          final result = await service.setEnabled(value);
          if (!context.mounted) return;
          if (result == PushResult.denied) toast(context, l.pushDenied);
          if (result == PushResult.unavailable) {
            toast(context, l.pushUnavailable);
          }
        },
      ),
    );
  }
}
