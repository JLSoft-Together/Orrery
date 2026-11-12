import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/security/device_auth.dart';
import 'package:orrery_remote/security/lock_gate.dart';
import 'package:orrery_remote/security/lock_settings_page.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';

import 'support/fonts.dart';
import 'support/lock_fakes.dart';

final bool enabled = Platform.environment['WP27_SHOTS'] != null;
final String outDir = Platform.environment['WP27_OUT'] ?? 'shots';

Future<void> shot(WidgetTester tester, GlobalKey key, String name) async {
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

Future<GlobalKey> pumpApp(
  WidgetTester tester,
  Widget home,
  Locale locale,
  Brightness brightness,
) async {
  final key = GlobalKey();
  tester.view.physicalSize = const Size(780, 1560);
  tester.view.devicePixelRatio = 2;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    RepaintBoundary(
      key: key,
      child: MaterialApp(
        debugShowCheckedModeBanner: false,
        theme: buildOrreryTheme(
          RemoteTheme.fallback,
          brightness,
          fontFamily: 'Roboto',
        ),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        locale: locale,
        builder: (context, child) => child!,
        home: home,
      ),
    ),
  );
  return key;
}

void main() {
  setUpAll(loadFonts);

  for (final dark in [false, true]) {
    final tag = dark ? 'dark' : 'light';
    final b = dark ? Brightness.dark : Brightness.light;
    final locale = dark ? const Locale('vi') : const Locale('en');

    testWidgets('lock screens $tag', (tester) async {
      final auth = FakeAuth()..hold = null;
      final lock = makeLock(auth);
      await lock.load();
      auth.queue.add(AuthOutcome.cancelled);
      final key = await pumpApp(
        tester,
        LockGate(lock: lock, child: const Text('secret')),
        locale,
        b,
      );
      await tester.pumpAndSettle();
      await shot(tester, key, '01-lock-cancelled-$tag');
      auth.fallback = AuthOutcome.notAvailable;
      await tester.tap(find.byKey(const ValueKey('lock-unlock')));
      await tester.pumpAndSettle();
      await shot(tester, key, '02-lock-no-screen-lock-$tag');
      await tester.tap(find.byKey(const ValueKey('lock-turn-off')));
      await tester.pumpAndSettle();
      await shot(tester, key, '03-lock-turn-off-warn-$tag');
      await tester.pumpWidget(const SizedBox());
      lock.dispose();
    }, skip: !enabled);

    testWidgets('lock settings $tag', (tester) async {
      final lock = makeLock(FakeAuth());
      await lock.load();
      await lock.unlock();
      final key = await pumpApp(
        tester,
        LockSettingsPage(lock: lock),
        locale,
        b,
      );
      await tester.pumpAndSettle();
      await shot(tester, key, '04-settings-default-$tag');
      await tester.tap(find.byKey(const ValueKey('lock-timeout-1')));
      await tester.pumpAndSettle();
      await shot(tester, key, '05-settings-15min-$tag');
      await tester.pumpWidget(const SizedBox());
      lock.dispose();
    }, skip: !enabled);
  }
}
