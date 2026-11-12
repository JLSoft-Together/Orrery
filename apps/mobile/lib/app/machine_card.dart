import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../remote/link_state.dart';
import '../remote/machines.dart';
import '../remote/providers.dart';
import '../ui/kit/kit.dart';
import '../ui/model/workspace_model.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/ui_providers.dart';
import '../ui/widgets/common.dart';
import '../ui/widgets/name_dialog.dart';
import '../ui/workspace/workspace_page.dart';

class MachineIcon extends StatelessWidget {
  const MachineIcon({super.key, required this.active});

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

class MachineCard extends ConsumerWidget {
  const MachineCard({super.key, required this.machine});

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
                MachineIcon(active: link.status == LinkStatus.ready),
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
                LinkPill(
                  link: link,
                  latencyMs: ref.watch(latencyProvider(machine.id)).value,
                ),
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
