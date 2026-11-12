import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/pair_page.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/pairing.dart';
import 'package:orrery_remote/remote/providers.dart';

class FakeMachines extends MachinesNotifier {
  final List<String> codes = [];

  @override
  List<Machine> build() => const [];

  @override
  Future<Machine> pair(String qrText) async {
    codes.add(qrText);
    throw const PairException(PairFailure.badQr);
  }
}

void main() {
  testWidgets('debug pair code field submits the typed text to pair', (
    tester,
  ) async {
    final fake = FakeMachines();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [machinesProvider.overrideWith(() => fake)],
        child: const MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: PairPage(scanner: SizedBox()),
        ),
      ),
    );
    await tester.enterText(
      find.byKey(const ValueKey('pair-code')),
      ' orrery://pair?a=1&b=2 ',
    );
    await tester.tap(find.byKey(const ValueKey('pair-code-submit')));
    await tester.pump();
    expect(fake.codes, ['orrery://pair?a=1&b=2']);
  });
}
