import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/ui/kit/kit.dart';
import 'package:orrery_remote/ui/theme/orrery_shape.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';

import 'support/fonts.dart';

final bool enabled = Platform.environment['WP29_SHOTS'] != null;
final String outDir = Platform.environment['WP29_OUT'] ?? 'shots';

Widget sheet() => Builder(
  builder: (context) => Scaffold(
    body: SafeArea(
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text('Kit', style: Theme.of(context).textTheme.headlineMedium),
          const SectionLabel('Chip trạng thái', count: 5),
          const Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              OrreryChip(
                tone: ChipTone.working,
                label: '2 đang chạy',
                icon: Icons.autorenew_rounded,
              ),
              OrreryChip(
                tone: ChipTone.waiting,
                label: '1 chờ bạn',
                icon: Icons.schedule_rounded,
              ),
              OrreryChip(
                tone: ChipTone.idle,
                label: '2 rảnh',
                icon: Icons.nightlight_outlined,
              ),
              OrreryChip(
                tone: ChipTone.done,
                label: 'Xong',
                icon: Icons.check_circle_outline_rounded,
              ),
              OrreryChip(
                tone: ChipTone.error,
                label: 'Lỗi',
                icon: Icons.error_outline_rounded,
              ),
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
              LinkPill(link: LinkState(status: LinkStatus.reconnecting)),
            ],
          ),
          const SectionLabel('Thẻ'),
          OrreryCard(
            barColor: context.orrery.agentClaude,
            onTap: () {},
            child: const Row(
              children: [
                Expanded(child: Text('DESKTOP-DAUPH')),
                OrreryChip(
                  tone: ChipTone.info,
                  label: 'Relay · 84 ms',
                  icon: Icons.cloud_outlined,
                ),
              ],
            ),
          ),
          const SectionLabel('Nút'),
          OrreryButton(
            label: 'Ghép máy',
            icon: Icons.qr_code_scanner_rounded,
            onPressed: () {},
          ),
          const SizedBox(height: 8),
          OrreryButton(
            label: 'Nhập mã thủ công',
            kind: ButtonKind.secondary,
            onPressed: () {},
          ),
          const SizedBox(height: 8),
          OrreryButton(
            label: 'Dọn 2 bản cũ',
            icon: Icons.delete_outline_rounded,
            kind: ButtonKind.danger,
            onPressed: () {},
          ),
          const SizedBox(height: 12),
          const ConnectionBanner(
            link: LinkState(status: LinkStatus.reconnecting, attempt: 2),
            title: 'Mất kết nối tới DESKTOP-DAUPH',
            detail: 'Đang nối lại qua relay.',
            retryLabel: 'Thử lại',
          ),
          const NoticeCard(
            tone: NoticeTone.error,
            title: 'Mã ghép đã hết hạn',
            body: 'Mã chỉ dùng được 5 phút.',
          ),
          const SizedBox(height: 8),
          const NoticeCard(
            tone: NoticeTone.info,
            title: 'Kết nối mã hoá đầu cuối',
            body: 'Relay chỉ chuyển byte đã mã hoá.',
          ),
          const SizedBox(height: 8),
          const SizedBox(
            height: 220,
            child: EmptyState(
              title: 'Chưa có máy nào',
              body: 'Mở Orrery trên máy tính rồi quét mã QR.',
            ),
          ),
        ],
      ),
    ),
  ),
);

void main() {
  setUpAll(loadFonts);
  for (final dark in [false, true]) {
    final tag = dark ? 'dark' : 'light';
    testWidgets('kit sheet $tag', (tester) async {
      final key = GlobalKey();
      tester.view.physicalSize = const Size(720, 1800);
      tester.view.devicePixelRatio = 2;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        RepaintBoundary(
          key: key,
          child: MaterialApp(
            debugShowCheckedModeBanner: false,
            locale: const Locale('vi'),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            theme: buildOrreryTheme(
              RemoteTheme.fallback,
              dark ? Brightness.dark : Brightness.light,
              fontFamily: 'Roboto',
            ),
            builder: (context, child) => MediaQuery(
              data: MediaQuery.of(context).copyWith(disableAnimations: true),
              child: child!,
            ),
            home: sheet(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.runAsync(() async {
        final boundary =
            key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
        final image = await boundary.toImage(pixelRatio: 2);
        final data = await image.toByteData(format: ui.ImageByteFormat.png);
        final file = File('$outDir/kit-$tag.png');
        await file.create(recursive: true);
        await file.writeAsBytes(data!.buffer.asUint8List());
      });
    }, skip: !enabled);
  }
}
