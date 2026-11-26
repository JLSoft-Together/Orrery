import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/menu_action.dart';
import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../../remote/queue_cap.dart';
import '../chat/chat_nav.dart';
import '../chat/reconnect_strip.dart';
import '../danger_guard.dart';
import '../kit/buttons.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import 'queue_driver.dart';
import 'queue_model.dart';
import 'queue_stage.dart';

void openQueue(BuildContext context, String machineId) {
  Navigator.of(
    context,
  ).push(machineRoute<void>(machineId, (_) => QueuePage(machineId: machineId)));
}

class QueuePage extends ConsumerStatefulWidget {
  const QueuePage({
    super.key,
    required this.machineId,
    this.undo = menuActionUndo,
    this.snoozeUndo = queueSnoozeUndo,
    this.goneDelay = queueGoneDelay,
  });

  final String machineId;
  final Duration undo;
  final Duration snoozeUndo;
  final Duration goneDelay;

  @override
  ConsumerState<QueuePage> createState() => _QueuePageState();
}

bool _hintSeen = false;

class _QueuePageState extends ConsumerState<QueuePage> {
  final QueueFlow _flow = QueueFlow();
  late final QueueDriver _driver = QueueDriver(
    flow: _flow,
    api: ref.read(machineApiProvider(widget.machineId)),
    paneContext: () => mounted ? context : null,
    rules: () => ref.read(dangerRulesProvider(widget.machineId)),
    notify: _notify,
    isMounted: () => mounted,
    leave: (ms) => motion(context, ms),
    undo: widget.undo,
    snoozeUndo: widget.snoozeUndo,
    goneDelay: widget.goneDelay,
  );
  bool _synced = false;

  @override
  void initState() {
    super.initState();
    _sync(notify: false);
  }

  @override
  void dispose() {
    _driver.dispose();
    super.dispose();
  }

  void _notify() {
    if (mounted) setState(() {});
  }

  bool _live() {
    final id = widget.machineId;
    final link = ref.read(linkProvider(id));
    final view = ref.read(machineViewProvider(id));
    return link.status == LinkStatus.ready &&
        view.workspace != null &&
        !view.stale;
  }

  void _sync({bool notify = true}) {
    final live = _live();
    if (live) _synced = true;
    final changed = _flow.sync(
      ref.read(workspaceModelProvider(widget.machineId)),
      live: live,
    );
    _driver.watchGone();
    if (changed && notify) _notify();
  }

  void _seen() => _hintSeen = true;

  @override
  Widget build(BuildContext context) {
    final id = widget.machineId;
    ref.listen(workspaceModelProvider(id), (_, _) => _sync());
    ref.listen(linkProvider(id).select((s) => s.status), (_, _) => _sync());
    ref.listen(machineViewProvider(id).select((v) => v.stale), (_, _) => _sync());
    final l = context.l10n;
    final live = ref.watch(
      linkProvider(id).select((s) => s.status == LinkStatus.ready),
    );
    final stale = ref.watch(machineViewProvider(id).select((v) => v.stale));
    final cap = ref.watch(
      machineViewProvider(id).select((v) => v.hasCap(queueCap)),
    );
    final enabled = live && !stale;
    final slot = _flow.current;
    return Scaffold(
      appBar: AppBar(
        title: Text(l.queueTitle),
        actions: [
          if (slot != null)
            Padding(
              padding: const EdgeInsets.only(right: 16),
              child: Center(
                child: Text(
                  l.queueCounter(_flow.position, _flow.total),
                  key: const ValueKey('queue-counter'),
                  style: TextStyle(
                    fontWeight: FontWeight.w600,
                    color: context.orrery.inkMuted,
                  ),
                ),
              ),
            ),
        ],
      ),
      body: Column(
        children: [
          ReconnectStrip(machineId: id),
          Expanded(
            child: AnimatedSwitcher(
              duration: motion(context),
              transitionBuilder: (child, animation) => FadeTransition(
                opacity: animation,
                child: SlideTransition(
                  position: Tween(
                    begin: const Offset(0.12, 0),
                    end: Offset.zero,
                  ).animate(animation),
                  child: child,
                ),
              ),
              child: slot == null
                  ? _Done(
                      key: const ValueKey('queue-done'),
                      synced: _synced,
                      skipped: _flow.skippedCount,
                      onReview: () => setState(_flow.reviewSkipped),
                    )
                  : QueueStage(
                      key: ValueKey(slot),
                      slot: slot,
                      enabled: enabled,
                      cap: cap,
                      driver: _driver,
                      showHint: !_hintSeen,
                      onSeen: _seen,
                      onOpen: () => openSession(
                        context,
                        ref,
                        id,
                        slot.tab,
                        slot.pane,
                      ),
                    ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Done extends StatelessWidget {
  const _Done({
    super.key,
    required this.synced,
    required this.skipped,
    required this.onReview,
  });

  final bool synced;
  final int skipped;
  final VoidCallback onReview;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    if (!synced) {
      return const Center(child: CircularProgressIndicator());
    }
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.task_alt_rounded, size: 56, color: c.done),
            const SizedBox(height: 12),
            Text(
              l.queueDone,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 18,
                fontWeight: FontWeight.w600,
                color: c.ink,
              ),
            ),
            if (skipped > 0) ...[
              const SizedBox(height: 8),
              Text(
                l.queueSkippedCount(skipped),
                style: TextStyle(color: c.inkMuted),
              ),
              const SizedBox(height: 12),
              OrreryButton(
                key: const ValueKey('queue-review'),
                label: l.queueReview,
                kind: ButtonKind.secondary,
                expand: false,
                onPressed: onReview,
              ),
            ],
            const SizedBox(height: 16),
            OrreryButton(
              key: const ValueKey('queue-back'),
              label: l.queueBack,
              expand: false,
              onPressed: () => Navigator.of(context).maybePop(),
            ),
          ],
        ),
      ),
    );
  }
}
