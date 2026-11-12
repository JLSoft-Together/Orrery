import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../remote/machines.dart';
import '../remote/providers.dart';
import '../ui/commands.dart';
import '../ui/kit/kit.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'machine_card.dart';

String _date(int seconds) {
  final d = DateTime.fromMillisecondsSinceEpoch(seconds * 1000);
  return '${d.day.toString().padLeft(2, '0')}/${d.month.toString().padLeft(2, '0')}';
}

class HomeDuplicates extends ConsumerWidget {
  const HomeDuplicates({super.key, required this.machines});

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
            DuplicateRow(machine: m),
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

class DuplicateRow extends ConsumerWidget {
  const DuplicateRow({super.key, required this.machine});

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
          const MachineIcon(active: false),
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
