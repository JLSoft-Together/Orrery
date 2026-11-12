@Tags(['integration'])
library;

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/main.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/crypto_rust.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/remote/remote_host.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/remote/wire.dart';
import 'package:xterm/xterm.dart';

import 'support/fonts.dart';
import 'support/gateway.dart';

String screen(WidgetTester tester) {
  final lines = tester
      .widget<TerminalView>(find.byType(TerminalView))
      .terminal
      .buffer
      .lines;
  return [for (var i = 0; i < lines.length; i++) lines[i].getText()].join('\n');
}

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
  for (var i = 0; i < 8; i++) {
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

  testWidgets(
    'open tab, attach, type, echo with claim, close pane after confirm',
    (tester) async {
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
        cacheDirectory: Directory.systemTemp.createTempSync('ccm_ui_it'),
        deviceName: 'UI test phone',
      );
      addTearDown(() => tester.runAsync(host.dispose));
      await tester.runAsync(() async {
        await host.init();
        await host.pairWithQr(gateway.qr);
      });
      await tester.pumpWidget(
        ProviderScope(
          overrides: [remoteHostProvider.overrideWithValue(host)],
          child: OrreryRemoteApp(
            rustStatus: '',
            host: host,
            linkSource: const _NoLinks(),
          ),
        ),
      );
      await until(
        tester,
        () => find.text('Fake Desktop').evaluate().isNotEmpty,
      );
      await glide(tester);
      await tester.tap(find.text('Fake Desktop'));
      await glide(tester);
      await until(
        tester,
        () =>
            find.byKey(const ValueKey('pane-open-full')).evaluate().isNotEmpty,
      );
      await glide(tester);
      await tester.tap(find.byKey(const ValueKey('pane-open-full')));
      await glide(tester);
      await until(
        tester,
        () => find.byType(TerminalView).evaluate().isNotEmpty,
      );
      await until(tester, () => screen(tester).contains('replay'));

      await glide(tester);
      await tester.enterText(find.byKey(const ValueKey('input')), 'ls');
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('send')));
      await until(
        tester,
        () => screen(tester).contains('claim+echo:ls'),
        () => screen(tester),
      );

      await tester.tap(find.byTooltip('Tab actions'));
      await glide(tester);
      await tester.tap(find.byKey(const ValueKey('action-close')));
      await glide(tester);
      await glide(tester);
      expect(find.text('Close pane'), findsWidgets);
      await tester.tap(find.widgetWithText(FilledButton, 'Close pane'));
      await until(
        tester,
        () => find.text('Close this pane?').evaluate().isNotEmpty,
      );
      await glide(tester);
      await tester.tap(find.text('Close'));
      final bundle = host.bundle(host.machines.all.single.id);
      await until(tester, () {
        final tabs = bundle.store.view.workspace?['tabs'] as List?;
        final panes = tabs?.isEmpty ?? true
            ? null
            : (tabs!.first as Map)['panes'] as List;
        return panes != null && panes.isEmpty;
      });
      await tester.pumpWidget(const SizedBox());
      for (var i = 0; i < 6; i++) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 100)),
        );
        await tester.pump(const Duration(milliseconds: 100));
      }
    },
  );
}

class _NoLinks implements LinkSource {
  const _NoLinks();

  @override
  Stream<Uri> get links => const Stream<Uri>.empty();
}
