import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../push/push_providers.dart';
import '../remote/link_state.dart';
import '../remote/machines.dart';
import '../remote/providers.dart';
import '../security/app_lock.dart';
import '../security/lock_settings_page.dart';
import '../ui/commands.dart';
import '../ui/kit/kit.dart';
import '../ui/model/workspace_model.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/ui_providers.dart';
import '../ui/widgets/common.dart';
import '../ui/widgets/name_dialog.dart';
import '../ui/workspace/workspace_page.dart';
import 'home_groups.dart';
import 'pair_page.dart';

class HomePage extends ConsumerWidget {
  const HomePage({super.key, this.onOpenSpike, this.lock});

  final VoidCallback? onOpenSpike;
  final AppLock? lock;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final machines = ref.watch(machinesProvider);
    final ready = <String>{
      for (final m in machines)
        if (ref.watch(linkProvider(m.id)).status == LinkStatus.ready) m.id,
    };
    final groups = groupMachines(machines, ready);
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(
        toolbarHeight: 64,
        title: Text(
          l.homeTitle,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        actions: [
          if (lock case final lock?)
            IconButton(
              key: const ValueKey('open-lock-settings'),
              onPressed: () => Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (_) => LockSettingsPage(lock: lock),
                ),
              ),
              icon: const Icon(Icons.tune_rounded),
              tooltip: l.lockSettingsTitle,
            ),
          if (onOpenSpike != null)
            IconButton(
              onPressed: onOpenSpike,
              icon: const Icon(Icons.speed_rounded),
              tooltip: l.spikeTooltip,
            ),
          const SizedBox(width: 4),
        ],
      ),
      body: AnimatedSwitcher(
        duration: motion(context),
        child: machines.isEmpty
            ? const _EmptyMachines(key: ValueKey('empty'))
            : ListView(
                key: const ValueKey('list'),
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 96),
                children: [
                  SectionLabel(l.homeConnected, count: groups.primary.length),
                  AnimatedItems<Machine>(
                    items: groups.primary,
                    keyOf: (m) => m.id,
                    builder: (context, m) => _MachineCard(machine: m),
                  ),
                  if (groups.duplicates.isNotEmpty) ...[
                    SectionLabel(
                      l.homeDuplicates,
                      count: groups.duplicates.length,
                    ),
                    _Duplicates(machines: groups.duplicates),
                  ],
                  const SizedBox(height: 10),
                  const PushTile(),
                ],
              ),
      ),
      floatingActionButton: machines.isEmpty
          ? null
          : FloatingActionButton.extended(
              onPressed: () => _pair(context),
              icon: const Icon(Icons.add_rounded),
              label: Text(l.pairMachine),
            ),
    );
  }
}

void _pair(BuildContext context, {bool code = false}) => Navigator.of(context)
    .push(MaterialPageRoute<void>(builder: (_) => PairPage(startOnCode: code)));

class _EmptyMachines extends StatelessWidget {
  const _EmptyMachines({super.key});

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return EmptyState(
      title: l.noMachines,
      body: l.noMachinesHint,
      actions: [
        OrreryButton(
          label: l.pairMachine,
          icon: Icons.qr_code_scanner_rounded,
          expand: false,
          onPressed: () => _pair(context),
        ),
        if (kDebugMode)
          OrreryButton(
            label: l.enterCodeManual,
            kind: ButtonKind.text,
            expand: false,
            onPressed: () => _pair(context, code: true),
          ),
      ],
    );
  }
}

class _MachineIcon extends StatelessWidget {
  const _MachineIcon({required this.active});

  final bool active;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return TweenAnimationBuilder<Color?>(
      tween: ColorTween(end: active ? c.selection : c.raised),
      duration: motion(context),
      builder: (context, bg, _) => DecoratedBox(
        decoration: BoxDecoration(
          color: bg,
          borderRadius: context.shape.controlRadius,
        ),
        child: SizedBox.square(
          dimension: 40,
          child: Icon(
            Icons.computer_rounded,
            size: 22,
            color: active ? c.accent : c.inkFaint,
          ),
        ),
      ),
    );
  }
}

class _MachineCard extends ConsumerWidget {
  const _MachineCard({required this.machine});

  final Machine machine;

