import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../push/push_providers.dart';
import '../remote/machines.dart';
import '../remote/providers.dart';
import '../ui/theme/orrery_theme.dart';
import '../ui/ui_providers.dart';
import '../ui/widgets/common.dart';
import '../ui/widgets/link_widgets.dart';
import '../ui/workspace/workspace_page.dart';
import 'pair_page.dart';

class HomePage extends ConsumerWidget {
  const HomePage({super.key, this.onOpenSpike});

  final VoidCallback? onOpenSpike;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final machines = ref.watch(machinesProvider);
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(
        title: Text(l.appTitle),
        actions: [
          if (onOpenSpike != null)
            IconButton(
              onPressed: onOpenSpike,
              icon: const Icon(Icons.speed_rounded),
              tooltip: l.spikeTooltip,
            ),
        ],
      ),
      body: AnimatedSwitcher(
        duration: motion(context),
        child: machines.isEmpty
            ? const _EmptyMachines(key: ValueKey('empty'))
            : ListView.separated(
                key: const ValueKey('list'),
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 96),
                itemCount: machines.length + 1,
                separatorBuilder: (_, _) => const SizedBox(height: 10),
                itemBuilder: (context, index) => index == machines.length
                    ? const PushTile()
                    : Appear(
                        key: ValueKey(machines[index].id),
                        delay: index * 40,
                        child: _MachineCard(machine: machines[index]),
                      ),
              ),
      ),
      floatingActionButton: machines.isEmpty
          ? null
          : FloatingActionButton.extended(
              onPressed: () => _pair(context),
              icon: const Icon(Icons.qr_code_scanner_rounded),
              label: Text(l.pairMachine),
            ),
    );
  }
}

void _pair(BuildContext context) =>
    Navigator.of(context)
        .push(MaterialPageRoute<void>(builder: (_) => const PairPage()));

class _EmptyMachines extends StatelessWidget {
  const _EmptyMachines({super.key});

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.devices_other_rounded, size: 56, color: scheme.primary),
            const SizedBox(height: 16),
            Text(
              l.noMachines,
              style: Theme.of(context).textTheme.titleMedium,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              l.noMachinesHint,
              textAlign: TextAlign.center,
              style: TextStyle(color: scheme.onSurfaceVariant),
            ),
            const SizedBox(height: 20),
            FilledButton.icon(
              onPressed: () => _pair(context),
              icon: const Icon(Icons.qr_code_scanner_rounded),
              label: Text(l.pairMachine),
            ),
          ],
        ),
      ),
    );
  }
}

class _MachineCard extends ConsumerWidget {
  const _MachineCard({required this.machine});

  final Machine machine;

  Future<bool> _confirm(BuildContext context) async {
    final l = context.l10n;
    final ok = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l.forgetTitle(machine.name)),
        content: Text(l.forgetBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(l.forget),
          ),
        ],
      ),
    );
    return ok ?? false;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final link = ref.watch(linkProvider(machine.id));
    final view = ref.watch(machineViewProvider(machine.id));
    final model = ref.watch(workspaceModelProvider(machine.id));
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final at = view.updatedAt;
    final stale = view.snapshot != null && view.stale;
    final needs = model.panes.where((p) => p.needsUser).length;
    final last = stale && at != null
        ? l.lastSeenAt(clock(at))
        : l.panesCount(model.panes.length);
    return Dismissible(
      key: ValueKey('dismiss-${machine.id}'),
      direction: DismissDirection.endToStart,
      confirmDismiss: (_) => _confirm(context),
      onDismissed: (_) =>
          ref.read(machinesProvider.notifier).forget(machine.id),
      background: Container(
        alignment: Alignment.centerRight,
        padding: const EdgeInsets.only(right: 24),
        decoration: BoxDecoration(
          color: scheme.error.withValues(alpha: 0.16),
          borderRadius: BorderRadius.circular(14),
        ),
        child: Icon(Icons.delete_outline_rounded, color: scheme.error),
      ),
      child: Card(
        child: PressScale(
          onTap: () {
            ref.read(activeMachineProvider.notifier).select(machine.id);
            Navigator.of(context).push(
              MaterialPageRoute<void>(
                builder: (_) => WorkspacePage(machineId: machine.id),
              ),
            );
          },
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: Row(
              children: [
                Icon(Icons.computer_rounded, size: 30, color: scheme.primary),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        machine.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          fontSize: 17,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        last,
                        style: TextStyle(color: scheme.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    LinkChip(link: link),
                    AnimatedSize(
                      duration: motion(context),
                      child: needs > 0
                          ? Padding(
                              padding: const EdgeInsets.only(top: 6),
                              child: _NeedsBadge(count: needs),
                            )
                          : const SizedBox(width: 0, height: 0),
                    ),
                  ],
                ),
                const SizedBox(width: 4),
                Icon(
                  Icons.chevron_right_rounded,
                  color: scheme.onSurfaceVariant,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _NeedsBadge extends StatelessWidget {
  const _NeedsBadge({required this.count});

  final int count;

  @override
  Widget build(BuildContext context) {
    final color = context.statusColors.asked;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.16),
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.front_hand_outlined, size: 13, color: color),
          const SizedBox(width: 4),
          Text(
            context.l10n.needsYouCount(count),
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: color,
            ),
          ),
        ],
      ),
    );
  }
}
