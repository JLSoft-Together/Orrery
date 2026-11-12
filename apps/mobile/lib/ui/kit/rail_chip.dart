import 'package:flutter/material.dart';

import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'status_chip.dart';

class RailLook {
  const RailLook(this.tone, this.icon, this.label, {this.pulse = false});

  final ChipTone tone;
  final IconData icon;
  final String label;
  final bool pulse;
}

RailLook railLook(BuildContext context, RailState state) {
  final l = context.l10n;
  return switch (state) {
    RailState.working || RailState.subagents => RailLook(
      ChipTone.working,
      Icons.autorenew_rounded,
      l.statusWorking,
      pulse: true,
    ),
    RailState.asked => RailLook(
      ChipTone.waiting,
      Icons.schedule_rounded,
      l.chipWaiting,
    ),
    RailState.failed => RailLook(
      ChipTone.error,
      Icons.warning_amber_rounded,
      l.statusFailed,
    ),
    RailState.done => RailLook(
      ChipTone.done,
      Icons.check_rounded,
      l.statusDone,
    ),
    RailState.starting => RailLook(
      ChipTone.idle,
      Icons.hourglass_top_rounded,
      l.statusStarting,
    ),
    RailState.ended => RailLook(
      ChipTone.idle,
      Icons.stop_circle_outlined,
      l.statusEnded,
    ),
    RailState.parked => RailLook(
      ChipTone.idle,
      Icons.pause_circle_outline_rounded,
      l.statusParked,
    ),
    RailState.idle => RailLook(
      ChipTone.idle,
      Icons.nightlight_outlined,
      l.statusIdle,
    ),
  };
}

class RailChip extends StatelessWidget {
  const RailChip({super.key, required this.state, this.dense = false});

  final RailState state;
  final bool dense;

  @override
  Widget build(BuildContext context) {
    final look = railLook(context, state);
    return OrreryChip(
      tone: look.tone,
      icon: look.icon,
      label: look.label,
      pulse: look.pulse,
      dense: dense,
    );
  }
}

Color agentColor(BuildContext context, String? agent) {
  final c = context.orrery;
  return switch (agent) {
    'claude' => c.agentClaude,
    'codex' => c.agentCodex,
    _ => c.agentShell,
  };
}

IconData agentIcon(String? agent) => switch (agent) {
  'claude' => Icons.auto_awesome_rounded,
  'codex' => Icons.hexagon_outlined,
  _ => Icons.terminal_rounded,
};

class AgentTile extends StatelessWidget {
  const AgentTile({super.key, required this.agent, this.size = 36});

  final String? agent;
  final double size;

  @override
  Widget build(BuildContext context) {
    final color = agentColor(context, agent);
    return SizedBox(
      width: size,
      height: size,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.14),
          borderRadius: context.shape.controlRadius,
        ),
        child: Icon(agentIcon(agent), size: size * 0.55, color: color),
      ),
    );
  }
}

String agentLabel(String? agent) => switch (agent) {
  'claude' => 'Claude',
  'codex' => 'Codex',
  'deepseek' => 'DeepSeek',
  _ => 'Shell',
};
