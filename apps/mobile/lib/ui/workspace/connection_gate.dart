import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../theme/orrery_theme.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';

class ConnectionGate extends ConsumerWidget {
  const ConnectionGate({
    super.key,
    required this.machineId,
    required this.child,
  });

  final String machineId;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final status = ref.watch(
      linkProvider(machineId).select((link) => link.status),
    );
    return switch (status) {
      LinkStatus.revoked => RevokedView(machineId: machineId),
      LinkStatus.protoTooOld => ProtoTooOldView(machineId: machineId),
      _ => child,
    };
  }
}

class _Terminal extends StatelessWidget {
  const _Terminal({
    required this.icon,
    required this.title,
    required this.body,
    required this.action,
    required this.onAction,
  });

  final IconData icon;
  final String title;
  final String body;
  final String action;
  final VoidCallback onAction;

  @override
  Widget build(BuildContext context) {
    final color = context.statusColors.failed;
    return Scaffold(
      appBar: AppBar(),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 64, color: color),
              const SizedBox(height: 16),
              Text(
                title,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 8),
              Text(
                body,
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 24),
              FilledButton(onPressed: onAction, child: Text(action)),
            ],
          ),
        ),
      ),
    );
  }
}

class RevokedView extends ConsumerStatefulWidget {
  const RevokedView({super.key, required this.machineId});

  final String machineId;

  @override
  ConsumerState<RevokedView> createState() => _RevokedViewState();
}

class _RevokedViewState extends ConsumerState<RevokedView> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        ref.read(machineApiProvider(widget.machineId)).clearCache();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return _Terminal(
      icon: Icons.phonelink_erase_rounded,
      title: l.revokedTitle,
      body: l.revokedBody,
      action: l.removeMachine,
      onAction: () async {
        final navigator = Navigator.of(context);
        await ref.read(machinesProvider.notifier).forget(widget.machineId);
        if (mounted) navigator.popUntil((route) => route.isFirst);
      },
    );
  }
}

class ProtoTooOldView extends ConsumerWidget {
  const ProtoTooOldView({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    return _Terminal(
      icon: Icons.system_update_rounded,
      title: l.protoTitle,
      body: l.protoBody,
      action: l.back,
      onAction: () => Navigator.of(context).maybePop(),
    );
  }
}
