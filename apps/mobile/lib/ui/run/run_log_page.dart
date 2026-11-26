import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../theme/orrery_theme.dart';
import '../viewers/async_panel.dart';
import '../viewers/viewer_api.dart';
import '../viewers/viewer_models.dart' show monoFallback;
import '../widgets/common.dart';
import 'crash_send.dart';
import 'run_log_model.dart';

const Duration runLogDebounce = Duration(milliseconds: 250);
const int _pullLoops = 12;

class RunLogPage extends ConsumerStatefulWidget {
  const RunLogPage({
    super.key,
    required this.machineId,
    required this.sessionId,
    required this.title,
    required this.cwd,
  });

  final String machineId;
  final String sessionId;
  final String title;
  final String cwd;

  @override
  ConsumerState<RunLogPage> createState() => _RunLogPageState();
}

class _RunLogPageState extends ConsumerState<RunLogPage> {
  final RunLogBuffer _buffer = RunLogBuffer();
  final ScrollController _scroll = ScrollController();
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));
  StreamSubscription<JsonMap>? _sub;
  Timer? _timer;
  Object? _error;
  bool _loading = true;
  bool _pulling = false;
  bool _again = false;
  bool _watching = false;
  String? _stream;

  String get _watchRoot => widget.cwd.isEmpty ? '-' : widget.cwd;

  @override
  void initState() {
    super.initState();
    _api.watch('run', _watchRoot);
    _watching = true;
    _sub = _api.events.listen(_onEvent);
    _pull(first: true);
  }

  @override
  void dispose() {
    _timer?.cancel();
    _sub?.cancel();
    _scroll.dispose();
    if (_watching) _api.unwatch('run', _watchRoot);
    super.dispose();
  }

  void _onEvent(JsonMap event) {
    final kind = event['t'];
    if (kind == 'hello') {
      _schedule();
      return;
    }
    if (kind != 'run.event') return;
    final body = event['event'];
    if (body is! Map || body['type'] != 'log') return;
    final id = body['sessionId'];
    if (id == widget.sessionId) _schedule();
  }

  void _schedule() {
    _timer?.cancel();
    _timer = Timer(runLogDebounce, () => _pull());
  }

  Future<void> _pull({bool first = false}) async {
    if (_pulling) {
      _again = true;
      return;
    }
    _pulling = true;
    try {
      var fresh = first || _buffer.last == 0;
      for (var i = 0; i < _pullLoops; i++) {
        final reply = await _api.call('run.log', {
          'session': widget.sessionId,
          if (!fresh) 'since': _buffer.last,
        });
        if (!mounted) return;
        final chunk = RunLogChunk.parse(reply.data);
        final follow = _nearEnd();
        setState(() {
          _buffer.add(chunk, first: fresh);
          _error = null;
          _loading = false;
        });
        if (follow) _toEnd();
        fresh = false;
        if (!chunk.more) break;
      }
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error;
        _loading = false;
      });
    } finally {
      _pulling = false;
    }
    if (_again && mounted) {
      _again = false;
      unawaited(_pull());
    }
  }

  bool _nearEnd() {
    if (!_scroll.hasClients) return true;
    final p = _scroll.position;
    return p.maxScrollExtent - p.pixels < 120;
  }

  void _toEnd() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scroll.hasClients) {
        _scroll.jumpTo(_scroll.position.maxScrollExtent);
      }
    });
  }

  Future<void> _sendCrash() async {
    final crash = crashExcerpt(_buffer.lines);
    if (crash == null) return;
    await sendCrashToAgent(
      context,
      ref,
      machineId: widget.machineId,
      cwd: widget.cwd,
      crash: crash,
    );
  }

  List<RunLogLine> get _visible => _stream == null
      ? _buffer.lines
      : [
          for (final l in _buffer.lines)
            if (l.stream == _stream) l,
        ];

  Widget _body(BuildContext context) {
    final l = context.l10n;
    if (_error != null && _buffer.lines.isEmpty) {
      final unsupported =
          _error is ViewerFailure &&
          (_error! as ViewerFailure).code == 'denied';
      return ErrorState(
        message: unsupported
            ? l.runLogUnsupported
            : viewerErrorText(l, _error!),
        onRetry: () => _pull(first: true),
      );
    }
    if (_loading) return const Center(child: CircularProgressIndicator());
    final lines = _visible;
    if (lines.isEmpty) {
      return EmptyState(icon: Icons.subject_rounded, text: l.runLogEmpty);
    }
    return ListView.builder(
      key: const ValueKey('run-log-list'),
      controller: _scroll,
      padding: const EdgeInsets.fromLTRB(12, 4, 12, 12),
      itemCount: lines.length + (_buffer.gap ? 1 : 0),
      itemBuilder: (context, i) {
        if (_buffer.gap && i == 0) {
          return Padding(
            padding: const EdgeInsets.only(bottom: 6),
            child: Text(
              l.runLogGap,
              style: TextStyle(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
                fontSize: 12,
              ),
            ),
          );
        }
        return RunLogRow(line: lines[i - (_buffer.gap ? 1 : 0)]);
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final hasCrash = _buffer.hasCrash;
    return Scaffold(
      appBar: AppBar(title: Text('${l.runLogTitle}, ${widget.title}')),
      body: Column(
        children: [
          SizedBox(
            height: 48,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 12),
              children: [
                for (final entry in [
                  (null, l.runLogAll),
                  ('build', 'build'),
                  ('app', 'app'),
                  ('logcat', 'logcat'),
                ])
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: ChoiceChip(
                      key: ValueKey('run-log-stream-${entry.$1 ?? 'all'}'),
                      label: Text(entry.$2),
                      selected: _stream == entry.$1,
                      onSelected: (_) => setState(() => _stream = entry.$1),
                    ),
                  ),
              ],
            ),
          ),
          Expanded(child: _body(context)),
          if (hasCrash)
            SafeArea(
              top: false,
              child: Padding(
                padding: const EdgeInsets.fromLTRB(12, 4, 12, 8),
                child: SizedBox(
                  width: double.infinity,
                  child: FilledButton.icon(
                    key: const ValueKey('run-log-send-crash'),
                    onPressed: _sendCrash,
                    icon: const Icon(Icons.bug_report_outlined),
                    label: Text(l.runLogSendCrash),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class RunLogRow extends StatelessWidget {
  const RunLogRow({super.key, required this.line});

  final RunLogLine line;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final colors = context.statusColors;
    final color = line.crash
        ? colors.failed
        : switch (line.level) {
            'E' || 'F' => colors.failed,
            'W' => colors.warning,
            _ => scheme.onSurface,
          };
    final text = Text(
      line.text,
      style: TextStyle(
        fontFamily: 'monospace',
        fontFamilyFallback: monoFallback,
        fontSize: 11.5,
        height: 1.3,
        color: color,
        fontWeight: line.crash ? FontWeight.w600 : null,
      ),
    );
    if (!line.crash) return text;
    return Container(
      key: ValueKey('crash-${line.n}'),
      decoration: BoxDecoration(
        color: colors.failed.withValues(alpha: 0.10),
        border: Border(left: BorderSide(color: colors.failed, width: 3)),
      ),
      padding: const EdgeInsets.only(left: 6),
      child: text,
    );
  }
}
