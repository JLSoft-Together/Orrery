import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/providers.dart';
import '../../remote/store.dart';
import '../commands.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'connection_gate.dart';

Route<T> machineRoute<T>(String machineId, WidgetBuilder builder) =>
    MaterialPageRoute<T>(
      builder: (context) =>
          MachineScope(machineId: machineId, child: builder(context)),
    );

class MachineScope extends ConsumerStatefulWidget {
  const MachineScope({super.key, required this.machineId, required this.child});

  final String machineId;
  final Widget child;

  @override
  ConsumerState<MachineScope> createState() => _MachineScopeState();
}

final Set<String> _openSheets = {};

class _MachineScopeState extends ConsumerState<MachineScope> {
  Timer? _retry;

  @override
  void dispose() {
    _retry?.cancel();
    super.dispose();
  }

  void _maybeShow(List<ConfirmAsk> confirms) {
    if (confirms.isEmpty || !mounted) return;
    if (ModalRoute.of(context)?.isCurrent != true) {
      _retry?.cancel();
      _retry = Timer(const Duration(milliseconds: 150), () {
        if (!mounted) return;
        _maybeShow(ref.read(machineViewProvider(widget.machineId)).confirms);
      });
      return;
    }
    if (!_openSheets.add(widget.machineId)) return;
    showModalBottomSheet<void>(
      context: context,
      isDismissible: false,
      enableDrag: false,
      isScrollControlled: true,
      builder: (_) => PopScope(
        canPop: false,
        child: ConfirmSheet(machineId: widget.machineId),
      ),
    ).whenComplete(() {
      _openSheets.remove(widget.machineId);
      if (!mounted) return;
      _maybeShow(ref.read(machineViewProvider(widget.machineId)).confirms);
    });
  }

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _maybeShow(ref.read(machineViewProvider(widget.machineId)).confirms);
    });
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(
      machineViewProvider(widget.machineId).select((v) => v.confirms),
      (_, next) => _maybeShow(next),
    );
    return ConnectionGate(machineId: widget.machineId, child: widget.child);
  }
}

class ConfirmSheet extends ConsumerStatefulWidget {
  const ConfirmSheet({super.key, required this.machineId});

  final String machineId;

  @override
  ConsumerState<ConfirmSheet> createState() => _ConfirmSheetState();
}

class _ConfirmSheetState extends ConsumerState<ConfirmSheet> {
  bool _closing = false;

  void _close() {
    if (_closing) return;
    _closing = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) Navigator.of(context).pop();
    });
  }

  bool _answering = false;

  Future<void> _answer(ConfirmAsk ask, bool ok) async {
    if (_answering) return;
    setState(() => _answering = true);
    try {
      await ref
          .read(machineApiProvider(widget.machineId))
          .answerConfirm(ask.cid, ok);
    } on Object catch (error) {
      if (mounted) toast(context, errorText(context.l10n, error));
    } finally {
      if (mounted) setState(() => _answering = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final confirms = ref.watch(
      machineViewProvider(widget.machineId).select((v) => v.confirms),
    );
    if (confirms.isEmpty) {
      _close();
      return const SizedBox(height: 1);
    }
    final ask = confirms.first;
    final expired = DateTime.now().difference(ask.receivedAt).inSeconds >= 60;
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 12, 20, 16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(ask.title, style: Theme.of(context).textTheme.titleMedium),
            if (ask.detail != null) ...[
              const SizedBox(height: 8),
              Text(
                ask.detail!,
                style: TextStyle(color: scheme.onSurfaceVariant),
              ),
            ],
            const SizedBox(height: 12),
            _Countdown(ask: ask),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: OutlinedButton(
                    onPressed: _answering ? null : () => _answer(ask, false),
                    child: Text(ask.cancel),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: FilledButton(
                    onPressed: _answering || expired
                        ? null
                        : () => _answer(ask, true),
                    child: Text(ask.ok),
                  ),
                ),
              ],
            ),
            if (confirms.length > 1)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  l.confirmMore(confirms.length - 1),
                  style: TextStyle(color: scheme.onSurfaceVariant),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _Countdown extends StatelessWidget {
  const _Countdown({required this.ask});

  final ConfirmAsk ask;

  @override
  Widget build(BuildContext context) {
    return Ticker1s(
      builder: (context, now) {
        final left = 60 - now.difference(ask.receivedAt).inSeconds;
        final value = (left.clamp(0, 60)) / 60;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(4),
              child: LinearProgressIndicator(value: value, minHeight: 4),
            ),
            const SizedBox(height: 4),
            Text(
              context.l10n.confirmExpires(left.clamp(0, 60)),
              style: const TextStyle(fontSize: 12),
            ),
          ],
        );
      },
    );
  }
}
