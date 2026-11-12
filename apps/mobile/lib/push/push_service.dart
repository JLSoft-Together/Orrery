import 'dart:async';
import 'dart:convert';

import '../app/link_source.dart';
import '../app/pane_link.dart';
import '../remote/secure_kv.dart';
import 'push_config.dart';
import 'push_gateway.dart';

abstract class PushSession {
  String get machineId;

  Future<void> send(String type, Map<String, Object?> fields);
}

class PushConfigUpdate {
  const PushConfigUpdate(this.machineId, this.config);

  final String machineId;
  final PushConfig? config;
}

abstract class PushHost {
  Stream<PushSession> get readySessions;

  List<PushSession> get readyNow;

  Stream<PushConfigUpdate> get configs;
}

enum PushResult { on, off, denied, unavailable }

const String pushEnabledKey = 'push.enabled';
const String pushConfigsKey = 'push.configs';

Uri? pushLink(Map<String, String> data) {
  final machine = data['m'];
  final pane = data['p'];
  if (machine == null || pane == null) return null;
  if (!RegExp(r'^[A-Za-z0-9_-]{1,128}$').hasMatch(machine)) return null;
  if (!RegExp(r'^[A-Za-z0-9_.:-]{1,128}$').hasMatch(pane)) return null;
  return Uri(
    scheme: appLinkScheme,
    host: 'm',
    pathSegments: [machine, 'p', pane],
  );
}

class PushService implements LinkSource {
  PushService({
    required this.gateway,
    required this.kv,
    required this.host,
    required this.platform,
    required this.localeCode,
  });

  final PushGateway gateway;
  final SecureKv kv;
  final PushHost host;
  final String platform;
  final String Function() localeCode;

  final StreamController<void> _changes = StreamController.broadcast();
  final StreamController<Uri> _taps = StreamController.broadcast();
  final List<StreamSubscription<Object?>> _subs = [];
  final Map<String, PushConfig?> _configs = {};
  final Map<String, PushSession> _sessions = {};
  final Map<String, String> _tokens = {};
  bool _enabled = false;
  Uri? _initial;

  bool get enabled => _enabled;

  bool get configured => _configs.values.any((c) => c != null);

  Stream<void> get changes => _changes.stream;

  @override
  Stream<Uri> get links => Stream.multi((controller) {
    final first = _initial;
    _initial = null;
    if (first != null) controller.add(first);
    final sub = _taps.stream.listen(controller.add);
    controller.onCancel = sub.cancel;
  });

  Future<void> init() async {
    _enabled = (await kv.read(pushEnabledKey)) == 'on';
    await _loadConfigs();
    _subs
      ..add(gateway.tokenRefresh.listen(_onRefresh, onError: (Object _) {}))
      ..add(gateway.opened.listen(_onOpened, onError: (Object _) {}))
      ..add(host.readySessions.listen(_onSession))
      ..add(host.configs.listen(_onConfig));
    for (final session in host.readyNow) {
      _sessions[session.machineId] = session;
    }
    if (_enabled) {
      final initial = await gateway.initialOpen(_distinct());
      if (initial != null) _initial = pushLink(initial);
      await _syncAll();
    }
  }

  Future<void> dispose() async {
    for (final sub in _subs) {
      await sub.cancel();
    }
    _subs.clear();
    await _changes.close();
    await _taps.close();
  }

  Future<void> _loadConfigs() async {
    final raw = await kv.read(pushConfigsKey);
    if (raw == null) return;
    try {
      final map = jsonDecode(raw);
      if (map is! Map<String, dynamic>) return;
      for (final entry in map.entries) {
        final value = entry.value;
        _configs[entry.key] = value is Map<String, dynamic>
            ? PushConfig.tryParse(value)
            : null;
      }
    } on FormatException {
      return;
    }
  }

  Future<void> _saveConfigs() => kv.write(
    pushConfigsKey,
    jsonEncode({for (final e in _configs.entries) e.key: e.value?.toJson()}),
  );

  List<PushConfig> _distinct() {
    final seen = <String, PushConfig>{};
    for (final config in _configs.values) {
      if (config != null) seen[config.projectId] = config;
    }
    return seen.values.toList();
  }

  void _notify() {
    if (!_changes.isClosed) _changes.add(null);
  }

  void _onOpened(Map<String, String> data) {
    final uri = pushLink(data);
    if (uri != null && !_taps.isClosed) _taps.add(uri);
  }

  void _onRefresh(PushTokenChange change) {
    _tokens[change.projectId] = change.token;
    if (!_enabled) return;
    for (final entry in _configs.entries) {
      if (entry.value?.projectId == change.projectId) {
        unawaited(_register(entry.key));
      }
    }
  }

  void _onSession(PushSession session) {
    _sessions[session.machineId] = session;
    unawaited(_register(session.machineId));
  }

  void _onConfig(PushConfigUpdate update) {
    if (_configs.containsKey(update.machineId) &&
        _configs[update.machineId] == update.config) {
      unawaited(_register(update.machineId));
      return;
    }
    _configs[update.machineId] = update.config;
    unawaited(_saveConfigs());
    _notify();
    unawaited(_register(update.machineId));
  }

  Future<void> _send(PushSession session, String type, Map<String, Object?> f) async {
    try {
      await session.send(type, f);
    } on Object {
      return;
    }
  }

  Future<String?> _tokenFor(PushConfig config) async {
    final known = _tokens[config.projectId];
    if (known != null) return known;
    final fresh = await gateway.token(config);
    if (fresh != null) _tokens[config.projectId] = fresh;
    return fresh;
  }

  Future<void> _register(String machineId) async {
    final config = _configs[machineId];
    final session = _sessions[machineId];
    if (!_enabled || config == null || session == null) return;
    final token = await _tokenFor(config);
    if (token == null || !_enabled) return;
    await _send(session, 'push.register', {
      'token': token,
      'platform': platform,
      'locale': localeCode() == 'vi' ? 'vi' : 'en',
    });
  }

  Future<void> _syncAll() async {
    for (final machineId in _sessions.keys.toList()) {
      await _register(machineId);
    }
  }

  Future<PushResult> setEnabled(bool on) async {
    if (on == _enabled) return on ? PushResult.on : PushResult.off;
    if (!on) {
      _enabled = false;
      await kv.write(pushEnabledKey, 'off');
      _notify();
      for (final session in _sessions.values) {
        await _send(session, 'push.unregister', const {});
      }
      for (final config in _distinct()) {
        await gateway.deleteToken(config);
      }
      _tokens.clear();
      return PushResult.off;
    }
    final configs = _distinct();
    if (configs.isEmpty) return PushResult.unavailable;
    final permission = await gateway.requestPermission(configs.first);
    if (permission != PushPermission.granted) return PushResult.denied;
    var any = false;
    for (final config in configs) {
      if (await _tokenFor(config) != null) any = true;
    }
    if (!any) return PushResult.unavailable;
    _enabled = true;
    await kv.write(pushEnabledKey, 'on');
    _notify();
    await _syncAll();
    return PushResult.on;
  }
}
