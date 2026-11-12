@Tags(['integration'])
library;

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/crypto_rust.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/remote/remote_host.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/remote/wire.dart';
import 'package:orrery_remote/ui/viewers/explorer_page.dart';

import 'support/fonts.dart';
import 'support/gateway.dart';

Future<void> until(
  WidgetTester tester,
  bool Function() done, [
  String Function()? dump,
]) async {
  for (var i = 0; i < 500; i++) {
    if (done()) return;
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pump(const Duration(milliseconds: 50));
  }
  fail('condition not reached ${dump?.call()}');
}

Future<void> glide(WidgetTester tester) async {
  for (var i = 0; i < 6; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

void main() {
  late Gateway gateway;

  setUpAll(() async {
    await buildGateway();
    await loadFonts();
  });
  setUp(() async => gateway = await Gateway.start());
  tearDown(() => gateway.stop());

  testWidgets('explorer reads a file, saves with base, then gets fs.changed', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final kv = MemorySecureKv();
    const crypto = RustRemoteCrypto();
    final host = RemoteHost(
      deps: ConnectionDeps(
        crypto: crypto,
        connector: IoWireSocket.connect,
        keys: KeyStore(kv, crypto),
        machines: MachineRepository(secure: kv),
        config: const ConnectionConfig(
          handshakeTimeout: Duration(seconds: 5),
          backoff: [Duration(milliseconds: 100)],
        ),
      ),
      cacheDirectory: Directory.systemTemp.createTempSync('ccm_viewer_it'),
      deviceName: 'Viewer test phone',
    );
    addTearDown(() => tester.runAsync(host.dispose));
    await tester.runAsync(() async {
      await host.init();
      await host.pairWithQr(gateway.qr);
    });
    final id = host.machines.all.single.id;
    await tester.runAsync(() => host.setForeground(true));
    await until(
      tester,
      () => host.bundle(id).connection.state.status == LinkStatus.ready,
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [remoteHostProvider.overrideWithValue(host)],
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: ExplorerPage(machineId: id, root: '/work/demo'),
        ),
      ),
    );
    await until(tester, () => find.text('main.dart').evaluate().isNotEmpty);
    await tester.tap(find.text('main.dart'));
    await glide(tester);
    await until(
      tester,
      () => find.byKey(const ValueKey('file-edit')).evaluate().isNotEmpty,
    );
    await tester.tap(find.byKey(const ValueKey('file-edit')));
    await glide(tester);
    await tester.enterText(
      find.byKey(const ValueKey('file-editor')),
      'void main() { print(1); }\n',
    );
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('file-save')));
    await until(
      tester,
      () => find
          .descendant(of: find.byType(SnackBar), matching: find.text('Saved'))
          .evaluate()
          .isNotEmpty,
    );
    await until(
      tester,
      () =>
          find.byKey(const ValueKey('conflict-banner')).evaluate().isEmpty &&
          find
              .textContaining('print', findRichText: true)
              .evaluate()
              .isNotEmpty,
      () => [
        for (final e in find.byType(RichText).evaluate())
          (e.widget as RichText).text.toPlainText(),
        'banner=${find.byKey(const ValueKey('conflict-banner')).evaluate().length}',
      ].join(' | '),
    );
    await tester.tap(find.byKey(const ValueKey('file-edit')));
    await glide(tester);
    await tester.enterText(find.byKey(const ValueKey('file-editor')), 'second');
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('file-save')));
    await until(
      tester,
      () =>
          find
              .descendant(
                of: find.byType(SnackBar),
                matching: find.text('Saved'),
              )
              .evaluate()
              .length ==
          1,
    );
    await tester.pumpWidget(const SizedBox());
    for (var i = 0; i < 6; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump(const Duration(milliseconds: 100));
    }
  });
}
