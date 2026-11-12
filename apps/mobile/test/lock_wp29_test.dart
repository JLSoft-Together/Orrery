import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/security/device_auth.dart';
import 'package:orrery_remote/security/lock_gate.dart';
import 'package:orrery_remote/ui/kit/kit.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';

import 'support/lock_fakes.dart';

Widget app(Widget Function(BuildContext, Widget?) builder) => MaterialApp(
  theme: buildOrreryTheme(RemoteTheme.fallback, Brightness.light),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  builder: builder,
  home: const Text('secret'),
);

void main() {
  testWidgets('lock screen fades in and shakes only on a failed check', (
    tester,
  ) async {
    final auth = FakeAuth()
      ..queue.addAll([AuthOutcome.cancelled, AuthOutcome.failed]);
    final lock = makeLock(auth);
    await lock.load();
    await tester.pumpWidget(
      app((context, child) => LockGate(lock: lock, child: child!)),
    );
    expect(find.byType(FadeIn), findsOneWidget);
    await tester.pumpAndSettle();
    int shakes() => tester.widget<ShakeBox>(find.byType(ShakeBox)).trigger;
    expect(shakes(), 0);
    await tester.tap(find.byKey(const ValueKey('lock-unlock')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 40));
    expect(shakes(), 1);
    await tester.pumpAndSettle();
    lock.dispose();
  });
}
