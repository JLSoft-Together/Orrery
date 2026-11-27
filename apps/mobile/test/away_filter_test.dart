import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/away_store.dart';
import 'package:orrery_remote/remote/away_summary.dart';
import 'package:orrery_remote/ui/away/away_controller.dart';

void main() {
  test('leaving clears the away filter of every machine', () async {
    final container = ProviderContainer(
      overrides: [awayStoreProvider.overrideWithValue(MemoryAwayStore())],
    );
    addTearDown(container.dispose);
    container.listen(awayProvider, (_, _) {});
    container.listen(awayFilterProvider('m1'), (_, _) {});
    container.listen(awayFilterProvider('m2'), (_, _) {});
    await Future<void>.delayed(Duration.zero);
    container.read(awayFilterProvider('m1').notifier).toggle(AwayGroup.done);
    container.read(awayFilterProvider('m2').notifier).toggle(AwayGroup.failed);
    expect(container.read(awayFilterProvider('m1')), AwayGroup.done);
    await container.read(awayProvider.notifier).leave();
    expect(container.read(awayFilterProvider('m1')), isNull);
    expect(container.read(awayFilterProvider('m2')), isNull);
  });
}
