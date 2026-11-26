import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/away_summary.dart';
import 'package:orrery_remote/ui/away/away_card.dart';

AwaySummary summary({bool partial = false}) => AwaySummary(
  since: DateTime(2026, 11, 27, 14, 5),
  until: DateTime(2026, 11, 27, 15, 25),
  done: const ['a', 'b'],
  waiting: const ['c'],
  failed: const ['d'],
  completedAgents: 2,
  agentDoneIds: const [],
  compactions: 1,
  compactIds: const ['a'],
  partial: partial,
);

Widget host(Widget child, {Locale locale = const Locale('vi')}) => MaterialApp(
  locale: locale,
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: SingleChildScrollView(child: child)),
);

void main() {
  testWidgets('card shows title, chips with counts and tap targets >= 48', (
    tester,
  ) async {
    AwayGroup? picked;
    var seen = 0;
    await tester.pumpWidget(
      host(
        AwayCard(
          summary: summary(),
          selected: null,
          onSelect: (g) => picked = g,
          onSeen: () => seen++,
        ),
      ),
    );
    expect(find.byKey(const ValueKey('away-card')), findsOneWidget);
    expect(find.text('Từ 14:05 (1g 20p trước)'), findsOneWidget);
    expect(find.text('Xong 2'), findsOneWidget);
    expect(find.text('Chờ bạn 1'), findsOneWidget);
    expect(find.text('Lỗi 1'), findsOneWidget);
    expect(find.text('Compact 1'), findsOneWidget);
    expect(find.byKey(const ValueKey('away-partial')), findsNothing);
    for (final k in ['done', 'waiting', 'failed', 'compact']) {
      final size = tester.getSize(find.byKey(ValueKey('away-chip-$k')));
      expect(size.height, greaterThanOrEqualTo(48));
    }
    final seenSize = tester.getSize(find.byKey(const ValueKey('away-seen')));
    expect(seenSize.height, greaterThanOrEqualTo(48));
    await tester.tap(find.byKey(const ValueKey('away-chip-waiting')));
    expect(picked, AwayGroup.waiting);
    await tester.tap(find.byKey(const ValueKey('away-seen')));
    expect(seen, 1);
  });

  testWidgets('group without ids is shown but not tappable', (tester) async {
    AwayGroup? picked;
    await tester.pumpWidget(
      host(
        AwayCard(
          summary: summary(),
          selected: null,
          onSelect: (g) => picked = g,
          onSeen: () {},
        ),
      ),
    );
    expect(find.text('Agent con xong 2'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('away-chip-agentsDone')));
    expect(picked, isNull);
  });

  testWidgets('partial summary shows the incomplete note', (tester) async {
    await tester.pumpWidget(
      host(
        AwayCard(
          summary: summary(partial: true),
          selected: AwayGroup.done,
          onSelect: (_) {},
          onSeen: () {},
        ),
      ),
    );
    expect(find.text('Số liệu có thể thiếu'), findsOneWidget);
  });

  testWidgets('english copy and 360 dp width without overflow', (tester) async {
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      host(
        locale: const Locale('en'),
        AwayCard(
          summary: summary(partial: true),
          selected: null,
          onSelect: (_) {},
          onSeen: () {},
        ),
      ),
    );
    expect(tester.takeException(), isNull);
    expect(find.text('Done 2'), findsOneWidget);
  });
}
