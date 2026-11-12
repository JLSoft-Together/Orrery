import 'dart:async';

import 'package:flutter/material.dart';

import '../../l10n/app_localizations.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_theme.dart';

extension L10nX on BuildContext {
  AppLocalizations get l10n => AppLocalizations.of(this);
}

bool reduceMotion(BuildContext context) =>
    MediaQuery.maybeOf(context)?.disableAnimations ?? false;

Duration motion(BuildContext context, [int ms = 220]) =>
    reduceMotion(context) ? Duration.zero : Duration(milliseconds: ms);

class StatusStyle {
  const StatusStyle(this.icon, this.color, this.label);

  final IconData icon;
  final Color color;
  final String label;
}

StatusStyle statusStyle(BuildContext context, RailState state) {
  final l = context.l10n;
  final c = context.statusColors;
  return switch (state) {
    RailState.working => StatusStyle(
      Icons.autorenew_rounded,
      c.working,
      l.statusWorking,
    ),
    RailState.subagents => StatusStyle(
      Icons.account_tree_outlined,
      c.working,
      l.statusSubagents,
    ),
    RailState.asked => StatusStyle(
      Icons.front_hand_outlined,
      c.asked,
      l.statusAsked,
    ),
    RailState.failed => StatusStyle(
      Icons.error_outline_rounded,
      c.failed,
      l.statusFailed,
    ),
    RailState.done => StatusStyle(
      Icons.check_circle_outline_rounded,
      c.done,
      l.statusDone,
    ),
    RailState.starting => StatusStyle(
      Icons.hourglass_top_rounded,
      c.idle,
      l.statusStarting,
    ),
    RailState.ended => StatusStyle(
      Icons.stop_circle_outlined,
      c.idle,
      l.statusEnded,
    ),
    RailState.parked => StatusStyle(
      Icons.pause_circle_outline_rounded,
      c.idle,
      l.statusParked,
    ),
    RailState.idle => StatusStyle(
      Icons.radio_button_unchecked_rounded,
      c.idle,
      l.statusIdle,
    ),
  };
}

class StatusBadge extends StatelessWidget {
  const StatusBadge({
    super.key,
    required this.state,
    this.compact = false,
    this.iconOnly = false,
  });

  final RailState state;
  final bool compact;
  final bool iconOnly;

  @override
  Widget build(BuildContext context) {
    final style = statusStyle(context, state);
    return AnimatedSwitcher(
      duration: motion(context),
      switchInCurve: Curves.easeOut,
      switchOutCurve: Curves.easeIn,
      child: Semantics(
        key: ValueKey(state),
        label: iconOnly ? style.label : null,
        child: Container(
          padding: EdgeInsets.symmetric(
            horizontal: compact ? 6 : 8,
            vertical: 3,
          ),
          decoration: BoxDecoration(
            color: style.color.withValues(alpha: 0.14),
            borderRadius: BorderRadius.circular(20),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(style.icon, size: 14, color: style.color),
              if (!iconOnly) const SizedBox(width: 4),
              if (!iconOnly)
                Flexible(
                  child: Text(
                    style.label,
                    maxLines: 1,
                    softWrap: false,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: style.color,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class Appear extends StatelessWidget {
  const Appear({super.key, required this.child, this.delay = 0});

  final Widget child;
  final int delay;

  @override
  Widget build(BuildContext context) {
    if (reduceMotion(context)) return child;
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: 1),
      duration: Duration(milliseconds: 240 + delay),
      curve: Curves.easeOutCubic,
      builder: (context, value, child) => Opacity(
        opacity: value,
        child: Transform.translate(
          offset: Offset(0, (1 - value) * 12),
          child: child,
        ),
      ),
      child: child,
    );
  }
}

class PressScale extends StatefulWidget {
  const PressScale({
    super.key,
    required this.child,
    required this.onTap,
    this.onLongPress,
  });

  final Widget child;
  final VoidCallback? onTap;
  final VoidCallback? onLongPress;

  @override
  State<PressScale> createState() => _PressScaleState();
}

class _PressScaleState extends State<PressScale> {
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerDown: (_) => setState(() => _down = true),
      onPointerUp: (_) => setState(() => _down = false),
      onPointerCancel: (_) => setState(() => _down = false),
      child: AnimatedScale(
        scale: _down && !reduceMotion(context) ? 0.97 : 1,
        duration: const Duration(milliseconds: 90),
        child: InkWell(
          borderRadius: BorderRadius.circular(14),
          onTap: widget.onTap,
          onLongPress: widget.onLongPress,
          child: widget.child,
        ),
      ),
    );
  }
}

String clock(DateTime time) =>
    '${time.hour.toString().padLeft(2, '0')}:${time.minute.toString().padLeft(2, '0')}';

String elapsedText(AppLocalizations l, Duration d) {
  if (d.isNegative) d = Duration.zero;
  if (d.inHours > 0) {
    return '${l.durationHours(d.inHours)} ${l.durationMinutes(d.inMinutes % 60)}';
  }
  if (d.inMinutes > 0) {
    return '${l.durationMinutes(d.inMinutes)} ${l.durationSeconds(d.inSeconds % 60)}';
  }
  return l.durationSeconds(d.inSeconds);
}

String shortElapsed(AppLocalizations l, Duration d) {
  if (d.isNegative || d.inSeconds < 60) return l.durationLessThanMinute;
  if (d.inDays >= 1) return l.durationDays(d.inDays);
  if (d.inHours >= 1) return l.durationHours(d.inHours);
  return l.durationMinutes(d.inMinutes);
}

class Ticker1s extends StatefulWidget {
  const Ticker1s({super.key, required this.builder});

  final Widget Function(BuildContext context, DateTime now) builder;

  @override
  State<Ticker1s> createState() => _Ticker1sState();
}

class _Ticker1sState extends State<Ticker1s> {
  Timer? _timer;
  DateTime _now = DateTime.now();

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted) setState(() => _now = DateTime.now());
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, _now);
}

class AgentGlyph extends StatelessWidget {
  const AgentGlyph({super.key, this.agent, this.kind = 'term', this.size = 20});

  final String? agent;
  final String kind;
  final double size;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final icon = switch (agent) {
      'claude' => Icons.auto_awesome_rounded,
      'codex' => Icons.code_rounded,
      'deepseek' => Icons.water_drop_outlined,
      _ => kind == 'term' ? Icons.terminal_rounded : Icons.description_outlined,
    };
    return Icon(icon, size: size, color: scheme.onSurfaceVariant);
  }
}
