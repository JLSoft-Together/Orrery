import 'dart:async';

import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import 'push_service.dart';

class PausePromptHost extends StatefulWidget {
  const PausePromptHost({
    super.key,
    required this.push,
    required this.navigatorKey,
    required this.child,
  });

  final PushService? push;
  final GlobalKey<NavigatorState> navigatorKey;
  final Widget child;

  @override
  State<PausePromptHost> createState() => _PausePromptHostState();
}

class _PausePromptHostState extends State<PausePromptHost> {
  StreamSubscription<void>? _sub;
  bool _open = false;

  @override
  void initState() {
    super.initState();
    _sub = widget.push?.pauseRequests.listen((_) => _prompt());
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  Future<void> _prompt() async {
    final push = widget.push;
    final context = widget.navigatorKey.currentContext;
    if (push == null || context == null || !context.mounted || _open) return;
    _open = true;
    try {
      await showPausePrompt(context, push);
    } finally {
      _open = false;
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

String _time(BuildContext context, DateTime at) =>
    MaterialLocalizations.of(context)
        .formatTimeOfDay(TimeOfDay.fromDateTime(at));

Future<void> showPausePrompt(BuildContext context, PushService push) async {
  final l = AppLocalizations.of(context);
  final until = push.pausedUntil;
  final resume = until != null;
  final choice = await showDialog<bool>(
    context: context,
    builder: (dialog) => AlertDialog(
      title: Text(
        resume ? l.pushPausedTitle(_time(dialog, until)) : l.pushPauseTitle,
      ),
      content: Text(resume ? l.pushResumeBody : l.pushPauseBody),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialog).pop(false),
          child: Text(l.pushPauseCancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(dialog).pop(true),
          child: Text(resume ? l.pushResumeAction : l.pushPauseAction),
        ),
      ],
    ),
  );
  if (choice != true) return;
  if (resume) {
    await push.resumeNow();
  } else {
    await push.pauseOneHour();
  }
}
