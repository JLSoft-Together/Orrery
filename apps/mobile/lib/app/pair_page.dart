import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../l10n/app_localizations.dart';
import '../remote/pairing.dart';
import '../remote/providers.dart';

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
  const PairPage({super.key});

  @override
  ConsumerState<PairPage> createState() => _PairPageState();
}

class _PairPageState extends ConsumerState<PairPage> {
  final MobileScannerController _controller = MobileScannerController(
    formats: const [BarcodeFormat.qrCode],
  );
  bool _busy = false;
  String? _message;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _onDetect(BarcodeCapture capture) async {
    if (_busy) return;
    final text = capture.barcodes
        .map((b) => b.rawValue)
        .whereType<String>()
        .firstOrNull;
    if (text == null) return;
    setState(() {
      _busy = true;
      _message = AppLocalizations.of(context).pairing;
    });
    final navigator = Navigator.of(context);
    try {
      await ref.read(machinesProvider.notifier).pair(text);
      if (!mounted) return;
      navigator.pop();
    } on PairException catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _message = pairFailureText(AppLocalizations.of(context), error.failure);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(AppLocalizations.of(context).pairMachine)),
      body: Column(
        children: [
          Expanded(
            child: MobileScanner(controller: _controller, onDetect: _onDetect),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Text(_message ?? AppLocalizations.of(context).pairScanHint),
          ),
        ],
      ),
    );
  }
}
