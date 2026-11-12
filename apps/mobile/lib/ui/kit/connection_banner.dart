import 'package:flutter/material.dart';

import '../../remote/link_state.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import '../widgets/link_widgets.dart';
import 'buttons.dart';
import 'status_chip.dart';

class LinkPill extends StatelessWidget {
  const LinkPill({super.key, required this.link, this.latencyMs});

  final LinkState link;
  final int? latencyMs;

  @override
  Widget build(BuildContext context) {
    final (tone, icon, pulse) = switch (link.status) {
      LinkStatus.ready => (
        link.route == LinkRoute.relay ? ChipTone.info : ChipTone.done,
        link.route == LinkRoute.relay
            ? Icons.cloud_outlined
            : Icons.wifi_rounded,
        false,
      ),
      LinkStatus.discovering ||
      LinkStatus.handshaking ||
      LinkStatus.reconnecting => (ChipTone.waiting, Icons.sync_rounded, true),
      LinkStatus.revoked ||
      LinkStatus.protoTooOld => (ChipTone.error, Icons.block_rounded, false),
      _ => (ChipTone.idle, Icons.cloud_off_rounded, false),
    };
    final base = linkLabel(context, link);
    final ms = latencyMs;
    final label =
        link.status == LinkStatus.ready &&
            link.route == LinkRoute.lan &&
            ms != null
        ? '$base · $ms ms'
        : base;
    return OrreryChip(tone: tone, icon: icon, label: label, pulse: pulse);
  }
}

class ConnectionBanner extends StatelessWidget {
  const ConnectionBanner({
    super.key,
    required this.link,
    required this.title,
    this.detail,
    this.retryLabel,
    this.onRetry,
  });

  final LinkState link;
  final String title;
  final String? detail;
  final String? retryLabel;
  final VoidCallback? onRetry;

  bool get _connecting =>
      link.status == LinkStatus.discovering ||
      link.status == LinkStatus.handshaking ||
      link.status == LinkStatus.reconnecting;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final show = link.status != LinkStatus.ready && !link.isTerminal;
    final color = _connecting ? c.waiting : c.idle;
    final body = !show
        ? const SizedBox(width: double.infinity)
        : Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: color.withValues(alpha: 0.1),
                borderRadius: context.shape.cardRadius,
                border: Border.all(color: color.withValues(alpha: 0.38)),
              ),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(12, 10, 8, 10),
                child: Row(
                  children: [
                    Icon(Icons.cloud_off_rounded, size: 20, color: color),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            title,
                            style: TextStyle(
                              fontSize: 14,
                              height: 20 / 14,
                              fontWeight: FontWeight.w600,
                              color: c.ink,
                            ),
                          ),
                          if (detail != null)
                            Text(
                              detail!,
                              style: TextStyle(
                                fontSize: 12,
                                height: 16 / 12,
                                color: c.inkMuted,
                              ),
                            ),
                        ],
                      ),
                    ),
                    if (onRetry != null && retryLabel != null)
                      OrreryButton(
                        label: retryLabel!,
                        kind: ButtonKind.text,
                        expand: false,
                        onPressed: onRetry,
                      ),
                  ],
                ),
              ),
            ),
          );
    if (reduceMotion(context)) return body;
    return AnimatedSize(
      duration: motion(context),
      curve: context.tokens.out,
      alignment: Alignment.topCenter,
      child: body,
    );
  }
}
