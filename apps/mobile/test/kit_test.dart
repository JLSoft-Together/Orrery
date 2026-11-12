import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/ui/kit/kit.dart';
import 'package:orrery_remote/ui/theme/orrery_colors.dart';
import 'package:orrery_remote/ui/theme/orrery_shape.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';

Widget host(
  Widget child, {
  Brightness brightness = Brightness.light,
  bool quiet = false,
}) => MaterialApp(
  theme: buildOrreryTheme(RemoteTheme.fallback, brightness),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  builder: (context, app) => MediaQuery(
    data: MediaQuery.of(context).copyWith(disableAnimations: quiet),
    child: app!,
  ),
  home: Scaffold(body: child),
);

void main() {
  group('theme', () {
    testWidgets('light uses orrery-light tokens and radius 8', (tester) async {
      late BuildContext captured;
      await tester.pumpWidget(
        host(
          Builder(
            builder: (context) {
              captured = context;
              return const SizedBox();
            },
          ),
        ),
      );
      final theme = Theme.of(captured);
      expect(theme.colorScheme.primary, OrreryColors.light.accent);
      expect(theme.scaffoldBackgroundColor, const Color(0xFFF3F3F6));
      expect(captured.shape.control, 8);
      expect(captured.shape.card, 10);
      expect(captured.orrery.well, const Color(0xFF18181C));
    });

    testWidgets('dark uses galaxy tokens', (tester) async {
      late BuildContext captured;
      await tester.pumpWidget(
        host(
          Builder(
            builder: (context) {
              captured = context;
              return const SizedBox();
            },
          ),
          brightness: Brightness.dark,
        ),
      );
      final theme = Theme.of(captured);
      expect(theme.colorScheme.primary, const Color(0xFFE8835D));
      expect(theme.scaffoldBackgroundColor, const Color(0xFF000000));
      expect(captured.orrery.surface, const Color(0xFF08080A));
      expect(captured.statusColors.working, const Color(0xFFF5A524));
    });

    test('lerp mixes colours and keeps motion tokens in range', () {
      final mid = OrreryColors.light.lerp(OrreryColors.dark, 0.5);
      expect(mid.canvas, isNot(OrreryColors.light.canvas));
      const motion = OrreryMotion();
      for (final d in [motion.fast, motion.base, motion.slow, motion.pop]) {
        expect(d.inMilliseconds, inInclusiveRange(150, 300));
      }
    });
  });

  group('chip', () {
    testWidgets('changes label, icon and colour with animation', (
      tester,
    ) async {
      final tone = ValueNotifier(ChipTone.idle);
      await tester.pumpWidget(
        host(
          ValueListenableBuilder(
            valueListenable: tone,
            builder: (context, value, _) => OrreryChip(
              tone: value,
              label: value == ChipTone.idle ? 'Rảnh' : 'Xong',
              icon: value == ChipTone.idle
                  ? Icons.nightlight_outlined
                  : Icons.check_circle_outline_rounded,
            ),
          ),
        ),
      );
      expect(find.text('Rảnh'), findsOneWidget);
      tone.value = ChipTone.done;
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 110));
      expect(find.text('Xong'), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('Rảnh'), findsNothing);
      final text = tester.widget<Text>(find.text('Xong'));
      expect(text.style!.color, OrreryColors.light.done);
      expect(find.byIcon(Icons.check_circle_outline_rounded), findsOneWidget);
    });

    testWidgets('pulse chip settles when reduce motion is on', (tester) async {
      await tester.pumpWidget(
        host(
          const OrreryChip(
            tone: ChipTone.waiting,
            label: 'Đang nối',
            pulse: true,
          ),
          quiet: true,
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(PulseDot), findsOneWidget);
    });

    testWidgets('pulse dot animates opacity when motion is allowed', (
      tester,
    ) async {
      await tester.pumpWidget(host(const PulseDot(color: Colors.orange)));
      Opacity opacity() => tester.widget<Opacity>(
        find.descendant(
          of: find.byType(PulseDot),
          matching: find.byType(Opacity),
        ),
      );
      final first = opacity().opacity;
      await tester.pump(const Duration(milliseconds: 600));
      expect(opacity().opacity, lessThan(first));
      await tester.pumpWidget(const SizedBox());
    });
  });

  group('animated items', () {
    Widget list(List<String> names) => host(
      SingleChildScrollView(
        child: AnimatedItems<String>(
          items: names,
          keyOf: (s) => s,
          builder: (context, s) => SizedBox(height: 40, child: Text(s)),
        ),
      ),
    );

    testWidgets('adds slide in, removals leave then disappear', (tester) async {
      await tester.pumpWidget(list(['a', 'b']));
      expect(find.text('a'), findsOneWidget);
      await tester.pumpWidget(list(['c', 'a', 'b']));
      await tester.pump(const Duration(milliseconds: 20));
      expect(find.text('c'), findsOneWidget);
      final fade = tester.widget<FadeTransition>(
        find
            .ancestor(of: find.text('c'), matching: find.byType(FadeTransition))
            .first,
      );
      expect(fade.opacity.value, lessThan(1));
      await tester.pump(const Duration(milliseconds: 300));
      expect(fade.opacity.value, 1);

      await tester.pumpWidget(list(['c', 'b']));
      await tester.pump(const Duration(milliseconds: 20));
      expect(find.text('a'), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('a'), findsNothing);
      expect(find.text('c'), findsOneWidget);
      expect(find.text('b'), findsOneWidget);
    });

    testWidgets('no animation lingers when reduce motion is on', (
      tester,
    ) async {
      Widget quietList(List<String> names) => host(
        AnimatedItems<String>(
          items: names,
          keyOf: (s) => s,
          builder: (context, s) => Text(s),
        ),
        quiet: true,
      );
      await tester.pumpWidget(quietList(['a', 'b']));
      await tester.pumpWidget(quietList(['b']));
      await tester.pump();
      expect(find.text('a'), findsNothing);
    });

    testWidgets('rename cross-fades to the new text', (tester) async {
      Widget rename(String t) => host(AnimatedRename(text: t));
      await tester.pumpWidget(rename('DESKTOP'));
      await tester.pumpWidget(rename('Máy bàn'));
      await tester.pump(const Duration(milliseconds: 20));
      expect(find.text('DESKTOP'), findsOneWidget);
      expect(find.text('Máy bàn'), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.text('DESKTOP'), findsNothing);
    });
  });

  group('banner', () {
    testWidgets('shows while reconnecting and hides when ready', (
      tester,
    ) async {
      Widget banner(LinkState link) => host(
        ConnectionBanner(
          link: link,
          title: 'Mất kết nối',
          detail: 'Đang nối lại',
          retryLabel: 'Thử lại',
          onRetry: () {},
        ),
        quiet: true,
      );
      await tester.pumpWidget(
        banner(const LinkState(status: LinkStatus.reconnecting, attempt: 2)),
      );
      await tester.pumpAndSettle();
      expect(find.text('Mất kết nối'), findsOneWidget);
      expect(find.text('Thử lại'), findsOneWidget);
      await tester.pumpWidget(
        banner(const LinkState(status: LinkStatus.ready, route: LinkRoute.lan)),
      );
      await tester.pumpAndSettle();
      expect(find.text('Mất kết nối'), findsNothing);
    });

    testWidgets('link pill names the route and only shows a measured latency', (
      tester,
    ) async {
      await tester.pumpWidget(
        host(
          const Column(
            children: [
              LinkPill(
                link: LinkState(status: LinkStatus.ready, route: LinkRoute.lan),
                latencyMs: 12,
              ),
              LinkPill(
                link: LinkState(
                  status: LinkStatus.ready,
                  route: LinkRoute.relay,
                ),
              ),
            ],
          ),
        ),
      );
      expect(find.text('LAN · 12 ms'), findsOneWidget);
      expect(find.text('Relay'), findsOneWidget);
    });
  });

  group('overlays and misc', () {
    testWidgets('confirm dialog returns the choice', (tester) async {
      bool? result;
      await tester.pumpWidget(
        host(
          Builder(
            builder: (context) => TextButton(
              onPressed: () async => result = await confirmOrrery(
                context,
                title: 'Gỡ máy?',
                body: 'Điện thoại sẽ quên máy này.',
                confirmLabel: 'Gỡ',
                cancelLabel: 'Huỷ',
                danger: true,
              ),
              child: const Text('open'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expect(find.text('Gỡ máy?'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('orrery-dialog-confirm')));
      await tester.pumpAndSettle();
      expect(result, true);
    });

    testWidgets('sheet opens and closes', (tester) async {
      await tester.pumpWidget(
        host(
          Builder(
            builder: (context) => TextButton(
              onPressed: () => showOrrerySheet<void>(
                context,
                builder: (_) =>
                    const SizedBox(height: 120, child: Text('sheet')),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expect(find.text('sheet'), findsOneWidget);
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();
      expect(find.text('sheet'), findsNothing);
    });

    testWidgets('shake moves the child and ends at rest', (tester) async {
      Widget box(int n) => host(
        ShakeBox(
          trigger: n,
          child: const SizedBox(key: ValueKey('x')),
        ),
      );
      await tester.pumpWidget(box(0));
      final rest = tester.getTopLeft(find.byKey(const ValueKey('x'))).dx;
      await tester.pumpWidget(box(1));
      await tester.pump(const Duration(milliseconds: 40));
      expect(
        tester.getTopLeft(find.byKey(const ValueKey('x'))).dx,
        isNot(rest),
      );
      await tester.pump(const Duration(milliseconds: 300));
      expect(tester.getTopLeft(find.byKey(const ValueKey('x'))).dx, rest);
    });

    testWidgets('button busy disables taps, empty state shows actions', (
      tester,
    ) async {
      var taps = 0;
      await tester.pumpWidget(
        host(
          Column(
            children: [
              OrreryButton(label: 'Ghép', busy: true, onPressed: () => taps++),
              const Expanded(
                child: EmptyState(title: 'Chưa có máy nào', body: 'Quét QR'),
              ),
            ],
          ),
          quiet: true,
        ),
      );
      await tester.tap(find.text('Ghép'));
      expect(taps, 0);
      expect(find.text('Chưa có máy nào'), findsOneWidget);
    });
  });
}
