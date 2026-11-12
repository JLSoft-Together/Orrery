import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/pair_page.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/pairing.dart';
import 'package:orrery_remote/remote/providers.dart';

class FakeMachines extends MachinesNotifier {
  final List<String> codes = [];

  @override
  List<Machine> build() => const [];

  @override
  Future<Machine> pair(String qrText, {PairRouteListener? onRoute}) async {
    codes.add(qrText);
    onRoute?.call(LinkRoute.relay);
    throw const PairException(PairFailure.badQr);
  }
}

class SlowMachines extends MachinesNotifier {
  final Completer<Machine> done = Completer<Machine>();
  PairRouteListener? listener;

  @override
  List<Machine> build() => const [];

  @override
  Future<Machine> pair(String qrText, {PairRouteListener? onRoute}) {
    listener = onRoute;
    onRoute?.call(LinkRoute.lan);
    return done.future;
  }
}

Widget _app(MachinesNotifier fake, {Locale? locale}) => ProviderScope(
  overrides: [machinesProvider.overrideWith(() => fake)],
  child: MaterialApp(
    locale: locale,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: const PairPage(scanner: SizedBox()),
  ),
);

void main() {
  testWidgets('the pair page shows the route in use, LAN then relay', (
    tester,
  ) async {
    final fake = SlowMachines();
    await tester.pumpWidget(_app(fake, locale: const Locale('vi')));
    await tester.tap(find.byKey(const ValueKey('pair-tab-1')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const ValueKey('pair-code')), 'qr');
    await tester.tap(find.byKey(const ValueKey('pair-code-submit')));
    await tester.pump();
    expect(find.byKey(const ValueKey('pair-route-lan')), findsOneWidget);
    expect(find.text('Đang ghép qua Wi-Fi (LAN)...'), findsOneWidget);
    expect(find.byIcon(Icons.wifi_rounded), findsOneWidget);
    fake.listener!(LinkRoute.relay);
    await tester.pump();
    expect(find.byKey(const ValueKey('pair-route-relay')), findsOneWidget);
    expect(find.text('Khác mạng, đang ghép qua relay...'), findsOneWidget);
    expect(find.byIcon(Icons.cloud_outlined), findsOneWidget);
    fake.done.completeError(const PairException(PairFailure.network));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('pair-route-relay')), findsNothing);
    expect(find.byKey(const ValueKey('pair-error')), findsOneWidget);
  });

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
    await tester.tap(find.byKey(const ValueKey('pair-tab-1')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('pair-code')),
      ' orrery://pair?a=1&b=2 ',
    );
    await tester.tap(find.byKey(const ValueKey('pair-code-submit')));
    await tester.pump();
    await tester.pumpAndSettle();
    expect(fake.codes, ['orrery://pair?a=1&b=2']);
    expect(find.byKey(const ValueKey('pair-error')), findsOneWidget);
  });
}
