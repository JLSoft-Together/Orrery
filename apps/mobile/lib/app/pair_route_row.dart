import 'package:flutter/material.dart';

import '../remote/link_state.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';

class PairRouteRow extends StatelessWidget {
  const PairRouteRow({super.key, required this.route});

  final LinkRoute? route;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final muted = context.orrery.inkMuted;
    final (icon, text) = switch (route) {
      LinkRoute.lan => (Icons.wifi_rounded, l.pairViaLan),
      LinkRoute.relay => (Icons.cloud_outlined, l.pairViaRelay),
      null => (Icons.hourglass_empty_rounded, l.pairing),
    };
    return Row(
      key: ValueKey('pair-route-${route?.name ?? 'none'}'),
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Icon(icon, size: 16, color: muted),
        const SizedBox(width: 6),
        Flexible(
          child: Text(
            text,
            textAlign: TextAlign.center,
            style: TextStyle(color: muted),
          ),
        ),
      ],
    );
  }
}
