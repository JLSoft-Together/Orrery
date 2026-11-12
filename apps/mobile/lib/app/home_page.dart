import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../push/push_providers.dart';
import '../remote/link_state.dart';
import '../remote/machines.dart';
import '../remote/providers.dart';
import '../security/app_lock.dart';
import '../security/lock_settings_page.dart';
import '../ui/kit/kit.dart';
import '../ui/widgets/common.dart';
import 'home_duplicates.dart';
import 'home_groups.dart';
import 'machine_card.dart';
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
                    builder: (context, m) => MachineCard(machine: m),
                  ),
                  if (groups.duplicates.isNotEmpty) ...[
                    SectionLabel(
                      l.homeDuplicates,
                      count: groups.duplicates.length,
                    ),
                    HomeDuplicates(machines: groups.duplicates),
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
