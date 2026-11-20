import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

const int slowLatencyMs = 150;

class ReconnectStrip extends ConsumerStatefulWidget {
  const ReconnectStrip({
    super.key,
    required this.machineId,
    this.queued = 0,
    this.showLatency = false,
  });

  final String machineId;
  final int queued;
  final bool showLatency;

  @override
  ConsumerState<ReconnectStrip> createState() => _ReconnectStripState();
}

class _ReconnectStripState extends ConsumerState<ReconnectStrip> {
  bool _rejoined = false;
  Timer? _timer;

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(linkProvider(widget.machineId).select((s) => s.status), (
      prev,
      next,
    ) {
      if (prev != LinkStatus.ready &&
          next == LinkStatus.ready &&
          prev != null) {
        _timer?.cancel();
        setState(() => _rejoined = true);
        _timer = Timer(const Duration(milliseconds: 1600), () {
          if (mounted) setState(() => _rejoined = false);
        });
      } else if (next != LinkStatus.ready && _rejoined) {
        _timer?.cancel();
        setState(() => _rejoined = false);
      }
    });
    final c = context.orrery;
    final l = context.l10n;
    final link = ref.watch(linkProvider(widget.machineId));
    final online = link.status == LinkStatus.ready;
    final latency = widget.showLatency
        ? ref.watch(latencyProvider(widget.machineId)).value
        : null;
    final slow = online && latency != null && latency > slowLatencyMs;
    final queued = widget.queued;
    String? text;
    Color color = c.waiting;
    var spin = false;
    IconData? icon;
    if (!online && !link.isTerminal) {
      text = queued > 0 ? l.chatReconnectingQueued(queued) : l.chatReconnecting;
      spin = true;
    } else if (!online) {
      text = l.chatOffline;
      icon = Icons.cloud_off_rounded;
      color = c.idle;
    } else if (_rejoined) {
      text = l.chatRejoined;
      icon = Icons.check_rounded;
      color = c.done;
    } else if (queued > 0) {
      text = l.chatQueued(queued);
      icon = Icons.schedule_send_rounded;
    } else if (slow) {
      text = l.chatSlow(latency);
      icon = Icons.network_check_rounded;
    }
    final body = text == null
        ? const SizedBox(key: ValueKey('none'), width: double.infinity)
        : SizedBox(
            key: const ValueKey('strip'),
            height: 28,
            width: double.infinity,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                if (spin)
                  SizedBox.square(
                    dimension: 12,
                    child: CircularProgressIndicator(
                      strokeWidth: 1.75,
                      color: color,
                    ),
                  )
                else if (icon != null)
                  Icon(icon, size: 14, color: color),
                const SizedBox(width: 8),
                Flexible(
                  child: Text(
                    text,
                    key: const ValueKey('reconnect-text'),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w500,
                      color: color,
                    ),
                  ),
                ),
              ],
            ),
          );
    return AnimatedSwitcher(duration: motion(context), child: body);
  }
}
