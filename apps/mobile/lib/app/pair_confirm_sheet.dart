import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../remote/crypto.dart';
import '../remote/link_state.dart';
import '../remote/pairing.dart';
import '../remote/providers.dart';
import '../ui/kit/buttons.dart';
import '../ui/kit/overlays.dart';
import '../ui/kit/states.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'pair_page.dart';
import 'pair_request.dart';
import 'pair_route_row.dart';

Future<bool?> showPairConfirm(
  BuildContext context, {
  required PairRequest request,
  required QrData info,
  required bool networkBlocked,
}) => showOrrerySheet<bool>(
  context,
  builder: (_) => PairConfirmSheet(
    request: request,
    info: info,
    networkBlocked: networkBlocked,
  ),
);

class PairConfirmSheet extends ConsumerStatefulWidget {
  const PairConfirmSheet({
    super.key,
    required this.request,
    required this.info,
    required this.networkBlocked,
  });

  final PairRequest request;
  final QrData info;
  final bool networkBlocked;

  @override
  ConsumerState<PairConfirmSheet> createState() => _PairConfirmSheetState();
}

class _PairConfirmSheetState extends ConsumerState<PairConfirmSheet> {
  bool _busy = false;
  PairFailure? _failure;
  LinkRoute? _route;

  Future<void> _pair() async {
    if (_busy || widget.networkBlocked) return;
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
            widget.request.qr,
            onRoute: (route) {
              if (mounted) setState(() => _route = route);
            },
          );
      if (!mounted) return;
      navigator.pop(true);
    } on PairException catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _failure = error.failure;
        _route = null;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final text = Theme.of(context).textTheme;
    final req = widget.request;
    final ip = qrFirstIp(widget.info, skipLoopback: !req.fromLink);
    final usbAddress = ip == null && qrHasLoopback(widget.info);
    final sourceText = req.fromShell
        ? l.pairConfirmSourceUsb
        : req.fromLink
        ? l.pairConfirmSourceWeb
        : l.pairConfirmSourceDevice;
    final failure = _failure;
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      child: Column(
        key: const ValueKey('pair-confirm'),
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            l.pairConfirmTitle(widget.info.name),
            key: const ValueKey('pair-confirm-title'),
            style: text.titleMedium,
          ),
          const SizedBox(height: 10),
          Row(
            children: [
              Icon(
                req.fromLink ? Icons.language_rounded : Icons.usb_rounded,
                size: 16,
                color: c.inkMuted,
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  sourceText,
                  key: const ValueKey('pair-confirm-source'),
                  style: TextStyle(color: c.inkMuted),
                ),
              ),
            ],
          ),
          if (ip != null || usbAddress) ...[
            const SizedBox(height: 4),
            Row(
              children: [
                Icon(
                  usbAddress ? Icons.usb_rounded : Icons.wifi_rounded,
                  size: 16,
                  color: c.inkMuted,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    usbAddress
                        ? l.pairConfirmAddressUsb
                        : l.pairConfirmAddress(ip!),
                    key: const ValueKey('pair-confirm-ip'),
                    style: TextStyle(color: c.inkMuted),
                  ),
                ),
              ],
            ),
          ],
          if (widget.networkBlocked) ...[
            const SizedBox(height: 14),
            NoticeCard(
              key: const ValueKey('pair-confirm-blocked'),
              tone: NoticeTone.error,
              title: l.pairWrongNetwork,
            ),
          ],
          if (failure != null) ...[
            const SizedBox(height: 14),
            NoticeCard(
              key: const ValueKey('pair-confirm-error'),
              tone: NoticeTone.error,
              title: pairFailureText(l, failure),
            ),
          ],
          if (_busy) ...[
            const SizedBox(height: 14),
            PairRouteRow(route: _route),
          ],
          const SizedBox(height: 18),
          Row(
            children: [
              Expanded(
                child: OrreryButton(
                  key: const ValueKey('pair-confirm-cancel'),
                  label: l.cancel,
                  kind: ButtonKind.secondary,
                  onPressed: _busy ? null : () => Navigator.pop(context, false),
                ),
              ),
              if (!widget.networkBlocked) ...[
                const SizedBox(width: 10),
                Expanded(
                  child: OrreryButton(
                    key: const ValueKey('pair-confirm-pair'),
                    label: l.pairConfirmAction,
                    busy: _busy,
                    onPressed: _busy ? null : _pair,
                  ),
                ),
              ],
            ],
          ),
        ],
      ),
    );
  }
}
