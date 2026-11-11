import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../remote/link_state.dart';
import '../remote/machines.dart';
import '../remote/providers.dart';
import 'pair_page.dart';

String linkText(LinkState state) {
  final route = switch (state.route) {
    LinkRoute.lan => ' (LAN)',
    LinkRoute.relay => ' (relay)',
    null => '',
  };
  return switch (state.status) {
    LinkStatus.idle => 'Chưa nối',
    LinkStatus.discovering => 'Đang tìm máy',
    LinkStatus.handshaking => 'Đang bắt tay$route',
    LinkStatus.ready => 'Đã nối$route',
    LinkStatus.reconnecting => 'Đang nối lại, lần ${state.attempt}',
    LinkStatus.offline => 'Ngoại tuyến',
    LinkStatus.protoTooOld => 'App quá cũ, cần cập nhật',
    LinkStatus.revoked => 'Máy đã thu hồi thiết bị này',
  };
}

class HomePage extends ConsumerWidget {
  const HomePage({super.key, this.onOpenSpike});

  final VoidCallback? onOpenSpike;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final machines = ref.watch(machinesProvider);
    return Scaffold(
      appBar: AppBar(
        title: const Text('Orrery Remote'),
        actions: [
          if (onOpenSpike != null)
            IconButton(
              onPressed: onOpenSpike,
              icon: const Icon(Icons.terminal),
              tooltip: 'Spike terminal',
            ),
        ],
      ),
      body: machines.isEmpty
          ? const Center(child: Text('Chưa có máy nào'))
          : ListView(
              children: [for (final m in machines) _MachineTile(machine: m)],
            ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => Navigator.of(context)
            .push(MaterialPageRoute<void>(builder: (_) => const PairPage())),
        icon: const Icon(Icons.qr_code_scanner),
        label: const Text('Ghép máy'),
      ),
    );
  }
}

class _MachineTile extends ConsumerWidget {
  const _MachineTile({required this.machine});

  final Machine machine;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final link = ref.watch(linkProvider(machine.id));
    final view = ref.watch(machineViewProvider(machine.id));
    final terminals = (view.snapshot?['terminals'] as List?)?.length ?? 0;
    final stale = view.snapshot != null && view.stale;
    final at = view.updatedAt;
    final hh = at?.hour.toString().padLeft(2, '0');
    final mm = at?.minute.toString().padLeft(2, '0');
    final label = stale && at != null ? 'cũ, lúc $hh:$mm' : '$terminals phiên';
    return ListTile(
      title: Text(machine.name),
      subtitle: Text('${linkText(link)} · $label'),
      trailing: IconButton(
        icon: const Icon(Icons.delete_outline),
        onPressed: () => ref.read(machinesProvider.notifier).forget(machine.id),
      ),
    );
  }
}
