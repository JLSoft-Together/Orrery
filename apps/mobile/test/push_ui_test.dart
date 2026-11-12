import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/deep_link_host.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/push/push_gateway.dart';
import 'package:orrery_remote/push/push_providers.dart';
import 'package:orrery_remote/push/push_service.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';

import 'support/ui_samples.dart';

import 'support/push_fakes.dart';

void main() {
  testWidgets('tapping a notification opens the pane in the app', (
    tester,
  ) async {
    final h = harness();
    final r = Rig();
    await r.service.init();
    final navigator = GlobalKey<NavigatorState>();
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox());
      tester.view.reset();
    });
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: h.container = ProviderContainer(overrides: h.overrides),
        child: MaterialApp(
          navigatorKey: navigator,
          theme: buildOrreryTheme(RemoteTheme.fallback, Brightness.light),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          builder: (context, child) => DeepLinkHost(
            navigatorKey: navigator,
            source: MergedLinkSource([r.service]),
            child: child ?? const SizedBox.shrink(),
          ),
          home: const Scaffold(body: Text('home')),
        ),
      ),
    );
    await tester.pump();
    r.gateway.open_.add({'m': 'm1', 'p': 'b', 'k': 'done'});
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(h.api.attached, contains('b'));
  });

  testWidgets('the switch shows only when the desktop has Firebase set up', (
    tester,
  ) async {
    final r = Rig();
    await r.service.init();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [pushServiceProvider.overrideWithValue(r.service)],
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          home: const Scaffold(body: PushTile()),
        ),
      ),
    );
    expect(find.byKey(const ValueKey('push-switch')), findsNothing);
    expect(find.byKey(const ValueKey('push-not-set-up')), findsOneWidget);
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('push-switch')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('push-switch')));
    await tester.pump();
    await tester.pump();
    expect(r.service.enabled, isTrue);
  });

  testWidgets('a refused permission says so and leaves the switch off', (
    tester,
  ) async {
    final r = Rig();
    r.gateway.asked = PushPermission.denied;
    await r.service.init();
    r.host.config.add(const PushConfigUpdate('m1', pushConfigA));
    await tester.pump();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [pushServiceProvider.overrideWithValue(r.service)],
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          home: const Scaffold(body: PushTile()),
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('push-switch')));
    await tester.pump();
    await tester.pump();
    expect(find.textContaining('permission was refused'), findsOneWidget);
    expect(r.service.enabled, isFalse);
  });
}
