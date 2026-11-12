import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../model/workspace_model.dart' show baseName;
import '../widgets/common.dart';
import 'async_panel.dart';
import 'run_models.dart';
import 'run_widgets.dart';
import 'viewer_actions.dart';
import 'viewer_api.dart';
import 'viewer_models.dart';

const int _maxLines = 200;

class RunPage extends ConsumerStatefulWidget {
  const RunPage({super.key, required this.machineId, required this.root});

  final String machineId;
  final String root;

  @override
  ConsumerState<RunPage> createState() => _RunPageState();
}

class _RunPageState extends ConsumerState<RunPage> {
  List<RunConfigInfo> _configs = const [];
  List<RunDeviceInfo> _devices = const [];
  final Map<String, RunSessionInfo> _sessions = {};
  final Map<String, List<RunLine>> _logs = {};
  String? _configId;
  String? _deviceId;
  bool _loading = true;
  Object? _error;
  bool _starting = false;
  StreamSubscription<JsonMap>? _sub;
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));

  @override
  void initState() {
    super.initState();
    _sub = _api.events.listen(_onEvent);
    _load();
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  bool _mine(RunSessionInfo s) => s.cwd.isEmpty || samePath(s.cwd, widget.root);

  void _onEvent(JsonMap event) {
    if (event['t'] != 'run.event') return;
    final body = event['event'];
    if (body is! Map || !mounted) return;
    switch (body['type']) {
      case 'session':
        final session = RunSessionInfo.parse(body['session']);
        if (session != null && _mine(session)) {
          setState(() => _sessions[session.id] = session);
        }
      case 'log':
        final id = body['sessionId'];
        if (id is! String) return;
        final added = [
          for (final m in maps(body['lines']))
            RunLine(
              str(m['text']) ?? '',
              str(m['level']),
              str(m['stream']) ?? 'app',
            ),
        ];
        setState(() {
          final list = [...?_logs[id], ...added];
          _logs[id] = list.length > _maxLines
              ? list.sublist(list.length - _maxLines)
              : list;
        });
    }
  }

  Future<void> _load() async {
    setState(() => _loading = true);
    try {
      final configs = await _api.call('run.configs', {'root': widget.root});
      final devices = await _api.call('run.devices');
      final sessions = await _api.call('run.sessions');
      if (!mounted) return;
      setState(() {
        _configs = RunConfigInfo.parseAll(configs.data);
        _devices = RunDeviceInfo.parseAll(devices.data);
        for (final s in RunSessionInfo.parseAll(sessions.data)) {
          if (_mine(s)) _sessions[s.id] = s;
        }
        _configId ??= _configs.isEmpty ? null : _configs.first.id;
        _deviceId ??= _devices.where((d) => d.ready).firstOrNull?.id;
        _error = null;
        _loading = false;
      });
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  Future<void> _start() async {
    final id = _configId;
    if (id == null) return;
    setState(() => _starting = true);
    final reply = await viewerAction(
      context,
      ref,
      widget.machineId,
      'run.start',
      {'root': widget.root, 'configId': id, 'deviceId': _deviceId},
      timeout: slowCommand,
    );
    if (!mounted) return;
    setState(() => _starting = false);
    final sid = reply?.map['sessionId'];
    if (sid is String && !_sessions.containsKey(sid)) {
      _load();
    }
  }

  Future<void> _stop(RunSessionInfo s) => viewerAction(
    context,
    ref,
    widget.machineId,
    'run.stop',
    {'sessionId': s.id},
  );

  Future<void> _reload(RunSessionInfo s, bool hot) => viewerAction(
    context,
    ref,
    widget.machineId,
    'run.reload',
    {'sessionId': s.id, 'hot': hot},
  );

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final sessions = _sessions.values.toList()
      ..sort((a, b) => (b.live ? 1 : 0).compareTo(a.live ? 1 : 0));
    return Scaffold(
      appBar: AppBar(title: Text('${l.vRun} · ${baseName(widget.root)}')),
      body: _error != null && _configs.isEmpty
          ? ErrorState(message: viewerErrorText(l, _error!), onRetry: _load)
          : _loading && _configs.isEmpty
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 32),
              children: [
                if (_configs.isEmpty)
                  EmptyState(
                    icon: Icons.play_disabled_rounded,
                    text: l.vRunNoConfigs,
                  )
                else
                  Card(
                    elevation: 0,
                    color: scheme.surface,
                    child: Padding(
                      padding: const EdgeInsets.all(14),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          DropdownButtonFormField<String>(
                            key: const ValueKey('run-config'),
                            initialValue: _configId,
                            isExpanded: true,
                            decoration: InputDecoration(
                              labelText: l.vRunConfig,
                            ),
                            items: [
                              for (final c in _configs)
                                DropdownMenuItem(
                                  value: c.id,
                                  child: Text(
                                    '${c.name} (${c.type})',
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                            ],
                            onChanged: (v) => setState(() => _configId = v),
                          ),
                          const SizedBox(height: 12),
                          DropdownButtonFormField<String>(
                            key: const ValueKey('run-device'),
                            initialValue: _devices.any((d) => d.id == _deviceId)
                                ? _deviceId
                                : null,
                            isExpanded: true,
                            decoration: InputDecoration(
                              labelText: l.vRunDevice,
                            ),
                            items: [
                              for (final d in _devices)
                                DropdownMenuItem(
                                  value: d.id,
                                  enabled: d.ready,
                                  child: Text(
                                    d.name,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                            ],
                            onChanged: (v) => setState(() => _deviceId = v),
                          ),
                          const SizedBox(height: 14),
                          FilledButton.icon(
                            key: const ValueKey('run-start'),
                            onPressed: _starting || _configId == null
                                ? null
                                : _start,
                            icon: const Icon(Icons.play_arrow_rounded),
                            label: Text(l.vRunStart),
                          ),
                        ],
                      ),
                    ),
                  ),
                const SizedBox(height: 8),
                for (final s in sessions)
                  RunSessionCard(
                    session: s,
                    lines: _logs[s.id] ?? const [],
                    onStop: () => _stop(s),
                    onReload: (hot) => _reload(s, hot),
                  ),
              ],
            ),
    );
  }
}
