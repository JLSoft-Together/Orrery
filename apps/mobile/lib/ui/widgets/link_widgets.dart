import 'package:flutter/material.dart';

import '../../remote/link_state.dart';
import '../theme/orrery_theme.dart';
import 'common.dart';

String linkLabel(BuildContext context, LinkState link) {
  final l = context.l10n;
  return switch (link.status) {
    LinkStatus.idle => l.linkIdle,
    LinkStatus.discovering => l.linkDiscovering,
    LinkStatus.handshaking => l.linkHandshaking,
    LinkStatus.ready => link.route == LinkRoute.relay ? l.linkRelay : l.linkLan,
    LinkStatus.reconnecting => l.linkReconnecting(link.attempt),
    LinkStatus.offline => l.linkOffline,
    LinkStatus.protoTooOld => l.linkProtoTooOld,
    LinkStatus.revoked => l.linkRevoked,
  };
}

String reasonLabel(BuildContext context, LinkState link) {
  final l = context.l10n;
  return switch (link.reason) {
    LinkReason.desktopAbsent => l.reasonDesktopAbsent,
    LinkReason.remoteOff => l.reasonRemoteOff,
    LinkReason.restarting => l.reasonRestarting,
    LinkReason.tokenExpired => l.reasonTokenExpired,
    LinkReason.rateLimited => l.reasonRateLimited,
    LinkReason.networkLost => l.reasonNetworkLost,
    LinkReason.noRoute => l.reasonNoRoute,
    LinkReason.handshakeFailed => l.reasonHandshakeFailed,
    _ => '',
  };
}

class LinkChip extends StatelessWidget {
  const LinkChip({super.key, required this.link});

  final LinkState link;

  @override
  Widget build(BuildContext context) {
    final colors = context.statusColors;
    final (icon, color) = switch (link.status) {
      LinkStatus.ready => (
        link.route == LinkRoute.relay
            ? Icons.cloud_outlined
            : Icons.wifi_rounded,
        colors.done,
      ),
      LinkStatus.discovering ||
      LinkStatus.handshaking ||
      LinkStatus.reconnecting => (Icons.sync_rounded, colors.warning),
      LinkStatus.revoked ||
      LinkStatus.protoTooOld => (Icons.block_rounded, colors.failed),
      _ => (Icons.cloud_off_rounded, colors.idle),
    };
    return AnimatedContainer(
      duration: motion(context),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 4),
          Flexible(
            child: Text(
              linkLabel(context, link),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w600,
                color: color,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class OfflineBanner extends StatelessWidget {
  const OfflineBanner({
    super.key,
    required this.link,
    required this.updatedAt,
    required this.onRetry,
  });

  final LinkState link;
  final DateTime? updatedAt;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final show = link.status != LinkStatus.ready && !link.isTerminal;
    final l = context.l10n;
    final colors = context.statusColors;
    final at = updatedAt;
    final reason = reasonLabel(context, link);
    return AnimatedSize(
      duration: motion(context),
      curve: Curves.easeOutCubic,
      alignment: Alignment.topCenter,
      child: show
          ? Material(
              color: colors.warning.withValues(alpha: 0.14),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 6, 8, 6),
                child: Row(
                  children: [
                    Icon(
                      Icons.cloud_off_rounded,
                      size: 18,
                      color: colors.warning,
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            at == null
                                ? l.offlineNoData
                                : l.offlineBanner(clock(at)),
                            style: const TextStyle(
                              fontWeight: FontWeight.w600,
                              fontSize: 13,
                            ),
                          ),
                          if (reason.isNotEmpty)
                            Text(reason, style: const TextStyle(fontSize: 12)),
                        ],
                      ),
                    ),
                    TextButton(onPressed: onRetry, child: Text(l.retry)),
                  ],
                ),
              ),
            )
          : const SizedBox(width: double.infinity),
    );
  }
}
