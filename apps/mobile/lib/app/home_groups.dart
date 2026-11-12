import '../remote/machines.dart';

class MachineGroups {
  const MachineGroups(this.primary, this.duplicates);

  final List<Machine> primary;
  final List<Machine> duplicates;
}

MachineGroups groupMachines(List<Machine> machines, Set<String> readyIds) {
  final byName = <String, List<Machine>>{};
  for (final m in machines) {
    byName.putIfAbsent(m.name, () => []).add(m);
  }
  final keep = <String>{};
  for (final group in byName.values) {
    group.sort((a, b) {
      final ready =
          (readyIds.contains(b.id) ? 1 : 0) - (readyIds.contains(a.id) ? 1 : 0);
      if (ready != 0) return ready;
      final alive = (a.revoked ? 1 : 0) - (b.revoked ? 1 : 0);
      if (alive != 0) return alive;
      return b.pairedAt.compareTo(a.pairedAt);
    });
    keep.add(group.first.id);
  }
  return MachineGroups(
    [
      for (final m in machines)
        if (keep.contains(m.id)) m,
    ],
    [
      for (final m in machines)
        if (!keep.contains(m.id)) m,
    ]..sort((a, b) => b.pairedAt.compareTo(a.pairedAt)),
  );
}
