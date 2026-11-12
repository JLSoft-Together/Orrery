import 'package:flutter/material.dart';

import '../../remote/link_state.dart';
import '../kit/kit.dart';
import '../widgets/common.dart';
import '../widgets/link_widgets.dart';

class OfflineNotice extends StatelessWidget {
  const OfflineNotice({
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
    final l = context.l10n;
    final at = updatedAt;
    final reason = reasonLabel(context, link);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: ConnectionBanner(
        link: link,
        title: at == null ? l.offlineNoData : l.offlineBanner(clock(at)),
        detail: reason.isEmpty ? null : reason,
        retryLabel: l.retry,
        onRetry: onRetry,
      ),
    );
  }
}