  Future<void> _rename(BuildContext context, WidgetRef ref) async {
    final l = context.l10n;
    final name = await showNameDialog(
      context,
      title: l.renameMachine,
      initial: machine.alias ?? '',
      maxLength: maxMachineAlias,
      hint: l.renameMachineHint,
    );
    if (name == null) return;
    await ref.read(machinesProvider.notifier).rename(machine.id, name);
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final link = ref.watch(linkProvider(machine.id));
    final view = ref.watch(machineViewProvider(machine.id));
    final model = ref.watch(workspaceModelProvider(machine.id));
    final l = context.l10n;
    final c = context.orrery;
    final at = view.updatedAt;
    final stale = view.snapshot != null && view.stale;
    final needs = model.panes.where((p) => p.needsUser).length;
    final working = model.panes
        .where(
          (p) => p.rail == RailState.working || p.rail == RailState.subagents,
        )
        .length;
    final idle = model.panes.where((p) => p.rail == RailState.idle).length;
    final last = stale && at != null
        ? l.lastSeenAt(clock(at))
        : l.panesCount(model.panes.length);
    return Dismissible(
      key: ValueKey('dismiss-${machine.id}'),
      direction: DismissDirection.endToStart,
      confirmDismiss: (_) => confirmOrrery(
        context,
        title: l.forgetTitle(machine.displayName),
        body: l.forgetBody,
        confirmLabel: l.forget,
        cancelLabel: l.cancel,
        danger: true,
      ),
      onDismissed: (_) =>
          ref.read(machinesProvider.notifier).forget(machine.id),
      background: DecoratedBox(
        decoration: BoxDecoration(
          color: c.error.withValues(alpha: 0.14),
          borderRadius: context.shape.cardRadius,
        ),
        child: Align(
          alignment: Alignment.centerRight,
          child: Padding(
            padding: const EdgeInsets.only(right: 24),
            child: Icon(Icons.delete_outline_rounded, color: c.error),
          ),
        ),
      ),
      child: OrreryCard(
        onLongPress: () => _rename(context, ref),
        onTap: () {
          ref.read(activeMachineProvider.notifier).select(machine.id);
          Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => WorkspacePage(machineId: machine.id),
            ),
          );
        },
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                _MachineIcon(active: link.status == LinkStatus.ready),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      AnimatedRename(
                        text: machine.displayName,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      Text(
                        last,
                        style: TextStyle(fontSize: 12, color: c.inkMuted),
                      ),
                      if (machine.alias != null)
                        Text(
                          l.machineOriginal(machine.name),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(fontSize: 12, color: c.inkFaint),
                        ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                LinkPill(link: link),
                PopupMenuButton<String>(
                  key: ValueKey('machine-menu-${machine.id}'),
                  tooltip: l.more,
                  padding: EdgeInsets.zero,
                  icon: Icon(Icons.more_vert_rounded, color: c.inkMuted),
                  onSelected: (_) => _rename(context, ref),
                  itemBuilder: (_) => [
                    PopupMenuItem<String>(
                      key: const ValueKey('machine-rename'),
                      value: 'rename',
                      child: Text(l.renameMachine),
                    ),
                  ],
                ),
              ],
            ),
            AnimatedSize(
              duration: motion(context),
              alignment: Alignment.topLeft,
              child: needs + working + idle == 0
                  ? const SizedBox(width: double.infinity)
                  : Padding(
                      padding: const EdgeInsets.only(top: 10),
                      child: Wrap(
                        spacing: 8,
                        runSpacing: 6,
                        children: [
                          if (working > 0)
                            OrreryChip(
                              tone: ChipTone.working,
                              icon: Icons.autorenew_rounded,
                              label: l.runningCount(working),
                            ),
                          if (needs > 0)
                            OrreryChip(
                              tone: ChipTone.waiting,
                              icon: Icons.front_hand_outlined,
                              label: l.needsYouCount(needs),
                            ),
                          if (idle > 0)
                            OrreryChip(
                              tone: ChipTone.idle,
                              icon: Icons.nightlight_outlined,
                              label: l.idleCount(idle),
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

String _date(int seconds) {
  final d = DateTime.fromMillisecondsSinceEpoch(seconds * 1000);
  return '${d.day.toString().padLeft(2, '0')}/${d.month.toString().padLeft(2, '0')}';
}

class _Duplicates extends ConsumerWidget {
  const _Duplicates({required this.machines});

  final List<Machine> machines;

  Future<void> _clean(BuildContext context, WidgetRef ref) async {
    final l = context.l10n;
    final ok = await confirmOrrery(
      context,
      title: l.cleanOldTitle,
      body: l.cleanOldBody(machines.length),
      confirmLabel: l.forget,
      cancelLabel: l.cancel,
      danger: true,
    );
    if (!ok) return;
    final notifier = ref.read(machinesProvider.notifier);
    for (final m in machines) {
      await notifier.forget(m.id);
    }
    if (context.mounted) toast(context, l.cleanOldDone);
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final c = context.orrery;
    return OrreryCard(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 4),
      child: Column(
        children: [
          for (final m in machines) ...[
            _DuplicateRow(machine: m),
            Divider(color: c.line, height: 1),
          ],
          OrreryButton(
            key: const ValueKey('clean-old'),
            label: l.cleanOld(machines.length),
            icon: Icons.delete_outline_rounded,
            kind: ButtonKind.text,
            onPressed: () => _clean(context, ref),
          ),
        ],
      ),
    );
  }
}

class _DuplicateRow extends ConsumerWidget {
  const _DuplicateRow({required this.machine});

  final Machine machine;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final link = ref.watch(linkProvider(machine.id));
    final l = context.l10n;
    final c = context.orrery;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 10),
      child: Row(
        children: [
          const _MachineIcon(active: false),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  machine.displayName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                Text(
                  l.pairedOn(_date(machine.pairedAt)),
                  style: TextStyle(fontSize: 12, color: c.inkMuted),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          machine.revoked
              ? OrreryChip(
                  tone: ChipTone.idle,
                  icon: Icons.block_rounded,
                  label: l.linkRevoked,
                )
              : LinkPill(link: link),
        ],
      ),
    );
  }
}
