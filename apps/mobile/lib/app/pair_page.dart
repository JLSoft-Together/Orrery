import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../remote/pairing.dart';
import '../remote/providers.dart';

String pairFailureText(PairFailure failure) => switch (failure) {
  PairFailure.badQr => 'Mã QR không đúng',
  PairFailure.expired => 'Mã QR đã hết hạn, tạo mã mới trên máy tính',
  PairFailure.noRoute => 'Không thấy máy trong mạng LAN',
  PairFailure.network => 'Mất kết nối khi ghép, quét lại mã',
  PairFailure.keyMismatch => 'Khoá máy không khớp mã QR, dừng ghép',
  PairFailure.timeout => 'Máy không trả lời kịp',
  PairFailure.denied => 'Máy tính từ chối',
  PairFailure.protocol => 'Lỗi giao thức khi ghép',
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
      _message = 'Đang ghép...';
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
        _message = pairFailureText(error.failure);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Ghép máy')),
      body: Column(
        children: [
          Expanded(
            child: MobileScanner(controller: _controller, onDetect: _onDetect),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Text(_message ?? 'Quét mã QR trong Settings của Orrery'),
          ),
        ],
      ),
    );
  }
}
