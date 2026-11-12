import 'package:flutter/material.dart';

import '../theme/orrery_theme.dart';
import '../widgets/common.dart';
import 'run_models.dart';
import 'viewer_models.dart';

class RunSessionCard extends StatelessWidget {
  const RunSessionCard({
    super.key,
    required this.session,
    required this.lines,
    required this.onStop,
    required this.onReload,
  });

  final RunSessionInfo session;
  final List<RunLine> lines;
  final VoidCallback onStop;
  final ValueChanged<bool> onReload;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    final color = session.live
        ? (session.state == 'running' ? colors.done : colors.working)
        : (session.state == 'failed' ? colors.failed : colors.idle);
    return Card(
      elevation: 0,
      color: scheme.surface,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    session.configName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 3,
                  ),
                  decoration: BoxDecoration(
                    color: color.withValues(alpha: 0.14),
                    borderRadius: BorderRadius.circular(20),
                  ),
                  child: Text(
                    session.state,
                    key: const ValueKey('run-state'),
                    style: TextStyle(
                      color: color,
                      fontWeight: FontWeight.w700,
                      fontSize: 12,
                    ),
                  ),
                ),
              ],
            ),
            if (session.deviceName != null || session.progress != null)
              Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Text(
                  [?session.deviceName, ?session.progress].join(' · '),
                  style: TextStyle(
                    color: scheme.onSurfaceVariant,
                    fontSize: 13,
                  ),
                ),
              ),
            if (session.live)
              Padding(
                padding: const EdgeInsets.only(top: 10),
                child: Wrap(
                  spacing: 8,
                  children: [
                    OutlinedButton(
                      key: const ValueKey('run-hot'),
                      onPressed: () => onReload(true),
                      child: Text(l.vRunHotReload),
                    ),
                    OutlinedButton(
                      onPressed: () => onReload(false),
                      child: Text(l.vRunRestart),
                    ),
                    OutlinedButton(
                      key: const ValueKey('run-stop'),
                      style: OutlinedButton.styleFrom(
                        foregroundColor: scheme.error,
                      ),
                      onPressed: onStop,
                      child: Text(l.vRunStop),
                    ),
                  ],
                ),
              ),
            const SizedBox(height: 10),
            RunLogView(lines: lines),
          ],
        ),
      ),
    );
  }
}

class RunLogView extends StatefulWidget {
  const RunLogView({super.key, required this.lines});

  final List<RunLine> lines;

  @override
  State<RunLogView> createState() => _LogViewState();
}

class _LogViewState extends State<RunLogView> {
  final ScrollController _scroll = ScrollController();

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(RunLogView old) {
    super.didUpdateWidget(old);
    if (old.lines.length == widget.lines.length) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scroll.hasClients) return;
      final position = _scroll.position;
      if (position.maxScrollExtent - position.pixels < 120) {
        _scroll.jumpTo(position.maxScrollExtent);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    return Container(
      height: 220,
      width: double.infinity,
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(10),
      ),
      child: widget.lines.isEmpty
          ? Center(
              child: Text(
                context.l10n.vRunNoLog,
                style: TextStyle(color: scheme.onSurfaceVariant),
              ),
            )
          : ListView.builder(
              key: const ValueKey('run-log'),
              controller: _scroll,
              padding: const EdgeInsets.all(8),
              itemCount: widget.lines.length,
              itemBuilder: (context, i) {
                final line = widget.lines[i];
                final color = switch (line.level) {
                  'E' || 'F' => colors.failed,
                  'W' => colors.warning,
                  _ => scheme.onSurface,
                };
                return Text(
                  line.text,
                  style: TextStyle(
                    fontFamily: 'monospace',
                    fontFamilyFallback: monoFallback,
                    fontSize: 11.5,
                    height: 1.3,
                    color: color,
                  ),
                );
              },
            ),
    );
  }
}
