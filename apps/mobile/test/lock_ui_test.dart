import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/deep_link_host.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/security/app_lock.dart';
import 'package:orrery_remote/security/device_auth.dart';
import 'package:orrery_remote/security/lock_gate.dart';
import 'package:orrery_remote/security/lock_settings_page.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/lock_fakes.dart';
import 'support/ui_samples.dart';

class _Source implements LinkSource {
  final StreamController<Uri> controller = StreamController<Uri>.broadcast();

  @override
  Stream<Uri> get links => controller.stream;
}

Future<void> _size(WidgetTester tester) async {
  tester.view.physicalSize = const Size(390, 844);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
}

MaterialApp _app(Widget home, {TransitionBuilder? builder, Locale? locale}) =>
    MaterialApp(
      theme: buildOrreryTheme(RemoteTheme.fallback, Brightness.light),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      locale: locale ?? const Locale('en'),
      builder: builder,
      home: home,
    );

Future<AppLock> _loaded(FakeAuth auth) async {
  final lock = makeLock(auth);
  await lock.load();
  return lock;
}

void main() {
  testWidgets('gate hides the app, asks on the first frame and opens', (
    tester,
  ) async {
    await _size(tester);
    final auth = FakeAuth();
    final lock = await _loaded(auth);
    await tester.pumpWidget(
      _app(
        const Text('secret'),
        builder: (context, child) => LockGate(lock: lock, child: child!),
      ),
    );
    expect(find.byKey(const ValueKey('lock-screen')), findsOneWidget);
    expect(find.text('secret'), findsNothing);
    await tester.pumpAndSettle();
    expect(auth.calls, 1);
    expect(find.byKey(const ValueKey('lock-screen')), findsNothing);
    expect(find.text('secret'), findsOneWidget);
    lock.dispose();
  });

  testWidgets('a cancelled prompt stays locked and the button retries', (
    tester,
  ) async {
    await _size(tester);
    final auth = FakeAuth()..queue.add(AuthOutcome.cancelled);
    final lock = await _loaded(auth);
    await tester.pumpWidget(
      _app(
        const Text('secret'),
        builder: (context, child) => LockGate(lock: lock, child: child!),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Verification was cancelled.'), findsOneWidget);
    expect(find.text('secret'), findsNothing);
    expect(find.byKey(const ValueKey('lock-turn-off')), findsNothing);
    await tester.tap(find.byKey(const ValueKey('lock-unlock')));
    await tester.pumpAndSettle();
    expect(auth.calls, 2);
    expect(find.text('secret'), findsOneWidget);
    lock.dispose();
  });

  testWidgets(
    'without a screen lock the lock screen explains and can turn off',
    (tester) async {
      await _size(tester);
      final auth = FakeAuth()
        ..fallback = AuthOutcome.notAvailable
        ..insecure = true;
      final lock = await _loaded(auth);
      await tester.pumpWidget(
        _app(
          const Text('secret'),
          builder: (context, child) => LockGate(lock: lock, child: child!),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.textContaining('no screen lock'), findsOneWidget);
      expect(find.text('secret'), findsNothing);
      await tester.tap(find.byKey(const ValueKey('lock-turn-off')));
      await tester.pumpAndSettle();
      expect(find.textContaining('Anyone who holds'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('lock-turn-off-cancel')));
      await tester.pumpAndSettle();
      expect(lock.enabled, isTrue);
      await tester.tap(find.byKey(const ValueKey('lock-turn-off')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('lock-turn-off-confirm')));
      await tester.pumpAndSettle();
      expect(lock.enabled, isFalse);
      expect(find.text('secret'), findsOneWidget);
      lock.dispose();
    },
  );

  testWidgets('vietnamese lock screen', (tester) async {
    await _size(tester);
    final lock = await _loaded(FakeAuth()..fallback = AuthOutcome.failed);
    await tester.pumpWidget(
      _app(
        const Text('secret'),
        locale: const Locale('vi'),
        builder: (context, child) => LockGate(lock: lock, child: child!),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Orrery Remote đang khoá'), findsOneWidget);
    expect(find.text('Xác thực lỗi, thử lại.'), findsOneWidget);
    lock.dispose();
  });

  testWidgets('settings change needs verification first', (tester) async {
    await _size(tester);
    final auth = FakeAuth();
    final lock = await _loaded(auth);
    await lock.unlock();
    auth.calls = 0;
    await tester.pumpWidget(_app(LockSettingsPage(lock: lock)));
    await tester.pumpAndSettle();
    auth.queue.add(AuthOutcome.cancelled);
    await tester.tap(find.byKey(const ValueKey('lock-switch')));
    await tester.pumpAndSettle();
    expect(auth.calls, 1);
    expect(lock.enabled, isTrue);
    auth.queue.add(AuthOutcome.failed);
    await tester.tap(find.byKey(const ValueKey('lock-timeout-3')));
    await tester.pumpAndSettle();
    expect(lock.timeout, LockTimeout.hour1);
    await tester.tap(find.byKey(const ValueKey('lock-timeout-3')));
    await tester.pumpAndSettle();
    expect(lock.timeout, LockTimeout.hours4);
    expect(auth.calls, 3);
    await tester.tap(find.byKey(const ValueKey('lock-switch')));
    await tester.pumpAndSettle();
    expect(lock.enabled, isFalse);
    expect(auth.calls, 4);
    lock.dispose();
  });

  testWidgets('turning off on a phone with no screen lock warns first', (
    tester,
  ) async {
    await _size(tester);
    final auth = FakeAuth();
    final lock = await _loaded(auth);
    await lock.unlock();
    await tester.pumpWidget(_app(LockSettingsPage(lock: lock)));
    await tester.pumpAndSettle();
    auth.fallback = AuthOutcome.notAvailable;
    auth.insecure = true;
    await tester.tap(find.byKey(const ValueKey('lock-switch')));
    await tester.pumpAndSettle();
    expect(find.text('Turn off app lock?'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(lock.enabled, isTrue);
    await tester.tap(find.byKey(const ValueKey('lock-switch')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('lock-turn-off-confirm')));
    await tester.pumpAndSettle();
    expect(lock.enabled, isFalse);
    lock.dispose();
  });

  testWidgets('a pane link tapped while locked opens only after unlock', (
    tester,
  ) async {
    await _size(tester);
    final h = harness();
    final source = _Source();
    final navigator = GlobalKey<NavigatorState>();
    final auth = FakeAuth()..queue.add(AuthOutcome.cancelled);
    final lock = await _loaded(auth);
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox());
      await source.controller.close();
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
          builder: (context, child) => LockGate(
            lock: lock,
            child: DeepLinkHost(
              navigatorKey: navigator,
              source: source,
              lock: lock,
              child: child ?? const SizedBox.shrink(),
            ),
          ),
          home: const Scaffold(body: Text('home')),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(lock.locked, isTrue);
    source.controller.add(Uri.parse('orrery-remote://m/m1/p/b'));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsNothing);
    expect(find.byType(WorkspacePage), findsNothing);
    expect(h.api.attached, isEmpty);
    await tester.tap(find.byKey(const ValueKey('lock-unlock')));
    await tester.pumpAndSettle();
    expect(lock.locked, isFalse);
    expect(find.byType(TabPage), findsOneWidget);
    expect(h.api.attached, contains('b'));
    lock.dispose();
  });

  testWidgets('turn off is refused when the device turns out to be secure', (
    tester,
  ) async {
    await _size(tester);
    final auth = FakeAuth()
      ..fallback = AuthOutcome.notAvailable
      ..insecure = true;
    final lock = await _loaded(auth);
    await tester.pumpWidget(
      _app(
        const Text('secret'),
        builder: (context, child) => LockGate(lock: lock, child: child!),
      ),
    );
    await tester.pumpAndSettle();
    auth.insecure = false;
    await tester.tap(find.byKey(const ValueKey('lock-turn-off')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('lock-turn-off-confirm')), findsNothing);
    expect(lock.enabled, isTrue);
    expect(find.text('secret'), findsNothing);
    lock.dispose();
  });
}
