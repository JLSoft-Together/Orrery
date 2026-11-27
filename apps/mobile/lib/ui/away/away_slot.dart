import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../kit/quiet_size.dart';
import 'away_card.dart';
import 'away_controller.dart';

class AwaySlot extends ConsumerWidget {
  const AwaySlot({super.key, required this.machineId});

  final String machineId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final summary = ref.watch(awayProvider.select((m) => m[machineId]));
    final selected = ref.watch(awayFilterProvider(machineId));
    return QuietSize(
      child: summary == null
          ? const SizedBox(width: double.infinity)
          : AwayCard(
              summary: summary,
              selected: selected,
              onSelect: (g) =>
                  ref.read(awayFilterProvider(machineId).notifier).toggle(g),
              onSeen: () => ref.read(awayProvider.notifier).dismiss(machineId),
            ),
    );
  }
}
