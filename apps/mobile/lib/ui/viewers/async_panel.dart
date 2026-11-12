import 'dart:async';

import 'package:flutter/material.dart';

import '../../l10n/app_localizations.dart';
import '../../remote/protocol.dart';
import '../commands.dart';
import '../kit/kit.dart' show ButtonKind, NoticeCard, NoticeTone, OrreryButton;
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'viewer_api.dart';

String viewerErrorText(AppLocalizations l, Object error) {
  if (error is ViewerFailure) {
    final base = switch (error.code) {
      'denied' => l.errDenied,
      'no-git' || 'not-repo' => l.vErrNotRepo,
      'conflict' => l.vErrConflict,
      'missing' => l.vErrMissing,
      'too-large' => l.vErrTooLarge,
      'exists' => l.vErrExists,
      'not-merged' => l.vErrNotMerged,
      'current' => l.vErrCurrent,
      'no-upstream' => l.vErrNoUpstream,
      'non-ff' => l.vErrNonFf,
      'auth' => l.vErrAuth,
      'nothing' => l.vErrNothing,
      'empty-message' => l.vErrEmptyMessage,
      'pushed' => l.vErrPushed,
      'hook' => l.vErrHook,
      'timeout' => l.errTimeout,
      'busy' => l.errBusy,
      _ => l.vErrFailed,
    };
    final detail = error.detail;
    return detail == null || detail.isEmpty ? base : '$base\n$detail';
  }
  return errorText(l, error);
}

class AsyncPanel<T> extends StatefulWidget {
  const AsyncPanel({
    super.key,
    required this.load,
    required this.builder,
    this.events,
    this.reloadOn,
  });

  final Future<T> Function() load;
  final Widget Function(
    BuildContext context,
    T data,
    Future<void> Function() reload,
  )
  builder;
  final Stream<JsonMap>? events;
  final bool Function(JsonMap event)? reloadOn;

  @override
  State<AsyncPanel<T>> createState() => _AsyncPanelState<T>();
}

class _AsyncPanelState<T> extends State<AsyncPanel<T>> {
  T? _data;
  bool _has = false;
  Object? _error;
  bool _loading = true;
  int _gen = 0;
  StreamSubscription<JsonMap>? _sub;
  Timer? _debounce;

  @override
  void initState() {
    super.initState();
    _sub = widget.events?.listen((event) {
      if (widget.reloadOn?.call(event) ?? false) {
        _debounce?.cancel();
        _debounce = Timer(const Duration(milliseconds: 350), reload);
      }
    });
    reload();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _sub?.cancel();
    super.dispose();
  }

  Future<void> reload() async {
    final gen = ++_gen;
    if (mounted) setState(() => _loading = true);
    try {
      final value = await widget.load();
      if (!mounted || gen != _gen) return;
      setState(() {
        _data = value;
        _has = true;
        _error = null;
        _loading = false;
      });
    } on Object catch (error) {
      if (!mounted || gen != _gen) return;
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    if (!_has) {
      if (_error != null) {
        return ErrorState(
          message: viewerErrorText(l, _error!),
          onRetry: reload,
        );
      }
      return const Center(child: CircularProgressIndicator());
    }
    return Stack(
      children: [
        Column(
          children: [
            if (_error != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 4, 16, 8),
                child: NoticeCard(
                  tone: NoticeTone.error,
                  title: viewerErrorText(l, _error!),
                  trailing: OrreryButton(
                    label: l.vRetry,
                    kind: ButtonKind.text,
                    expand: false,
                    onPressed: reload,
                  ),
                ),
              ),
            Expanded(child: widget.builder(context, _data as T, reload)),
          ],
        ),
        if (_loading)
          const Positioned(
            left: 0,
            right: 0,
            top: 0,
            child: LinearProgressIndicator(minHeight: 2),
          ),
      ],
    );
  }
}

class ErrorState extends StatelessWidget {
  const ErrorState({super.key, required this.message, this.onRetry});

  final String message;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Align(
      alignment: Alignment.topCenter,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            NoticeCard(tone: NoticeTone.error, title: message),
            if (onRetry != null) ...[
              const SizedBox(height: 12),
              OrreryButton(
                key: const ValueKey('error-retry'),
                label: l.vRetry,
                icon: Icons.refresh_rounded,
                kind: ButtonKind.secondary,
                onPressed: onRetry,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class EmptyState extends StatelessWidget {
  const EmptyState({super.key, required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            DecoratedBox(
              decoration: BoxDecoration(
                color: c.raised,
                shape: BoxShape.circle,
                border: Border.all(color: c.line),
              ),
              child: SizedBox.square(
                dimension: 72,
                child: Icon(icon, size: 32, color: c.inkFaint),
              ),
            ),
            const SizedBox(height: 14),
            Text(
              text,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.bodyMedium
                  ?.copyWith(color: c.inkMuted),
            ),
          ],
        ),
      ),
    );
  }
}

typedef Reload = Future<void> Function();
