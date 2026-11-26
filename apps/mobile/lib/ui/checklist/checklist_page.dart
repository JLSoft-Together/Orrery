import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../kit/kit.dart' show ButtonKind, OrreryButton;
import '../model/workspace_model.dart' show baseName;
import '../theme/orrery_shape.dart';
import '../viewers/async_panel.dart';
import '../viewers/viewer_actions.dart';
import '../viewers/viewer_api.dart';
import '../widgets/common.dart';
import 'checklist_model.dart';
import 'checklist_parts.dart';

const Duration checklistPoll = Duration(seconds: 3);
const Duration checklistSlowPoll = Duration(seconds: 30);
const int checklistPollMax = 200;

class ChecklistPage extends ConsumerStatefulWidget {
  const ChecklistPage({super.key, required this.machineId, required this.root});

  final String machineId;
  final String root;

  @override
  ConsumerState<ChecklistPage> createState() => _ChecklistPageState();
}

class _ChecklistPageState extends ConsumerState<ChecklistPage> {
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));
  Timer? _timer;
  int _polls = 0;
  bool _starting = false;
  Future<void> Function()? _reload;

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<ChecklistData> _load() async {
    final data = ChecklistData.parse(
      await _api.map('checklist.get', {'dir': widget.root}),
    );
    _sync(data);
    return data;
  }

  void _sync(ChecklistData data) {
    if (data.view != CheckView.running) {
      _timer?.cancel();
      _timer = null;
      _polls = 0;
      return;
    }
    _timer ??= Timer.periodic(checklistPoll, _tick);
  }

  void _tick(Timer timer) {
    if (++_polls == checklistPollMax) {
      timer.cancel();
      _timer = Timer.periodic(checklistSlowPoll, _tick);
    }
    _reload?.call();
  }

  Future<void> _toggle(ChecklistItem item, bool done, Reload reload) =>
      viewerAction(context, ref, widget.machineId, 'checklist.toggle', {
        'dir': widget.root,
        'id': item.id,
        'done': done,
      }, after: reload);

  Future<void> _run(Reload reload) async {
    setState(() => _starting = true);
    _polls = 0;
    await viewerAction(
      context,
      ref,
      widget.machineId,
      'checklist.run',
      {'dir': widget.root},
      success: context.l10n.vClRunStarted,
      after: reload,
    );
    if (mounted) setState(() => _starting = false);
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      appBar: AppBar(title: Text('${l.vChecklist}, ${baseName(widget.root)}')),
      body: AsyncPanel<ChecklistData>(
        load: _load,
        builder: (context, data, reload) {
          _reload = reload;
          return _content(context, data, reload);
        },
      ),
    );
  }

  Widget _content(BuildContext context, ChecklistData data, Reload reload) {
    final l = context.l10n;
    final c = context.orrery;
    return RefreshIndicator(
      onRefresh: reload,
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 32),
        children: [
          ChecklistSummary(data: data),
          const SizedBox(height: 4),
          if (data.items.isEmpty)
            EmptyState(icon: Icons.checklist_rounded, text: l.vClEmpty)
          else
            for (final item in data.items)
              ChecklistRow(
                item: item,
                onChanged: (v) => _toggle(item, v, reload),
              ),
          const Divider(height: 32),
          Text(
            l.vClCheck,
            style: Theme.of(context).textTheme.labelLarge
                ?.copyWith(color: c.inkMuted),
          ),
          const SizedBox(height: 8),
          if (data.script.isNotEmpty)
            Text(
              data.script,
              key: const ValueKey('cl-script'),
              style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
            ),
          if (data.script.isNotEmpty) const SizedBox(height: 8),
          CheckStateRow(view: data.view),
          const SizedBox(height: 12),
          OrreryButton(
            key: const ValueKey('cl-run'),
            label: l.vClRun,
            icon: Icons.play_arrow_rounded,
            kind: ButtonKind.secondary,
            busy: _starting,
            onPressed: data.canRun ? () => _run(reload) : null,
          ),
        ],
      ),
    );
  }
}
