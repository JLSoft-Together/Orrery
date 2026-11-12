import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/app/pair_page.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/pairing.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';

import 'home_wp29_test.dart' show paired;

import 'package:orrery_remote/security/device_auth.dart';
import 'package:orrery_remote/security/lock_gate.dart';
import 'package:orrery_remote/security/lock_settings_page.dart';

import 'support/fonts.dart';
import 'support/lock_fakes.dart';
import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

final bool enabled = Platform.environment['WP29_SHOTS'] != null;
final String outDir = Platform.environment['WP29_OUT'] ?? 'shots';

Future<void> shot(WidgetTester tester, GlobalKey key, String name) async {
  await tester.pump(const Duration(milliseconds: 700));
  await tester.pump(const Duration(milliseconds: 700));
  await tester.runAsync(() async {
    final boundary =
        key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final image = await boundary.toImage(pixelRatio: 2);
    final data = await image.toByteData(format: ui.ImageByteFormat.png);
    final file = File('$outDir/$name.png');
    await file.create(recursive: true);
    await file.writeAsBytes(data!.buffer.asUint8List());
  });
}

class FailMachines extends MachinesNotifier {
  @override
  List<Machine> build() => const [];

  @override
  Future<Machine> pair(String qrText) =>
      Future.error(const PairException(PairFailure.expired));
}

void main() {
  setUpAll(loadFonts);

  for (final dark in [false, true]) {
    final tag = dark ? 'dark' : 'light';
    final b = dark ? Brightness.dark : Brightness.light;
    final locale = const Locale('vi');

    testWidgets('home $tag', (tester) async {
      final key = GlobalKey();
      final h = Harness(
        api: FakeMachineApi(),
        viewState: view(ws: sample()),
        linkState: linkReady,
        machines: [
          paired('m1', 'DESKTOP-DAUPH', 1762000000),
          paired('m4', 'MacBook-Pro-Vu', 1761000000),
          paired('m2', 'DESKTOP-DAUPH', 1730500000, revoked: true),
          paired('m3', 'DESKTOP-DAUPH', 1729900000),
        ],
        extraLinks: const {
          'm4': LinkState(status: LinkStatus.ready, route: LinkRoute.relay),
        },
      );
      await h.pump(
        tester,
        const HomePage(),
        size: const Size(720, 1600),
        boundary: key,
        locale: locale,
        brightness: b,
        fontFamily: 'Roboto',
      );
      await tester.pumpAndSettle();
      await shot(tester, key, 'home-$tag');
    }, skip: !enabled);

    testWidgets('home empty $tag', (tester) async {
      final key = GlobalKey();
      final h = Harness(
        api: FakeMachineApi(),
        viewState: view(),
        linkState: linkReady,
        machines: const [],
      );
      await h.pump(
        tester,
        const HomePage(),
        size: const Size(720, 1600),
        boundary: key,
        locale: locale,
        brightness: b,
        fontFamily: 'Roboto',
      );
      await tester.pumpAndSettle();
      await shot(tester, key, 'home-empty-$tag');
    }, skip: !enabled);

    Future<GlobalKey> pairApp(WidgetTester tester) async {
      final key = GlobalKey();
      tester.view.physicalSize = const Size(720, 1600);
      tester.view.devicePixelRatio = 2;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [machinesProvider.overrideWith(FailMachines.new)],
          child: RepaintBoundary(
            key: key,
            child: MaterialApp(
              debugShowCheckedModeBanner: false,
              theme: buildOrreryTheme(
                RemoteTheme.fallback,
                b,
                fontFamily: 'Roboto',
              ),
              locale: locale,
              localizationsDelegates: AppLocalizations.localizationsDelegates,
              supportedLocales: AppLocalizations.supportedLocales,
              home: const PairPage(scanner: ColoredBox(color: Colors.black)),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      return key;
    }

    testWidgets('pair $tag', (tester) async {
      final key = await pairApp(tester);
      await shot(tester, key, 'pair-qr-$tag');
      await tester.tap(find.byKey(const ValueKey('pair-tab-1')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const ValueKey('pair-code')),
        '7K3F-92QX',
      );
      await shot(tester, key, 'pair-code-$tag');
      await tester.tap(find.byKey(const ValueKey('pair-code-submit')));
      await tester.pumpAndSettle();
      await shot(tester, key, 'pair-error-$tag');
    }, skip: !enabled);

    Future<GlobalKey> lockApp(
      WidgetTester tester,
      Widget Function(BuildContext, Widget?) builder,
      Widget home,
    ) async {
      final key = GlobalKey();
      tester.view.physicalSize = const Size(720, 1600);
      tester.view.devicePixelRatio = 2;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        RepaintBoundary(
          key: key,
          child: MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: buildOrreryTheme(
              RemoteTheme.fallback,
              b,
              fontFamily: 'Roboto',
            ),
            locale: locale,
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            builder: builder,
            home: home,
          ),
        ),
      );
      return key;
    }

    testWidgets('lock $tag', (tester) async {
      final auth = FakeAuth()..queue.add(AuthOutcome.cancelled);
      final lock = makeLock(auth);
      await lock.load();
      final key = await lockApp(
        tester,
        (context, child) => LockGate(lock: lock, child: child!),
        const Text('secret'),
      );
      await tester.pumpAndSettle();
      await shot(tester, key, 'lock-$tag');
      await tester.pumpWidget(const SizedBox());
      lock.dispose();
    }, skip: !enabled);

    testWidgets('lock settings $tag', (tester) async {
      final lock = makeLock(FakeAuth());
      await lock.load();
      await lock.unlock();
      final key = await lockApp(
        tester,
        (context, child) => child!,
        LockSettingsPage(lock: lock),
      );
      await tester.pumpAndSettle();
      await shot(tester, key, 'lock-settings-$tag');
      await tester.pumpWidget(const SizedBox());
      lock.dispose();
    }, skip: !enabled);
  }
}
