import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/install/apk_flow.dart';
import 'package:orrery_remote/install/apk_install_sheet.dart';
import 'package:orrery_remote/install/apk_io.dart';
import 'package:orrery_remote/install/apk_offer.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';

class _Installer implements ApkInstaller {
  bool allowed = false;
  final List<String> installed = [];

  @override
  Future<bool> canInstall() async => allowed;

  @override
  Future<void> openSettings() async {}

  @override
  Future<int?> freeBytes(String path) async => null;

  @override
  Future<InstallStart> install(String path) async {
    installed.add(path);
    return InstallStart.started;
  }
}

class _Source implements ApkSource {
  @override
  Future<ApkResponse> open(Uri url) async =>
      throw const SocketException('offline');

  @override
  void close() {}
}

Widget host(Widget child, [Locale locale = const Locale('en')]) => MaterialApp(
  locale: locale,
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: child),
);

void main() {
  final offer = ApkOffer.parse({
    'urls': ['http://192.168.1.5:4000/apk/t'],
    'size': 2048,
    'sha256': 'a' * 64,
    'app': 'dev.test.app',
    'variant': 'debug',
    'exp': DateTime.now().millisecondsSinceEpoch ~/ 1000 + 600,
  })!;

  testWidgets(
    'permission missing shows the settings button and retries after',
    (tester) async {
      final dir = Directory.systemTemp.createTempSync('apk_sheet');
      addTearDown(() => dir.deleteSync(recursive: true));
      final installer = _Installer();
      var opened = 0;
      final flow = ApkInstallFlow(
        offer: offer,
        source: _Source(),
        installer: installer,
        dir: dir,
      );
      addTearDown(flow.dispose);
      await tester.pumpWidget(
        host(
          ApkInstallSheet(
            flow: flow,
            autoStart: false,
            openSettings: () async => opened++,
          ),
        ),
      );
      expect(find.text('Install build'), findsOneWidget);
      expect(find.textContaining('dev.test.app, debug'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('apk-start')));
      await tester.pump();
      await tester.pump();
      expect(flow.state.phase, ApkPhase.needPermission);
      expect(find.byKey(const ValueKey('apk-settings')), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('apk-settings')));
      await tester.pump();
      expect(opened, 1);
      expect(installer.installed, isEmpty);
    },
  );

  testWidgets('off Wi-Fi shows the same-Wi-Fi message in Vietnamese', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('apk_sheet');
    addTearDown(() => dir.deleteSync(recursive: true));
    final flow = ApkInstallFlow(
      offer: offer,
      source: _Source(),
      installer: _Installer()..allowed = true,
      dir: dir,
    );
    addTearDown(flow.dispose);
    await tester.pumpWidget(
      host(
        ApkInstallSheet(
          flow: flow,
          autoStart: false,
          openSettings: () async {},
        ),
        const Locale('vi'),
      ),
    );
    await tester.runAsync(flow.start);
    await tester.pump();
    expect(find.text('Cần cùng Wi-Fi với máy tính để tải.'), findsOneWidget);
    expect(find.byKey(const ValueKey('apk-start')), findsOneWidget);
  });
}
