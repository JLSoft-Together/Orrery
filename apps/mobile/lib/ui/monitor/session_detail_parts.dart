import 'package:flutter/material.dart';

import '../kit/kit.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'monitor_cards.dart';
import 'monitor_model.dart';

String _compact(int n) {
  if (n >= 1000000) return '${(n / 1000000).toStringAsFixed(1)}M';
  if (n >= 1000) return '${(n / 1000).toStringAsFixed(1)}k';
  return '$n';
}

class SessionHead extends StatelessWidget {
  const SessionHead({
    super.key,
    required this.session,
    required this.project,
    required this.machine,
  });

  final MonitorSession session;
  final String project;
  final String machine;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final since = session.statusSince;
    return OrreryCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              AgentTile(agent: session.provider, size: 40),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      agentLabel(session.provider),
                      style: TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w700,
                        color: c.ink,
                      ),
                    ),
                    if (project.isNotEmpty)
                      Text(
                        project,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: monoStyle(context),
                      ),
                  ],
                ),
              ),
              RailChip(state: session.rail),
            ],
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 16,
            runSpacing: 8,
            children: [
              if (session.branch != null)
                _Fact(Icons.call_split_rounded, session.branch!, mono: true),
              if (since != null)
                _Fact(
                  Icons.schedule_rounded,
                  l.detailStarted(clock(since)),
                  mono: true,
                ),
              if (machine.isNotEmpty) _Fact(Icons.laptop_rounded, machine),
            ],
          ),
        ],
      ),
    );
  }
}

class _Fact extends StatelessWidget {
  const _Fact(this.icon, this.text, {this.mono = false});

  final IconData icon;
  final String text;
  final bool mono;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 16, color: c.inkFaint),
        const SizedBox(width: 6),
        Flexible(
          child: Text(
            text,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: mono
                ? monoStyle(context, size: 13)
                : TextStyle(fontSize: 13, color: c.inkMuted),
          ),
        ),
      ],
    );
  }
}

class SessionFacts extends StatelessWidget {
  const SessionFacts({super.key, required this.session});

  final MonitorSession session;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final asked = session.rail == RailState.asked && session.waitingFor != null;
    final tool = session.tool == null
        ? null
        : [session.tool!, ?session.toolSummary].join(' · ');
    final pct = session.contextPct;
    final meta = [
      ?session.model,
      if (session.totalTokens != null)
        l.vTokens(_compact(session.totalTokens!)),
    ].join(' · ');
    if (!asked && tool == null && pct == null && meta.isEmpty) {
      return const SizedBox.shrink();
    }
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: OrreryCard(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (asked) ...[
              Text(
                l.detailWaiting.toUpperCase(),
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.4,
                  color: c.waiting,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                session.waitingFor!,
                style: monoStyle(context, size: 13, color: c.ink),
              ),
              const SizedBox(height: 10),
            ],
            if (tool != null) ...[
              Text(
                l.detailTool.toUpperCase(),
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.4,
                  color: c.inkFaint,
                ),
              ),
              const SizedBox(height: 2),
              Text(tool, style: monoStyle(context, size: 13, color: c.ink)),
              const SizedBox(height: 10),
            ],
            if (pct != null) _ContextBar(percent: pct),
            if (meta.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 6),
                child: Text(meta, style: monoStyle(context)),
              ),
          ],
        ),
      ),
    );
  }
}

class _ContextBar extends StatelessWidget {
  const _ContextBar({required this.percent});

  final int percent;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final color = percent >= 85
        ? c.error
        : percent >= 65
        ? c.waiting
        : c.working;
    return Row(
      children: [
        Expanded(
          child: ClipRRect(
            borderRadius: BorderRadius.circular(3),
            child: TweenAnimationBuilder<double>(
              tween: Tween(end: percent / 100),
              duration: motion(context),
              curve: context.tokens.out,
              builder: (context, v, _) => LinearProgressIndicator(
                value: v,
                minHeight: 5,
                color: color,
                backgroundColor: c.raised,
              ),
            ),
          ),
        ),
        const SizedBox(width: 8),
        Text(context.l10n.vContextPct(percent), style: monoStyle(context)),
      ],
    );
  }
}

class SessionSubagents extends StatelessWidget {
  const SessionSubagents({super.key, required this.session});

  final MonitorSession session;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    if (session.subagents.isEmpty) {
      return Padding(
        padding: const EdgeInsets.fromLTRB(4, 4, 4, 0),
        child: Text(
          context.l10n.detailNoSubagents,
          style: TextStyle(color: c.inkMuted),
        ),
      );
    }
    return OrreryCard(
      padding: EdgeInsets.zero,
      child: AnimatedItems<MonitorAgent>(
        items: session.subagents,
        keyOf: (a) => a.id,
        gap: 0,
        builder: (context, a) {
          final detail = [
            ?a.tool,
            ?a.toolSummary,
            if (a.contextPct != null) context.l10n.vContextPct(a.contextPct!),
            if (a.totalTokens != null)
              context.l10n.vTokens(_compact(a.totalTokens!)),
          ].join(' · ');
          return DecoratedBox(
            decoration: BoxDecoration(
              border: a == session.subagents.first
                  ? null
                  : Border(top: BorderSide(color: c.line)),
            ),
            child: Padding(
              padding: EdgeInsets.fromLTRB(
                14 + (a.depth - 1) * 14.0,
                10,
                14,
                10,
              ),
              child: Row(
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          a.name ?? a.id,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            color: c.ink,
                          ),
                        ),
                        if (detail.isNotEmpty)
                          Text(
                            detail,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: monoStyle(context),
                          ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  RailChip(state: a.rail),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}
