import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/deep_link_host.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';
import 'package:orrery_remote/ui/ui_providers.dart';
import 'package:orrery_remote/ui/workspace/tab_page.dart';
import 'package:orrery_remote/ui/workspace/workspace_page.dart';

import 'support/ui_fakes.dart';
import 'support/ui_samples.dart';

class _Source implements LinkSource {
  final StreamController<Uri> controller = StreamController<Uri>.broadcast();

  @override
  Stream<Uri> get links => controller.stream;
}

const _machine = 'm1';

Future<_Source> _pump(WidgetTester tester, Harness h) async {
  final source = _Source();
  final navigator = GlobalKey<NavigatorState>();
  tester.view.physicalSize = const Size(390, 844);
  tester.view.devicePixelRatio = 1;
  addTearDown(() async {
    await tester.pumpWidget(const SizedBox());
    await source.controller.close();
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
          source: source,
          child: child ?? const SizedBox.shrink(),
        ),
        home: const Scaffold(body: Text('home')),
      ),
    ),
  );
  await tester.pump();
  return source;
}

void main() {
  testWidgets('custom scheme link opens the machine, tab and pane', (
    tester,
  ) async {
    final h = harness();
    final source = await _pump(tester, h);
    source.controller.add(Uri.parse('orrery-remote://m/$_machine/p/b'));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(h.api.attached, contains('b'));
    expect(h.container.read(activeMachineProvider), _machine);
  });

  testWidgets('relay https link opens the same pane', (tester) async {
    final h = harness();
    final source = await _pump(tester, h);
    source.controller.add(
      Uri.parse('https://orrery-relay.jinvu.workers.dev/o/$_machine/c'),
    );
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(h.api.attached, contains('c'));
  });

  testWidgets('unknown machine says it is not paired', (tester) async {
    final h = harness();
    final source = await _pump(tester, h);
    source.controller.add(Uri.parse('orrery-remote://m/other/p/b'));
    await tester.pumpAndSettle();
    expect(find.text('This computer is not paired on this phone'), findsOne);
    expect(find.byType(WorkspacePage), findsNothing);
  });

  testWidgets('forged pane key stays on the workspace and says it is gone', (
    tester,
  ) async {
    final h = harness();
    final source = await _pump(tester, h);
    source.controller.add(Uri.parse('orrery-remote://m/$_machine/p/forged'));
    await tester.pumpAndSettle();
    expect(find.byType(WorkspacePage), findsOneWidget);
    expect(find.byType(TabPage), findsNothing);
    expect(find.text('That pane is no longer open'), findsOneWidget);
    expect(h.api.attached, isEmpty);
  });

  testWidgets('garbage links are ignored', (tester) async {
    final h = harness();
    final source = await _pump(tester, h);
    source.controller.add(Uri.parse('https://evil.example/o/$_machine/b'));
    await tester.pumpAndSettle();
    expect(find.text('home'), findsOneWidget);
    expect(find.byType(WorkspacePage), findsNothing);
  });

  testWidgets('link before the workspace arrives waits for it', (tester) async {
    final h = Harness(
      api: FakeMachineApi(),
      viewState: view(),
      linkState: linkReady,
    );
    final source = await _pump(tester, h);
    source.controller.add(Uri.parse('orrery-remote://m/$_machine/p/a'));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsNothing);
    expect(find.byType(WorkspacePage), findsOneWidget);
    h.viewNotifier.emit(view(ws: sample(), snapshot: dashboard()));
    await tester.pumpAndSettle();
    expect(find.byType(TabPage), findsOneWidget);
    expect(h.api.attached, contains('a'));
  });
}
