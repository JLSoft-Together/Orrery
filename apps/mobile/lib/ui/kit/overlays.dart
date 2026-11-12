import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'buttons.dart';

Future<T?> showOrrerySheet<T>(
  BuildContext context, {
  required WidgetBuilder builder,
  bool scrollControlled = true,
}) {
  final tokens = context.tokens;
  final quiet = reduceMotion(context);
  return showModalBottomSheet<T>(
    context: context,
    isScrollControlled: scrollControlled,
    useSafeArea: true,
    sheetAnimationStyle: quiet
        ? AnimationStyle.noAnimation
        : AnimationStyle(
            duration: tokens.slow,
            curve: tokens.spring,
            reverseDuration: tokens.pop,
            reverseCurve: tokens.inCurve,
          ),
    builder: builder,
  );
}

Future<T?> showOrreryDialog<T>(
  BuildContext context, {
  required WidgetBuilder builder,
  bool barrierDismissible = true,
}) {
  final tokens = context.tokens;
  final quiet = reduceMotion(context);
  final c = context.orrery;
  return showGeneralDialog<T>(
    context: context,
    barrierDismissible: barrierDismissible,
    barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
    barrierColor: c.scrim,
    transitionDuration: quiet ? Duration.zero : tokens.base,
    pageBuilder: (context, _, _) => SafeArea(child: Builder(builder: builder)),
    transitionBuilder: (context, animation, _, child) {
      final curved = CurvedAnimation(
        parent: animation,
        curve: tokens.out,
        reverseCurve: tokens.inCurve,
      );
      return FadeTransition(
        opacity: curved,
        child: ScaleTransition(
          scale: Tween<double>(begin: 0.96, end: 1).animate(curved),
          child: child,
        ),
      );
    },
  );
}

class OrreryDialog extends StatelessWidget {
  const OrreryDialog({
    super.key,
    required this.title,
    this.body,
    this.content,
    required this.actions,
  });

  final String title;
  final String? body;
  final Widget? content;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final shape = context.shape;
    final text = Theme.of(context).textTheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 420),
          child: Material(
            color: c.surface,
            shape: RoundedRectangleBorder(
              borderRadius: shape.cardRadius,
              side: BorderSide(color: c.line),
            ),
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(title, style: text.titleMedium),
                  if (body != null) ...[
                    const SizedBox(height: 8),
                    Text(
                      body!,
                      style: text.bodyMedium?.copyWith(color: c.inkMuted),
                    ),
                  ],
                  if (content != null) ...[
                    const SizedBox(height: 12),
                    content!,
                  ],
                  const SizedBox(height: 16),
                  Wrap(
                    alignment: WrapAlignment.end,
                    spacing: 8,
                    runSpacing: 8,
                    children: actions,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

Future<bool> confirmOrrery(
  BuildContext context, {
  required String title,
  required String body,
  required String confirmLabel,
  required String cancelLabel,
  bool danger = false,
}) async {
  final ok = await showOrreryDialog<bool>(
    context,
    builder: (context) => OrreryDialog(
      title: title,
      body: body,
      actions: [
        OrreryButton(
          key: const ValueKey('orrery-dialog-cancel'),
          label: cancelLabel,
          kind: ButtonKind.text,
          expand: false,
          onPressed: () => Navigator.pop(context, false),
        ),
        OrreryButton(
          key: const ValueKey('orrery-dialog-confirm'),
          label: confirmLabel,
          kind: danger ? ButtonKind.danger : ButtonKind.primary,
          expand: false,
          onPressed: () => Navigator.pop(context, true),
        ),
      ],
    ),
  );
  return ok ?? false;
}
