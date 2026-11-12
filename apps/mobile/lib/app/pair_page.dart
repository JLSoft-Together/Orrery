import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../l10n/app_localizations.dart';
import '../remote/link_state.dart';
import '../remote/pairing.dart';
import '../remote/providers.dart';
import '../ui/kit/kit.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'pair_route_row.dart';

String pairFailureText(AppLocalizations l, PairFailure failure) =>
    switch (failure) {
      PairFailure.badQr => l.pairBadQr,
      PairFailure.expired => l.pairExpired,
      PairFailure.noRoute => l.pairNoRoute,
      PairFailure.network => l.pairNetwork,
      PairFailure.keyMismatch => l.pairKeyMismatch,
      PairFailure.timeout => l.pairTimeout,
      PairFailure.denied => l.pairDenied,
      PairFailure.protocol => l.pairProtocol,
    };

class PairPage extends ConsumerStatefulWidget {
  const PairPage({super.key, this.scanner, this.startOnCode = false});

  final Widget? scanner;
  final bool startOnCode;

  @override
  ConsumerState<PairPage> createState() => _PairPageState();
}

class _PairPageState extends ConsumerState<PairPage> {
  MobileScannerController? _controller;
  final TextEditingController _code = TextEditingController();
  late int _tab = kDebugMode && widget.startOnCode ? 1 : 0;
  bool _busy = false;
  PairFailure? _failure;
  LinkRoute? _route;

  MobileScannerController get _scan => _controller ??= MobileScannerController(
    formats: const [BarcodeFormat.qrCode],
  );

  @override
  void dispose() {
    _code.dispose();
    _controller?.dispose();
    super.dispose();
  }

  Future<void> _onDetect(BarcodeCapture capture) async {
    if (_busy) return;
    final text = capture.barcodes
        .map((b) => b.rawValue)
        .whereType<String>()
        .firstOrNull;
    if (text == null) return;
    await _pair(text);
  }

  Future<void> _pair(String text) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _failure = null;
      _route = null;
    });
    final navigator = Navigator.of(context);
    try {
      await ref
          .read(machinesProvider.notifier)
          .pair(
            text,
            onRoute: (route) {
              if (mounted) setState(() => _route = route);
            },
          );
      if (!mounted) return;
      navigator.pop();
    } on PairException catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _failure = error.failure;
        _route = null;
      });
    }
  }

  Widget _scanTab(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    return Column(
      key: const ValueKey('tab-scan'),
      children: [
        Expanded(
          child: ClipRRect(
            borderRadius: context.shape.cardRadius,
            child: ColoredBox(
              color: c.well,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  widget.scanner ??
                      MobileScanner(controller: _scan, onDetect: _onDetect),
                  IgnorePointer(
                    child: CustomPaint(painter: _Brackets(c.accent)),
                  ),
                ],
              ),
            ),
          ),
        ),
        const SizedBox(height: 16),
        Text(
          l.pairScanTitle,
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.titleSmall,
        ),
        const SizedBox(height: 4),
        Text(
          l.pairScanHint,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 12, color: c.inkMuted),
        ),
        const SizedBox(height: 16),
        NoticeCard(
          tone: NoticeTone.info,
          title: l.pairE2eTitle,
          body: l.pairE2eBody,
        ),
      ],
    );
  }

  Widget _codeTab(BuildContext context) {
    final l = context.l10n;
    return Column(
      key: const ValueKey('tab-code'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l.pairCodeLabel, style: Theme.of(context).textTheme.titleSmall),
        const SizedBox(height: 10),
        TextField(
          key: const ValueKey('pair-code'),
          controller: _code,
          decoration: InputDecoration(labelText: l.pairCodeField),
        ),
        const Spacer(),
        OrreryButton(
          key: const ValueKey('pair-code-submit'),
          label: l.pairCodeSubmit,
          busy: _busy,
          onPressed: _busy ? null : () => _pair(_code.text.trim()),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final failure = _failure;
    return Scaffold(
      appBar: AppBar(title: Text(l.pairMachine)),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 4, 16, 16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (kDebugMode) ...[
                OrrerySegmented(
                  keyPrefix: 'pair-tab',
                  labels: [l.pairTabScan, l.pairTabCode],
                  index: _tab,
                  onChanged: (i) => setState(() => _tab = i),
                ),
                const SizedBox(height: 14),
              ],
              AnimatedSize(
                duration: motion(context),
                curve: context.tokens.out,
                alignment: Alignment.topCenter,
                child: failure == null
                    ? const SizedBox(width: double.infinity)
                    : Padding(
                        padding: const EdgeInsets.only(bottom: 12),
                        child: NoticeCard(
                          key: const ValueKey('pair-error'),
                          tone: NoticeTone.error,
                          title: pairFailureText(l, failure),
                        ),
                      ),
              ),
              Expanded(
                child: AnimatedSwitcher(
                  duration: motion(context),
                  child: _tab == 0 ? _scanTab(context) : _codeTab(context),
                ),
              ),
              if (_busy)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: PairRouteRow(route: _route),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Brackets extends CustomPainter {
  const _Brackets(this.color);

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final side = size.shortestSide * 0.62;
    final r = Rect.fromCenter(
      center: size.center(Offset.zero),
      width: side,
      height: side,
    );
    const len = 28.0;
    final p = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 4
      ..strokeCap = StrokeCap.round;
    for (final (corner, dx, dy) in [
      (r.topLeft, 1.0, 1.0),
      (r.topRight, -1.0, 1.0),
      (r.bottomLeft, 1.0, -1.0),
      (r.bottomRight, -1.0, -1.0),
    ]) {
      canvas.drawLine(corner, corner.translate(len * dx, 0), p);
      canvas.drawLine(corner, corner.translate(0, len * dy), p);
    }
  }

  @override
  bool shouldRepaint(_Brackets old) => old.color != color;
}
