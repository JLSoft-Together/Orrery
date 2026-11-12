import 'package:flutter/material.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'monitor_model.dart';
import 'since_text.dart';

TextStyle monoStyle(BuildContext context, {double size = 12, Color? color}) =>
    TextStyle(
      fontFamily: 'OrreryMono',
      fontSize: size,
      height: 16 / 12,
      color: color ?? context.orrery.inkMuted,
    );

class SessionCard extends StatelessWidget {
  const SessionCard({
    super.key,
    required this.session,
    required this.onTap,
    required this.onLongPress,
  });

  final MonitorSession session;
  final VoidCallback onTap;
  final VoidCallback onLongPress;

  String? get _strip {
    if (session.rail == RailState.asked && session.waitingFor != null) {
      return session.waitingFor;
    }
    if (session.tool != null) {
      return [session.tool!, ?session.toolSummary].join(' · ');
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final strip = _strip;
    final extra = strip == null && session.subagents.isNotEmpty
        ? l.subagentsCount(session.subagents.length)
        : null;
    final line = strip ?? extra;
    return OrreryCard(
      key: ValueKey('mon-${session.terminalId}'),
      barColor: agentColor(context, session.provider),
      onTap: onTap,
      onLongPress: onLongPress,
      padding: const EdgeInsets.fromLTRB(16, 14, 14, 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AgentTile(agent: session.provider),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    AnimatedRename(
                      text: session.title,
                      style: TextStyle(
                        fontSize: 15,
                        height: 20 / 15,
                        fontWeight: FontWeight.w600,
                        color: c.ink,
                      ),
                    ),
                    if (session.branch != null)
                      Row(
                        children: [
                          Icon(
                            Icons.call_split_rounded,
                            size: 14,
                            color: c.inkFaint,
                          ),
                          const SizedBox(width: 4),
                          Expanded(
                            child: Text(
                              session.branch!,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: monoStyle(context),
                            ),
                          ),
                        ],
                      ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Column(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  RailChip(state: session.rail),
                  const SizedBox(height: 4),
                  SinceText(since: session.statusSince),
                ],
              ),
            ],
          ),
          QuietSize(
            alignment: Alignment.topCenter,
            child: line == null
                ? const SizedBox(width: double.infinity)
                : Padding(
                    padding: const EdgeInsets.only(top: 10),
                    child: SizedBox(
                      width: double.infinity,
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          color: c.raised,
                          borderRadius: context.shape.innerRadius,
                        ),
                        child: Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 10,
                            vertical: 7,
                          ),
                          child: Text(
                            '▸ $line',
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: monoStyle(
                              context,
                              color: session.rail == RailState.asked
                                  ? c.waiting
                                  : c.inkMuted,
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
          ),
        ],
      ),
    );
  }
}

class SessionRow extends StatelessWidget {
  const SessionRow({
    super.key,
    required this.session,
    required this.first,
    required this.onTap,
    required this.onLongPress,
  });

  final MonitorSession session;
  final bool first;
  final VoidCallback onTap;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final look = railLook(context, session.rail);
    final tint = look.tone.resolve(context);
    final sub = [
      ?session.branch,
      if (session.subagents.isNotEmpty)
        context.l10n.subagentsCount(session.subagents.length),
    ].join(' · ');
    return DecoratedBox(
      decoration: BoxDecoration(
        border: first ? null : Border(top: BorderSide(color: c.line)),
      ),
      child: InkWell(
        key: ValueKey('mon-${session.terminalId}'),
        onTap: onTap,
        onLongPress: onLongPress,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 10, 14, 10),
          child: Row(
            children: [
              AnimatedSwitcher(
                duration: motion(context, 150),
                child: Icon(
                  look.icon,
                  key: ValueKey(look.icon),
                  size: 20,
                  color: tint,
                ),
              ),
              const SizedBox(width: 12),
              AgentTile(agent: session.provider, size: 32),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    AnimatedRename(
                      text: session.title,
                      style: TextStyle(
                        fontSize: 14,
                        height: 20 / 14,
                        fontWeight: FontWeight.w600,
                        color: c.ink,
                      ),
                    ),
                    if (sub.isNotEmpty)
                      Text(
                        sub,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: monoStyle(context),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              SinceText(since: session.statusSince),
            ],
          ),
        ),
      ),
    );
  }
}
